//! Scenarios from the configuration spec about externals sets and areas: a name that could never
//! match, and an area that means nothing, are refused.
#![cfg(test)]

#[path = "support/elements.rs"]
mod elements;

use archangel_config::{ConfigProblem, ProblemCode, parse_json};
use elements::{declared_elements, only_problem, problems};
use serde_json::{Value, json};

/// The one problem of a config with these externals sets.
fn externals_problems(sets: &Value) -> Vec<ConfigProblem> {
    problems(&json!({ "parts": { "src": "src/**" }, "externals": sets }))
}

#[test]
fn an_externals_name_that_could_never_match_is_refused() {
    let found = externals_problems(&json!({ "a": ["fs"], "b": ["@types/react"] }));
    let fields: Vec<_> = found.iter().map(|p| p.field.as_deref()).collect();
    assert_eq!(
        fields,
        [Some("externals.a"), Some("externals.b")],
        "{found:?}"
    );
    assert!(
        found.iter().all(|p| p.code == ProblemCode::ConfigInvalid),
        "{found:?}"
    );
    assert!(
        found[0].message.contains("`node:fs`"),
        "{}",
        found[0].message
    );
    assert!(
        found[1]
            .message
            .contains("judges the package that an import names")
            && found[1].message.contains("`react`"),
        "{}",
        found[1].message
    );
}

#[test]
fn an_externals_name_of_a_workspace_package_is_refused_once_the_workspace_is_known() {
    let config = parse_json(
        &json!({
            "parts": { "src": "src/**" },
            "externals": { "c": ["@enhance/contract"], "d": ["pg"], "e": ["@enhance/*"] },
        })
        .to_string(),
    )
    .expect("the config alone cannot know a workspace's packages");
    let workspace = config.externals_in_workspace(|name| name == "@enhance/contract");
    assert_eq!(workspace.len(), 1, "{workspace:?}");
    assert_eq!(
        workspace[0].code,
        ProblemCode::ConfigInvalid,
        "{workspace:?}"
    );
    assert_eq!(
        workspace[0].field.as_deref(),
        Some("externals.c"),
        "{workspace:?}"
    );
    assert!(
        workspace[0]
            .message
            .contains("resolves to a repository file")
            && workspace[0].message.contains("its part judges it"),
        "{}",
        workspace[0].message
    );
}

#[test]
fn an_externals_name_with_a_subpath_is_refused_unless_it_is_a_built_in() {
    let found = externals_problems(&json!({
        "a": ["pg/lib/client"],
        "b": ["node:fs/promises"],
        "c": ["@acme/ui/button"],
    }));
    let fields: Vec<_> = found.iter().map(|p| p.field.as_deref()).collect();
    assert_eq!(
        fields,
        [Some("externals.a"), Some("externals.c")],
        "{found:?}"
    );
    assert!(found[0].message.contains("`pg`"), "{}", found[0].message);
    assert!(
        found[1].message.contains("`@acme/ui`"),
        "{}",
        found[1].message
    );
}

#[test]
fn an_externals_set_with_the_name_of_a_part_or_group_is_refused() {
    let group = only_problem(&json!({
        "parts": { "src": "src/**" },
        "groups": { "driver": "src/drivers/**" },
        "externals": { "driver": ["pg"] },
    }));
    assert_eq!(group.code, ProblemCode::ConfigInvalid, "{group:?}");
    assert_eq!(
        group.field.as_deref(),
        Some("externals.driver"),
        "{group:?}"
    );
    assert!(
        group.message.contains("is also a group"),
        "{}",
        group.message
    );
    let part = only_problem(&json!({
        "parts": { "driver": "src/drivers/**" },
        "externals": { "driver": ["pg"] },
    }));
    assert_eq!(part.field.as_deref(), Some("externals.driver"), "{part:?}");
    assert!(part.message.contains("is also a part"), "{}", part.message);
}

#[test]
fn parts_refuse_packages() {
    let found = only_problem(&json!({ "parts": { "react": { "packages": ["react"] } } }));
    assert_eq!(found.code, ProblemCode::ConfigInvalid, "{found:?}");
    assert_eq!(found.field.as_deref(), Some("parts.react"), "{found:?}");
    assert!(found.message.contains("`externals`"), "{}", found.message);
}

#[test]
fn within_on_a_condition_that_checks_no_import_is_refused() {
    let found = only_problem(&json!({
        "elements": declared_elements(),
        "rules": [{ "files": "src/**", "mustBeNamed": "kebab-case", "within": "src/**" }],
    }));
    assert_eq!(found.code, ProblemCode::ConfigInvalid, "{found:?}");
    assert_eq!(found.field.as_deref(), Some("rules[0].within"), "{found:?}");
    assert!(found.message.contains("`mustBeNamed`"), "{}", found.message);
    assert!(
        found.message.contains("checks no import"),
        "{}",
        found.message
    );
}

#[test]
fn within_on_may_not_import_is_refused_with_the_narrowed_side_as_the_fix() {
    let found = only_problem(&json!({
        "elements": { "entity": "src/entity/**", "use-case": "src/use-case/**" },
        "rules": [{ "files": "entity", "mayNotImport": "use-case", "within": "apps/api/src/**" }],
    }));
    assert_eq!(found.code, ProblemCode::ConfigInvalid, "{found:?}");
    assert_eq!(found.field.as_deref(), Some("rules[0].within"), "{found:?}");
    assert!(
        found
            .message
            .contains("mayNotImport: { all: ['use-case', 'apps/api/src/**'] }"),
        "{}",
        found.message
    );
}

#[test]
fn within_naming_an_undeclared_selector_is_refused_with_the_closest_name() {
    let found = only_problem(&json!({
        "elements": { "entity": "src/entity/**", "value-object": "src/value/**" },
        "selectors": { "api-src": "apps/api/src/**" },
        "rules": [{ "files": "entity", "mayOnlyImport": ["value-object"], "within": "api-scr" }],
    }));
    assert_eq!(found.code, ProblemCode::RuleUnknownElement, "{found:?}");
    assert_eq!(found.field.as_deref(), Some("rules[0].within"), "{found:?}");
    assert!(found.message.contains("api-src"), "{}", found.message);
}

#[test]
fn an_empty_area_is_refused() {
    for area in [json!([]), json!({ "any": [] })] {
        let found = only_problem(&json!({
            "elements": { "entity": "src/entity/**", "value-object": "src/value/**" },
            "rules": [{ "files": "entity", "mayOnlyImport": ["value-object"], "within": area }],
        }));
        assert_eq!(found.code, ProblemCode::ConfigInvalid, "{found:?}");
        assert!(
            found
                .field
                .as_deref()
                .is_some_and(|f| f.starts_with("rules[0].within")),
            "{found:?}"
        );
        assert!(
            found.message.contains("at least one value"),
            "{}",
            found.message
        );
    }
}

#[test]
fn an_empty_may_only_import_needs_an_area() {
    let found = only_problem(&json!({
        "elements": { "entity": "src/entity/**" },
        "rules": [{ "files": "entity", "mayOnlyImport": [] }],
    }));
    assert_eq!(found.code, ProblemCode::ConfigInvalid, "{found:?}");
    assert_eq!(
        found.field.as_deref(),
        Some("rules[0].mayOnlyImport"),
        "{found:?}"
    );
    assert!(found.message.contains("`within`"), "{}", found.message);
    let with_area = json!({
        "elements": { "entity": "src/entity/**" },
        "rules": [{ "files": "entity", "mayOnlyImport": [], "within": "src/**" }],
    });
    assert!(
        parse_json(&with_area.to_string()).is_ok(),
        "an empty list with an area"
    );
}
