//! Scenario from the configuration spec: the JSON schema the npm package ships has exactly the
//! fields the parser knows, so editors complete what archangel accepts.
#![cfg(test)]

use archangel_config::parse_json;
use serde_json::{Value, json};

const SCHEMA: &str = include_str!("../../../npm/archangel/archangel.schema.json");

const CONDITIONS: [&str; 10] = [
    "mayNotImport",
    "mayOnlyImport",
    "mayOnlyBeImportedBy",
    "mustBeNamed",
    "mustContain",
    "mustNotExist",
    "mustExport",
    "mayNotExport",
    "mustOnlyReExport",
    "noCycles",
];

/// The conditions a rule about folders takes, with `folders` in place of `files`.
const FOLDER_CONDITIONS: [&str; 3] = ["mustBeNamed", "mustContain", "mustNotExist"];

/// A value the parser accepts for each condition.
fn condition_value(condition: &str) -> Value {
    match condition {
        "mustBeNamed" => json!("kebab-case"),
        "mustContain" => json!("index.ts"),
        "mustExport" => json!("{FileName}"),
        "mayNotExport" => json!("default"),
        "mustNotExist" | "mustOnlyReExport" | "noCycles" => json!(true),
        _ => json!("app"),
    }
}

fn elements() -> Value {
    json!({ "app": "src/app/**", "feature": "src/features/{feature}/**" })
}

/// The fields the parser lists when it refuses an unknown one in `rule`, or at the top level
/// when `rule` is `None`.
fn known_fields(rule: Option<Value>) -> Vec<String> {
    let config = match rule {
        Some(rule) => json!({ "elements": elements(), "rules": [rule] }),
        None => json!({ "elements": elements(), "unknownField": 1 }),
    };
    let problems = parse_json(&config.to_string()).unwrap_err();
    let message = &problems
        .iter()
        .find(|p| {
            p.field
                .as_deref()
                .is_some_and(|f| f.ends_with("unknownField"))
        })
        .unwrap()
        .message;
    let list = message.split("the known fields are ").nth(1).unwrap();
    let mut fields: Vec<String> = list.split(", ").map(str::to_owned).collect();
    fields.sort();
    fields
}

/// Whether the parser refuses `option` itself in a sentence with `condition`.
fn refuses_option(condition: &str, option: &str) -> bool {
    let mut rule = json!({ "files": "feature" });
    rule[condition] = condition_value(condition);
    let value = match option {
        "imports" => json!("all"),
        "names" => json!(["x"]),
        "specifier" => json!("relative"),
        "instance" => json!("different"),
        "dependency" => json!("dev"),
        "transitively" => json!(true),
        "within" => json!("app"),
        // A chain's end means nothing without a chain.
        "stopAt" => {
            rule["transitively"] = json!(true);
            json!("app")
        }
        other => panic!("no sample value for the option `{other}`"),
    };
    rule[option] = value;
    let config = json!({ "elements": elements(), "rules": [rule] });
    let field = format!("rules[0].{option}");
    parse_json(&config.to_string())
        .err()
        .unwrap_or_default()
        .iter()
        .any(|p| p.field.as_deref() == Some(field.as_str()))
}

fn keys(object: &Value) -> Vec<String> {
    let mut keys: Vec<String> = object.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    keys
}

/// The condition that a sentence schema requires, with `files` or, for a rule about folders,
/// `folders`, and which of the two it requires.
fn required_condition(rule: &Value) -> (&'static str, &'static str) {
    let required: Vec<&str> = rule["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r.as_str().unwrap())
        .collect();
    let found: Vec<&'static str> = CONDITIONS
        .into_iter()
        .filter(|c| required.contains(c))
        .collect();
    assert_eq!(
        found.len(),
        1,
        "each sentence schema requires one condition"
    );
    let scope = if required.contains(&"folders") {
        "folders"
    } else {
        "files"
    };
    assert!(
        required.contains(&scope),
        "each sentence schema requires `files` or `folders`"
    );
    (found[0], scope)
}

/// The fields a sentence with `condition` about `scope`, `files` or `folders`, takes: the
/// parser's sentence fields, without the other conditions, the other scope and the options the
/// parser refuses with this condition. A rule about folders takes no option.
fn sentence_fields(condition: &str, scope: &str) -> Vec<String> {
    let fields = known_fields(Some(
        json!({ "files": "app", "noCycles": true, "unknownField": 1 }),
    ));
    let common = ["name", "reason", "because", "link", "examples"];
    fields
        .into_iter()
        .filter(|field| {
            let field = field.as_str();
            if CONDITIONS.contains(&field) {
                return field == condition;
            }
            if matches!(field, "files" | "folders") {
                return field == scope;
            }
            common.contains(&field) || (scope == "files" && !refuses_option(condition, field))
        })
        .collect()
}

/// The schema of a rule of several sentences.
fn several_sentences(schema: &Value) -> &Value {
    schema["properties"]["rules"]["items"]["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .find(|rule| rule["properties"].get("sentences").is_some())
        .unwrap()
}

/// The schemas of the shorthands, of the sentences, and of the plugin rule.
fn rule_schemas(schema: &Value) -> (Vec<&Value>, Vec<&Value>, Vec<&Value>) {
    let (plugin, rest): (Vec<&Value>, Vec<&Value>) =
        schema["properties"]["rules"]["items"]["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|rule| rule["properties"].get("sentences").is_none())
            .partition(|rule| rule["properties"].get("plugin").is_some());
    let (shorthands, sentences) = rest
        .into_iter()
        .partition(|rule| rule["properties"].get("kind").is_some());
    (shorthands, sentences, plugin)
}

#[test]
fn the_published_schema_has_exactly_the_fields_the_parser_knows() {
    let schema: Value = serde_json::from_str(SCHEMA).unwrap();
    assert_eq!(
        keys(&schema["properties"]),
        known_fields(None),
        "the top-level fields; edit npm/archangel/archangel.schema.json to match the parser"
    );
    let (shorthands, sentences, _) = rule_schemas(&schema);
    let kinds: Vec<&str> = shorthands
        .iter()
        .map(|rule| rule["properties"]["kind"]["const"].as_str().unwrap())
        .collect();
    assert_eq!(
        kinds,
        ["layers", "isolate", "entry", "mayImport"],
        "one schema per shorthand"
    );
    for (rule, kind) in shorthands.iter().zip(kinds) {
        assert_eq!(
            keys(&rule["properties"]),
            known_fields(Some(json!({ "kind": kind, "unknownField": 1 }))),
            "the fields of {kind}; edit npm/archangel/archangel.schema.json to match the parser"
        );
    }
    let found: Vec<(&str, &str)> = sentences
        .iter()
        .map(|rule| required_condition(rule))
        .collect();
    let expected: Vec<(&str, &str)> = CONDITIONS
        .iter()
        .flat_map(|&c| {
            let folders = FOLDER_CONDITIONS.contains(&c).then_some((c, "folders"));
            std::iter::once((c, "files")).chain(folders)
        })
        .collect();
    assert_eq!(
        found, expected,
        "one schema per condition, and one more for each condition a rule about folders takes"
    );
    for (rule, (condition, scope)) in sentences.iter().zip(found) {
        assert_eq!(
            keys(&rule["properties"]),
            sentence_fields(condition, scope),
            "the fields of a {condition} sentence about {scope}; edit \
             npm/archangel/archangel.schema.json to match the parser"
        );
    }
}

#[test]
fn the_schema_of_a_rule_of_several_sentences_has_the_parsers_fields() {
    let schema: Value = serde_json::from_str(SCHEMA).unwrap();
    assert_eq!(
        keys(&several_sentences(&schema)["properties"]),
        known_fields(Some(
            json!({ "name": "r", "sentences": [], "unknownField": 1 })
        )),
        "the fields of a rule of several sentences; edit npm/archangel/archangel.schema.json to \
         match the parser"
    );
}

#[test]
fn neither_the_schema_nor_the_parser_takes_plugins_in_a_json_config() {
    let schema: Value = serde_json::from_str(SCHEMA).unwrap();
    let (_, _, plugin_rules) = rule_schemas(&schema);
    let config = json!({ "elements": elements(), "plugins": {}, "rules": [] });
    assert!(
        plugin_rules.is_empty() && schema["properties"]["plugins"]["not"] == json!({}),
        "the schema takes no plugin and no plugin rule, because a JSON config loads no code"
    );
    assert!(
        parse_json(&config.to_string()).is_err(),
        "the parser refuses plugins in a JSON config"
    );
}
