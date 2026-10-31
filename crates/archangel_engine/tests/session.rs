//! Scenarios from the config, baseline and command-line specs, at the session: what stops a check,
//! the baseline in use and its pruning, a full check after lost events, and the stages that
//! `--stats` names.
#![cfg(test)]

use std::collections::BTreeSet;

use archangel_engine::{BASELINE_NAME, Change, PathEvent, Session};
use archangel_rules::Code;
use archangel_testkit::{FixtureRepo, app_and_shared};
use serde_json::json;

/// Three shared files that each import the app, which the layers rule forbids.
fn three_violations() -> FixtureRepo {
    FixtureRepo::new()
        .file("src/app/main.ts", "")
        .file("src/x.ts", "import './app/main';\n")
        .file("src/y.ts", "import './app/main';\n")
        .file("src/z.ts", "import './app/main';\n")
        .json_config(&app_and_shared(&[
            json!({ "kind": "layers", "name": "layers", "order": ["app", "shared"] }),
        ]))
}

fn open(repo: &FixtureRepo) -> Session {
    Session::open(repo.path(), None).expect("the session opens")
}

fn codes(session: &Session) -> Vec<(Code, Option<String>)> {
    session
        .report()
        .diagnostics
        .iter()
        .map(|d| (d.code, d.file.clone()))
        .collect()
}

#[test]
fn a_config_that_does_not_validate_stops_the_open_with_its_problems() {
    let repo = three_violations().json_config(&json!({ "elements": {}, "rules": "none" }));
    let stop = Session::open(repo.path(), None).expect_err("the config stops the check");
    assert!(!stop.diagnostics.is_empty(), "the problems are given");
    assert!(
        stop.diagnostics
            .iter()
            .all(|d| d.code == Code::ConfigInvalid
                && d.file.as_deref() == Some("archangel.config.json")),
        "each is a config-invalid diagnostic in the config file: {:?}",
        stop.diagnostics
    );
}

#[test]
fn a_config_change_that_does_not_validate_replaces_every_diagnostic() {
    let repo = three_violations();
    let mut session = open(&repo);
    assert_eq!(
        codes(&session).len(),
        3,
        "the three violations are there first"
    );
    repo.write(
        "archangel.config.json",
        r#"{ "elements": {}, "rules": "none" }"#,
    );
    let config = session.config_path().to_path_buf();
    let applied = session.apply(vec![Change::Paths(vec![PathEvent::Changed(
        config.clone(),
    )])]);
    let found = codes(&session);
    assert!(
        !found.is_empty()
            && found.iter().all(|(code, file)| *code == Code::ConfigInvalid
                && file.as_deref() == Some("archangel.config.json")),
        "only the config's problems show, not results from rules that no longer load: {found:?}"
    );
    assert_eq!(
        applied.changed,
        BTreeSet::from(
            ["archangel.config.json", "src/x.ts", "src/y.ts", "src/z.ts"].map(String::from)
        ),
        "the files whose diagnostics changed"
    );
}

fn folder_in_place_of_the_baseline(repo: &FixtureRepo) {
    std::fs::create_dir(repo.join(BASELINE_NAME)).expect("a folder where the baseline goes");
}

#[test]
fn a_baseline_that_cannot_be_read_stops_the_open() {
    let repo = three_violations();
    folder_in_place_of_the_baseline(&repo);
    let stop = Session::open(repo.path(), None).expect_err("the baseline stops the check");
    let found: Vec<_> = stop
        .diagnostics
        .iter()
        .map(|d| (d.code, d.file.as_deref(), d.message.contains("cannot read")))
        .collect();
    assert_eq!(
        found,
        [(Code::BaselineInvalid, Some(BASELINE_NAME), true)],
        "one baseline-invalid diagnostic that says the file cannot be read"
    );
}

#[test]
fn a_baseline_change_that_does_not_parse_replaces_every_diagnostic() {
    let repo = three_violations();
    let mut session = open(&repo);
    repo.write(BASELINE_NAME, "{");
    session.apply(vec![Change::Paths(vec![PathEvent::Created(
        repo.join(BASELINE_NAME),
    )])]);
    assert_eq!(
        codes(&session),
        [(Code::BaselineInvalid, Some(BASELINE_NAME.to_owned()))],
        "only the baseline's problem shows"
    );
}

#[test]
fn the_session_uses_the_baseline_on_disk_or_none() {
    let repo = three_violations();
    assert_eq!(open(&repo).baseline(), None, "no file, no baseline");
    let written = open(&repo).baseline_add().expect("the baseline is written");
    assert_eq!(
        open(&repo).baseline(),
        Some(&written),
        "the baseline read is the one written"
    );
    assert_eq!(written.entries.len(), 3, "an entry for each violation");
}

#[test]
fn pruning_lowers_the_baseline_to_the_violations_left_and_writes_it() {
    let repo = three_violations();
    open(&repo).baseline_add().expect("the baseline is written");
    repo.write("src/x.ts", "");
    let pruned = open(&repo)
        .baseline_prune()
        .expect("the baseline is pruned");
    let files: Vec<_> = pruned.entries.iter().map(|e| e.file.as_deref()).collect();
    assert_eq!(
        files,
        [Some("src/y.ts"), Some("src/z.ts")],
        "the entry of the fixed file is gone"
    );
    assert_eq!(
        open(&repo).baseline(),
        Some(&pruned),
        "the file holds the pruned baseline"
    );
}

#[test]
fn a_reload_checks_everything_again_and_names_what_changed() {
    let repo = three_violations();
    let mut session = open(&repo);
    repo.write("src/x.ts", "");
    let applied = session.reload();
    assert_eq!(
        applied.changed,
        BTreeSet::from(["src/x.ts".to_owned()]),
        "the file whose diagnostics changed, which no event named"
    );
    assert_eq!(
        applied.work.files_parsed, 4,
        "every file parsed again, as after lost events"
    );
    assert_eq!(codes(&session).len(), 2, "two violations are left");
}

#[test]
fn a_reload_reads_the_config_and_the_baseline_again() {
    let repo = three_violations();
    let mut session = open(&repo);
    open(&repo).baseline_add().expect("the baseline is written");
    session.reload();
    assert_eq!(
        (
            codes(&session),
            session.report().summary.baselined.values().sum::<u64>()
        ),
        (vec![], 3),
        "the baseline written since the open permits the three violations"
    );
    let _repo = repo.json_config(&app_and_shared(&[]));
    session.reload();
    assert!(
        session.architecture().config().rules.is_empty(),
        "the config written since the open, which has no rules, is in use"
    );
}

#[test]
fn a_fix_to_a_file_that_a_refused_config_imported_loads_the_config_again() {
    let config = |rules: &str| {
        format!(
            "{rules}export default {{ elements: {{ app: 'src/app/**', shared: 'src/**' }}, rules }};\n"
        )
    };
    let repo = FixtureRepo::new()
        .file("src/app/main.ts", "")
        .file("src/x.ts", "import './app/main';\n")
        .file("archangel.config.mjs", &config("const rules = [];\n"));
    let mut session = open(&repo);
    assert_eq!(codes(&session), [], "the config has no rule at first");
    repo.write("rules.mjs", "throw new Error('the rules are broken');\n");
    repo.write(
        "archangel.config.mjs",
        &config("import { rules } from './rules.mjs';\n"),
    );
    session.apply(vec![Change::Paths(vec![PathEvent::Changed(
        repo.join("archangel.config.mjs"),
    )])]);
    assert_eq!(
        codes(&session),
        [(
            Code::ConfigLoadFailed,
            Some("archangel.config.mjs".to_owned())
        )],
        "the config that imports the broken file is refused"
    );
    repo.write(
        "rules.mjs",
        "export const rules = [{ kind: 'layers', name: 'layers', order: ['app', 'shared'] }];\n",
    );
    session.apply(vec![Change::Paths(vec![PathEvent::Changed(
        repo.join("rules.mjs"),
    )])]);
    assert_eq!(
        codes(&session),
        [(Code::Layers, Some("src/x.ts".to_owned()))],
        "the fixed file loads the config again, and the rule it gives finds the violation"
    );
}

#[test]
fn stats_names_each_stage_in_the_order_they_run() {
    let repo = three_violations();
    let timings = open(&repo).timings();
    let names = timings.stages().map(|(name, _)| name);
    assert_eq!(
        names,
        [
            "config",
            "graph",
            "  list files",
            "  read and parse",
            "  resolve",
            "rules"
        ],
        "the stages, with the graph's steps under it"
    );
}
