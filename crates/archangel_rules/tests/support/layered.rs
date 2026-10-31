//! Shared by the rule and condition tests: one set of elements for every scenario, and a check of
//! a repository against rules over them.
#![cfg(test)]

use archangel_rules::Report;
use archangel_testkit::FixtureRepo;
use serde_json::{Value, json};

/// Elements for every rule and condition scenario: layers, instances, views and packages.
pub(crate) fn elements() -> Value {
    json!({
        "app": "src/app/**",
        "feature": "src/features/{feature}/**",
        "entity": "src/entities/{entity}/**",
        "view": "src/views/**/*.ts",
        "model": "src/model/**",
        "shared": "src/shared/**",
        "scripts": "scripts/**",
        "api": "api/**",
        "db": "db/**",
        "web": "web/**",
        "react": { "packages": ["react"] },
    })
}

/// The report of a check of the repository with the rules over [`elements`].
pub(crate) fn report(rules: &Value, repo: &FixtureRepo) -> Report {
    crate::support::check(
        &json!({ "elements": elements(), "rules": rules }),
        repo,
        None,
    )
    .0
}
