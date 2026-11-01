//! Shared by the tests of import tables: the enhance3 slice, and what a check gives.
#![cfg(test)]

use archangel_rules::{Diagnostic, Report};
use archangel_testkit::{FixtureRepo, table};
use serde_json::{Value, json};

#[path = "check.rs"]
mod check;

/// The domain table of the enhance3 slice in the design, with its parts and groups.
pub(crate) fn slice() -> Value {
    json!({
        "parts": {
            "api": "apps/api/**",
            "contract": "packages/contract/**",
            "canvas-format": "packages/canvas-format/**",
        },
        "groups": {
            "tests": ["**/*.{test,spec}.{ts,tsx}"],
            "domain": "apps/api/src/*/domain/**",
        },
        "rules": [
            slice_table(&json!({})),
        ],
    })
}

pub(crate) fn slice_table(extra: &Value) -> Value {
    let mut options = json!({
        "files": { "all": ["domain", { "not": "tests" }] },
        "kinds": "**/*.{kind}.ts",
        "everyRowMayImport": ["apps/api/src/shared/result.ts"],
        "mayNotImportItself": ["aggregate-root", "domain-event"],
        "because": "Each kind of domain file depends only on the kinds below it.",
    });
    for (key, value) in extra.as_object().unwrap() {
        options[key] = value.clone();
    }
    table(
        "domain",
        &json!({
            "entity": ["value-object", "domain-event"],
            "aggregate-root": ["entity", "value-object"],
            "value-object": [],
            "domain-event": ["value-object"],
        }),
        &options,
    )
}

pub(crate) fn slice_repo() -> FixtureRepo {
    repo_of(&[
        "apps/api/src/shared/result.ts",
        "apps/api/src/board/application/save.use-case.ts",
        "packages/contract/src/canvas.ts",
        "packages/canvas-format/src/format.ts",
        "apps/api/src/board/domain/keep.value-object.ts",
    ])
}

/// A repository of empty files.
pub(crate) fn repo_of(paths: &[&str]) -> FixtureRepo {
    paths
        .iter()
        .fold(FixtureRepo::new(), |repo, path| repo.file(path, ""))
}

/// The violations of one rule.
pub(crate) fn of<'r>(report: &'r Report, rule: &str) -> Vec<&'r Diagnostic> {
    report
        .diagnostics
        .iter()
        .filter(|d| d.code.is_rule() && d.rule.as_deref() == Some(rule))
        .collect()
}

/// The rules that gave a violation, sorted, one for each violation.
pub(crate) fn violated(report: &Report) -> Vec<String> {
    let mut found: Vec<String> = report
        .diagnostics
        .iter()
        .filter(|d| d.code.is_rule())
        .filter_map(|d| d.rule.clone())
        .collect();
    found.sort();
    found
}

pub(crate) fn report_of(config: &Value, repo: &FixtureRepo) -> Report {
    check::check(config, repo, None).0
}

/// A repository that also has these packages in `node_modules`.
pub(crate) fn with_packages(repo: FixtureRepo, names: &[&str]) -> FixtureRepo {
    let mut repo = repo.file(".gitignore", "node_modules/\n");
    for name in names {
        repo = repo
            .file(
                &format!("node_modules/{name}/package.json"),
                &format!(r#"{{ "name": "{name}", "main": "index.js" }}"#),
            )
            .file(&format!("node_modules/{name}/index.js"), "")
            .file(&format!("node_modules/{name}/lib/client.js"), "");
    }
    repo
}
