//! Scenarios from the presets spec that need no Node: how archangel merges the presets that the
//! loader sends, unmerged and each with its name, into the config.
#![cfg(test)]

#[path = "support/loading.rs"]
mod loading;

use archangel_config::{Config, ElementId, ElementKind, ProblemCode, Term, from_value};
use loading::loader_problem;
use serde_json::{Value, json};

/// A preset's result as the loader sends it, from the package `archangel-plugin-<name>`.
fn preset(name: &str, root: &str, elements: &Value, rules: &Value) -> Value {
    json!({
        "name": name,
        "package": format!("archangel-plugin-{name}"),
        "root": root,
        "elements": elements,
        "rules": rules,
    })
}

fn merged(config: &Value) -> Config {
    from_value(config).unwrap_or_else(|problems| panic!("the config is valid: {problems:?}"))
}

/// Each element's name with its patterns.
fn elements(config: &Config) -> Vec<(String, Vec<String>)> {
    config
        .elements
        .iter()
        .map(|element| {
            let patterns = match &element.kind {
                ElementKind::Files(patterns) | ElementKind::Packages(patterns) => {
                    patterns.iter().map(|p| p.source().to_owned()).collect()
                }
            };
            (element.name.clone(), patterns)
        })
        .collect()
}

fn element_id(config: &Config, name: &str) -> u32 {
    let index = config
        .elements
        .iter()
        .position(|e| e.name == name)
        .unwrap_or_else(|| panic!("the config has the element {name}"));
    u32::try_from(index).unwrap()
}

#[test]
fn preset_names_carry_the_presets_prefix() {
    let config = merged(&json!({
        "extends": [
            preset("a", "", &json!({ "shared": "a/shared/**" }), &json!([])),
            preset("b", "", &json!({ "shared": "b/shared/**" }), &json!([])),
        ],
        "rules": [],
    }));
    assert_eq!(
        elements(&config),
        [
            ("a/shared".to_owned(), vec!["a/shared/**".to_owned()]),
            ("b/shared".to_owned(), vec!["b/shared/**".to_owned()]),
        ],
        "each preset's shared carries its prefix and keeps its own pattern"
    );
}

#[test]
fn a_config_element_never_replaces_a_preset_element() {
    let config = merged(&json!({
        "elements": { "shared": "lib/shared/**" },
        "extends": [preset(
            "fsd",
            "src",
            &json!({ "shared": "shared/**", "features": "features/{feature}/**" }),
            &json!([{ "name": "shared-alone", "files": "shared", "mayNotImport": "features" }]),
        )],
        "rules": [],
    }));
    let names: Vec<String> = elements(&config)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert!(
        names.contains(&"shared".to_owned()) && names.contains(&"fsd/shared".to_owned()),
        "the config has both shared and fsd/shared: {names:?}"
    );
    let rule = &config.rules[0];
    assert_eq!(
        rule.name, "fsd/shared-alone",
        "the preset's rule carries the prefix"
    );
    assert_eq!(
        rule.sentences[0].files.terms,
        [Term::Element(ElementId(element_id(&config, "fsd/shared")))],
        "the preset's rule is about fsd/shared, not the config's shared"
    );
    let side = rule.sentences[0].condition.side().expect("an import rule");
    assert_eq!(
        side.files[0].terms,
        [Term::Element(ElementId(element_id(
            &config,
            "fsd/features"
        )))],
        "the preset's rule names fsd/features"
    );
    assert!(
        elements(&config).contains(&("fsd/shared".to_owned(), vec!["src/shared/**".to_owned()])),
        "the preset's patterns are under its root: {:?}",
        elements(&config)
    );
}

#[test]
fn a_config_rule_replaces_the_preset_rule_of_the_same_name() {
    let config = merged(&json!({
        "extends": [preset(
            "team",
            "",
            &json!({ "shared": "src/shared/**", "model": "src/model/**", "views": "src/views/**" }),
            &json!([{ "name": "views-thin", "files": "views", "mayOnlyImport": ["shared"] }]),
        )],
        "rules": [{
            "name": "team/views-thin",
            "files": "team/views",
            "mayOnlyImport": ["team/shared", "team/model"],
        }],
    }));
    let named: Vec<_> = config
        .rules
        .iter()
        .filter(|r| r.name == "team/views-thin")
        .collect();
    assert_eq!(
        named.len(),
        1,
        "one rule team/views-thin: {:?}",
        config.rules
    );
    assert_eq!(config.rules.len(), 1, "the preset's own rule is gone");
    let side = named[0].sentences[0]
        .condition
        .side()
        .expect("an import rule");
    let permitted: Vec<&Term> = side.files.iter().flat_map(|set| &set.terms).collect();
    assert_eq!(
        permitted,
        [
            &Term::Element(ElementId(element_id(&config, "team/shared"))),
            &Term::Element(ElementId(element_id(&config, "team/model"))),
        ],
        "the rule permits team/shared and team/model"
    );
}

#[test]
fn an_off_name_that_matches_nothing_is_refused() {
    let problem = loader_problem(&json!({
        "extends": [preset(
            "fsd",
            "src",
            &json!({ "features": "features/{feature}/**" }),
            &json!([{ "name": "insignificant-slice", "files": "features", "noCycles": true }]),
        )],
        "rules": [],
        "off": ["fsd/insignificant-slices"],
    }));
    assert_eq!(problem.field.as_deref(), Some("off"), "{problem:?}");
    assert!(
        problem.message.contains("`fsd/insignificant-slice`"),
        "the closest rule name is suggested: {}",
        problem.message
    );
}

#[test]
fn a_preset_rule_without_a_name_is_refused() {
    let problem = loader_problem(&json!({
        "extends": [preset(
            "team",
            "",
            &json!({ "views": "src/views/**" }),
            &json!([{ "files": "views", "noCycles": true }]),
        )],
        "rules": [],
    }));
    for named in ["\"team\"", "archangel-plugin-team", "author"] {
        assert!(
            problem.message.contains(named),
            "the message names {named}: {}",
            problem.message
        );
    }
}

#[test]
fn one_preset_twice_with_one_name_is_refused() {
    let elements = json!({ "shared": "shared/**" });
    let problem = loader_problem(&json!({
        "extends": [
            preset("fsd", "apps/web/src", &elements, &json!([])),
            preset("fsd", "apps/admin/src", &elements, &json!([])),
        ],
        "rules": [],
    }));
    assert_eq!(problem.field.as_deref(), Some("extends[1]"), "{problem:?}");
    assert!(
        problem.message.contains("`fsd`") && problem.message.contains("`name`"),
        "the message says the second use needs its own name: {}",
        problem.message
    );
    assert_eq!(
        problem.code,
        ProblemCode::ConfigInvalid,
        "a second use is config-invalid"
    );
}

/// A config rule that names a preset's prefix must replace one of its rules, so a misspelt
/// replacement is refused, as a misspelt `off` name is.
#[test]
fn a_config_rule_with_a_presets_prefix_must_replace_one_of_its_rules() {
    let problem = loader_problem(&json!({
        "extends": [preset(
            "team",
            "",
            &json!({ "views": "src/views/**" }),
            &json!([{ "name": "views-thin", "files": "views", "noCycles": true }]),
        )],
        "rules": [{ "name": "team/view-thin", "files": "team/views", "noCycles": true }],
    }));
    assert_eq!(
        problem.field.as_deref(),
        Some("rules[0].name"),
        "{problem:?}"
    );
    assert!(
        problem.message.contains("`team/views-thin`"),
        "the closest preset rule is suggested: {}",
        problem.message
    );
}

/// `off` drops a rule by name after the merge, and a preset's examples move under its root.
#[test]
fn off_drops_a_rule_and_examples_move_under_the_root() {
    let config = merged(&json!({
        "extends": [preset(
            "fsd",
            "apps/web/src",
            &json!({ "features": "features/{feature}/**" }),
            &json!([
                { "name": "a", "files": "features", "noCycles": true,
                  "examples": { "rejected": [{ "features/x/a.ts": "import '../y/b';\n" }] } },
                { "name": "b", "files": "features", "noCycles": true },
            ]),
        )],
        "rules": [],
        "off": ["fsd/b"],
    }));
    let names: Vec<&str> = config.rules.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["fsd/a"], "fsd/b is off");
    let paths: Vec<&String> = config.rules[0].examples.rejected[0].keys().collect();
    assert_eq!(
        paths,
        ["apps/web/src/features/x/a.ts"],
        "the example is under the root"
    );
}

#[test]
fn a_declared_name_wins_over_a_pattern() {
    let config = from_value(&json!({
        "extends": [{ "name": "fsd", "package": "@enhance-eng/archangel", "root": "src",
                      "elements": { "shared": "shared/**", "app": "app/**" } }],
        "rules": [{ "files": "fsd/shared", "mayNotImport": "src/app/**" }],
    }))
    .expect("a valid config");
    let shared = config.element("fsd/shared").unwrap();
    assert!(
        config.rules[0].sentences[0].files.element() == Some(shared),
        "fsd/shared is the declared element, not a pattern: {:?}",
        config.rules[0].sentences[0].files
    );
}

#[test]
fn a_preset_needs_a_name_without_a_slash() {
    for name in ["", "team/web"] {
        let problem = loader_problem(&json!({
            "elements": { "app": "src/app/**" },
            "extends": [preset(name, "", &json!({ "views": "src/views/**" }), &json!([]))],
            "rules": [],
        }));
        assert_eq!(
            problem.field.as_deref(),
            Some("extends[0].name"),
            "the preset named `{name}` is refused: {problem:?}"
        );
        assert!(
            problem.message.contains("without a slash"),
            "{}",
            problem.message
        );
    }
}

#[test]
fn each_preset_records_the_elements_and_selectors_it_declared() {
    let config = merged(&json!({
        "elements": { "app": "src/app/**" },
        "selectors": { "tests": "**/*.test.ts" },
        "extends": [
            preset("a", "", &json!({ "one": "a/one/**" }), &json!([])),
            {
                "name": "b",
                "package": "archangel-plugin-b",
                "root": "b",
                "elements": { "two": "two/**", "three": "three/**" },
                "selectors": { "entries": "*/index.ts" },
                "rules": [],
            },
        ],
        "rules": [],
    }));
    let presets: Vec<(&str, &str, Vec<u32>, Vec<u32>)> = config
        .presets
        .iter()
        .map(|p| {
            (
                p.name.as_str(),
                p.root.as_str(),
                p.elements.iter().map(|e| e.0).collect(),
                p.selectors.iter().map(|s| s.0).collect(),
            )
        })
        .collect();
    assert_eq!(
        presets,
        [("a", "", vec![0], vec![]), ("b", "b", vec![1, 2], vec![0]),],
        "each preset, in order, with the ids of what it declared; the config's own come after"
    );
    assert_eq!(
        (
            config.element("app").map(|id| id.0),
            config.selector("tests").map(|id| id.0),
            config.selector("b/entries").map(|id| id.0),
            config.selector("entries"),
        ),
        (Some(3), Some(1), Some(0), None),
        "the config's own element and selector follow the presets', and a preset's selector \
         carries its prefix"
    );
}

#[test]
fn a_name_off_cannot_find_gets_the_rules_there_are() {
    let off = |rules: &Value, name: &str| {
        loader_problem(&json!({
            "extends": [preset("fsd", "", &json!({ "features": "features/{feature}/**" }), rules)],
            "rules": [],
            "off": [name],
        }))
        .message
    };
    let none = off(&json!([]), "fsd/public-api");
    assert!(
        none.contains("there are no such rules"),
        "with no rules to name: {none}"
    );
    let far = off(
        &json!([
            { "name": "no-cycles", "files": "features", "noCycles": true },
            { "name": "isolated", "kind": "isolate", "element": "features" },
        ]),
        "fsd/public-api",
    );
    assert!(
        far.contains("the rules are `fsd/no-cycles`, `fsd/isolated`"),
        "with no rule near the name, every rule is listed: {far}"
    );
}

/// The loader hosts the plugins of every preset and then the config's, so a plugin the config
/// names replaces a preset's plugin of that name; validation must check rules against the plugin
/// that runs them.
#[test]
fn a_preset_brings_its_plugins_and_the_configs_plugin_of_the_same_name_replaces_its_own() {
    let plugin = |condition: &str| {
        json!({ "schemaVersion": 1, "conditions": {
            condition: { "description": "", "reads": [], "optionsSchema": null },
        } })
    };
    let mut team = preset("team", "", &json!({ "views": "src/views/**" }), &json!([]));
    team["plugins"] = json!({ "lint": plugin("no-barrels"), "own": plugin("from-preset") });
    let config = merged(&json!({
        "extends": [team, preset("empty", "", &json!({}), &json!([]))],
        "plugins": { "own": plugin("from-config") },
        "rules": [
            { "files": "team/views", "plugin": "lint/no-barrels" },
            { "files": "team/views", "plugin": "own/from-config" },
        ],
    }));
    let plugins: Vec<(&str, Vec<&str>)> = config
        .plugins
        .iter()
        .map(|p| {
            (
                p.name.as_str(),
                p.conditions.iter().map(|c| c.name.as_str()).collect(),
            )
        })
        .collect();
    assert_eq!(
        plugins,
        [("lint", vec!["no-barrels"]), ("own", vec!["from-config"])],
        "the preset's plugin, and the config's in place of the preset's plugin of that name; a \
         preset without plugins adds none"
    );
    let mut team = preset("team", "", &json!({ "views": "src/views/**" }), &json!([]));
    team["plugins"] = json!({ "own": plugin("from-preset") });
    let replaced = loader_problem(&json!({
        "extends": [team],
        "plugins": { "own": plugin("from-config") },
        "rules": [{ "files": "team/views", "plugin": "own/from-preset" }],
    }));
    assert_eq!(
        replaced.field.as_deref(),
        Some("rules[0].plugin"),
        "the preset's plugin no longer runs, so no rule may use it: {replaced:?}"
    );
}

#[test]
fn a_config_without_elements_or_a_preset_is_refused() {
    let problem = loader_problem(&json!({ "rules": [] }));
    assert_eq!(problem.field.as_deref(), Some("elements"), "{problem:?}");
    assert!(
        problem.message.contains("declare at least one element"),
        "{}",
        problem.message
    );
}

#[test]
fn a_declared_name_cannot_contain_a_slash() {
    let found = from_value(&json!({
        "elements": { "ui/kit": "src/kit/**", "shared": "src/shared/**" },
        "selectors": { "test/unit": "**/*.test.ts" },
    }))
    .expect_err("two names are refused");
    let fields: Vec<Option<&str>> = found.iter().map(|p| p.field.as_deref()).collect();
    assert_eq!(
        fields,
        [Some("elements.ui/kit"), Some("selectors.test/unit")],
        "a slash separates a preset's name from its own: {found:?}"
    );
    assert!(
        found
            .iter()
            .all(|p| p.message.contains("cannot contain a slash")),
        "{found:?}"
    );
}

#[test]
fn a_preset_whose_plugins_are_not_an_object_is_refused() {
    for plugins in [Value::Null, json!([{ "schemaVersion": 1 }])] {
        let mut team = preset("team", "", &json!({ "views": "src/views/**" }), &json!([]));
        team["plugins"] = plugins;
        let problem = loader_problem(&json!({ "extends": [team], "rules": [] }));
        assert_eq!(
            (problem.code, problem.field.as_deref()),
            (ProblemCode::ConfigInvalid, Some("extends[0].plugins")),
            "{problem:?}"
        );
        for named in ["\"team\"", "archangel-plugin-team", "author"] {
            assert!(
                problem.message.contains(named),
                "the message names {named}: {}",
                problem.message
            );
        }
    }
}
