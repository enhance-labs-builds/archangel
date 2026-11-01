//! Scenarios from the import-tables spec about which files a table holds: source files and never
//! the config file; a preset of tables; and one instance from the values of the captures.
#![cfg(test)]

#[path = "support/tables.rs"]
mod support;

use archangel_rules::Report;
use archangel_testkit::{FixtureRepo, table};
use serde_json::{Value, json};
use support::{of, repo_of, report_of, violated};

fn codes(report: &Report) -> Vec<&'static str> {
    report.diagnostics.iter().map(|d| d.code.as_str()).collect()
}

/// The report of a check whose config file is at `config_file`.
fn report_with_config_file(config: &Value, repo: &FixtureRepo, config_file: &str) -> Report {
    let parsed = archangel_config::from_value(config).expect("a valid config");
    let mut work = archangel_graph::Work::default();
    let graph = archangel_graph::ImportGraph::build(
        repo.path(),
        &archangel_graph::GraphOptions::default(),
        &mut work,
    );
    archangel_rules::Architecture::compile(parsed)
        .with_config_file(config_file)
        .check(&graph, None, &mut work)
}

/// The report of a check of a config with a preset, which only a loaded config can have.
fn preset_report(config: &Value, repo: &FixtureRepo) -> Report {
    let parsed = archangel_config::from_value(config).expect("a valid config");
    let mut work = archangel_graph::Work::default();
    let graph = archangel_graph::ImportGraph::build(
        repo.path(),
        &archangel_graph::GraphOptions::default(),
        &mut work,
    );
    archangel_rules::Architecture::compile(parsed).check(&graph, None, &mut work)
}

fn in_a_preset(name: &str, rules: &Value) -> Value {
    json!({
        "extends": [{ "name": name, "package": format!("archangel-plugin-{name}"), "root": "", "rules": rules }],
        "rules": [],
    })
}

#[test]
fn a_preset_that_is_only_tables_and_has_no_file_is_preset_empty() {
    let repo = repo_of(&["src/a.entity.ts"]);
    let config = in_a_preset(
        "team",
        &json!([table(
            "roles",
            &json!({ "entity": [], "policy": [] }),
            &json!({ "kinds": "**/*.{kind}.tz" }),
        )]),
    );
    let report = preset_report(&config, &repo);
    assert!(
        codes(&report).contains(&"preset-empty"),
        "a table is a part of its preset, so an empty one fails: {:?}",
        report.diagnostics
    );
}

#[test]
fn a_preset_whose_table_has_a_file_is_not_preset_empty() {
    let repo = repo_of(&["src/a.entity.ts"]);
    let config = in_a_preset(
        "team",
        &json!([table(
            "roles",
            &json!({ "entity": [], "policy": [] }),
            &json!({ "kinds": "**/*.{kind}.ts" }),
        )]),
    );
    let report = preset_report(&config, &repo);
    assert!(
        !codes(&report).contains(&"preset-empty"),
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn a_table_whose_only_files_are_not_source_files_is_empty() {
    let repo = repo_of(&["docs/guide/a.md", "src/a.ts"]);
    let config = json!({
        "rules": [table("t", &json!({ "guide": [] }), &json!({ "kinds": "docs/{kind}/**" }))],
    });
    let report = report_of(&config, &repo);
    assert!(
        codes(&report).contains(&"table-empty"),
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn a_row_with_only_files_that_are_not_source_files_is_a_row_with_no_file() {
    let repo = repo_of(&["src/domain/a.ts", "src/assets/s.css"]);
    let config = json!({
        "rules": [table(
            "t",
            &json!({ "domain": [], "assets": [] }),
            &json!({ "kinds": "src/{kind}/**" }),
        )],
    });
    let report = report_of(&config, &repo);
    assert_eq!(
        report.summary.rows_with_no_file.as_deref(),
        Some(&["t.assets".to_owned()][..]),
        "only a source file counts"
    );
}

#[test]
fn a_file_that_is_not_source_needs_no_row() {
    let repo = repo_of(&["src/domain/a.ts", "src/docs/r.md", "src/tsconfig.json"]);
    let config = json!({
        "rules": [table(
            "t",
            &json!({ "domain": [] }),
            &json!({ "kinds": "src/{kind}/**", "files": "src/**" }),
        )],
    });
    let report = report_of(&config, &repo);
    assert_eq!(
        violated(&report),
        [] as [&str; 0],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn an_instance_comes_from_the_captures_across_two_patterns_of_a_part() {
    let repo = FixtureRepo::new()
        .file("a/foo/x.ts", "import '../../b/foo/y';\n")
        .file("b/foo/y.ts", "import '../../a/bar/z';\n")
        .file("a/bar/z.ts", "");
    let config = json!({
        "parts": { "f": ["a/{x}/**", "b/{x}/**"] },
        "rules": [table("t", &json!({ "f": [] }), &json!({}))],
    });
    let report = report_of(&config, &repo);
    let found = of(&report, "t.f");
    assert_eq!(found.len(), 1, "{:?}", report.diagnostics);
    assert_eq!(
        found[0].file.as_deref(),
        Some("b/foo/y.ts"),
        "foo in a and foo in b are one instance, and foo and bar are two"
    );
}

#[test]
fn may_only_import_and_the_instance_option_agree_on_an_instance() {
    let repo = FixtureRepo::new()
        .file("a/foo/x.ts", "import '../../b/foo/y';\n")
        .file("b/foo/y.ts", "");
    let config = json!({
        "parts": { "f": ["a/{x}/**", "b/{x}/**"] },
        "rules": [
            { "name": "only", "files": "f", "mayOnlyImport": ["other"] },
            { "name": "different", "files": "f", "mayNotImport": "f", "instance": "different" },
        ],
        "groups": { "other": "a/**" },
    });
    let report = report_of(&config, &repo);
    assert_eq!(
        violated(&report),
        [] as [&str; 0],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn the_config_file_is_never_in_a_table() {
    let repo = FixtureRepo::new()
        .file("src/a.entity.ts", "")
        .file("archangel.config.ts", "import './src/a.entity';\n");
    let config = json!({
        "rules": [table("t", &json!({ "entity": [], "config": [] }), &json!({ "kinds": "**/*.{kind}.ts" }))],
    });
    let report = report_with_config_file(&config, &repo, "archangel.config.ts");
    assert_eq!(
        violated(&report),
        [] as [&str; 0],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn the_config_file_needs_no_row() {
    let repo = FixtureRepo::new()
        .file("src/a.entity.ts", "")
        .file("archangel.config.ts", "");
    let config = json!({
        "rules": [table(
            "t",
            &json!({ "entity": [] }),
            &json!({ "kinds": "**/*.{kind}.ts", "files": "**" }),
        )],
    });
    let report = report_with_config_file(&config, &repo, "archangel.config.ts");
    assert_eq!(
        violated(&report),
        [] as [&str; 0],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn a_file_with_no_row_has_the_code_every_file_has_a_kind() {
    let repo = repo_of(&["src/domain/a.ts", "src/other/b.ts"]);
    let config = json!({
        "rules": [table(
            "t",
            &json!({ "domain": [] }),
            &json!({ "kinds": "src/{kind}/**", "files": "src/**" }),
        )],
    });
    let report = report_of(&config, &repo);
    assert_eq!(
        codes(&report),
        ["every-file-has-a-kind"],
        "{:?}",
        report.diagnostics
    );
    assert!(
        report.diagnostics[0].code.is_about_a_path(),
        "only the baseline can permit it"
    );
}
