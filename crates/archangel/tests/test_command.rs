//! Scenarios from the rule-examples spec that need the binary: `archangel test`, its exit status,
//! and that it writes no file.
#![cfg(test)]

#[path = "support/cli.rs"]
mod support;

use std::collections::BTreeMap;
use std::time::SystemTime;

use archangel_testkit::FixtureRepo;
use serde_json::{Value, json};
use support::run;

fn elements() -> Value {
    json!({ "app": "src/app/**", "shared": "src/shared/**" })
}

fn repo(rules: &Value) -> FixtureRepo {
    FixtureRepo::new()
        .file("src/app/main.ts", "")
        .file("src/shared/s.ts", "")
        .json_config(&json!({ "elements": elements(), "rules": rules }))
}

fn passing() -> Value {
    json!([
        { "kind": "layers", "name": "layers", "order": ["app", "shared"], "examples": {
            "accepted": [{ "src/app/x.ts": "import '../shared/s';\n" }],
            "rejected": [{ "src/shared/x.ts": "import '../app/main';\n" }],
        } },
        { "name": "forbid", "files": "app", "mayNotImport": "app", "examples": {
            "accepted": [{ "src/app/y.ts": "import '../shared/s';\n" }],
            "rejected": [{ "src/app/y.ts": "import './main';\n" }],
        } },
    ])
}

#[test]
fn all_examples_pass_with_status_zero() {
    let found = run(&repo(&passing()), &["test"]);
    assert_eq!(
        found.status, 0,
        "every example passes: {}{}",
        found.stdout, found.stderr
    );
    assert_eq!(
        found.stdout,
        "pass  layers  accepted #1  src/app/x.ts\n\
         pass  layers  rejected #1  src/shared/x.ts\n\
         pass  forbid  accepted #1  src/app/y.ts\n\
         pass  forbid  rejected #1  src/app/y.ts\n\
         4 examples, none failed; 0 rules without a rejected example\n",
        "a line per example, then the count"
    );
}

/// Three examples of which one fails, and two rules without a rejected example.
fn one_failed_two_unproven() -> FixtureRepo {
    repo(&json!([
        { "kind": "layers", "name": "layers", "order": ["app", "shared"], "examples": {
            "accepted": [{ "src/app/x.ts": "import '../shared/s';\n" }],
            "rejected": [
                { "src/shared/x.ts": "import '../app/main';\n" },
                { "src/shared/y.ts": "import './s';\n" },
            ],
        } },
        { "name": "forbid", "files": "app", "mayNotImport": "app" },
        { "name": "forbid2", "files": "shared", "mayNotImport": "app" },
    ]))
}

const FAILED: &str =
    "no rule rejected the example, and \"layers\" should have; fix the rule or the example";

fn unproven(rule: &str) -> String {
    format!(
        "the rule \"{rule}\" has no rejected example, so nothing shows that it can find a violation; add one under examples.rejected"
    )
}

#[test]
fn test_ends_with_how_many_examples_failed_and_rules_are_unproven() {
    let found = run(&one_failed_two_unproven(), &["test"]);
    assert_eq!(found.status, 1, "a failed example exits 1");
    assert_eq!(
        found.stdout.lines().last(),
        Some("3 examples, 1 failed; 2 rules without a rejected example"),
        "the count of failed examples, not of passed ones: {}",
        found.stdout
    );
}

#[test]
fn test_github_format_annotates_each_failure_then_counts() {
    let found = run(&one_failed_two_unproven(), &["test", "--format", "github"]);
    assert_eq!(found.status, 1, "a failed example exits 1");
    assert_eq!(
        found.stdout,
        format!(
            "::error title=archangel layers::{FAILED}\n\
             ::error title=archangel forbid::{}\n\
             ::error title=archangel forbid2::{}\n\
             3 examples, 1 failed; 2 rules without a rejected example\n",
            unproven("forbid"),
            unproven("forbid2")
        ),
        "the failed example, each unproven rule, and the count"
    );
}

#[test]
fn test_sarif_format_has_a_result_for_each_failure() {
    let found = run(&one_failed_two_unproven(), &["test", "--format", "sarif"]);
    assert_eq!(found.status, 1, "a failed example exits 1");
    let log: Value = serde_json::from_str(&found.stdout)
        .unwrap_or_else(|error| panic!("a SARIF log: {error}\n{}", found.stdout));
    let results: Vec<(String, String, String)> = log["runs"][0]["results"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|result| {
            (
                result["ruleId"].as_str().unwrap_or_default().to_owned(),
                result["properties"]["rule"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
                result["message"]["text"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
            )
        })
        .collect();
    assert_eq!(
        results,
        [
            (
                "example-failed".to_owned(),
                "layers".to_owned(),
                FAILED.to_owned()
            ),
            (
                "rule-unproven".to_owned(),
                "forbid".to_owned(),
                unproven("forbid")
            ),
            (
                "rule-unproven".to_owned(),
                "forbid2".to_owned(),
                unproven("forbid2")
            ),
        ],
        "the failed example, then each unproven rule: {}",
        found.stdout
    );
}

#[test]
fn failed_example_gives_status_one() {
    let rules = json!([{ "kind": "layers", "name": "layers", "order": ["app", "shared"], "examples": {
        "rejected": [{ "src/shared/x.ts": "import './s';\n" }],
    } }]);
    let found = run(&repo(&rules), &["test"]);
    assert_eq!(found.status, 1, "a failed example exits 1");
    let line = found
        .stdout
        .lines()
        .find(|l| l.contains("fail"))
        .unwrap_or_default();
    assert!(
        line.contains("src/shared/x.ts"),
        "the failed line names the example's file: {}",
        found.stdout
    );
}

#[test]
fn failed_example_shows_its_own_text_where_it_replaces_a_repository_file() {
    let rules = json!([
        { "kind": "layers", "name": "layers", "order": ["app", "shared"] },
        { "name": "forbid", "files": "shared", "mayNotImport": "shared", "examples": {
            "rejected": [{ "src/shared/s.ts": "import { main } from '../app/main';\n" }],
        } },
    ]);
    let found = run(&repo(&rules), &["test"]);
    assert_eq!(
        (found.status, found.stdout.as_str()),
        (
            1,
            "fail  forbid  rejected #1  src/shared/s.ts\n      \
             \"layers\" rejected the example, but \"forbid\" did not; make the example break \
             \"forbid\" alone\n\
             error[layers]: \"shared\" may not import \"app\", an earlier layer, through \
             \"../app/main\"\n \
             --> src/shared/s.ts:1:22\n  \
             |\n\
             1 | import { main } from '../app/main';\n  \
             |                      ^^^^^^^^^^^^^ \"shared\" may import only \"shared\"\n\n\
             error[rule-unproven]: the rule \"layers\" has no rejected example, so nothing shows \
             that it can find a violation; add one under examples.rejected\n \
             |\n \
             = note: rule: layers\n\n\
             1 examples, 1 failed; 1 rules without a rejected example\n"
        ),
        "the mark is under the example's import, not the empty repository file it replaces: {}",
        found.stderr
    );
}

#[test]
fn rule_without_a_rejected_example_is_rule_unproven() {
    let rules = json!([{ "name": "forbid", "files": "app", "mayNotImport": "app", "examples": {
        "accepted": [{ "src/app/y.ts": "import '../shared/s';\n" }],
    } }]);
    let found = run(&repo(&rules), &["test"]);
    assert_eq!(found.status, 1, "an unproven rule exits 1");
    assert!(
        found.stdout.contains("rule-unproven") && found.stdout.contains("\"forbid\""),
        "it names the rule: {}",
        found.stdout
    );
}

/// The bytes and modification time of every file in the repository.
fn snapshot(repo: &FixtureRepo) -> BTreeMap<String, (Vec<u8>, SystemTime)> {
    let mut found = BTreeMap::new();
    let mut folders = vec![repo.path().to_path_buf()];
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(folder).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                folders.push(path);
            } else {
                let meta = std::fs::metadata(&path).unwrap();
                found.insert(
                    path.display().to_string(),
                    (std::fs::read(&path).unwrap(), meta.modified().unwrap()),
                );
            }
        }
    }
    found
}

/// Ten examples, one of which replaces a repository file.
fn ten() -> FixtureRepo {
    let rejected: Vec<Value> = (0..9)
        .map(|i| json!({ format!("src/shared/e{i}.ts"): "import '../app/main';\n" }))
        .chain(std::iter::once(
            json!({ "src/shared/s.ts": "import '../app/main';\n" }),
        ))
        .collect();
    repo(
        &json!([{ "kind": "layers", "name": "layers", "order": ["app", "shared"], "examples": { "rejected": rejected } }]),
    )
}

/// The files written by `archangel test --format json`, from its work counts.
fn files_written(found: &support::Finished) -> Option<u64> {
    let document: Value = serde_json::from_str(&found.stdout).ok()?;
    document["summary"]["work"]["filesWritten"].as_u64()
}

#[test]
fn test_command_writes_no_file() {
    let repo = ten();
    let before = snapshot(&repo);
    let found = run(&repo, &["test", "--format", "json"]);
    assert_eq!(found.status, 0, "the ten examples pass: {}", found.stdout);
    assert_eq!(
        snapshot(&repo),
        before,
        "every file keeps its bytes and modification time"
    );
    assert_eq!(
        files_written(&found),
        Some(0),
        "the count of files written: {}",
        found.stdout
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn files_written_count_rejects_examples_on_disk() {
    let repo = ten();
    let found: support::Finished = support::command(&repo)
        .args(["test", "--format", "json"])
        .env("FAILPOINTS", "engine::write_examples=return")
        .output()
        .unwrap()
        .into();
    assert_ne!(
        files_written(&found),
        Some(0),
        "the count catches files written: {}",
        found.stdout
    );
}
