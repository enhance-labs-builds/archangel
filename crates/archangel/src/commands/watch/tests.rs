//! What one check of watch mode prints, from the sessions before and after a change, so a test
//! reads the exact bytes without a file watcher or a wait.

use std::collections::BTreeSet;

use archangel_engine::{Applied, Change, Session};
use archangel_testkit::{FixtureRepo, Gate};
use serde_json::json;

use super::Screen;
use crate::{Color, Format, Options};

/// Three problems: an element with no file, a tsconfig that does not parse, and a layers
/// violation on line 1.
fn broken() -> FixtureRepo {
    FixtureRepo::new()
        .file(
            "src/app/main.ts",
            "import '../shared/s';\nimport '../shared/t';\n",
        )
        .file("src/shared/s.ts", "")
        .file("src/shared/t.ts", "")
        .file("src/shared/bad.ts", "import '../app/main';\n")
        .file(
            "tsconfig.json",
            "{\n  \"compilerOptions\": {\n    \"strict\": true\n    \"baseUrl\": \".\"\n  }\n}\n",
        )
        .json_config(&json!({
            "elements": { "app": "src/app/**", "shared": "src/shared/**", "ghost": "src/ghost/**" },
            "rules": [{ "kind": "layers", "name": "layers", "order": ["app", "shared"] }],
        }))
}

/// The same repository with all three problems fixed, and the same number of imports.
fn fix(repo: &FixtureRepo) {
    repo.write("src/shared/bad.ts", "import './s';\n");
    repo.write("src/ghost/g.ts", "");
    repo.write("tsconfig.json", "{}\n");
}

fn options(format: Format) -> Options {
    Options {
        config: None,
        format,
        stats: false,
        color: Color::Never,
    }
}

/// A screen that prints into a buffer.
fn screen(options: &Options) -> Screen<'_, Vec<u8>> {
    Screen {
        options,
        shown: Vec::new(),
        out: Vec::new(),
    }
}

/// What the screen prints for a check of the repository as it is now.
fn print(screen: &mut Screen<'_, Vec<u8>>, repo: &FixtureRepo, polling: Option<&str>) -> String {
    let session = Session::open(repo.path(), None)
        .unwrap_or_else(|stop| panic!("the session opens: {:?}", stop.diagnostics));
    let applied = Applied {
        changed: BTreeSet::new(),
        work: *session.work(),
    };
    screen
        .print(&session, &applied, polling)
        .unwrap_or_else(|error| panic!("a buffer takes every byte: {error}"));
    String::from_utf8(std::mem::take(&mut screen.out))
        .unwrap_or_else(|error| panic!("the output is UTF-8: {error}"))
}

const EMPTY: &str = "the element \"ghost\" matches nothing (src/ghost/**), so no rule about it can fire; fix the pattern or remove the element";
const TSCONFIG: &str =
    "it is not valid JSON at line 4, column 5: expected `,` or `}` at line 4 column 5";
const LAYERS: &str = "\"shared\" may not import \"app\", an earlier layer, through \"../app/main\"";

/// The first check prints every diagnostic as new, as `check` draws it, then the summary line.
#[test]
fn text_prints_the_new_diagnostics_then_the_summary() {
    let options = options(Format::Text);
    let mut screen = screen(&options);
    assert_eq!(
        print(&mut screen, &broken(), None),
        format!(
            "error[element-empty]: {EMPTY}\n\n\
             error[layers]: {LAYERS}\n \
             --> src/shared/bad.ts:1:8\n  \
             |\n\
             1 | import '../app/main';\n  \
             |        ^^^^^^^^^^^^^ \"shared\" may import only \"shared\"\n\n\
             error[resolution-file-invalid]: {TSCONFIG}\n\
             --> tsconfig.json\n\n\
             checked 4 files and 3 imports: 3 problems\n"
        ),
        "every diagnostic is new in the first check"
    );
}

/// A fixed diagnostic prints one line with its code, its place and its message.
#[test]
fn text_prints_a_gone_line_for_each_fixed_diagnostic_with_its_place() {
    let options = options(Format::Text);
    let mut screen = screen(&options);
    let repo = broken();
    print(&mut screen, &repo, None);
    fix(&repo);
    assert_eq!(
        print(&mut screen, &repo, None),
        format!(
            "gone: [element-empty] the config: {EMPTY}\n\
             gone: [layers] src/shared/bad.ts:1: {LAYERS}\n\
             gone: [resolution-file-invalid] tsconfig.json: {TSCONFIG}\n\
             checked 5 files and 3 imports: no problems\n"
        ),
        "a diagnostic in no file is in the config; one about a whole file has no line"
    );
}

/// A check that finds what the last one found prints only its summary line.
#[test]
fn text_prints_nothing_new_or_gone_for_a_check_that_finds_the_same() {
    let options = options(Format::Text);
    let mut screen = screen(&options);
    let repo = broken();
    print(&mut screen, &repo, None);
    assert_eq!(
        print(&mut screen, &repo, None),
        "checked 4 files and 3 imports: 3 problems\n",
        "only the summary"
    );
}

/// When the watcher polls, the summary line says why.
#[test]
fn text_puts_the_polling_note_on_the_summary_line() {
    let options = options(Format::Text);
    let mut screen = screen(&options);
    let repo = broken();
    print(&mut screen, &repo, None);
    assert_eq!(
        print(&mut screen, &repo, Some("archangel polls")),
        "checked 4 files and 3 imports: 3 problems; archangel polls\n",
        "the note ends the summary line"
    );
}

/// The JSON of each diagnostic the fixed repository no longer has, in the order they were shown.
fn gone_json() -> [serde_json::Value; 3] {
    let empty = json!({
        "code": "element-empty", "rule": null, "file": null, "line": 0, "column": 0, "span": null,
        "specifier": null, "message": EMPTY, "help": null, "reason": null, "link": null,
        "id": null, "cycle": [], "group": [], "chain": [], "fix": null,
    });
    let layers = json!({
        "code": "layers", "rule": "layers", "file": "src/shared/bad.ts", "line": 1, "column": 8,
        "span": [7, 20], "specifier": "../app/main", "message": LAYERS,
        "help": "\"shared\" may import only \"shared\"", "reason": null, "link": null,
        "id": { "rule": "layers", "file": "src/shared/bad.ts", "specifier": "../app/main", "occurrence": 0 },
        "cycle": [], "group": [], "chain": [], "fix": null,
    });
    let tsconfig = json!({
        "code": "resolution-file-invalid", "rule": null, "file": "tsconfig.json", "line": 0,
        "column": 0, "span": null, "specifier": null, "message": TSCONFIG, "help": null,
        "reason": null, "link": null, "id": null, "cycle": [], "group": [], "chain": [], "fix": null,
    });
    [empty, layers, tsconfig]
}

/// The codes of a JSON line's new diagnostics.
fn new_codes(line: &str) -> Vec<String> {
    serde_json::from_str::<serde_json::Value>(line)
        .unwrap_or_else(|error| panic!("one JSON line: {error}\n{line}"))["new"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|d| d["code"].as_str().unwrap_or_default().to_owned())
        .collect()
}

/// The first JSON line has every diagnostic as new.
#[test]
fn json_prints_every_diagnostic_as_new_in_the_first_check() {
    let options = options(Format::Json);
    let mut screen = screen(&options);
    let first = print(&mut screen, &broken(), None);
    assert_eq!(
        new_codes(&first),
        ["element-empty", "layers", "resolution-file-invalid"],
        "every diagnostic: {first}"
    );
}

/// A later JSON line has the new, the gone and the summary of its check, with that check's work
/// and the polling note, on one line.
#[test]
fn json_prints_one_line_with_the_new_the_gone_and_the_summary() {
    let options = options(Format::Json);
    let mut screen = screen(&options);
    let repo = broken();
    print(&mut screen, &repo, None);
    fix(&repo);
    let line = print(&mut screen, &repo, Some("archangel polls"));
    let work = serde_json::from_str::<serde_json::Value>(&line)
        .unwrap_or_else(|error| panic!("one JSON line: {error}\n{line}"))["summary"]["work"]
        .clone();
    assert_eq!(work["filesParsed"], 5, "the check's own work: {line}");
    let expected = json!({
        "schemaVersion": 1,
        "new": [],
        "gone": gone_json(),
        "summary": {
            "files": 5,
            "imports": 3,
            "problems": 0,
            "exceptions": {},
            "baselined": {},
            "work": work,
            "polling": "archangel polls",
        },
    });
    assert_eq!(line, format!("{expected}\n"), "the whole line");
}

/// The JSON lines that answering a change prints.
fn answer_lines(session: &mut Session, applied: &Applied) -> Vec<serde_json::Value> {
    let options = options(Format::Json);
    let mut screen = screen(&options);
    screen
        .answer(session, applied, None)
        .unwrap_or_else(|error| panic!("a buffer takes every byte: {error}"));
    String::from_utf8_lossy(&screen.out)
        .lines()
        .map(|line| {
            serde_json::from_str(line)
                .unwrap_or_else(|error| panic!("one JSON line: {error}\n{line}"))
        })
        .collect()
}

/// A change that the plugin process has nothing to do with prints one line.
#[test]
fn a_check_without_plugins_running_prints_one_line() {
    let repo = broken();
    let mut session = Session::open(repo.path(), None)
        .unwrap_or_else(|stop| panic!("the session opens: {:?}", stop.diagnostics));
    let applied = session.apply(vec![Change::Text {
        path: "src/shared/bad.ts".into(),
        text: Some("import './s';\n".into()),
    }]);
    assert_eq!(
        answer_lines(&mut session, &applied).len(),
        1,
        "one line for the one check"
    );
}

/// While the plugin process works, archangel's own diagnostics print first, and the plugin rules'
/// follow on a line of their own once it answers.
#[test]
fn a_check_with_a_plugin_running_prints_again_when_it_answers() {
    let gate = Gate::closed();
    let repo = FixtureRepo::new()
        .file("src/app/main.ts", "")
        .file("src/s.ts", "export const s = 1;\n")
        .plugin_config(&json!({
            "elements": { "app": "src/app/**", "shared": "src/**" },
            "rules": [{ "name": "waits", "files": "src/**", "plugin": "fixture/waits",
                        "options": { "port": gate.port(), "specifier": "./app/main" } }],
        }));
    let mut session = Session::open(repo.path(), None)
        .unwrap_or_else(|stop| panic!("the session opens: {:?}", stop.diagnostics));
    let applied = session.apply(vec![Change::Text {
        path: "src/s.ts".into(),
        text: Some("import './app/main';\n".into()),
    }]);
    gate.open();
    let lines = answer_lines(&mut session, &applied);
    let codes: Vec<Vec<String>> = lines
        .iter()
        .map(|line| new_codes(&line.to_string()))
        .collect();
    assert_eq!(
        codes,
        [vec![], vec!["plugin".to_owned()]],
        "the check, then the plugin's violation once the process answers"
    );
}
