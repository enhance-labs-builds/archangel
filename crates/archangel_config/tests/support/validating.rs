//! Shared by the validation tests: a config with every kind of rule, and the elements that rules
//! of sentences name.
#![cfg(test)]

use archangel_config::{Config, parse_json};
use serde_json::{Value, json};

use crate::elements::declared_elements;

/// A config with each shorthand kind and each condition.
pub(crate) fn every_kind() -> Config {
    parse_json(
        &json!({
            "elements": declared_elements(),
            "selectors": { "test": "**/*.test.ts" },
            "rules": [
                { "kind": "layers", "order": ["entity", "shared"] },
                { "kind": "isolate", "element": "entity" },
                { "kind": "entry", "element": "entity", "files": ["index.ts"] },
                { "files": "shared", "mayOnlyBeImportedBy": ["entity"] },
                { "files": "entity", "mayNotImport": "react" },
                { "files": { "not": "test" }, "mayOnlyImport": ["shared", { "packages": ["zod"] }] },
                { "files": "src/**", "noCycles": true },
            ],
        })
        .to_string(),
    )
    .expect("every rule is valid")
}

pub(crate) fn sentence_elements() -> Value {
    json!({
        "feature": "src/features/{feature}/**",
        "view": "src/views/**",
        "shared": "src/shared/**",
        "db": "src/db/**",
        "api": "src/api/**",
    })
}
