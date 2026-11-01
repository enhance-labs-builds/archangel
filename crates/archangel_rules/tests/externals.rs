//! Scenarios from the configuration spec: what an externals set names, and that an import counts as
//! the package it resolves to.
#![cfg(test)]

#[path = "support/check.rs"]
mod support;

use archangel_rules::{Code, Report};
use archangel_testkit::FixtureRepo;
use serde_json::{Value, json};

/// The specifier of each violation of the rule that refuses the set `name`.
fn refused(sets: &Value, name: &str, source: &str) -> Vec<String> {
    let repo = with_packages(FixtureRepo::new().file("src/a.ts", source));
    let config = json!({
        "parts": { "src": "src/**" },
        "externals": sets,
        "rules": [{ "name": "no-set", "files": "src/**", "mayNotImport": name }],
    });
    let (report, _) = support::check(&config, &repo, None);
    specifiers(&report)
}

/// The repository with the packages that the scenarios import.
fn with_packages(repo: FixtureRepo) -> FixtureRepo {
    [
        "pg/index.js",
        "pg/lib/client.js",
        "pg-format/index.js",
        "@aws-sdk/client-s3/index.js",
        "@aws-sdkx/y/index.js",
        "knex-utils/index.js",
        "react/index.js",
    ]
    .into_iter()
    .fold(repo, |repo, file| {
        repo.file(&format!("node_modules/{file}"), "")
    })
}

fn specifiers(report: &Report) -> Vec<String> {
    report
        .diagnostics
        .iter()
        .filter(|d| d.code == Code::MayNotImport)
        .map(|d| d.specifier.clone().unwrap_or_default())
        .collect()
}

#[test]
fn a_name_matches_its_package_and_every_subpath() {
    assert_eq!(
        refused(
            &json!({ "driver": ["pg"] }),
            "driver",
            "import 'pg';\nimport 'pg/lib/client';\nimport 'pg-format';\n",
        ),
        ["pg", "pg/lib/client"],
        "the package and its subpath, and not a package with a longer name"
    );
}

#[test]
fn node_names_match_the_built_in_with_and_without_the_prefix() {
    assert_eq!(
        refused(
            &json!({ "fs": ["node:fs", "node:fs/promises"] }),
            "fs",
            "import 'fs';\nimport 'node:fs';\nimport 'fs/promises';\n\
             import 'node:fs/promises';\nimport 'node:path';\n",
        ),
        ["fs", "node:fs", "fs/promises", "node:fs/promises"],
        "each of the first four imports"
    );
}

#[test]
fn a_scope_name_matches_every_package_in_the_scope() {
    assert_eq!(
        refused(
            &json!({ "aws": ["@aws-sdk/*"] }),
            "aws",
            "import '@aws-sdk/client-s3';\nimport '@aws-sdkx/y';\n",
        ),
        ["@aws-sdk/client-s3"],
        "one violation"
    );
}

#[test]
fn an_externals_name_is_a_package_pattern_as_in_0_1() {
    let source = "import 'node:path';\nimport 'knex-utils';\nimport 'react';\n";
    assert_eq!(
        refused(&json!({ "builtins": ["node:*"] }), "builtins", source),
        ["node:path"],
        "every built-in"
    );
    assert_eq!(
        refused(&json!({ "db": ["knex*"] }), "db", source),
        ["knex-utils"],
        "a wildcard in the name"
    );
    assert_eq!(
        refused(&json!({ "all": ["*"] }), "all", source),
        ["node:path", "knex-utils", "react"],
        "every package"
    );
}

#[test]
fn a_package_is_in_every_set_that_lists_it() {
    let repo = with_packages(FixtureRepo::new().file("src/a.ts", "import 'pg';\n"));
    let config = json!({
        "parts": { "src": "src/**" },
        "externals": { "driver": ["pg"], "database": ["pg", "knex"] },
        "rules": [
            { "name": "no-driver", "files": "src/**", "mayNotImport": "driver" },
            { "name": "no-database", "files": "src/**", "mayNotImport": "database" },
        ],
    });
    let (report, _) = support::check(&config, &repo, None);
    let rules: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.code == Code::MayNotImport)
        .map(|d| d.rule.clone().unwrap_or_default())
        .collect();
    assert_eq!(rules, ["no-database", "no-driver"], "both sets list pg");
}

#[test]
fn an_externals_name_of_a_workspace_package_is_config_invalid_in_a_check() {
    let repo = FixtureRepo::new()
        .file(
            "packages/contract/package.json",
            r#"{ "name": "@enhance/contract" }"#,
        )
        .file("src/a.ts", "");
    let config = json!({
        "parts": { "src": "src/**" },
        "externals": { "c": ["@enhance/contract"] },
        "rules": [{ "name": "no-c", "files": "src/**", "mayNotImport": "c" }],
    });
    let (report, _) = support::check(&config, &repo, None);
    let found: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.code == Code::ConfigInvalid)
        .collect();
    assert_eq!(found.len(), 1, "{:?}", report.diagnostics);
    assert!(
        found[0].message.contains("@enhance/contract")
            && found[0].message.contains("resolves to a repository file"),
        "{}",
        found[0].message
    );
}
