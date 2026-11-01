//! Scenarios from the import-tables spec about loading: what a table may name, and which names it
//! refuses.
#![cfg(test)]

use archangel_config::{ConfigProblem, ProblemCode, parse_json};
use archangel_testkit::table;
use serde_json::{Value, json};

fn problems(config: &Value) -> Vec<ConfigProblem> {
    parse_json(&config.to_string()).expect_err("the config has a problem")
}

fn with_rules(rules: &[Value]) -> Value {
    json!({ "parts": { "api": "apps/api/**", "shared": "src/shared/**" }, "rules": rules })
}

const KINDS: &str = "**/*.{kind}.ts";

/// The problem of one code, which must be the only problem of that code.
fn one_of(config: &Value, code: ProblemCode) -> (String, Option<String>) {
    let found = problems(config);
    let of_code: Vec<_> = found.iter().filter(|p| p.code == code).collect();
    assert_eq!(of_code.len(), 1, "one problem of {code:?}: {found:?}");
    (of_code[0].message.clone(), of_code[0].field.clone())
}

#[test]
fn a_row_that_names_itself_and_may_not_import_itself_is_refused() {
    let config = with_rules(&[table(
        "t",
        &json!({ "port": ["port", "entity"], "entity": [] }),
        &json!({ "kinds": KINDS, "mayNotImportItself": ["port"] }),
    )]);
    let (message, field) = one_of(&config, ProblemCode::ConfigInvalid);
    assert!(message.contains("names itself"), "{message}");
    assert_eq!(
        field.as_deref(),
        Some("rules[0].rows.port"),
        "a row that names itself and may not import itself is refused"
    );
}

#[test]
fn a_kind_pattern_without_the_kind_capture_is_refused() {
    let config = with_rules(&[table(
        "t",
        &json!({ "entity": [] }),
        &json!({ "kinds": "**/*.entity.ts" }),
    )]);
    let (message, field) = one_of(&config, ProblemCode::ConfigInvalid);
    assert!(message.contains("{kind}"), "{message}");
    assert_eq!(
        field.as_deref(),
        Some("rules[0].kinds"),
        "a kind pattern without the kind capture is refused"
    );
}

#[test]
fn a_misspelt_cell_is_refused_with_the_closest_name() {
    let config = with_rules(&[table(
        "t",
        &json!({ "entity": ["value-objects"], "value-object": [] }),
        &json!({ "kinds": KINDS }),
    )]);
    let (message, field) = one_of(&config, ProblemCode::RuleUnknownElement);
    assert!(message.contains("value-object\""), "{message}");
    assert!(
        field.is_some_and(|f| f.starts_with("rules[0].rows.entity")),
        "a misspelt cell is refused with the closest name"
    );
}

#[test]
fn a_row_key_that_names_nothing_is_refused_with_the_closest_name() {
    let config = json!({
        "parts": { "canvas-document": "packages/canvas-document/**" },
        "rules": [table("t", &json!({ "canvas-documents": [] }), &json!({}))],
    });
    let (message, field) = one_of(&config, ProblemCode::RuleUnknownElement);
    assert!(message.contains("\"canvas-document\""), "{message}");
    assert_eq!(
        field.as_deref(),
        Some("rules[0].rows.canvas-documents"),
        "a row key that names nothing is refused with the closest name"
    );
}

#[test]
fn a_row_key_that_names_an_externals_set_is_refused() {
    let config = json!({
        "parts": { "api": "apps/api/**" },
        "externals": { "driver": ["pg"] },
        "rules": [table("t", &json!({ "driver": [] }), &json!({}))],
    });
    let (message, field) = one_of(&config, ProblemCode::ConfigInvalid);
    assert!(message.contains("no files that import"), "{message}");
    assert_eq!(
        field.as_deref(),
        Some("rules[0].rows.driver"),
        "a row key that names an externals set is refused"
    );
}

#[test]
fn a_row_name_that_another_rule_has_is_a_duplicate() {
    let config = with_rules(&[
        table(
            "domain",
            &json!({ "entity": [] }),
            &json!({ "kinds": KINDS }),
        ),
        json!({ "name": "domain.entity", "files": "api", "mayNotImport": "shared" }),
    ]);
    let (message, _) = one_of(&config, ProblemCode::RuleNameDuplicate);
    assert!(message.contains("domain.entity"), "{message}");
}

#[test]
fn two_tables_with_one_name_are_refused() {
    let config = with_rules(&[
        table(
            "domain",
            &json!({ "entity": [] }),
            &json!({ "kinds": KINDS }),
        ),
        table(
            "domain",
            &json!({ "policy": [] }),
            &json!({ "kinds": KINDS }),
        ),
    ]);
    let (message, _) = one_of(&config, ProblemCode::RuleNameDuplicate);
    assert!(
        message.contains("domain") && message.contains("its own name"),
        "{message}"
    );
}

#[test]
fn a_table_without_a_name_is_refused() {
    let config = json!({
        "parts": { "api": "apps/api/**" },
        "rules": [{ "kind": "mayImport", "rows": { "api": [] } }],
    });
    let (message, field) = one_of(&config, ProblemCode::ConfigInvalid);
    assert!(message.contains("needs a name"), "{message}");
    assert_eq!(
        field.as_deref(),
        Some("rules[0].name"),
        "a table without a name is refused"
    );
}

#[test]
fn a_table_name_with_a_colon_or_slash_is_refused() {
    let colon = with_rules(&[table("api:domain", &json!({ "api": [] }), &json!({}))]);
    let (message, field) = one_of(&colon, ProblemCode::ConfigInvalid);
    assert!(
        message.contains("colon") && message.contains("exception comment"),
        "{message}"
    );
    assert_eq!(
        field.as_deref(),
        Some("rules[0].name"),
        "a table name with a colon or slash is refused"
    );
    let slash = with_rules(&[table("api/domain", &json!({ "api": [] }), &json!({}))]);
    let (message, field) = one_of(&slash, ProblemCode::ConfigInvalid);
    assert!(message.contains("slash"), "{message}");
    assert_eq!(
        field.as_deref(),
        Some("rules[0].name"),
        "a table name with a colon or slash is refused"
    );
}

#[test]
fn a_row_key_with_a_colon_or_slash_is_refused() {
    for (key, word) in [("aggregate:root", "colon"), ("aggregate/root", "slash")] {
        let config = with_rules(&[table("t", &json!({ key: [] }), &json!({ "kinds": KINDS }))]);
        let (message, field) = one_of(&config, ProblemCode::ConfigInvalid);
        assert!(message.contains(word), "{message}");
        assert_eq!(
            field,
            Some(format!("rules[0].rows.{key}")),
            "a row key with a colon or slash is refused"
        );
    }
}

#[test]
fn a_dot_or_a_space_in_a_name_is_accepted_and_a_clash_is_a_duplicate() {
    let loads = with_rules(&[table(
        "feature-code",
        &json!({ "api.v2": ["shared"], "my views": ["shared"] }),
        &json!({ "kinds": "src/features/*/{kind}/**" }),
    )]);
    let config = parse_json(&loads.to_string()).expect("the names load");
    assert!(
        config.rules.iter().any(|r| r.name == "feature-code.api.v2"),
        "a dot or a space in a name is accepted and a clash is a duplicate"
    );
    assert!(
        config
            .rules
            .iter()
            .any(|r| r.name == "feature-code.my views"),
        "a dot or a space in a name is accepted and a clash is a duplicate"
    );
    let clash = with_rules(&[
        table("a.b", &json!({ "c": [] }), &json!({ "kinds": KINDS })),
        table("a", &json!({ "b.c": [] }), &json!({ "kinds": KINDS })),
    ]);
    let (message, _) = one_of(&clash, ProblemCode::RuleNameDuplicate);
    assert!(message.contains("a.b.c"), "{message}");
}

#[test]
fn instance_on_a_row_without_a_capture_is_refused() {
    let config = with_rules(&[table(
        "t",
        &json!({ "shared": [] }),
        &json!({ "instance": "different" }),
    )]);
    let (_, field) = one_of(&config, ProblemCode::RuleNeedsCapture);
    assert_eq!(
        field.as_deref(),
        Some("rules[0].rows.shared"),
        "instance on a row without a capture is refused"
    );
}

#[test]
fn only_the_kind_capture_may_repeat_in_a_kind_pattern() {
    let config = with_rules(&[
        table(
            "domain",
            &json!({ "entity": [] }),
            &json!({ "kinds": "**/{kind}s/*.{kind}.ts" }),
        ),
        table(
            "apps",
            &json!({ "entity": [] }),
            &json!({ "kinds": "apps/{app}/{app}/*.{kind}.ts" }),
        ),
    ]);
    let found = problems(&config);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        found[0].code,
        ProblemCode::PatternInvalid,
        "only the kind capture may repeat in a kind pattern"
    );
    for word in ["apps", "apps/{app}/{app}/*.{kind}.ts", "app"] {
        assert!(
            found[0].message.contains(word),
            "{word}: {}",
            found[0].message
        );
    }
}

#[test]
fn a_kind_pattern_that_cannot_mean_what_it_says_is_pattern_invalid() {
    let config = with_rules(&[table(
        "domain",
        &json!({ "entity": [] }),
        &json!({ "kinds": "**//*.{kind}.ts" }),
    )]);
    let (message, _) = one_of(&config, ProblemCode::PatternInvalid);
    assert!(
        message.contains("domain") && message.contains("**//*.{kind}.ts"),
        "{message}"
    );
}
