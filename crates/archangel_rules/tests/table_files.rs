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

/// A check of a graph with the classes that a session keeps from one check to the next.
fn check_again(
    architecture: &archangel_rules::Architecture,
    graph: &archangel_graph::ImportGraph,
    classes: &mut archangel_rules::Classes,
) -> Report {
    let mut work = archangel_graph::Work::default();
    let checks = architecture.check_files(graph, classes, None, &mut work);
    let structure = architecture.structure(graph, classes, &mut work);
    let results = archangel_rules::Results {
        checks: &checks,
        structure: &structure,
        plugin: &[],
    };
    architecture.assemble(graph, classes, results, None, &mut work)
}

/// A table with one kind, and a rule that gives a file with a directive another class.
fn table_and_directive() -> archangel_rules::Architecture {
    let config = json!({
        "parts": { "x": "lib/**" },
        "rules": [
            table("t", &json!({ "entity": [] }), &json!({ "kinds": "**/*.{kind}.ts" })),
            { "name": "client", "files": { "directive": "use client" }, "mayNotImport": "x" },
        ],
    });
    archangel_rules::Architecture::compile(
        archangel_config::from_value(&config).expect("a valid config"),
    )
}

#[test]
fn a_table_becomes_empty_when_its_last_file_goes_in_a_session() {
    let architecture = table_and_directive();
    let repo = repo_of(&["src/a.entity.ts", "lib/b.ts"]);
    let mut work = archangel_graph::Work::default();
    let mut graph = archangel_graph::ImportGraph::build(
        repo.path(),
        &archangel_graph::GraphOptions::default(),
        &mut work,
    );
    let mut classes = archangel_rules::Classes::new();
    classes.sync(&architecture, &graph, &mut work);
    let before = check_again(&architecture, &graph, &mut classes);
    assert!(
        !codes(&before).contains(&"table-empty"),
        "{:?}",
        before.diagnostics
    );

    graph.apply_file_events(&[], &["src/a.entity.ts".to_owned()], &mut work);
    classes.sync(&architecture, &graph, &mut work);
    let after = check_again(&architecture, &graph, &mut classes);
    assert!(
        codes(&after).contains(&"table-empty"),
        "{:?}",
        after.diagnostics
    );
}

#[test]
fn a_file_that_moves_to_another_class_is_counted_once_for_its_table() {
    let architecture = table_and_directive();
    let repo = repo_of(&["src/a.entity.ts", "lib/b.ts"]);
    let mut work = archangel_graph::Work::default();
    let mut graph = archangel_graph::ImportGraph::build(
        repo.path(),
        &archangel_graph::GraphOptions::default(),
        &mut work,
    );
    let mut classes = archangel_rules::Classes::new();
    classes.sync(&architecture, &graph, &mut work);

    graph.replace_file("src/a.entity.ts", Some("'use client';\n"), &mut work);
    let lookup = archangel_rules::Lookup::new(&architecture, &graph);
    assert!(
        classes.reclassify(lookup, "src/a.entity.ts", &mut work),
        "the directive moves the file"
    );

    graph.apply_file_events(&[], &["src/a.entity.ts".to_owned()], &mut work);
    classes.sync(&architecture, &graph, &mut work);
    let after = check_again(&architecture, &graph, &mut classes);
    assert!(
        codes(&after).contains(&"table-empty"),
        "{:?}",
        after.diagnostics
    );
}

#[test]
fn only_a_kind_row_with_no_file_is_listed_as_having_no_file() {
    let repo = repo_of(&["src/a.has.ts"]);
    let config = json!({
        "parts": { "empty": "gone/**" },
        "rules": [table(
            "t",
            &json!({ "has": [], "none": [], "empty": [] }),
            &json!({ "kinds": "**/*.{kind}.ts" }),
        )],
    });
    let report = report_of(&config, &repo);
    assert_eq!(
        report.summary.rows_with_no_file.as_deref(),
        Some(&["t.none".to_owned()][..])
    );
}

#[test]
fn a_table_is_empty_even_when_another_rule_has_a_pattern_that_matches_nothing() {
    let repo = repo_of(&["src/a.ts"]);
    let config = json!({
        "rules": [
            table("t", &json!({ "guide": [] }), &json!({ "kinds": "docs/{kind}/**", "files": "src/**" })),
            { "name": "other", "files": "nothing/**", "mustBeNamed": "kebab-case" },
        ],
    });
    let found = codes(&report_of(&config, &repo));
    assert!(found.contains(&"table-empty"), "{found:?}");
    assert!(found.contains(&"selector-empty"), "{found:?}");
}

#[test]
fn a_table_whose_files_are_an_empty_group_gives_only_selector_empty() {
    let repo = repo_of(&["src/a.entity.ts"]);
    let config = json!({
        "parts": { "src": "src/**" },
        "groups": { "here": "src/**", "gone": "gone/**" },
        "rules": [table("t", &json!({ "entity": [] }), &json!({ "kinds": "docs/{kind}/**", "files": "gone" }))],
    });
    let found = codes(&report_of(&config, &repo));
    assert!(found.contains(&"selector-empty"), "{found:?}");
    assert!(!found.contains(&"table-empty"), "{found:?}");
}

#[test]
fn a_table_whose_files_are_a_group_with_files_can_still_be_empty() {
    let repo = repo_of(&["src/a.entity.ts"]);
    let config = json!({
        "parts": { "src": "src/**" },
        "groups": { "gone": "gone/**", "here": "src/**" },
        "rules": [table("t", &json!({ "entity": [] }), &json!({ "kinds": "docs/{kind}/**", "files": "here" }))],
    });
    let found = codes(&report_of(&config, &repo));
    assert!(found.contains(&"table-empty"), "{found:?}");
}

#[test]
fn a_table_of_the_config_does_not_keep_a_preset_from_being_empty() {
    let repo = repo_of(&["src/a.entity.ts"]);
    let mut config = in_a_preset(
        "team",
        &json!([table(
            "roles",
            &json!({ "entity": [] }),
            &json!({ "kinds": "docs/{kind}/**" })
        )]),
    );
    config["rules"] = json!([table(
        "own",
        &json!({ "entity": [] }),
        &json!({ "kinds": "**/*.{kind}.ts" })
    )]);
    let found = codes(&preset_report(&config, &repo));
    assert!(found.contains(&"preset-empty"), "{found:?}");
}

#[test]
fn an_instance_limit_on_a_table_with_kinds_compares_the_other_captures() {
    let repo = FixtureRepo::new()
        .file(
            "src/cart/a.views.ts",
            "import './b.api';\nimport '../login/c.api';\n",
        )
        .file("src/cart/b.api.ts", "")
        .file("src/login/c.api.ts", "");
    let lines = |instance: &str| -> Vec<u32> {
        let config = json!({
            "rules": [table(
                "t",
                &json!({ "views": [], "api": [] }),
                &json!({ "kinds": "src/{feature}/*.{kind}.ts", "instance": instance }),
            )],
        });
        report_of(&config, &repo)
            .diagnostics
            .iter()
            .filter(|d| d.rule.as_deref() == Some("t.views"))
            .map(|d| d.line)
            .collect()
    };
    assert_eq!(lines("same"), [1], "the import in one instance");
    assert_eq!(lines("different"), [2], "the import between two instances");
}

#[test]
fn a_row_key_that_means_a_part_or_a_group_says_which_it_means() {
    let repo = repo_of(&[
        "src/x.api.ts",
        "src/y.gate.ts",
        "apps/api/a.ts",
        "gates/g.ts",
    ]);
    let config = json!({
        "parts": { "api": "apps/api/**", "src": "src/**" },
        "groups": { "gate": "gates/**" },
        "rules": [table(
            "t",
            &json!({ "api": [], "gate": [] }),
            &json!({ "kinds": "**/*.{kind}.ts", "files": "src" }),
        )],
    });
    let report = report_of(&config, &repo);
    let message = |file: &str| {
        report
            .diagnostics
            .iter()
            .find(|d| d.file.as_deref() == Some(file))
            .unwrap_or_else(|| panic!("no {file}: {:?}", report.diagnostics))
            .message
            .clone()
    };
    let api = message("src/x.api.ts");
    assert!(
        api.contains("means the part apps/api/**, not the kind"),
        "{api}"
    );
    let gate = message("src/y.gate.ts");
    assert!(
        gate.contains("means the group gates/**, not the kind"),
        "{gate}"
    );
}

/// A rule on the files of `src`, and a rule that gives a file with a directive another class.
fn set_and_directive() -> archangel_rules::Architecture {
    let config = json!({
        "parts": { "x": "lib/**" },
        "rules": [
            { "name": "r", "files": "src/**", "mayNotImport": "x" },
            { "name": "client", "files": { "directive": "use client" }, "mayNotImport": "x" },
        ],
    });
    archangel_rules::Architecture::compile(
        archangel_config::from_value(&config).expect("a valid config"),
    )
}

fn has_selector_empty_for_src(report: &Report) -> bool {
    report
        .diagnostics
        .iter()
        .any(|d| d.code.as_str() == "selector-empty" && d.message.contains("src/**"))
}

#[test]
fn a_set_becomes_empty_when_its_last_file_goes_in_a_session() {
    let architecture = set_and_directive();
    let repo = repo_of(&["src/a.ts", "lib/b.ts"]);
    let mut work = archangel_graph::Work::default();
    let mut graph = archangel_graph::ImportGraph::build(
        repo.path(),
        &archangel_graph::GraphOptions::default(),
        &mut work,
    );
    let mut classes = archangel_rules::Classes::new();
    classes.sync(&architecture, &graph, &mut work);
    let before = check_again(&architecture, &graph, &mut classes);
    assert!(
        !has_selector_empty_for_src(&before),
        "{:?}",
        before.diagnostics
    );

    graph.apply_file_events(&[], &["src/a.ts".to_owned()], &mut work);
    classes.sync(&architecture, &graph, &mut work);
    let after = check_again(&architecture, &graph, &mut classes);
    assert!(
        has_selector_empty_for_src(&after),
        "{:?}",
        after.diagnostics
    );
}

#[test]
fn a_file_that_moves_to_another_class_is_counted_once_for_its_set() {
    let architecture = set_and_directive();
    let repo = repo_of(&["src/a.ts", "lib/b.ts"]);
    let mut work = archangel_graph::Work::default();
    let mut graph = archangel_graph::ImportGraph::build(
        repo.path(),
        &archangel_graph::GraphOptions::default(),
        &mut work,
    );
    let mut classes = archangel_rules::Classes::new();
    classes.sync(&architecture, &graph, &mut work);

    graph.replace_file("src/a.ts", Some("'use client';\n"), &mut work);
    let lookup = archangel_rules::Lookup::new(&architecture, &graph);
    assert!(
        classes.reclassify(lookup, "src/a.ts", &mut work),
        "the directive moves the file"
    );

    graph.apply_file_events(&[], &["src/a.ts".to_owned()], &mut work);
    classes.sync(&architecture, &graph, &mut work);
    let after = check_again(&architecture, &graph, &mut classes);
    assert!(
        has_selector_empty_for_src(&after),
        "{:?}",
        after.diagnostics
    );
}
