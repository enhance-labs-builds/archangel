//! Scenarios from the language-server spec, over an in-memory connection. The server applies every
//! change before it answers a later request, so a test learns that its changes are handled from the
//! answer to a request it sends after them, never from a duration. Only a change that the server's
//! own watcher reports has no such answer. Every wait ends at `HUNG`, so a hung server fails its
//! test with a message instead of stopping the run.
#![cfg(test)]

#[path = "support/editor.rs"]
mod support;

use std::sync::{PoisonError, RwLock, RwLockReadGuard};

use archangel_testkit::{FixtureRepo, Gate};
use lsp_server::{Connection, Message, Notification, Request, RequestId};
use serde_json::{Value, json};
use support::{Editor, HUNG, file_uri};

/// Fail points are global to the process, and the server runs in it, so a test that configures one
/// holds this lock alone and every other test holds it shared.
static FAIL_POINTS: RwLock<()> = RwLock::new(());

fn no_faults() -> RwLockReadGuard<'static, ()> {
    FAIL_POINTS.read().unwrap_or_else(PoisonError::into_inner)
}

/// The fail points of one test, cleared before the other tests run again.
#[cfg(feature = "failpoints")]
struct Faults {
    _scenario: fail::FailScenario<'static>,
    _alone: std::sync::RwLockWriteGuard<'static, ()>,
}

#[cfg(feature = "failpoints")]
fn faults() -> Faults {
    let alone = FAIL_POINTS.write().unwrap_or_else(PoisonError::into_inner);
    Faults {
        _scenario: fail::FailScenario::setup(),
        _alone: alone,
    }
}

fn codes(diagnostics: &[Value]) -> Vec<String> {
    diagnostics
        .iter()
        .map(|d| d["code"].as_str().unwrap_or_default().to_owned())
        .collect()
}

/// The difference of one work counter between two readings.
fn more(after: &Value, before: &Value, counter: &str) -> u64 {
    after[counter].as_u64().unwrap_or(0) - before[counter].as_u64().unwrap_or(0)
}

fn config() -> Value {
    json!({
        "elements": { "app": "src/app/**", "entity": "src/entities/{entity}/**", "shared": "src/**" },
        "rules": [
            { "kind": "layers", "name": "layers", "order": ["app", "entity", "shared"] },
            { "kind": "entry", "name": "entry", "element": "entity", "files": ["index.ts"] },
        ],
    })
}

/// A repository whose `entity` element matches no file.
fn repo() -> FixtureRepo {
    FixtureRepo::new()
        .file("src/app/main.ts", "")
        .file("src/s.ts", "export const s = 1;\n")
        .json_config(&config())
}

const VIOLATION: &str = "import './app/main';\n";

#[test]
fn server_publishes_full_check_diagnostics_after_initialize() {
    let _faults = no_faults();
    let repo = repo()
        .file("src/a.ts", VIOLATION)
        .file("src/b.ts", VIOLATION);
    let mut editor = Editor::start(&repo, true);
    for path in ["src/a.ts", "src/b.ts"] {
        let found = editor.published(path, |d| !d.is_empty());
        assert_eq!(
            codes(found["diagnostics"].as_array().unwrap()),
            ["layers"],
            "{path} has its violation"
        );
    }
}

#[test]
fn unsaved_text_gets_diagnostics_for_its_version() {
    let _faults = no_faults();
    let repo = repo();
    let mut editor = Editor::start(&repo, true);
    editor.open("src/s.ts", 1, "export const s = 1;\n");
    editor.change("src/s.ts", 7, VIOLATION);
    let found = editor.published("src/s.ts", |d| !d.is_empty());
    assert_eq!(
        found["version"], 7,
        "the diagnostics are for version 7: {found}"
    );
}

#[test]
fn closing_an_unsaved_document_returns_to_the_text_on_disk() {
    let _faults = no_faults();
    let repo = repo();
    let mut editor = Editor::start(&repo, true);
    editor.open("src/s.ts", 1, VIOLATION);
    editor.published("src/s.ts", |d| !d.is_empty());
    editor.notify(
        "textDocument/didClose",
        &json!({ "textDocument": { "uri": editor.uri("src/s.ts") } }),
    );
    editor.published("src/s.ts", <[Value]>::is_empty);
}

#[test]
fn queued_edits_publish_only_the_newest_version() {
    let _faults = no_faults();
    let repo = repo();
    // Every version waits before the server's first turn, as edits typed during a check do.
    let mut editor = Editor::start_after(&repo, true, |editor| {
        editor.open("src/s.ts", 1, "export const s = 1;\n");
        editor.change("src/s.ts", 3, VIOLATION);
        editor.change("src/s.ts", 4, "export const s = 2;\n");
        editor.change("src/s.ts", 5, &format!("{VIOLATION}{VIOLATION}"));
    });
    editor.work();
    let published: Vec<(Value, usize)> = editor
        .waiting_for("src/s.ts")
        .iter()
        .map(|p| {
            let found = p["diagnostics"].as_array().map_or(0, Vec::len);
            (p["version"].clone(), found)
        })
        .collect();
    assert_eq!(
        published,
        [(json!(5), 2)],
        "(version, diagnostics) of each publish: only version 5, with its two violations"
    );
}

/// 2,000 files, and `src/a.ts` with 6 imports.
fn big() -> FixtureRepo {
    repo()
        .synthetic("src/lib", 1999, 3)
        .file("src/a.ts", &six(""))
}

fn six(extra: &str) -> String {
    ["f0", "f1", "f2", "f3", "f4", "f5"]
        .map(|f| format!("import './lib/{f}';\n"))
        .concat()
        + extra
}

#[test]
fn edit_publishes_only_files_whose_diagnostics_changed() {
    let _faults = no_faults();
    let repo = big();
    let mut editor = Editor::start(&repo, true);
    let before = editor.work();
    editor.open("src/a.ts", 1, &six(""));
    let edited = six("import { s } from './app/main';\n").replace("import './lib/f5';\n", "");
    editor.change("src/a.ts", 2, &edited);
    editor.published("src/a.ts", |d| !d.is_empty());
    repo.write("src/a.ts", &edited);
    editor.files(&[("src/a.ts", 2)]);
    let after = editor.work();
    assert_eq!(
        (
            more(&after, &before, "filesParsed"),
            more(&after, &before, "resolutions")
        ),
        (1, 6),
        "(parsed, resolutions) for the edit and the save together"
    );
    let others: Vec<&Value> = editor
        .waiting
        .iter()
        .filter(|n| {
            n["method"] == "textDocument/publishDiagnostics"
                && n["params"]["uri"] != editor.uri("src/a.ts").as_str()
        })
        .collect();
    assert!(others.is_empty(), "no other file is published: {others:?}");
}

fn importer() -> FixtureRepo {
    repo()
        .file("src/a.ts", "import './b';\nimport './a.module.css';\n")
        .file("src/b.ts", "")
        .file("src/a.module.css", "")
}

#[test]
fn deleted_file_makes_its_importers_unresolved() {
    let _faults = no_faults();
    let repo = importer();
    let mut editor = Editor::start(&repo, true);
    let before = editor.work();
    repo.remove("src/b.ts");
    editor.files(&[("src/b.ts", 3)]);
    editor.published("src/a.ts", |d| codes(d) == ["import-unresolved"]);
    assert_eq!(
        more(&editor.work(), &before, "filesParsed"),
        0,
        "no file is parsed"
    );
}

/// A file the walk ignores is known only to the resolution that found it, and only a deletion, not
/// a change of its text, can make that resolution fail.
#[test]
fn deleted_ignored_file_makes_its_importer_unresolved() {
    let _faults = no_faults();
    let repo = repo()
        .file(".gitignore", "src/generated.ts\n")
        .file("src/a.ts", "import './generated';\n")
        .file("src/generated.ts", "");
    let check = archangel_engine::Session::open(repo.path(), None).expect("a session");
    let unresolved =
        |d: &&archangel_rules::Diagnostic| d.code == archangel_rules::Code::ImportUnresolved;
    assert_eq!(
        check.report().diagnostics.iter().filter(unresolved).count(),
        0,
        "the import resolves to the ignored file"
    );
    let mut editor = Editor::start(&repo, true);
    repo.remove("src/generated.ts");
    editor.files(&[("src/generated.ts", 3)]);
    editor.published("src/a.ts", |d| codes(d) == ["import-unresolved"]);
}

#[test]
fn deleted_css_file_makes_its_importer_unresolved() {
    let _faults = no_faults();
    let repo = importer();
    let mut editor = Editor::start(&repo, true);
    repo.remove("src/a.module.css");
    editor.files(&[("src/a.module.css", 3)]);
    editor.published("src/a.ts", |d| codes(d) == ["import-unresolved"]);
}

/// A manifest that no resolution read changes the set of manifests only when it goes or comes, so
/// only its deletion walks again.
#[test]
fn a_deleted_manifest_walks_again_and_a_changed_one_does_not() {
    let _faults = no_faults();
    let repo = repo().file("packages/kit/package.json", r#"{ "name": "kit" }"#);
    let mut editor = Editor::start(&repo, true);
    let before = editor.work();
    editor.files(&[("packages/kit/package.json", 2)]);
    let changed = editor.work();
    repo.remove("packages/kit/package.json");
    editor.files(&[("packages/kit/package.json", 3)]);
    let deleted = editor.work();
    assert_eq!(
        (
            more(&changed, &before, "filesFound"),
            more(&deleted, &changed, "filesFound") > 0
        ),
        (0, true),
        "(files found for the change, whether the deletion walked)"
    );
}

#[test]
fn server_watches_files_itself_when_the_editor_cannot() {
    let _faults = no_faults();
    let repo = repo().file("src/a.ts", "import './c';\n");
    let mut editor = Editor::start(&repo, false);
    editor.published("src/a.ts", |d| codes(d) == ["import-unresolved"]);
    assert!(
        editor.asked.is_empty(),
        "the server asks the editor to watch nothing: {:?}",
        editor.asked
    );
    repo.write("src/c.ts", "");
    // Only the server's own watcher sees the new file, so no request can follow it.
    let fixed = editor.about("src/a.ts", <[Value]>::is_empty);
    editor.until("textDocument/publishDiagnostics", fixed);
}

#[test]
fn server_asks_an_editor_that_can_watch_to_report_every_file() {
    let _faults = no_faults();
    let repo = repo();
    let mut editor = Editor::start(&repo, true);
    editor.work();
    let registration = json!({
        "method": "client/registerCapability",
        "params": { "registrations": [{
            "id": "archangel-files",
            "method": "workspace/didChangeWatchedFiles",
            "registerOptions": { "watchers": [{ "globPattern": "**/*" }] },
        }] },
    });
    assert_eq!(
        editor.asked,
        [registration],
        "one registration of a watch on every file"
    );
}

#[test]
fn created_css_file_resolves_its_importer() {
    let _faults = no_faults();
    let repo = repo().file("src/a.ts", "import './a.module.css';\n");
    let mut editor = Editor::start(&repo, true);
    editor.published("src/a.ts", |d| codes(d) == ["import-unresolved"]);
    repo.write("src/a.module.css", "");
    editor.files(&[("src/a.module.css", 1)]);
    editor.published("src/a.ts", <[Value]>::is_empty);
}

fn aliased(tsconfig: &str, alias_file: &str) -> FixtureRepo {
    repo()
        .file("tsconfig.json", tsconfig)
        .file(
            alias_file,
            r#"{ "compilerOptions": { "baseUrl": ".", "paths": { "@/*": ["src/one/*"] } } }"#,
        )
        .file("src/one/x.ts", "")
        .file("src/b.ts", "import '@/x';\n")
}

/// Points the alias at `src/two/`, where nothing is, and waits for the unresolved import.
fn move_alias(repo: &FixtureRepo, editor: &mut Editor, alias_file: &str) {
    repo.write(
        alias_file,
        r#"{ "compilerOptions": { "baseUrl": ".", "paths": { "@/*": ["src/two/*"] } } }"#,
    );
    editor.files(&[(alias_file, 2)]);
    editor.published("src/b.ts", |d| codes(d) == ["import-unresolved"]);
}

#[test]
fn tsconfig_change_publishes_the_new_resolutions() {
    let _faults = no_faults();
    let repo = aliased("{}", "tsconfig.json");
    let mut editor = Editor::start(&repo, true);
    let before = editor.work();
    move_alias(&repo, &mut editor, "tsconfig.json");
    let after = editor.work();
    assert_eq!(more(&after, &before, "filesParsed"), 0, "no file is parsed");
    assert_eq!(
        more(&after, &before, "resolutions"),
        1,
        "the one import is resolved once"
    );
}

#[test]
fn extended_tsconfig_change_publishes_the_new_resolutions() {
    let _faults = no_faults();
    let repo = aliased(
        r#"{ "extends": "./configs/base.json" }"#,
        "configs/base.json",
    );
    let mut editor = Editor::start(&repo, true);
    move_alias(&repo, &mut editor, "configs/base.json");
}

#[test]
fn referenced_tsconfig_change_publishes_the_new_resolutions() {
    let _faults = no_faults();
    let repo = aliased(
        r#"{ "files": [], "references": [{ "path": "./tsconfig.app.json" }] }"#,
        "tsconfig.app.json",
    );
    let repo = {
        let app = r#"{ "include": ["src"], "compilerOptions": { "baseUrl": ".", "paths": { "@/*": ["src/one/*"] } } }"#;
        repo.file("tsconfig.app.json", app)
    };
    let mut editor = Editor::start(&repo, true);
    repo.write(
        "tsconfig.app.json",
        r#"{ "include": ["src"], "compilerOptions": { "baseUrl": ".", "paths": { "@/*": ["src/two/*"] } } }"#,
    );
    editor.files(&[("tsconfig.app.json", 2)]);
    editor.published("src/b.ts", |d| codes(d) == ["import-unresolved"]);
}

#[test]
fn pnp_file_change_publishes_the_new_resolutions() {
    let _faults = no_faults();
    let repo = repo()
        .file(
            "package.json",
            r#"{ "name": "root", "dependencies": { "left-pad": "1.0.0" } }"#,
        )
        .file(
            ".pnp.cjs",
            &archangel_testkit::pnp_manifest(&[("left-pad", "./.yarn/cache/left-pad/")]),
        )
        .file(
            ".yarn/cache/left-pad/package.json",
            r#"{ "name": "left-pad", "main": "index.js" }"#,
        )
        .file(".yarn/cache/left-pad/index.js", "")
        .file(".gitignore", ".yarn/\n")
        .file("src/b.ts", "import 'left-pad';\n");
    let mut editor = Editor::start(&repo, true);
    editor.work();
    repo.write(".pnp.cjs", &archangel_testkit::pnp_manifest(&[]));
    editor.files(&[(".pnp.cjs", 2)]);
    editor.published("src/b.ts", |d| codes(d) == ["import-unresolved"]);
}

#[test]
fn baseline_change_publishes_the_files_with_no_violations() {
    let _faults = no_faults();
    let repo = repo()
        .file("src/x.ts", VIOLATION)
        .file("src/y.ts", VIOLATION)
        .file("src/z.ts", VIOLATION);
    let mut editor = Editor::start(&repo, true);
    for path in ["src/x.ts", "src/y.ts", "src/z.ts"] {
        editor.published(path, |d| !d.is_empty());
    }
    let before = editor.work();
    archangel_engine::Session::open(repo.path(), None)
        .expect("a session")
        .baseline_add()
        .expect("the baseline");
    editor.files(&[("archangel.baseline.json", 1)]);
    for path in ["src/x.ts", "src/y.ts", "src/z.ts"] {
        editor.published(path, <[Value]>::is_empty);
    }
    let after = editor.work();
    let counts =
        ["filesParsed", "resolutions", "ruleEvaluations"].map(|c| more(&after, &before, c));
    assert_eq!(counts, [0, 0, 0], "(parsed, resolutions, evaluations)");
}

#[test]
fn config_change_publishes_the_new_rules_violations() {
    let _faults = no_faults();
    let repo = repo().file("src/x.ts", "import './app/main';\n");
    let mut editor = Editor::start(&repo, true);
    editor.published("src/x.ts", |d| !d.is_empty());
    let before = editor.work();
    let mut changed = config();
    changed["rules"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "name": "f", "files": "shared", "mayNotImport": "app" }));
    repo.write("archangel.config.json", &changed.to_string());
    editor.files(&[("archangel.config.json", 2)]);
    editor.published("src/x.ts", |d| {
        codes(d).contains(&"may-not-import".to_owned())
    });
    let after = editor.work();
    assert_eq!(
        (
            more(&after, &before, "filesParsed"),
            more(&after, &before, "resolutions")
        ),
        (0, 0),
        "(parsed, resolutions)"
    );
}

#[test]
fn config_ignore_change_removes_ignored_files_diagnostics() {
    let _faults = no_faults();
    let repo = repo()
        .file("fixtures/a.ts", "import '../src/app/main';\n")
        .json_config(&json!({
            "elements": { "app": "src/app/**", "shared": "**" },
            "rules": [{ "kind": "layers", "order": ["app", "shared"] }],
        }));
    let mut editor = Editor::start(&repo, true);
    editor.published("fixtures/a.ts", |d| !d.is_empty());
    repo.write(
        "archangel.config.json",
        &json!({ "elements": { "app": "src/app/**", "shared": "**" }, "ignore": ["fixtures/"], "rules": [{ "kind": "layers", "order": ["app", "shared"] }] }).to_string(),
    );
    editor.files(&[("archangel.config.json", 2)]);
    editor.published("fixtures/a.ts", <[Value]>::is_empty);
}

#[test]
fn gitignore_change_removes_ignored_files_diagnostics() {
    let _faults = no_faults();
    let repo = repo().file("src/gen/a.ts", VIOLATION.replace("./", "../").as_str());
    let mut editor = Editor::start(&repo, true);
    editor.published("src/gen/a.ts", |d| !d.is_empty());
    let before = editor.work();
    repo.write(".gitignore", "src/gen/\n");
    editor.files(&[(".gitignore", 1)]);
    editor.published("src/gen/a.ts", <[Value]>::is_empty);
    assert_eq!(
        more(&editor.work(), &before, "filesParsed"),
        0,
        "no file is parsed"
    );
}

#[test]
fn invalid_config_replaces_the_diagnostics_with_its_own() {
    let _faults = no_faults();
    let repo = repo().file("src/x.ts", VIOLATION);
    let mut editor = Editor::start(&repo, true);
    editor.published("src/x.ts", |d| !d.is_empty());
    let mut broken = config();
    broken["rules"][0]["reasn"] = json!("typo");
    repo.write("archangel.config.json", &broken.to_string());
    editor.files(&[("archangel.config.json", 2)]);
    editor.published("archangel.config.json", |d| codes(d) == ["config-invalid"]);
    editor.published("src/x.ts", <[Value]>::is_empty);
}

#[cfg(feature = "failpoints")]
#[test]
fn server_says_when_it_polls() {
    let _faults = faults();
    fail::cfg("engine::watcher_has_no_watches", "return").unwrap();
    let repo = repo();
    let mut editor = Editor::start(&repo, false);
    let message = editor.notified("window/showMessage", |_| true);
    let text = message["message"].as_str().unwrap_or_default();
    assert!(
        text.contains("polls") && text.contains("watch limit"),
        "the message says why: {text}"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn lost_events_make_a_full_check() {
    let _faults = faults();
    fail::cfg("engine::watcher_reports_lost_events", "return").unwrap();
    let repo = repo();
    let files = archangel_engine::Session::open(repo.path(), None)
        .expect("a session")
        .work()
        .counts_json()["filesParsed"]
        .clone();
    let mut editor = Editor::start(&repo, false);
    // The watcher reports the loss as it starts, which no request can follow, so the test asks
    // for the counts until the second check is in them, a bounded number of times.
    let both = json!(2 * files.as_u64().unwrap_or(0));
    let parsed = (0..support::MOST_MESSAGES)
        .map(|_| editor.work()["filesParsed"].clone())
        .find(|parsed| *parsed == both);
    assert_eq!(
        parsed,
        Some(both),
        "the start's check, then a full check after the lost events, of {files} files each"
    );
}

/// An editor with `src/app/a.ts` open, which has `text` and an entry violation with a fix, and
/// that violation's diagnostic.
fn entry_violation(text: &str) -> (FixtureRepo, Editor, Value) {
    entry_violation_of("canvas", text)
}

/// The same, with the entity in a folder of this name.
fn entry_violation_of(entity: &str, text: &str) -> (FixtureRepo, Editor, Value) {
    let repo = repo()
        .file(
            &format!("src/entities/{entity}/index.ts"),
            "export { store } from './model/store';\n",
        )
        .file(
            &format!("src/entities/{entity}/model/store.ts"),
            "export const store = 1;\n",
        )
        .file("src/app/a.ts", text);
    let mut editor = Editor::start(&repo, true);
    editor.open("src/app/a.ts", 1, text);
    let published = editor.published("src/app/a.ts", |d| codes(d).contains(&"entry".to_owned()));
    let diagnostic = published["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["code"] == "entry")
        .unwrap()
        .clone();
    (repo, editor, diagnostic)
}

fn code_actions(editor: &mut Editor, range: &Value, diagnostic: &Value) -> Vec<Value> {
    let uri = editor.uri("src/app/a.ts");
    let actions = editor.request(
        "textDocument/codeAction",
        &json!({
            "textDocument": { "uri": uri },
            "range": range,
            "context": { "diagnostics": [diagnostic] },
        }),
    );
    actions.as_array().cloned().unwrap_or_default()
}

const DEEP_IMPORT: &str = "import { store } from '../entities/canvas/model/store';\n";

#[test]
fn entry_violation_offers_its_fix_as_a_quick_fix() {
    let _faults = no_faults();
    let text = DEEP_IMPORT;
    let (_repo, mut editor, diagnostic) = entry_violation(text);
    let actions = code_actions(&mut editor, &diagnostic["range"], &diagnostic);
    assert_eq!(actions.len(), 1, "one quick fix: {actions:?}");
    let action = &actions[0];
    assert_eq!(
        (&action["kind"], &action["isPreferred"]),
        (&json!("quickfix"), &json!(true)),
        "(kind, preferred): {action}"
    );
    assert_eq!(
        action["title"], "Import through \"../entities/canvas\"",
        "the title names the new specifier"
    );
    assert_eq!(
        action["diagnostics"],
        json!([diagnostic]),
        "the quick fix names the diagnostic it removes"
    );
    let edit = &action["edit"]["changes"][editor.uri("src/app/a.ts")][0];
    assert_eq!(
        edit["newText"], "../entities/canvas",
        "the specifier points at the entry file: {edit}"
    );
    let character = |end: &str| {
        usize::try_from(edit["range"][end]["character"].as_u64().unwrap_or(0)).unwrap_or(0)
    };
    assert_eq!(
        &text[character("start")..character("end")],
        "../entities/canvas/model/store",
        "the range covers the old specifier"
    );
}

#[test]
fn a_range_away_from_the_violation_offers_no_quick_fix() {
    let _faults = no_faults();
    let text = format!("{DEEP_IMPORT}export const a = store;\n");
    let (_repo, mut editor, diagnostic) = entry_violation(&text);
    let second_line = json!({
        "start": { "line": 1, "character": 0 },
        "end": { "line": 1, "character": 5 },
    });
    let actions = code_actions(&mut editor, &second_line, &diagnostic);
    assert!(
        actions.is_empty(),
        "the violation is on the first line: {actions:?}"
    );
}

#[test]
fn shutdown_then_exit_in_one_batch_stops_the_server_cleanly() {
    let _faults = no_faults();
    let repo = repo().file("src/a.ts", VIOLATION);
    let (server, editor) = Connection::memory();
    let uri = file_uri(&dunce::canonicalize(repo.path()).unwrap());
    // Everything is queued before the server reads any of it, as an editor that closes a window
    // sends shutdown and exit together.
    let messages = [
        Message::Request(Request::new(
            RequestId::from(1),
            "initialize".to_owned(),
            json!({ "processId": null, "rootUri": uri, "capabilities": {} }),
        )),
        Message::Notification(Notification::new("initialized".to_owned(), json!({}))),
        Message::Request(Request::new(
            RequestId::from(2),
            "shutdown".to_owned(),
            Value::Null,
        )),
        Message::Notification(Notification::new("exit".to_owned(), Value::Null)),
    ];
    for message in messages {
        editor.sender.send(message).unwrap();
    }
    let (stopped, outcome) = crossbeam_channel::bounded(1);
    std::thread::spawn(move || stopped.send(archangel_lsp::serve(&server)));
    let outcome = outcome
        .recv_timeout(HUNG)
        .unwrap_or_else(|_| panic!("the server hangs: it did not stop within {HUNG:?} of exit"));
    assert_eq!(
        outcome,
        Ok(()),
        "the server stops without an error, so the editor sees exit code 0"
    );
}

#[test]
fn initialize_offers_whole_document_sync_and_quick_fixes() {
    let _faults = no_faults();
    let repo = repo();
    let editor = Editor::start(&repo, true);
    let capabilities = &editor.initialized["capabilities"];
    assert_eq!(
        capabilities["textDocumentSync"], 1,
        "the editor sends the whole text of each change: {capabilities}"
    );
    assert_eq!(
        capabilities["codeActionProvider"], true,
        "the server offers code actions: {capabilities}"
    );
    assert_eq!(
        editor.initialized["serverInfo"]["name"], "archangel",
        "the server names itself"
    );
}

#[test]
fn ranges_count_utf16_code_units_as_the_protocol_does() {
    let _faults = no_faults();
    // The rocket is one character, two UTF-16 code units and four bytes; the accent is one
    // character, one code unit and two bytes. Bytes or characters put both ranges off.
    let text = "/* 🚀 */ import { store } from '../entities/canvás/model/store';\n";
    let (_repo, mut editor, diagnostic) = entry_violation_of("canvás", text);
    let units: Vec<u16> = text.encode_utf16().collect();
    let covered = |range: &Value| {
        let at =
            |end: &str| usize::try_from(range[end]["character"].as_u64().unwrap_or(0)).unwrap_or(0);
        String::from_utf16_lossy(units.get(at("start")..at("end")).unwrap_or_default())
    };
    assert_eq!(
        covered(&diagnostic["range"]),
        "'../entities/canvás/model/store'",
        "the diagnostic covers the specifier and its quotes: {diagnostic}"
    );
    let actions = code_actions(&mut editor, &diagnostic["range"], &diagnostic);
    let edit = &actions[0]["edit"]["changes"][editor.uri("src/app/a.ts")][0];
    assert_eq!(
        covered(&edit["range"]),
        "../entities/canvás/model/store",
        "the quick fix replaces the specifier between its quotes: {edit}"
    );
}

#[test]
fn diagnostic_carries_the_checks_code_message_help_link_and_range() {
    let _faults = no_faults();
    let mut linked = config();
    linked["rules"][0]["link"] = json!("https://example.com/layers");
    let repo = repo()
        .file("src/a.ts", &format!("export const a = 1;\n{VIOLATION}"))
        .json_config(&linked);
    let check = archangel_engine::Session::open(repo.path(), None).expect("a session");
    let expected = check
        .report()
        .diagnostics
        .iter()
        .find(|d| d.file.as_deref() == Some("src/a.ts"))
        .expect("the full check finds the violation")
        .clone();
    let help = expected.help.expect("a layers violation says what to do");
    let mut editor = Editor::start(&repo, true);
    let published = editor.published("src/a.ts", |d| !d.is_empty());
    let diagnostic = &published["diagnostics"][0];
    assert_eq!(
        diagnostic["message"],
        format!("{}\n{help}", expected.message),
        "the check's message, then its help"
    );
    assert_eq!(
        (
            &diagnostic["code"],
            &diagnostic["severity"],
            &diagnostic["source"],
            &diagnostic["codeDescription"]["href"],
        ),
        (
            &json!("layers"),
            &json!(1),
            &json!("archangel"),
            &json!("https://example.com/layers"),
        ),
        "(code, severity error, source, the rule's link): {diagnostic}"
    );
    assert_eq!(
        diagnostic["range"],
        json!({
            "start": { "line": 1, "character": 7 },
            "end": { "line": 1, "character": 19 },
        }),
        "the range covers the specifier and its quotes on the second line"
    );
}

#[test]
fn path_with_a_space_round_trips_through_its_uri() {
    let _faults = no_faults();
    let repo = repo().file("src/my file.ts", VIOLATION);
    let mut editor = Editor::start(&repo, true);
    editor.published("src/my file.ts", |d| codes(d) == ["layers"]);
    editor.open("src/my file.ts", 1, VIOLATION);
    editor.change("src/my file.ts", 2, "export const s = 1;\n");
    let cleared = editor.published("src/my file.ts", <[Value]>::is_empty);
    assert_eq!(
        cleared["version"], 2,
        "the server reads the encoded URI as the open document: {cleared}"
    );
}

#[test]
fn diagnostic_about_no_file_is_shown_on_the_config() {
    let _faults = no_faults();
    let repo = repo();
    let mut editor = Editor::start(&repo, true);
    editor.published("archangel.config.json", |d| codes(d) == ["element-empty"]);
    repo.write("src/entities/canvas/index.ts", "");
    editor.files(&[("src/entities/canvas/index.ts", 1)]);
    editor.published("archangel.config.json", <[Value]>::is_empty);
    repo.remove("src/entities/canvas/index.ts");
    editor.files(&[("src/entities/canvas/index.ts", 3)]);
    editor.published("archangel.config.json", |d| codes(d) == ["element-empty"]);
}

#[test]
fn workspace_that_cannot_be_checked_is_checked_once_its_config_is_fixed() {
    let _faults = no_faults();
    let mut broken = config();
    broken["rules"][0]["reasn"] = json!("typo");
    let repo = repo().file("src/x.ts", VIOLATION).json_config(&broken);
    let mut editor = Editor::start(&repo, true);
    let shown = editor.notified("window/showMessage", |_| true);
    let text = shown["message"].as_str().unwrap_or_default();
    assert!(
        shown["type"] == 1
            && text.starts_with("archangel cannot check this workspace: [config-invalid] "),
        "an error that names the config's problem: {shown}"
    );
    repo.write("archangel.config.json", &config().to_string());
    editor.files(&[("archangel.config.json", 2)]);
    editor.published("src/x.ts", |d| codes(d) == ["layers"]);
}

#[test]
fn request_with_no_change_before_it_costs_no_work() {
    let _faults = no_faults();
    let repo = repo().file("src/a.ts", VIOLATION);
    let mut editor = Editor::start(&repo, true);
    let before = editor.work();
    let after = editor.work();
    assert_eq!(after, before, "the second request checks nothing again");
}

// Plugins.

/// A repository whose config loads the fixture plugin, with a layers rule and `plugin_rule`.
fn plugin_repo(plugin_rule: &Value) -> FixtureRepo {
    FixtureRepo::new()
        .file("src/app/main.ts", "")
        .file("src/s.ts", "export const s = 1;\n")
        .file("src/x.ts", "")
        .plugin_config(&json!({
            "elements": { "app": "src/app/**", "shared": "src/**" },
            "rules": [
                { "kind": "layers", "name": "layers", "order": ["app", "shared"] },
                plugin_rule,
            ],
        }))
}

#[test]
fn built_in_diagnostics_do_not_wait_for_plugins() {
    let _faults = no_faults();
    let gate = Gate::closed();
    let repo = plugin_repo(
        &json!({ "name": "waits", "files": "src/**", "plugin": "fixture/waits",
        "options": { "port": gate.port(), "specifier": "./app/main" } }),
    );
    let mut editor = Editor::start(&repo, true);
    editor.open("src/s.ts", 1, "export const s = 1;\n");
    editor.change("src/s.ts", 2, VIOLATION);
    let built_in = editor.published("src/s.ts", |d| !d.is_empty());
    assert_eq!(
        (
            built_in["version"].clone(),
            codes(built_in["diagnostics"].as_array().unwrap())
        ),
        (json!(2), vec!["layers".to_owned()]),
        "the built-in violation for version 2, while the gate is closed"
    );
    let begun = editor.notified("$/progress", |p| p["value"]["kind"] == "begin");
    assert!(
        begun["value"]["title"]
            .as_str()
            .is_some_and(|title| title.contains("fixture")),
        "the progress message names the plugin: {begun}"
    );
    gate.open();
    let with_plugin = editor.about("src/s.ts", |d| codes(d).contains(&"plugin".to_owned()));
    let answered = editor.until("textDocument/publishDiagnostics", with_plugin);
    assert_eq!(
        answered["version"], 2,
        "the plugin's diagnostics are for the same version: {answered}"
    );
    let token = begun["token"].clone();
    editor.until("$/progress", |p| {
        p["token"] == token && p["value"]["kind"] == "end"
    });
}

#[test]
fn no_plugin_process_runs_without_plugin_conditions() {
    let _faults = no_faults();
    let config = "import { writeFileSync } from 'node:fs';\n\
        process.on('exit', () => writeFileSync(new URL('./node-exited', import.meta.url), ''));\n\
        export default {\n\
          elements: { app: 'src/app/**', shared: 'src/**' },\n\
          rules: [{ kind: 'layers', name: 'layers', order: ['app', 'shared'] }],\n\
        };\n";
    let repo = FixtureRepo::new()
        .file("src/app/main.ts", "")
        .file("src/s.ts", VIOLATION)
        .file("archangel.config.mjs", config);
    let mut editor = Editor::start(&repo, true);
    editor.published("src/s.ts", |d| codes(d) == ["layers"]);
    assert!(
        repo.join("node-exited").exists(),
        "the Node process that loaded the config exited before the diagnostics came"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn a_stopped_plugin_process_is_started_again() {
    let _faults = faults();
    let repo = plugin_repo(&json!({ "name": "no-s", "files": "src/**",
        "plugin": "fixture/report-imports", "options": { "specifier": "./s" } }));
    let mut editor = Editor::start(&repo, true);
    editor.work();
    fail::cfg("engine::plugin_dies", "1*return").unwrap();
    editor.open("src/x.ts", 1, "import './s';\n");
    let failed = editor.about("archangel.config.mjs", |d| codes(d) == ["plugin-failed"]);
    editor.until("textDocument/publishDiagnostics", failed);
    editor.change("src/x.ts", 2, "import { s } from './s';\n");
    let again = editor.about("src/x.ts", |d| codes(d) == ["plugin"]);
    let found = editor.until("textDocument/publishDiagnostics", again);
    assert_eq!(
        found["version"], 2,
        "the plugin's diagnostics come again, for the second edit: {found}"
    );
}

#[test]
fn what_a_plugin_printed_goes_to_the_editors_log() {
    let _faults = no_faults();
    let repo =
        plugin_repo(&json!({ "name": "exits", "files": "src/**", "plugin": "fixture/exits" }));
    let mut editor = Editor::start(&repo, true);
    let logged = editor.notified("window/logMessage", |_| true);
    assert_eq!(
        logged["message"], "[fixture] the fixture stops its process",
        "the line the plugin printed before its process stopped, with its name"
    );
}

/// A gated plugin rule over `src/**` that waits while a file imports `src/app/main.ts`.
fn gated(gate: &Gate) -> FixtureRepo {
    plugin_repo(
        &json!({ "name": "waits", "files": "src/**", "plugin": "fixture/waits",
        "options": { "port": gate.port(), "specifier": "./app/main" } }),
    )
}

#[test]
fn an_editor_without_progress_gets_no_progress_messages() {
    let _faults = no_faults();
    let gate = Gate::closed();
    let repo = gated(&gate);
    let capabilities =
        json!({ "workspace": { "didChangeWatchedFiles": { "dynamicRegistration": true } } });
    let mut editor = Editor::start_with(&repo, &capabilities, |_| {});
    editor.open("src/s.ts", 1, "export const s = 1;\n");
    editor.change("src/s.ts", 2, VIOLATION);
    editor.published("src/s.ts", |d| !d.is_empty());
    gate.open();
    let with_plugin = editor.about("src/s.ts", |d| codes(d).contains(&"plugin".to_owned()));
    editor.until("textDocument/publishDiagnostics", with_plugin);
    editor.work();
    let progress: Vec<&Value> = editor
        .asked
        .iter()
        .chain(&editor.waiting)
        .filter(|message| {
            message["method"]
                .as_str()
                .is_some_and(|method| method.contains("rogress"))
        })
        .collect();
    assert_eq!(
        progress,
        Vec::<&Value>::new(),
        "no progress request or notification"
    );
}

#[test]
fn each_plugin_run_has_a_progress_message_of_its_own() {
    let _faults = no_faults();
    let gate = Gate::closed();
    let repo = gated(&gate);
    let mut editor = Editor::start(&repo, true);
    editor.open("src/s.ts", 1, "export const s = 1;\n");
    editor.change("src/s.ts", 2, VIOLATION);
    let first = editor.notified("$/progress", |p| p["value"]["kind"] == "begin");
    gate.open();
    let token = first["token"].clone();
    editor.until("$/progress", |p| {
        p["token"] == token && p["value"]["kind"] == "end"
    });
    editor.change("src/s.ts", 3, &format!("export const s = 1;\n{VIOLATION}"));
    let second = editor.notified("$/progress", |p| p["value"]["kind"] == "begin");
    assert_ne!(
        first["token"], second["token"],
        "the second run's progress has a token of its own"
    );
    let created: Vec<&Value> = editor
        .asked
        .iter()
        .filter(|asked| asked["method"] == "window/workDoneProgress/create")
        .map(|asked| &asked["params"]["token"])
        .collect();
    assert_eq!(
        created,
        [&first["token"], &second["token"]],
        "each token was created before its progress began"
    );
}
