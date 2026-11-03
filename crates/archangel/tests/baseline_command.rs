//! Scenarios from the exceptions-and-baselines spec that need the binary: `baseline add`,
//! `baseline prune`, and a baseline that does not parse.
#![cfg(test)]

#[path = "support/cli.rs"]
mod support;

use archangel_testkit::FixtureRepo;
use serde_json::{Value, json};
use support::run;

const BASELINE: &str = "archangel.baseline.json";

fn repo() -> FixtureRepo {
    FixtureRepo::new()
        .file("src/app/main.ts", "")
        .file("src/shared/s.ts", "")
        .file("src/shared/c.ts", "import '../app/main';\n")
        .file("src/shared/a.ts", "import '../app/main';\n")
        .file("src/shared/b.ts", "import '../app/main';\n")
        .json_config(&json!({
            "elements": { "app": "src/app/**", "shared": "src/shared/**" },
            "rules": [{ "kind": "layers", "name": "layers", "order": ["app", "shared"] }],
        }))
}

fn baseline(repo: &FixtureRepo) -> Value {
    serde_json::from_str(&repo.read(BASELINE)).unwrap()
}

fn files(baseline: &Value) -> Vec<String> {
    baseline["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["file"].as_str().unwrap_or_default().to_owned())
        .collect()
}

#[test]
fn baseline_add_records_every_violation_sorted() {
    let repo = repo();
    assert_eq!(
        run(&repo, &["baseline", "add"]).status,
        0,
        "the baseline is written"
    );
    let written = baseline(&repo);
    assert_eq!(
        files(&written),
        ["src/shared/a.ts", "src/shared/b.ts", "src/shared/c.ts"],
        "sorted by file"
    );
    let entry = &written["entries"][0];
    assert_eq!(
        (
            entry["rule"].as_str(),
            entry["specifier"].as_str(),
            entry["count"].as_u64(),
            entry.get("line")
        ),
        (Some("layers"), Some("../app/main"), Some(1), None),
        "rule, specifier and count, and no line: {written}"
    );
}

#[test]
fn baseline_add_two_times_writes_the_same_bytes() {
    let repo = repo();
    run(&repo, &["baseline", "add"]);
    let first = repo.read(BASELINE);
    run(&repo, &["baseline", "add"]);
    assert_eq!(repo.read(BASELINE), first, "the same bytes");
}

#[test]
fn check_does_not_write_the_baseline() {
    let repo = repo();
    run(&repo, &["baseline", "add"]);
    let repo = repo.file("src/shared/a.ts", "");
    let before = repo.read(BASELINE);
    let found = run(&repo, &["check"]);
    assert_eq!(found.status, 1, "the fixed entry is stale");
    assert!(
        found.stdout.contains("baseline-stale")
            && found.stdout.contains("archangel baseline prune"),
        "it names the command: {}",
        found.stdout
    );
    assert_eq!(repo.read(BASELINE), before, "the baseline keeps its bytes");
}

#[test]
fn new_violation_beside_a_baseline_exits_one() {
    let repo = repo();
    run(&repo, &["baseline", "add"]);
    assert_eq!(
        run(&repo, &["check"]).status,
        0,
        "the baseline permits the three"
    );
    let repo = repo.file("src/shared/d.ts", "import '../app/main';\n");
    let found = run(&repo, &["check"]);
    assert_eq!(found.status, 1, "a new violation fails");
    assert!(
        found.stdout.contains("src/shared/d.ts"),
        "it is printed: {}",
        found.stdout
    );
}

#[test]
fn prune_removes_fixed_entries_and_adds_nothing() {
    let repo = repo();
    run(&repo, &["baseline", "add"]);
    let repo = repo
        .file("src/shared/a.ts", "")
        .file("src/shared/d.ts", "import '../app/main';\n");
    assert_eq!(
        run(&repo, &["baseline", "prune"]).status,
        0,
        "prune succeeds"
    );
    assert_eq!(
        files(&baseline(&repo)),
        ["src/shared/b.ts", "src/shared/c.ts"],
        "a.ts is gone and d.ts is not added"
    );
}

#[test]
fn baseline_that_does_not_parse_is_baseline_invalid() {
    let repo = repo().file(
        BASELINE,
        "{\n  \"entries\": [\n    { \"rule\": \"layers\" \"count\": 1 }\n  ]\n}\n",
    );
    let found = run(&repo, &["check"]);
    assert_eq!(found.status, 2, "an invalid baseline exits 2");
    assert!(
        found.stderr.contains("baseline-invalid")
            && found.stderr.contains(&format!("{BASELINE}:3:")),
        "it gives line 3: {}",
        found.stderr
    );
}

/// A domain table and a rule, with one entity that breaks both.
fn row_repo() -> FixtureRepo {
    FixtureRepo::new()
        .file("apps/api/src/b/application/save.use-case.ts", "")
        .file("apps/api/src/b/domain/a.value-object.ts", "")
        .file(
            "apps/api/src/b/domain/board.entity.ts",
            "import '../application/save.use-case';\n",
        )
        .json_config(&json!({
            "parts": { "api": "apps/api/**" },
            "groups": { "domain": "apps/api/src/*/domain/**", "application": "apps/api/src/*/application/**" },
            "rules": [
                {
                    "kind": "mayImport",
                    "name": "domain",
                    "rows": { "entity": ["value-object"], "value-object": [] },
                    "files": "domain",
                    "kinds": "**/*.{kind}.ts",
                },
                { "name": "domain-not-to-application", "files": "domain", "mayNotImport": "application" },
            ],
        }))
}

fn entries(repo: &FixtureRepo) -> Vec<(String, u64)> {
    let mut found: Vec<(String, u64)> = baseline(repo)["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                e["rule"].as_str().unwrap().to_owned(),
                e["count"].as_u64().unwrap(),
            )
        })
        .collect();
    found.sort();
    found
}

#[test]
fn a_baseline_entry_records_the_name_of_a_table_row() {
    let repo = row_repo();
    assert_eq!(
        run(&repo, &["baseline", "add"]).status,
        0,
        "a baseline entry records the name of a table row"
    );
    assert!(
        entries(&repo)
            .iter()
            .any(|(rule, _)| rule == "domain.entity"),
        "{:?}",
        entries(&repo)
    );
}

#[test]
fn a_baseline_records_one_entry_for_each_rule_that_an_import_breaks() {
    let repo = row_repo();
    assert_eq!(
        run(&repo, &["baseline", "add"]).status,
        0,
        "a baseline records one entry for each rule that an import breaks"
    );
    assert_eq!(
        entries(&repo),
        [
            ("domain-not-to-application".to_owned(), 1),
            ("domain.entity".to_owned(), 1),
        ],
        "a baseline records one entry for each rule that an import breaks"
    );
}
