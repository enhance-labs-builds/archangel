//! Scenarios from the plugins spec that need no Node: the plugins the loader describes, the
//! facts their conditions read, and the rules that use them.
#![cfg(test)]

#[path = "support/loading.rs"]
mod loading;

use archangel_config::{Reads, from_value};
use loading::loader_problem;
use serde_json::{Value, json};

/// A config as the loader sends it, with these plugins and rules.
fn with_plugins(plugins: &Value, rules: &Value) -> Value {
    json!({ "elements": { "entity": "src/entities/{entity}/**" }, "plugins": plugins, "rules": rules })
}

/// The plugins "team", with two conditions, and "other", with none, and these rules.
fn team_config(rules: &Value) -> archangel_config::Config {
    from_value(&with_plugins(
        &json!({
            "team": { "schemaVersion": 1, "conditions": {
                "stores": { "description": "one store", "docs": "https://example.com/stores",
                            "reads": ["imports", "importers"], "optionsSchema": null },
                "barrels": { "description": "", "reads": ["exports"], "optionsSchema": null },
            } },
            "other": { "schemaVersion": 1, "conditions": {} },
        }),
        rules,
    ))
    .unwrap_or_else(|problems| panic!("the config is valid: {problems:?}"))
}

#[test]
fn each_plugin_condition_reads_the_facts_it_lists() {
    let config = team_config(&json!([]));
    let team = config.plugin("team").expect("the plugin team");
    let reads: Vec<(&str, Reads)> = team
        .conditions
        .iter()
        .map(|c| (c.name.as_str(), c.reads))
        .collect();
    let all = Reads {
        imports: true,
        importers: true,
        exports: true,
    };
    assert_eq!(
        reads,
        [
            (
                "stores",
                Reads {
                    exports: false,
                    ..all
                }
            ),
            (
                "barrels",
                Reads {
                    exports: true,
                    ..Reads::default()
                }
            ),
        ],
        "each condition reads the facts it lists"
    );
    assert_eq!(
        team.conditions[0].docs.as_deref(),
        Some("https://example.com/stores"),
        "the condition's documentation"
    );
    assert_eq!(
        (
            config.plugin("other").map(|p| p.name.as_str()),
            config.plugin("teams")
        ),
        (Some("other"), None),
        "a plugin by its name"
    );
}

#[test]
fn a_plugin_rule_uses_its_condition_and_keeps_the_plugin_process() {
    let config = team_config(&json!([
        { "files": "entity", "plugin": "team/stores" },
        { "files": "entity", "noCycles": true },
    ]));
    let used = config.rules[0].plugin_use().expect("a plugin rule");
    assert_eq!(
        (config.rules[0].name.as_str(), used.full_name(), used.reads),
        (
            "team/stores(entity)",
            "team/stores".to_owned(),
            Reads {
                imports: true,
                importers: true,
                exports: false
            }
        ),
        "the rule names the condition and reads what it reads"
    );
    assert!(
        config.rules[1].plugin_use().is_none() && config.uses_plugins(),
        "a rule of the config's own uses no plugin, and one plugin rule keeps the plugin process"
    );
    assert!(
        !team_config(&json!([{ "files": "entity", "noCycles": true }])).uses_plugins(),
        "plugins that no rule uses need no plugin process"
    );
}

#[test]
fn the_facts_two_conditions_read_are_those_either_reads() {
    let imports = Reads {
        imports: true,
        ..Reads::default()
    };
    let importers = Reads {
        importers: true,
        ..Reads::default()
    };
    let exports = Reads {
        exports: true,
        ..Reads::default()
    };
    assert_eq!(
        [
            imports.union(importers).union(exports),
            exports.union(imports),
            Reads::default().union(importers),
        ],
        [
            Reads {
                imports: true,
                importers: true,
                exports: true
            },
            Reads {
                imports: true,
                importers: false,
                exports: true
            },
            Reads {
                imports: false,
                importers: true,
                exports: false
            },
        ],
        "each fact that either condition reads"
    );
}

#[test]
fn a_plugin_needs_a_name_without_a_slash() {
    let plugin = json!({ "schemaVersion": 1, "conditions": {} });
    let found = from_value(&with_plugins(
        &json!({ "": plugin, "team/web": plugin, "team": plugin }),
        &json!([]),
    ))
    .expect_err("two plugin names are refused");
    let fields: Vec<Option<&str>> = found.iter().map(|p| p.field.as_deref()).collect();
    assert_eq!(
        fields,
        [Some("plugins."), Some("plugins.team/web")],
        "an empty name and a name with a slash: {found:?}"
    );
}

#[test]
fn a_problem_only_the_plugin_author_can_fix_says_to_report_it() {
    let problem = loader_problem(&with_plugins(
        &json!({ "team": { "conditions": {} } }),
        &json!([]),
    ));
    assert_eq!(
        (problem.field.as_deref(), problem.message.as_str()),
        (
            Some("plugins.team.schemaVersion"),
            "the plugin \"team\" declares no `schemaVersion`, the version of the view it was \
             written for; report it to the plugin's author"
        ),
        "{problem:?}"
    );
}

#[test]
fn plugins_that_are_not_an_object_of_plugins_are_refused() {
    let problem = loader_problem(&with_plugins(&json!([]), &json!([])));
    assert_eq!(
        (problem.field.as_deref(), problem.message.as_str()),
        (
            Some("plugins"),
            "must be an object of plugin names to plugins"
        ),
        "{problem:?}"
    );
}

#[test]
fn a_condition_that_names_its_schema_options_is_told_it_is_options_schema() {
    let problem = loader_problem(&with_plugins(
        &json!({ "team": { "schemaVersion": 1, "conditions": {
            "stores": { "description": "", "reads": [], "optionsSchema": null, "unknown": ["options"] },
        } } }),
        &json!([]),
    ));
    assert_eq!(
        (problem.field.as_deref(), problem.message.as_str()),
        (
            Some("plugins.team.conditions.stores.options"),
            "the plugin \"team\" gives its condition `stores` the field `options`, which conditions \
             do not have; the JSON Schema of its options is `optionsSchema`; report it to the \
             plugin's author"
        ),
        "{problem:?}"
    );
}

#[test]
fn a_condition_field_archangel_does_not_know_is_refused_with_the_fields_it_knows() {
    let problem = loader_problem(&with_plugins(
        &json!({ "team": { "schemaVersion": 1, "conditions": {
            "stores": { "description": "", "reads": [], "optionsSchema": null, "unknown": ["read"] },
        } } }),
        &json!([]),
    ));
    assert_eq!(
        (problem.field.as_deref(), problem.message.as_str()),
        (
            Some("plugins.team.conditions.stores.read"),
            "the plugin \"team\" gives its condition `stores` the field `read`, which conditions do \
             not have; a condition has name, description, docs, reads, optionsSchema, check; report \
             it to the plugin's author"
        ),
        "{problem:?}"
    );
}
