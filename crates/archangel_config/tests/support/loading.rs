//! Shared by the tests of configs as the loader sends them: their one problem.
#![cfg(test)]

use archangel_config::{ConfigProblem, ProblemCode, from_value};
use serde_json::Value;

/// The one problem of a config as the loader sends it, which is config-invalid.
pub(crate) fn loader_problem(config: &Value) -> ConfigProblem {
    let mut found = from_value(config).expect_err("the config has a problem");
    assert_eq!(found.len(), 1, "one problem, found {found:?}");
    assert_eq!(found[0].code, ProblemCode::ConfigInvalid, "{found:?}");
    found.remove(0)
}
