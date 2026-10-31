//! Scenarios from the plugins spec that need the binary: plugin violations through exceptions,
//! the baseline and every format, plugin rules proven by their examples, and plugins that fail.
#![cfg(test)]

#[path = "support/cli.rs"]
mod support;

use archangel_testkit::{FixtureRepo, entities_importing_shared};
use serde_json::{Value, json};
use support::run;

fn document(stdout: &str) -> Value {
    serde_json::from_str(stdout).unwrap_or_else(|e| panic!("one JSON document: {e}\n{stdout}"))
}

#[cfg(feature = "failpoints")]
fn codes(document: &Value) -> Vec<String> {
    document["diagnostics"]
        .as_array()
        .map(|all| {
            all.iter()
                .map(|d| d["code"].as_str().unwrap_or_default().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// The one `plugin-failed` diagnostic of a check that must fail.
fn plugin_failure(repo: &FixtureRepo) -> Value {
    let found = run(repo, &["check", "--format", "json"]);
    assert_eq!(
        found.status, 1,
        "a failed plugin fails the check: {}",
        found.stderr
    );
    let checked = document(&found.stdout);
    let failures: Vec<&Value> = checked["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|d| d["code"] == "plugin-failed")
        .collect();
    assert_eq!(failures.len(), 1, "one plugin-failed: {}", found.stdout);
    let failure = failures[0].clone();
    let message = failure["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("\"fixture\"") && message.contains("`off`"),
        "the message names the plugin and says the rule can be turned off: {message}"
    );
    failure
}

#[test]
fn a_plugin_violation_is_baselined_without_its_message() {
    let repo = entities_importing_shared(&json!([{ "name": "few-imports", "files": "entity",
                               "plugin": "fixture/count-imports", "options": { "max": 1 } }]));
    let added = run(&repo, &["baseline", "add"]);
    assert_eq!(added.status, 0, "the baseline is written: {}", added.stderr);
    repo.write(
        "src/entities/user/index.ts",
        "import '../../shared/s';\nimport '../../shared/t';\nimport '../post/index';\n",
    );
    let found = run(&repo, &["check", "--format", "json"]);
    assert_eq!(found.status, 0, "no violation: {}", found.stdout);
    assert_eq!(
        document(&found.stdout)["summary"]["baselined"]["few-imports"],
        1,
        "one baseline match for the plugin rule: {}",
        found.stdout
    );
}

#[test]
fn an_exception_permits_a_plugin_violation() {
    let repo = entities_importing_shared(&json!([{ "name": "no-s", "files": "entity",
                               "plugin": "fixture/report-imports",
                               "options": { "specifier": "../../shared/s" } }]));
    repo.write(
        "src/entities/user/index.ts",
        "// archangel-allow no-s: migrating\nimport { s } from '../../shared/s';\n",
    );
    let found = run(&repo, &["check", "--format", "json"]);
    assert_eq!(found.status, 0, "no violation: {}", found.stdout);
    assert_eq!(
        document(&found.stdout)["summary"]["exceptions"]["no-s"],
        1,
        "one exception for the plugin rule: {}",
        found.stdout
    );
}

#[test]
fn plugin_violations_appear_in_every_format_in_order() {
    let repo = entities_importing_shared(&json!([
        { "name": "no-s", "files": "src/**", "plugin": "fixture/report-imports",
          "options": { "specifier": "../shared/s" } },
        { "name": "shared-alone", "files": "shared", "mayNotImport": "entity" },
    ]))
    .file("src/a/one.ts", "import '../shared/s';\n")
    .file("src/b/two.ts", "import '../shared/s';\n")
    .file("src/shared/u.ts", "import '../entities/post/index';\n");
    let expected = [
        ("src/a/one.ts", "no-s"),
        ("src/b/two.ts", "no-s"),
        ("src/shared/u.ts", "shared-alone"),
    ];
    let json = document(&run(&repo, &["check", "--format", "json"]).stdout);
    let listed: Vec<(&str, &str)> = json["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|d| {
            (
                d["file"].as_str().unwrap_or_default(),
                d["rule"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(listed, expected, "json lists the three in order: {json}");
    for format in ["text", "github", "sarif"] {
        let found = run(&repo, &["check", "--format", format]);
        assert_eq!(found.status, 1, "{format} exits 1: {}", found.stderr);
        let mut from = 0;
        for (file, rule) in expected {
            let at = found.stdout[from..].find(file).unwrap_or_else(|| {
                panic!(
                    "{format} lists {file} after the one before:\n{}",
                    found.stdout
                )
            });
            from += at;
            assert!(
                found.stdout[from..].contains(rule),
                "{format} names the rule {rule} with {file}:\n{}",
                found.stdout
            );
        }
    }
}

fn proven(examples: &Value) -> FixtureRepo {
    entities_importing_shared(
        &json!([{ "name": "no-s", "files": "entity", "plugin": "fixture/report-imports",
                   "options": { "specifier": "../../shared/s" }, "examples": examples }]),
    )
}

#[test]
fn a_plugin_rule_is_proven_by_its_examples() {
    let repo = proven(&json!({
        "accepted": [{ "src/entities/tag/index.ts": "import '../../shared/t';\n" }],
        "rejected": [{ "src/entities/tag/index.ts": "import '../../shared/s';\n" }],
    }));
    let found = run(&repo, &["test", "--format", "json"]);
    assert_eq!(found.status, 0, "both examples pass: {}", found.stdout);
    let tested = document(&found.stdout);
    let work = &tested["summary"]["work"];
    assert_eq!(work["filesWritten"], 0, "nothing is written: {tested}");
    assert!(
        !repo.join("src/entities/tag").exists(),
        "the example's folder is not on disk"
    );
    assert_eq!(
        work["pluginRecordsSent"], 4,
        "each example sends its one record and takes it back, and the view is not sent again: {tested}"
    );
}

#[test]
fn a_plugin_rule_without_a_rejected_example_is_unproven() {
    let repo = proven(&json!({
        "accepted": [{ "src/entities/tag/index.ts": "import '../../shared/t';\n" }],
    }));
    let found = run(&repo, &["test"]);
    assert_eq!(found.status, 1, "an unproven rule fails: {}", found.stdout);
    assert!(
        found.stdout.contains("rule-unproven") && found.stdout.contains("no-s"),
        "the rule is unproven: {}",
        found.stdout
    );
}

#[test]
fn a_plugin_that_throws_fails_the_check() {
    let repo = entities_importing_shared(
        &json!([{ "name": "boom", "files": "entity", "plugin": "fixture/throws",
                               "options": { "message": "boom" } }]),
    );
    let failure = plugin_failure(&repo);
    assert!(
        failure["message"]
            .as_str()
            .unwrap_or_default()
            .contains("boom"),
        "the message gives the error: {failure}"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn a_plugin_process_that_stops_fails_the_check() {
    let repo = entities_importing_shared(&json!([{ "name": "few-imports", "files": "entity",
                               "plugin": "fixture/count-imports", "options": { "max": 5 } }]));
    let found: support::Finished = support::command(&repo)
        .args(["check", "--format", "json"])
        .env("FAILPOINTS", "engine::plugin_dies=return")
        .output()
        .unwrap()
        .into();
    assert_eq!(
        found.status, 1,
        "a stopped plugin fails the check: {}",
        found.stderr
    );
    let checked = document(&found.stdout);
    assert_eq!(
        codes(&checked),
        ["plugin-failed"],
        "one plugin-failed: {checked}"
    );
    let message = checked["diagnostics"][0]["message"]
        .as_str()
        .unwrap_or_default();
    assert!(
        message.contains("\"fixture\"") && message.contains("stopped"),
        "the message says the plugin process stopped: {message}"
    );
}

#[test]
fn a_plugin_process_that_exits_fails_the_check() {
    let repo = entities_importing_shared(
        &json!([{ "name": "exits", "files": "entity", "plugin": "fixture/exits" }]),
    );
    let failure = plugin_failure(&repo);
    assert!(
        failure["message"]
            .as_str()
            .unwrap_or_default()
            .contains("stopped"),
        "the message says the plugin process stopped: {failure}"
    );
}

#[test]
fn a_violation_outside_the_view_is_a_plugin_defect() {
    let repo = entities_importing_shared(&json!([{ "name": "outside", "files": "entity",
                               "plugin": "fixture/reports-outside",
                               "options": { "file": "src/shared/s.ts" } }]));
    let failure = plugin_failure(&repo);
    assert!(
        failure["message"]
            .as_str()
            .unwrap_or_default()
            .contains("src/shared/s.ts"),
        "the message names the file: {failure}"
    );
}

#[test]
fn a_plugin_violation_may_name_a_folder_that_holds_a_file_of_its_view() {
    let repo = entities_importing_shared(&json!([{ "name": "folder", "files": "entity",
                               "plugin": "fixture/reports-outside",
                               "options": { "file": "src/entities/user" } }]));
    let found = run(&repo, &["check", "--format", "json"]);
    assert_eq!(
        found.status, 1,
        "the violation fails the check: {}",
        found.stdout
    );
    let checked = document(&found.stdout);
    let at: Vec<(&str, &str)> = checked["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|d| {
            (
                d["code"].as_str().unwrap_or_default(),
                d["file"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(
        at,
        [("plugin", "src/entities/user")],
        "one violation of the folder"
    );
}

#[test]
fn a_plugin_violation_of_a_folder_outside_its_view_is_plugin_failed() {
    let repo = entities_importing_shared(&json!([{ "name": "folder", "files": "entity",
                               "plugin": "fixture/reports-outside",
                               "options": { "file": "src/shared" } }]));
    let failure = plugin_failure(&repo);
    assert!(
        failure["message"]
            .as_str()
            .unwrap_or_default()
            .contains("src/shared"),
        "the failure names the folder: {failure}"
    );
}

const PRINTED: &str = "[fixture] the fixture stops its process\n";

#[test]
fn what_a_plugin_printed_goes_to_standard_error_after_a_check() {
    let repo = entities_importing_shared(
        &json!([{ "name": "exits", "files": "entity", "plugin": "fixture/exits" }]),
    );
    let found = run(&repo, &["check"]);
    assert_eq!(
        found.stderr, PRINTED,
        "the line the plugin printed, with its name"
    );
}

#[test]
fn what_a_plugin_printed_goes_to_standard_error_after_its_examples() {
    let repo = entities_importing_shared(
        &json!([{ "name": "exits", "files": "entity", "plugin": "fixture/exits",
        "examples": { "rejected": [{ "src/entities/tag/index.ts": "export const tag = 1;\n" }] } }]),
    );
    let found = run(&repo, &["test"]);
    assert_eq!(
        found.stderr, PRINTED,
        "the line the plugin printed, with its name"
    );
}

#[test]
fn what_a_plugin_prints_as_its_process_ends_goes_to_standard_error() {
    let repo = entities_importing_shared(
        &json!([{ "name": "closing", "files": "entity", "plugin": "fixture/prints-when-closed" }]),
    );
    let found = run(&repo, &["check"]);
    assert_eq!(
        found.stderr, "[plugin process] the fixture prints as its input closes\n",
        "the check ends the process and waits for it, so the line printed last is not lost; outside a \
         check, no one plugin is known to have printed it"
    );
}

#[test]
fn a_line_a_plugin_writes_to_standard_output_goes_to_standard_error() {
    let repo = entities_importing_shared(&json!([{ "name": "raw", "files": "entity",
        "plugin": "fixture/writes-to-stdout", "options": { "specifier": "../../shared/s" } }]));
    let found = run(&repo, &["check"]);
    assert!(
        found
            .stderr
            .contains("[standard output] a line that is not the answer\n"),
        "a line written past the console is shown, not dropped: {}",
        found.stderr
    );
}

#[test]
fn a_plugin_that_names_its_schema_options_stops_the_check() {
    let repo = FixtureRepo::new()
        .file("src/entities/user/index.ts", "export const user = 1;\n")
        .file(
            "old-plugin.mjs",
            "export default { name: 'old', schemaVersion: 1, conditions: [{ name: 'stores', \
             description: '', reads: [], options: { type: 'object' }, check: () => [] }] };\n",
        )
        .file(
            "archangel.config.mjs",
            "import old from './old-plugin.mjs';\n\
             export default { elements: { entity: 'src/entities/{entity}/**' }, plugins: { old }, \
             rules: [] };\n",
        );
    let found = run(&repo, &["check"]);
    assert_eq!(
        found.status, 2,
        "the config cannot be used: {}",
        found.stderr
    );
    assert!(
        found
            .stderr
            .contains("plugins.old.conditions.stores.options")
            && found
                .stderr
                .contains("the JSON Schema of its options is `optionsSchema`"),
        "the problem names the field and its new name: {}",
        found.stderr
    );
}
