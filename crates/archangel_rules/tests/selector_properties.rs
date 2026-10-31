//! Selector membership agrees with a slow reference that matches each pattern on its own and
//! combines the results as the definition says, for random selectors and random paths.
#![cfg(test)]

use archangel_config::Pattern;
use archangel_graph::{GraphOptions, ImportGraph, Work};
use archangel_rules::Architecture;
use archangel_testkit::FixtureRepo;
use proptest::prelude::*;
use serde_json::{Value, json};

const PATTERNS: [&str; 6] = [
    "src/**",
    "**/*.test.ts",
    "src/{area}/**",
    "src/a/*.ts",
    "lib/**/x.ts",
    "**/b/**",
];

const PATHS: [&str; 8] = [
    "src/a/x.ts",
    "src/a/x.test.ts",
    "src/b/y.ts",
    "src/b/deep/x.ts",
    "lib/x.ts",
    "lib/c/x.ts",
    "lib/b/z.test.ts",
    "other.ts",
];

/// A selector definition over the patterns, the element `src`, any element, and the selectors
/// declared before it.
fn definition(earlier: usize) -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        (0..PATTERNS.len()).prop_map(|i| json!(PATTERNS[i])),
        Just(json!({ "element": "src" })),
        Just(json!({ "element": "*" })),
        (0..earlier.max(1)).prop_map(move |i| if earlier == 0 {
            json!("src/**")
        } else {
            json!(format!("s{i}"))
        }),
    ];
    leaf.prop_recursive(3, 12, 3, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 1..3).prop_map(|all| json!({ "all": all })),
            prop::collection::vec(inner.clone(), 1..3).prop_map(|any| json!({ "any": any })),
            inner.clone().prop_map(|not| json!({ "not": not })),
            prop::collection::vec(inner, 1..3).prop_map(Value::Array),
        ]
    })
}

fn selectors() -> impl Strategy<Value = Vec<Value>> {
    (definition(0), definition(1), definition(2)).prop_map(|(a, b, c)| vec![a, b, c])
}

/// Whether a path is in a definition, by matching each pattern alone.
fn reference(definition: &Value, path: &str, declared: &[Value]) -> bool {
    match definition {
        Value::String(text) => match text.strip_prefix('s').and_then(|i| i.parse::<usize>().ok()) {
            Some(index) => reference(&declared[index], path, declared),
            None => Pattern::parse(text).unwrap().matches(path).is_some(),
        },
        Value::Array(items) => items.iter().any(|item| reference(item, path, declared)),
        Value::Object(object) => {
            if let Some(Value::Array(all)) = object.get("all") {
                all.iter().all(|item| reference(item, path, declared))
            } else if let Some(Value::Array(any)) = object.get("any") {
                any.iter().any(|item| reference(item, path, declared))
            } else if let Some(not) = object.get("not") {
                !reference(not, path, declared)
            } else {
                // `{ element: 'src' }` and `{ element: '*' }`: the only element is `src/**`.
                path.starts_with("src/")
            }
        }
        _ => false,
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn selector_membership_agrees_with_the_reference_matcher(definitions in selectors()) {
        let mut repo = FixtureRepo::new();
        for path in PATHS {
            repo = repo.file(path, "");
        }
        let declared: serde_json::Map<String, Value> = definitions
            .iter()
            .enumerate()
            .map(|(i, definition)| (format!("s{i}"), definition.clone()))
            .collect();
        let config = json!({ "elements": { "src": "src/**" }, "selectors": declared });
        let config = archangel_config::parse_json(&config.to_string()).expect("a valid config");
        let graph = ImportGraph::build(repo.path(), &GraphOptions::default(), &mut Work::default());
        let architecture = Architecture::compile(config);
        for path in PATHS {
            let expected: Vec<String> = (0..definitions.len())
                .filter(|&i| reference(&definitions[i], path, &definitions))
                .map(|i| format!("s{i}"))
                .collect();
            prop_assert_eq!(architecture.selectors_of(&graph, path), expected, "the selectors of {}", path);
        }
    }
}
