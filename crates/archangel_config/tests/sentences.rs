//! Scenarios from the architecture-rules spec: a rule of several sentences holds the name, reason
//! and examples that its sentences may not.
#![cfg(test)]

use archangel_config::{ImportKinds, parse_json};
use serde_json::json;

#[test]
fn a_rule_of_several_sentences_needs_a_name() {
    let problems = parse_json(
        &json!({
            "elements": { "entity": "src/entities/{entity}/**", "shared": "src/shared/**" },
            "rules": [{ "sentences": [
                { "files": "entity", "mayNotImport": "shared" },
                { "folders": "src/*", "mustBeNamed": "kebab-case" },
            ] }],
        })
        .to_string(),
    )
    .expect_err("the config is refused");
    let [problem] = problems.as_slice() else {
        panic!("one problem: {problems:?}");
    };
    assert_eq!(
        problem.field.as_deref(),
        Some("rules[0].name"),
        "{problem:?}"
    );
    assert!(problem.message.contains("name"), "{}", problem.message);
}

#[test]
fn a_sentence_inside_a_rule_takes_no_name_or_examples() {
    let problems = parse_json(
        &json!({
            "elements": { "entity": "src/entities/{entity}/**", "shared": "src/shared/**" },
            "rules": [{ "name": "two", "sentences": [
                { "files": "entity", "mayNotImport": "shared", "reason": "belongs to the rule" },
            ] }],
        })
        .to_string(),
    )
    .expect_err("the config is refused");
    let [problem] = problems.as_slice() else {
        panic!("one problem: {problems:?}");
    };
    assert_eq!(
        problem.field.as_deref(),
        Some("rules[0].sentences[0].reason"),
        "{problem:?}"
    );
}

#[test]
fn a_rule_of_several_sentences_holds_each_sentence_in_order() {
    let config = parse_json(
        &json!({
            "elements": { "entity": "src/entities/{entity}/**", "shared": "src/shared/**" },
            "rules": [{ "name": "entities", "reason": "one rule", "sentences": [
                { "files": "entity", "mayNotImport": "shared" },
                { "kind": "isolate", "element": "entity" },
                { "folders": "src/*", "mustBeNamed": "kebab-case" },
            ] }],
        })
        .to_string(),
    )
    .expect("the rule is valid");
    let rule = &config.rules[0];
    let conditions: Vec<&str> = rule.sentences.iter().map(|s| s.condition.name()).collect();
    assert_eq!(
        (rule.name.as_str(), rule.reason.as_deref(), conditions),
        (
            "entities",
            Some("one rule"),
            vec!["mayNotImport", "mayNotImport", "mustBeNamed"]
        ),
        "the rule's name and reason, and its sentences with a shorthand's among them"
    );
}

#[test]
fn a_rule_with_a_transitive_sentence_follows_runtime_imports() {
    let elements = json!({ "entity": "src/entities/{entity}/**", "shared": "src/shared/**" });
    let sentences = json!([
        { "files": "shared", "mayNotImport": "entity" },
        { "files": "entity", "mayNotImport": "shared", "transitively": true },
    ]);
    let config = parse_json(
        &json!({
            "elements": elements,
            "rules": [{ "name": "apart", "sentences": sentences }],
        })
        .to_string(),
    )
    .expect("the rule is valid");
    assert_eq!(
        config.rules[0].imports,
        ImportKinds::Runtime,
        "a chain of imports runs, so the rule counts runtime imports by default"
    );
    let problems = parse_json(
        &json!({
            "elements": elements,
            "rules": [{ "name": "apart", "imports": "types", "sentences": sentences }],
        })
        .to_string(),
    )
    .expect_err("a chain of type imports never runs");
    assert_eq!(
        problems
            .iter()
            .map(|p| p.field.as_deref())
            .collect::<Vec<_>>(),
        [Some("rules[0].imports")],
        "{problems:?}"
    );
}
