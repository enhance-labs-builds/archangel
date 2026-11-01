//! Scenarios from the rule-examples spec about what proves a table: a row's refusal, and an import
//! that a row's limits leave in. A plain rule keeps its proof of 0.1.
#![cfg(test)]

use archangel_graph::{GraphOptions, ImportGraph, Work};
use archangel_rules::{Architecture, Code, ExampleKind, ExampleOutcome};
use archangel_testkit::FixtureRepo;
use serde_json::{Value, json};

fn graph_and_architecture(config: &Value, repo: &FixtureRepo) -> (ImportGraph, Architecture) {
    let config = archangel_config::parse_json(&config.to_string()).expect("a valid config");
    let graph = ImportGraph::build(repo.path(), &GraphOptions::default(), &mut Work::default());
    (graph, Architecture::compile(config))
}

/// The written examples of the first table.
fn table_outcomes(config: &Value, repo: &FixtureRepo) -> Vec<ExampleOutcome> {
    let (graph, architecture) = graph_and_architecture(config, repo);
    let mut work = Work::default();
    let table = &architecture.config().tables[0];
    let accepted = table
        .examples
        .accepted
        .iter()
        .map(|f| (ExampleKind::Accepted, f));
    let rejected = table
        .examples
        .rejected
        .iter()
        .map(|f| (ExampleKind::Rejected, f));
    let id = archangel_config::TableId(0);
    accepted
        .chain(rejected)
        .map(|(kind, files)| {
            let overlay = graph.overlay(files, &mut work);
            architecture.check_table_example(&graph, &overlay, id, kind, &mut work)
        })
        .collect()
}

/// The accepted examples of the first rule.
fn rule_outcomes(config: &Value, repo: &FixtureRepo) -> Vec<ExampleOutcome> {
    let (graph, architecture) = graph_and_architecture(config, repo);
    let mut work = Work::default();
    architecture.config().rules[0]
        .examples
        .accepted
        .iter()
        .map(|files| {
            let overlay = graph.overlay(files, &mut work);
            architecture.check_example(&graph, &overlay, 0, ExampleKind::Accepted, &mut work)
        })
        .collect()
}

#[test]
fn a_rejected_table_example_with_a_misspelt_path_fails() {
    let config = json!({
        "rules": [{
            "kind": "mayImport", "name": "t",
            "rows": { "infra": [], "domain": [] },
            "kinds": "src/{kind}/**",
            "examples": { "rejected": ["src/domian/a.ts imports src/infra/b.ts"] },
        }],
    });
    let repo = FixtureRepo::new().file("src/infra/c.ts", "");
    let outcomes = table_outcomes(&config, &repo);
    assert!(!outcomes[0].passed, "{outcomes:?}");
    let message = &outcomes[0].failure.as_ref().unwrap().message;
    assert!(
        message.contains("no row of \"t\" rejected") && message.contains("src/domian/a.ts"),
        "the message names the file that has no row: {message}"
    );
}

#[test]
fn an_accepted_table_example_that_every_row_skips_proves_nothing() {
    let config = json!({
        "rules": [{
            "kind": "mayImport", "name": "t",
            "rows": { "a": [], "b": [] },
            "kinds": "src/{kind}/**",
            "specifier": "parent",
            "examples": { "accepted": [{ "src/a/x.ts": "import './y';\n", "src/a/y.ts": "" }] },
        }],
    });
    let repo = FixtureRepo::new().file("src/b/c.ts", "");
    let outcomes = table_outcomes(&config, &repo);
    assert!(!outcomes[0].passed, "{outcomes:?}");
    assert_eq!(
        outcomes[0].failure.as_ref().unwrap().code,
        Code::ExampleProvesNothing,
        "{outcomes:?}"
    );
}

#[test]
fn a_plain_rule_keeps_its_accepted_example_whose_import_a_limit_skips() {
    let outcomes = rule_outcomes(
        &json!({
            "elements": { "app": "src/app/**", "shared": "src/shared/**" },
            "rules": [{
                "name": "r", "files": "shared", "mayNotImport": "app", "specifier": "parent",
                "examples": { "accepted": [{ "src/shared/x/a.ts": "import '@/app/b';\n" }] },
            }],
        }),
        &FixtureRepo::new().file("src/app/b.ts", "").file(
            "tsconfig.json",
            r#"{ "compilerOptions": { "baseUrl": ".", "paths": { "@/*": ["src/*"] } } }"#,
        ),
    );
    assert!(outcomes[0].passed, "as in 0.1: {outcomes:?}");
}
