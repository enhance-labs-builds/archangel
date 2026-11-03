//! Scenarios from the structure-rules spec: how a `mustBeNamed` list loads, and what it refuses.
#![cfg(test)]

use archangel_config::{Condition, ConfigProblem, NameRule, ProblemCode, Style, parse_json};
use serde_json::{Value, json};

fn only_problem(config: &Value) -> ConfigProblem {
    let mut found = parse_json(&config.to_string()).expect_err("the config has a problem");
    assert_eq!(found.len(), 1, "one problem, found {found:?}");
    found.remove(0)
}

fn config_of(rule: &Value) -> Value {
    json!({ "elements": { "src": "src/**" }, "rules": [rule] })
}

/// The name rules that a `mustBeNamed` value reads into, as the config wrote each one.
fn read(value: &Value) -> Vec<String> {
    let config = config_of(&json!({ "files": "src/**", "mustBeNamed": value }));
    let parsed = parse_json(&config.to_string()).expect("a valid config");
    let Condition::MustBeNamed(rules) = &parsed.rules[0].sentences[0].condition else {
        panic!("a naming condition: {:?}", parsed.rules[0]);
    };
    rules.iter().map(|rule| rule.source().to_owned()).collect()
}

fn refused(value: &Value) -> ConfigProblem {
    only_problem(&config_of(
        &json!({ "files": "src/**", "mustBeNamed": value }),
    ))
}

#[test]
fn a_list_reads_each_entry_in_order() {
    assert_eq!(
        read(&json!(["*.view", "kebab-case", "{file-name}.test"])),
        ["*.view", "kebab-case", "{file-name}.test"],
        "a rule keeps the order of its entries, so its messages list them as the config does"
    );
    assert_eq!(
        read(&json!("kebab-case")),
        ["kebab-case"],
        "a single value is one entry"
    );
}

#[test]
fn an_entry_is_a_style_when_its_text_names_one_and_a_pattern_otherwise() {
    let config = config_of(&json!({ "files": "src/**", "mustBeNamed": ["PascalCase", "*.view"] }));
    let parsed = parse_json(&config.to_string()).expect("a valid config");
    let Condition::MustBeNamed(rules) = &parsed.rules[0].sentences[0].condition else {
        panic!("a naming condition: {:?}", parsed.rules[0]);
    };
    assert!(
        matches!(rules[0], NameRule::Style(Style::Pascal)),
        "a style checks the name up to the first dot: {rules:?}"
    );
    assert!(
        matches!(&rules[1], NameRule::Pattern(pattern) if pattern.source() == "*.view"),
        "a pattern checks the full name: {rules:?}"
    );
}

#[test]
fn the_default_name_of_a_rule_writes_each_entry() {
    let config = config_of(&json!({ "files": "src/**", "mustBeNamed": ["*.view", "camelCase"] }));
    let parsed = parse_json(&config.to_string()).expect("a valid config");
    assert_eq!(
        parsed.rules[0].name, "mustBeNamed(src/**, *.view, camelCase)",
        "two rules with different lists must not have the same default name"
    );
}

#[test]
fn an_empty_list_is_refused_and_says_what_to_write() {
    let problem = refused(&json!([]));
    assert_eq!(
        (problem.code, problem.field.as_deref()),
        (ProblemCode::ConfigInvalid, Some("rules[0].mustBeNamed")),
        "an empty list permits no name, so a rule with one cannot be correct: {problem:?}"
    );
    assert_eq!(
        problem.message,
        "lists no names; list at least one style or name pattern, such as `kebab-case` or `*.view`",
        "the message says what to write"
    );
}

#[test]
fn an_entry_that_is_not_a_string_is_refused_at_its_place() {
    let problem = refused(&json!(["*.view", 3]));
    assert_eq!(
        (problem.code, problem.field.as_deref()),
        (ProblemCode::ConfigInvalid, Some("rules[0].mustBeNamed[1]")),
        "the problem points at the entry and not at the whole list: {problem:?}"
    );
    assert_eq!(
        problem.message, "must be a style or a name pattern, such as `kebab-case` or `*.view`",
        "the message says what an entry can be"
    );
}

#[test]
fn a_value_that_is_neither_text_nor_a_list_is_refused() {
    let problem = refused(&json!({ "style": "kebab-case" }));
    assert_eq!(
        (problem.code, problem.field.as_deref()),
        (ProblemCode::ConfigInvalid, Some("rules[0].mustBeNamed")),
        "an object is not a value of `mustBeNamed`: {problem:?}"
    );
    assert_eq!(
        problem.message,
        "must be a style, a name pattern or a list of them, such as `kebab-case` or \
         [`*.view`, `*.styles`]",
        "the message gives both forms"
    );
}

#[test]
fn a_problem_in_each_entry_is_reported() {
    let config = config_of(&json!({ "files": "src/**", "mustBeNamed": [1, "ok", 2] }));
    let found = parse_json(&config.to_string()).expect_err("two entries are not strings");
    let fields: Vec<Option<&str>> = found.iter().map(|p| p.field.as_deref()).collect();
    assert_eq!(
        fields,
        [
            Some("rules[0].mustBeNamed[0]"),
            Some("rules[0].mustBeNamed[2]")
        ],
        "an author can fix every entry in one pass"
    );
}

#[test]
fn an_unknown_placeholder_is_refused_at_its_entry() {
    let problem = refused(&json!(["*.view", "{Filename}.test"]));
    assert_eq!(
        (problem.code, problem.field.as_deref()),
        (ProblemCode::PatternInvalid, Some("rules[0].mustBeNamed[1]")),
        "an entry follows the rules of a single value: {problem:?}"
    );
    assert!(
        problem.message.contains("`{Filename}`"),
        "the message names the placeholder: {}",
        problem.message
    );
}

#[test]
fn a_folder_rule_refuses_a_file_name_in_any_entry() {
    let problem = only_problem(&config_of(
        &json!({ "folders": "src/*", "mustBeNamed": ["*-kit", "{FileName}"] }),
    ));
    assert_eq!(
        problem.field.as_deref(),
        Some("rules[0].mustBeNamed[1]"),
        "the problem points at the entry that names the file: {problem:?}"
    );
    assert!(
        problem.message.starts_with("a folder has no file name;"),
        "a folder rule cannot use the file's name in a list either: {}",
        problem.message
    );
}

#[test]
fn a_folder_rule_takes_a_style_and_the_folder_name_in_a_list() {
    let config =
        config_of(&json!({ "folders": "src/*", "mustBeNamed": ["kebab-case", "{FolderName}Kit"] }));
    assert!(
        parse_json(&config.to_string()).is_ok(),
        "only the name of the file is refused on a folder"
    );
}

#[test]
fn a_file_rule_takes_a_file_name_in_a_list() {
    assert_eq!(
        read(&json!(["{FileName}.test"])),
        ["{FileName}.test"],
        "a rule about files can use the file's own name"
    );
}
