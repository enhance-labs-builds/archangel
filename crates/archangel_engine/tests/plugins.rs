//! Scenarios from the plugins spec that the engine owns: the view sent to the plugin process holds
//! only what the filters cover, a change sends only the records it changed, a violation becomes a
//! diagnostic or a plugin failure, a process that stops is started again, examples run over the
//! view, and the view's fields match the types the npm package declares.
#![cfg(test)]

use std::collections::BTreeMap;

use archangel_engine::view::{ExportsRecord, FileRecord, ImportRecord, SCHEMA_VERSION, View};
use archangel_engine::{Change, PathEvent, Session};
use archangel_rules::{Code, Diagnostic};
use archangel_testkit::{FixtureRepo, Gate, entities_importing_shared};
use serde_json::{Value, json};

/// Holds the fail points for one test, because they are global to the process.
struct Faults {
    #[cfg(feature = "failpoints")]
    _scenario: fail::FailScenario<'static>,
}

fn faults() -> Faults {
    Faults {
        #[cfg(feature = "failpoints")]
        _scenario: fail::FailScenario::setup(),
    }
}

/// Runs `body` with the fail point on. The caller holds [`faults`].
#[cfg(feature = "failpoints")]
fn under<T>(point: &str, body: impl FnOnce() -> T) -> T {
    fail::cfg(point, "return").unwrap();
    let found = body();
    fail::remove(point);
    found
}

fn open(repo: &FixtureRepo) -> Session {
    Session::open(repo.path(), None).expect("the session opens")
}

/// 960 library files, and 40 entity files under `src/entities/`, each importing one library
/// file; the first entity is `user`. The config has `rule`, a fixture plugin rule.
fn entities(rule: &Value) -> FixtureRepo {
    let repo = FixtureRepo::new().synthetic("src/lib", 960, 2);
    (0..40)
        .fold(repo, |repo, i| {
            let name = if i == 0 {
                "user".to_owned()
            } else {
                format!("e{i}")
            };
            repo.file(
                &format!("src/entities/{name}/index.ts"),
                &format!("import '../../lib/f{i}';\nexport const {name} = {i};\n"),
            )
        })
        .plugin_config(&json!({
            "elements": { "entity": "src/entities/{entity}/**", "lib": "src/lib/**" },
            "rules": [rule],
        }))
}

fn counting_imports() -> Value {
    json!({ "name": "entity-imports", "files": "src/entities/**",
            "plugin": "fixture/count-imports", "options": { "max": 0 } })
}

fn counting_importers() -> Value {
    json!({ "name": "entity-importers", "files": "src/entities/**",
            "plugin": "fixture/importers-at-most", "options": { "max": 5 } })
}

/// The plugin violations of the session, as (rule, file, message).
fn plugin_violations(session: &Session) -> Vec<(String, String, String)> {
    session
        .report()
        .diagnostics
        .iter()
        .filter(|d| d.code == Code::Plugin)
        .map(|d| {
            (
                d.rule.clone().unwrap_or_default(),
                d.file.clone().unwrap_or_default(),
                d.message.clone(),
            )
        })
        .collect()
}

#[test]
fn the_view_holds_only_the_files_and_facts_a_filter_covers() {
    let _faults = faults();
    let repo = entities(&counting_imports());
    let session = open(&repo);
    assert_eq!(
        (
            session.work().plugin_records_sent,
            session.work().plugin_checks
        ),
        (40, 1),
        "one record per file in the filter, and one run of the one plugin rule"
    );
    let records = session.plugin_records();
    assert_eq!(records.len(), 40, "the view holds the 40 entity files");
    for record in &records {
        assert!(
            record.path.starts_with("src/entities/")
                && record.imports.as_ref().is_some_and(|i| i.len() == 1)
                && record.importers.is_none()
                && record.exports.is_none(),
            "each record has its imports and no exports or importers: {record:?}"
        );
    }
    assert_eq!(
        plugin_violations(&session).len(),
        40,
        "each entity file has one import, more than the maximum of 0"
    );
}

#[test]
fn an_edit_outside_every_filter_runs_no_plugin() {
    let _faults = faults();
    let repo = entities(&counting_imports());
    let mut session = open(&repo);
    let before = plugin_violations(&session);
    let mut applied = session.apply(vec![Change::Text {
        path: "src/lib/f3.ts".into(),
        text: Some("import './f1';\nexport const f3 = 3;\n".into()),
    }]);
    applied.work.add(&session.wait_for_plugins().work);
    assert_eq!(
        (
            applied.work.files_parsed,
            session
                .graph()
                .importers("src/lib/f1.ts")
                .contains(&"src/lib/f3.ts")
        ),
        (1, true),
        "the edit is parsed and resolved"
    );
    assert_eq!(
        (applied.work.plugin_records_sent, applied.work.plugin_checks),
        (0, 0),
        "nothing is sent to the plugin process, and no condition runs"
    );
    assert_eq!(
        plugin_violations(&session),
        before,
        "the plugin violations stay"
    );
}

#[test]
fn a_new_importer_changes_the_imported_files_record() {
    let _faults = faults();
    let repo = entities(&counting_importers());
    let mut session = open(&repo);
    let mut applied = session.apply(vec![Change::Text {
        path: "src/lib/f5.ts".into(),
        text: Some("import '../entities/user/index';\nexport const f5 = 5;\n".into()),
    }]);
    applied.work.add(&session.wait_for_plugins().work);
    assert_eq!(
        applied.work.plugin_records_sent, 1,
        "one changed record is sent"
    );
    let user = session
        .plugin_records()
        .into_iter()
        .find(|record| record.path == "src/entities/user/index.ts")
        .expect("the user entity is in the view");
    assert_eq!(
        user.importers.as_deref(),
        Some(&["src/lib/f5.ts".to_owned()][..]),
        "the record has the new importer"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn view_size_rejects_a_view_of_every_file() {
    let _faults = faults();
    let repo = entities(&counting_imports());
    assert_ne!(
        under("engine::send_every_file", || open(&repo))
            .work()
            .plugin_records_sent,
        40,
        "the count catches a view of every file"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn a_new_view_field_does_not_change_an_old_plugins_result() {
    let _faults = faults();
    let repo = entities(&counting_imports());
    let older = plugin_violations(&open(&repo));
    let newer = under("engine::view_gains_a_field", || {
        plugin_violations(&open(&repo))
    });
    assert_eq!(older.len(), 40, "the plugin finds its violations");
    assert_eq!(
        newer, older,
        "a field the plugin does not know changes none of its violations"
    );
}

#[test]
fn a_change_to_a_plugin_file_loads_the_config_again() {
    let _faults = faults();
    let repo = entities(&counting_imports());
    let mut session = open(&repo);
    let plugin = repo.read("fixture-plugin.mjs");
    repo.write(
        "fixture-plugin.mjs",
        &plugin.replace("imports, more than", "imports, which is more than"),
    );
    session.apply(vec![Change::Paths(vec![PathEvent::Changed(
        repo.join("fixture-plugin.mjs"),
    )])]);
    session.wait_for_plugins();
    let messages: Vec<String> = plugin_violations(&session)
        .into_iter()
        .map(|(_, _, message)| message)
        .collect();
    assert!(
        messages.len() == 40 && messages.iter().all(|m| m.contains("which is more than")),
        "the edited plugin gives the messages: {messages:?}"
    );
}

#[test]
fn an_edit_that_leaves_a_record_unchanged_sends_nothing() {
    let _faults = faults();
    let repo = entities(&counting_imports());
    let mut session = open(&repo);
    let mut applied = session.apply(vec![Change::Text {
        path: "src/entities/user/index.ts".into(),
        text: Some("import '../../lib/f0';\nexport const renamed = 0;\n".into()),
    }]);
    applied.work.add(&session.wait_for_plugins().work);
    assert_eq!(
        (
            applied.work.files_parsed,
            applied.work.plugin_records_sent,
            applied.work.plugin_checks
        ),
        (1, 0, 0),
        "the edit is parsed, and its record, whose imports are the same, is not sent again"
    );
    assert_eq!(
        plugin_violations(&session).len(),
        40,
        "the plugin violations stay"
    );
}

#[test]
fn deleting_a_file_of_the_view_sends_its_removal() {
    let _faults = faults();
    let repo = entities(&counting_imports());
    let mut session = open(&repo);
    repo.remove("src/entities/e5/index.ts");
    let mut applied = session.apply(vec![Change::Paths(vec![PathEvent::Deleted(
        repo.join("src/entities/e5/index.ts"),
    )])]);
    applied.work.add(&session.wait_for_plugins().work);
    assert_eq!(applied.work.plugin_records_sent, 1, "one removal is sent");
    assert!(
        session
            .plugin_records()
            .iter()
            .all(|record| record.path != "src/entities/e5/index.ts"),
        "the view no longer holds the file"
    );
    let violations = plugin_violations(&session);
    assert!(
        violations.len() == 39
            && violations
                .iter()
                .all(|(_, file, _)| file != "src/entities/e5/index.ts"),
        "the deleted file's violation is gone, and no plugin failure takes its place: {:?}",
        session.report().diagnostics
    );
}

#[test]
fn a_file_that_is_not_source_appearing_runs_no_plugin_rule() {
    let _faults = faults();
    let repo = entities(&counting_imports());
    let mut session = open(&repo);
    repo.write("src/entities/user/logo.png", "");
    let mut applied = session.apply(vec![Change::Paths(vec![PathEvent::Created(
        repo.join("src/entities/user/logo.png"),
    )])]);
    applied.work.add(&session.wait_for_plugins().work);
    assert!(
        session.graph().is_other("src/entities/user/logo.png"),
        "the graph has the new file"
    );
    assert_eq!(
        (applied.work.plugin_records_sent, applied.work.plugin_checks),
        (0, 0),
        "no record changed, so nothing is sent and no condition runs"
    );
}

fn shared(rules: &[Value]) -> FixtureRepo {
    entities_importing_shared(&Value::from(rules.to_vec()))
}

/// The session's diagnostics with these codes.
fn with_code(session: &Session, code: Code) -> Vec<Diagnostic> {
    session
        .report()
        .diagnostics
        .iter()
        .filter(|d| d.code == code)
        .cloned()
        .collect()
}

#[test]
fn a_violation_of_an_import_is_at_that_import_with_the_conditions_docs() {
    let _faults = faults();
    let repo = shared(&[json!({ "name": "no-t", "files": "entity",
        "plugin": "fixture/report-imports", "options": { "specifier": "../../shared/t" } })]);
    let session = open(&repo);
    let found: Vec<_> = with_code(&session, Code::Plugin)
        .into_iter()
        .map(|d| (d.rule, d.file, d.line, d.specifier, d.link))
        .collect();
    assert_eq!(
        found,
        [(
            Some("no-t".to_owned()),
            Some("src/entities/user/index.ts".to_owned()),
            2,
            Some("../../shared/t".to_owned()),
            Some("https://example.com/fixture/report-imports".to_owned()),
        )],
        "one violation, on the line of the import it names, linked to its condition's docs"
    );
}

/// The one `plugin-failed` message of the session.
fn failure(session: &Session) -> String {
    let failures = with_code(session, Code::PluginFailed);
    assert_eq!(
        failures.len(),
        1,
        "one plugin-failed: {:?}",
        session.report().diagnostics
    );
    failures[0].message.clone()
}

fn reports(file: &str) -> FixtureRepo {
    shared(&[json!({ "name": "reports", "files": "entity",
        "plugin": "fixture/reports-outside", "options": { "file": file } })])
}

#[test]
fn a_violation_may_name_a_folder_that_holds_a_file_of_its_view() {
    let _faults = faults();
    let session = open(&reports("src/entities/user"));
    assert_eq!(
        plugin_violations(&session),
        [(
            "reports".to_owned(),
            "src/entities/user".to_owned(),
            "src/entities/user is not in the view".to_owned()
        )],
        "one violation of the folder: {:?}",
        session.report().diagnostics
    );
}

#[test]
fn a_violation_of_a_folder_outside_the_view_is_a_plugin_failure() {
    let _faults = faults();
    let session = open(&reports("src/shared"));
    assert!(
        failure(&session).contains("src/shared, which is not in its view"),
        "the failure names the folder"
    );
    assert_eq!(plugin_violations(&session), [], "and no violation");
}

#[test]
fn a_violation_of_a_file_outside_the_view_is_a_plugin_failure() {
    let _faults = faults();
    let session = open(&reports("src/shared/s.ts"));
    assert!(
        failure(&session).contains("src/shared/s.ts, which is not in its view"),
        "the failure names the file"
    );
    assert_eq!(plugin_violations(&session), [], "and no violation");
}

#[test]
fn a_process_that_stops_is_a_failure_that_quotes_what_it_printed() {
    let _faults = faults();
    let repo = shared(&[json!({ "name": "exits", "files": "entity", "plugin": "fixture/exits" })]);
    let mut session = open(&repo);
    let message = failure(&session);
    assert!(
        message.contains(
            "the plugin process stopped while it ran the rule \"exits\"; it printed:\n\
             [fixture] the fixture stops its process"
        ),
        "the failure says the process stopped, and quotes what it printed: {message}"
    );
    assert_eq!(
        session.take_plugin_output(),
        ["[fixture] the fixture stops its process"],
        "what the plugin printed is kept after its process stopped"
    );
    assert_eq!(
        session.take_plugin_output(),
        Vec::<String>::new(),
        "and taken once"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn a_stopped_process_is_started_again_with_the_whole_view() {
    let _faults = faults();
    let repo = entities(&counting_imports());
    let mut session = under("engine::plugin_dies", || open(&repo));
    assert!(
        failure(&session).contains("stopped"),
        "the process that the fault stopped is a failure"
    );
    assert_eq!(plugin_violations(&session), [], "with no violation");
    session.apply(vec![Change::Text {
        path: "src/entities/user/index.ts".into(),
        text: Some("import '../../lib/f0';\nexport const renamed = 0;\n".into()),
    }]);
    session.wait_for_plugins();
    assert_eq!(
        (
            with_code(&session, Code::PluginFailed).len(),
            plugin_violations(&session).len(),
            session.plugin_records().len()
        ),
        (0, 40, 40),
        "the next change starts the process again, sends it every record, and runs the rule"
    );
}

/// Two entities that import nothing, and a rule whose condition waits at `gate` while a file of
/// its view imports `src/shared/s.ts`.
fn waiting(gate: &Gate) -> FixtureRepo {
    FixtureRepo::new()
        .file("src/shared/s.ts", "export const s = 1;\n")
        .file("src/entities/user/index.ts", "export const user = 1;\n")
        .file("src/entities/post/index.ts", "export const post = 1;\n")
        .plugin_config(&json!({
            "elements": { "entity": "src/entities/{entity}/**", "shared": "src/shared/**" },
            "rules": [{ "name": "waits", "files": "entity", "plugin": "fixture/waits",
                        "options": { "port": gate.port(), "specifier": "../../shared/s" } }],
        }))
}

fn edit_user(session: &mut Session, text: &str) {
    session.apply(vec![Change::Text {
        path: "src/entities/user/index.ts".into(),
        text: Some(text.into()),
    }]);
}

#[test]
fn only_the_answer_to_the_latest_check_is_taken() {
    let _faults = faults();
    let gate = Gate::closed();
    let repo = waiting(&gate);
    let mut session = open(&repo);
    assert_eq!(
        session.plugins_running(),
        None,
        "nothing runs after the open"
    );
    edit_user(&mut session, "import '../../shared/s';\n");
    assert_eq!(
        session.plugins_running(),
        Some(vec!["fixture".to_owned()]),
        "the plugin runs while its condition waits"
    );
    edit_user(&mut session, "export const user = 1;\n");
    let answers = session
        .plugin_answers()
        .expect("the answers arrive while the process runs");
    gate.open();
    let stale = answers.recv().expect("the first check's answer");
    session.receive_plugins(stale);
    assert_eq!(
        session.plugins_running(),
        Some(vec!["fixture".to_owned()]),
        "the first check's answer does not end the run of the second"
    );
    let latest = answers.recv().expect("the second check's answer");
    session.receive_plugins(latest);
    assert_eq!(
        (
            session.plugins_running(),
            plugin_violations(&session),
            with_code(&session, Code::PluginFailed)
        ),
        (None, vec![], vec![]),
        "the answer to the first check, which names an import the file no longer has, is not \
         taken, and the answer to the second is"
    );
}

#[test]
fn the_answer_that_a_check_waits_for_changes_the_report() {
    let _faults = faults();
    let gate = Gate::closed();
    let repo = waiting(&gate);
    let mut session = open(&repo);
    edit_user(&mut session, "import '../../shared/s';\n");
    let answers = session
        .plugin_answers()
        .expect("the answers arrive while the process runs");
    gate.open();
    let answer = answers.recv().expect("an answer");
    let applied = session.receive_plugins(answer);
    assert_eq!(
        (
            session.plugins_running(),
            applied.changed.into_iter().collect::<Vec<_>>()
        ),
        (None, vec!["src/entities/user/index.ts".to_owned()]),
        "the answer ends the run and names the file whose diagnostics changed"
    );
}

#[test]
fn dropping_a_session_stops_a_plugin_that_is_still_working() {
    let _faults = faults();
    let gate = Gate::closed();
    let repo = waiting(&gate);
    let mut session = open(&repo);
    edit_user(&mut session, "import '../../shared/s';\n");
    gate.await_a_waiter();
    drop(session);
    gate.await_the_waiters_leaving();
}

#[test]
fn a_config_without_plugin_conditions_leaves_no_node_process() {
    let _faults = faults();
    let config = "import { writeFileSync } from 'node:fs';\n\
        process.on('exit', () => writeFileSync(new URL('./node-exited', import.meta.url), ''));\n\
        export default {\n\
          elements: { app: 'src/app/**', shared: 'src/**' },\n\
          rules: [{ kind: 'layers', name: 'layers', order: ['app', 'shared'] }],\n\
        };\n";
    let repo = FixtureRepo::new()
        .file("src/app/main.ts", "")
        .file("archangel.config.mjs", config);
    open(&repo);
    assert!(
        repo.join("node-exited").exists(),
        "the Node process that loaded the config exited, on its own, before the session opened"
    );
}

/// What the `describes` condition says of each file: the fields of its record.
#[cfg(feature = "failpoints")]
fn described(session: &Session) -> Vec<String> {
    plugin_violations(session)
        .into_iter()
        .map(|(_, _, message)| message)
        .collect()
}

#[cfg(feature = "failpoints")]
#[test]
fn the_view_field_fault_adds_a_field_that_the_plugin_receives() {
    let _faults = faults();
    let repo = shared(&[json!({ "name": "describes", "files": "entity",
        "plugin": "fixture/describes" })]);
    let fields = "captures element exports importers imports path selectors";
    assert_eq!(
        described(&open(&repo)),
        [fields, fields],
        "each record has the fields of the view"
    );
    let newer =
        "captures element exports fieldFromALaterArchangel importers imports path selectors";
    assert_eq!(
        under("engine::view_gains_a_field", || described(&open(&repo))),
        [newer, newer],
        "under the fault, each record has one more"
    );
}

/// The records of a rule whose condition reads every fact, where the user entity imports a shared
/// file twice and a package, and exports a default.
fn described_records() -> (FixtureRepo, Session) {
    let repo = shared(&[json!({ "name": "describes", "files": "entity",
        "plugin": "fixture/describes" })])
    .file(
        "src/entities/user/index.ts",
        "import { s } from '../../shared/s';\nimport fs from 'fs';\nimport '../../shared/s';\n\
         export default fs;\n",
    );
    let session = open(&repo);
    (repo, session)
}

#[test]
fn a_record_holds_where_each_import_resolves_and_its_occurrence() {
    let _faults = faults();
    let (_repo, session) = described_records();
    let imports: Vec<_> = session
        .plugin_records()
        .iter()
        .flat_map(|record| record.imports.iter().flatten())
        .map(|import| {
            (
                import.specifier.clone(),
                import.occurrence,
                import.file.clone(),
                import.package.clone(),
            )
        })
        .collect();
    let at = |specifier: &str, occurrence, file: Option<&str>, package: Option<&str>| {
        (
            specifier.to_owned(),
            occurrence,
            file.map(str::to_owned),
            package.map(str::to_owned),
        )
    };
    assert_eq!(
        imports,
        [
            at("../../shared/s", 0, Some("src/shared/s.ts"), None),
            at("fs", 0, None, Some("node:fs")),
            at("../../shared/s", 1, Some("src/shared/s.ts"), None),
        ],
        "each import with its occurrence among those of its specifier, and its file or package"
    );
}

#[test]
fn a_record_says_whether_its_file_has_a_default_export() {
    let _faults = faults();
    let (_repo, session) = described_records();
    let defaults: Vec<_> = session
        .plugin_records()
        .iter()
        .map(|record| {
            (
                record.path.clone(),
                record.exports.as_ref().map(|exports| exports.default),
            )
        })
        .collect();
    assert_eq!(
        defaults,
        [
            ("src/entities/post/index.ts".to_owned(), Some(false)),
            ("src/entities/user/index.ts".to_owned(), Some(true)),
        ],
        "only the user entity has a default export"
    );
}

// Examples.

/// The outcome of each example, as (rule, passed, the codes its files gave).
fn examples(session: &mut Session) -> Vec<(String, bool, Vec<Code>)> {
    session
        .run_examples()
        .runs
        .into_iter()
        .map(|run| {
            let codes = run.outcome.diagnostics.iter().map(|d| d.code).collect();
            (run.rule, run.outcome.passed, codes)
        })
        .collect()
}

#[test]
fn a_plugin_rule_is_proven_by_its_examples_with_one_record_sent_and_taken_back_each() {
    let _faults = faults();
    let repo = shared(&[json!({ "name": "no-s", "files": "entity",
        "plugin": "fixture/report-imports", "options": { "specifier": "../../shared/s" },
        "examples": {
            "accepted": [{ "src/entities/tag/index.ts": "import '../../shared/t';\n" }],
            "rejected": [{ "src/entities/tag/index.ts": "import '../../shared/s';\n" }],
        } })]);
    let mut session = open(&repo);
    let report = session.run_examples();
    let passed: Vec<bool> = report.runs.iter().map(|run| run.outcome.passed).collect();
    assert_eq!(
        passed,
        [true, true],
        "both examples pass: {:?}",
        report.runs
    );
    assert_eq!(
        report.work.plugin_records_sent, 4,
        "each example sends its one record and takes it back"
    );
}

#[test]
fn an_example_files_importers_are_the_example_files_that_import_it() {
    let _faults = faults();
    let repo = shared(&[json!({ "name": "unimported", "files": "shared",
        "plugin": "fixture/importers-at-most", "options": { "max": 0 },
        "examples": {
            "accepted": [{
                "src/entities/user/index.ts": "export const user = 1;\n",
                "src/shared/s.ts": "export const s = 1;\n",
            }],
            "rejected": [{
                "src/shared/a.ts": "import './b';\n",
                "src/shared/b.ts": "export const b = 1;\n",
            }],
        } })]);
    let report = open(&repo).run_examples();
    let found: Vec<(bool, Vec<(String, String)>)> = report
        .runs
        .iter()
        .map(|run| {
            let violations = run.outcome.diagnostics.iter().filter_map(|d| {
                let file = d.file.clone()?;
                (d.code == Code::Plugin && run.files.contains(&file))
                    .then(|| (file, d.message.clone()))
            });
            (run.outcome.passed, violations.collect())
        })
        .collect();
    assert_eq!(
        found,
        [
            (true, vec![]),
            (
                true,
                vec![(
                    "src/shared/b.ts".to_owned(),
                    "src/shared/b.ts has 1 importers, more than 0".to_owned()
                )]
            ),
        ],
        "the replaced user entity no longer imports s, and a imports b: {:?}",
        report.runs
    );
}

#[test]
fn an_example_may_be_rejected_for_the_folder_that_holds_its_file() {
    let _faults = faults();
    let repo = shared(&[json!({ "name": "folder", "files": "entity",
        "plugin": "fixture/reports-outside", "options": { "file": "src/entities/tag" },
        "examples": { "rejected": [{ "src/entities/tag/index.ts": "export const tag = 1;\n" }] },
    })]);
    let found = examples(&mut open(&repo));
    assert_eq!(
        found,
        [("folder".to_owned(), true, vec![Code::Plugin])],
        "the example's folder is in its view"
    );
}

#[test]
fn an_example_whose_violation_is_outside_its_view_is_a_plugin_failure() {
    let _faults = faults();
    for outside in ["src/shared", "src/shared/s.ts"] {
        let repo = shared(&[json!({ "name": "reports", "files": "entity",
            "plugin": "fixture/reports-outside", "options": { "file": outside },
            "examples": { "rejected": [{ "src/entities/tag/index.ts": "export const tag = 1;\n" }] },
        })]);
        let found = examples(&mut open(&repo));
        assert_eq!(
            found,
            [("reports".to_owned(), false, vec![Code::PluginFailed])],
            "{outside} holds no file of the example's view"
        );
    }
}

#[test]
fn a_line_a_plugin_writes_to_standard_output_is_not_its_answer_to_an_example() {
    let _faults = faults();
    let repo = shared(&[json!({ "name": "no-s", "files": "entity",
        "plugin": "fixture/writes-to-stdout", "options": { "specifier": "../../shared/s" },
        "examples": { "rejected": [{ "src/entities/tag/index.ts": "import '../../shared/s';\n" }] },
    })]);
    let found: Vec<(String, bool)> = examples(&mut open(&repo))
        .into_iter()
        .map(|(rule, passed, _)| (rule, passed))
        .collect();
    assert_eq!(
        found,
        [("no-s".to_owned(), true)],
        "the condition's answer, after the line it wrote, rejects the example"
    );
}

// The view types.

/// A view with every field the engine can send.
fn full_view() -> Value {
    let record = FileRecord {
        path: "src/entities/user/index.ts".into(),
        element: Some("entity".into()),
        captures: BTreeMap::from([("entity".to_owned(), "user".to_owned())]),
        selectors: vec!["entities".into()],
        imports: Some(vec![ImportRecord {
            specifier: "../../lib/f0".into(),
            occurrence: 0,
            line: 1,
            column: 8,
            kind: "runtime",
            names: vec!["f0".into()],
            file: Some("src/lib/f0.ts".into()),
            package: None,
        }]),
        importers: Some(vec!["src/app/main.ts".into()]),
        exports: Some(ExportsRecord {
            names: vec!["user".into()],
            default: false,
            star: false,
        }),
    };
    let view = View {
        schema_version: SCHEMA_VERSION,
        files: vec![record],
    };
    serde_json::to_value(&view).expect("the view serializes")
}

/// The fields of each interface that a declaration file declares, by interface name: field name
/// to its type as written. Each field is on one line, as `readonly name?: Type;`.
fn interfaces(declarations: &str) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut found = BTreeMap::new();
    let mut current: Option<(String, BTreeMap<String, String>)> = None;
    for line in declarations.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("export interface ") {
            let name = rest.split([' ', '<', '{']).next().unwrap_or_default();
            current = Some((name.to_owned(), BTreeMap::new()));
        } else if line == "}" {
            if let Some((name, fields)) = current.take() {
                found.insert(name, fields);
            }
        } else if let Some((_, fields)) = current.as_mut()
            && let Some(field) = line.strip_prefix("readonly ")
            && let Some((name, ty)) = field.split_once(':')
        {
            let name = name.trim_end_matches('?').to_owned();
            fields.insert(name, ty.trim().trim_end_matches(';').to_owned());
        }
    }
    found
}

/// Whether a JSON value has the declared type, or the reason it does not.
fn conforms(
    value: &Value,
    ty: &str,
    declared: &BTreeMap<String, BTreeMap<String, String>>,
    at: &str,
) -> Result<(), String> {
    let alternatives: Vec<&str> = ty.split(" | ").map(str::trim).collect();
    if alternatives.len() > 1 {
        return alternatives
            .iter()
            .find_map(|one| conforms(value, one, declared, at).ok())
            .ok_or_else(|| format!("{at} is {value}, which is not {ty}"));
    }
    let fits = match (ty, value) {
        ("string", Value::String(_))
        | ("number", Value::Number(_))
        | ("boolean", Value::Bool(_))
        | ("null", Value::Null) => true,
        (literal, Value::String(text)) if literal.starts_with('\'') => {
            literal.trim_matches('\'') == text
        }
        (literal, Value::Number(number)) if literal.parse::<u64>().is_ok() => {
            number.as_u64() == literal.parse::<u64>().ok()
        }
        (list, Value::Array(items)) if list.starts_with("readonly ") && list.ends_with("[]") => {
            let item = &list["readonly ".len()..list.len() - 2];
            for (index, element) in items.iter().enumerate() {
                conforms(element, item, declared, &format!("{at}[{index}]"))?;
            }
            true
        }
        (map, Value::Object(entries)) if map.starts_with("Readonly<Record<string, ") => {
            let item = &map["Readonly<Record<string, ".len()..map.len() - 2];
            for (key, element) in entries {
                conforms(element, item, declared, &format!("{at}.{key}"))?;
            }
            true
        }
        (name, Value::Object(entries)) => {
            let fields = declared
                .get(name)
                .ok_or_else(|| format!("{at} is an object, and `{name}` is not an interface"))?;
            for (key, element) in entries {
                let field_type = fields.get(key).ok_or_else(|| {
                    format!("{at}.{key} is sent, and the interface `{name}` does not declare it")
                })?;
                conforms(element, field_type, declared, &format!("{at}.{key}"))?;
            }
            true
        }
        _ => false,
    };
    if fits {
        Ok(())
    } else {
        Err(format!("{at} is {value}, which is not {ty}"))
    }
}

#[test]
fn the_declared_view_types_match_the_engine() {
    let declared = interfaces(include_str!("../../../npm/archangel/plugin.d.ts"));
    let checked = conforms(&full_view(), "View", &declared, "view");
    assert_eq!(
        checked,
        Ok(()),
        "every field the engine sends is declared, with its type"
    );
}

#[test]
fn the_type_check_rejects_a_field_the_types_do_not_declare() {
    let declared = interfaces(
        "export interface View {\n  readonly schemaVersion: 1;\n  readonly files: readonly string[];\n}\n",
    );
    let mut view = json!({ "schemaVersion": 1, "files": ["a.ts"] });
    assert_eq!(
        conforms(&view, "View", &declared, "view"),
        Ok(()),
        "a match"
    );
    view["extra"] = json!(true);
    assert!(
        conforms(&view, "View", &declared, "view").is_err(),
        "an undeclared field is caught"
    );
    view = json!({ "schemaVersion": 1, "files": [3] });
    assert!(
        conforms(&view, "View", &declared, "view").is_err(),
        "a field of another type is caught"
    );
}
