//! Shared by the validation tests: the declared elements, and the problems of a config.
#![cfg(test)]

use archangel_config::{ConfigProblem, parse_json};
use serde_json::{Value, json};

pub(crate) fn problems(config: &Value) -> Vec<ConfigProblem> {
    parse_json(&config.to_string()).expect_err("the config has a problem")
}

pub(crate) fn only_problem(config: &Value) -> ConfigProblem {
    let mut found = problems(config);
    assert_eq!(found.len(), 1, "one problem, found {found:?}");
    found.remove(0)
}

pub(crate) fn declared_elements() -> Value {
    json!({
        "entity": "src/entities/{entity}/**",
        "shared": "src/shared/**",
        "react": { "packages": ["react"] },
    })
}
