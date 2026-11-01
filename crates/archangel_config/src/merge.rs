//! Merging the presets a config extends into it. The loader sends each preset's result
//! unmerged, with the preset's name, its package and its root, before the config's own fields
//! and `off`; the merge happens here, so the checks that need to know where a rule came from sit
//! beside the rest of the validation.

use serde_json::{Map, Value};

use crate::model::{Preset, Rule, RuleKind, Table};
use crate::parse::{nearest, string_list};
use crate::problem::{ConfigProblem, ProblemCode};
use crate::selectors::Placement;

/// One part of the config: a preset's result, or the config's own fields.
pub(crate) struct ConfigPart<'v> {
    /// The fields.
    pub(crate) object: &'v Map<String, Value>,
    /// Where the fields sit in the config, such as `extends[0].`, or empty for the config's own.
    pub(crate) field: String,
    /// The scope its names and paths were written in.
    pub(crate) scope: Placement,
    /// The package the preset came from, for a message its user cannot act on.
    pub(crate) package: Option<String>,
}

impl ConfigPart<'_> {
    /// A field of this part, as a problem names it.
    pub(crate) fn at(&self, key: &str) -> String {
        format!("{}{key}", self.field)
    }
}

/// The presets the config extends, in order, then the config's own fields.
pub(crate) fn config_parts<'v>(
    root: &'v Map<String, Value>,
    problems: &mut Vec<ConfigProblem>,
) -> Vec<ConfigPart<'v>> {
    let mut found = Vec::new();
    match root.get("extends") {
        None => {}
        Some(Value::Array(presets)) => {
            let mut names: Vec<String> = Vec::new();
            for (index, preset) in presets.iter().enumerate() {
                let field = format!("extends[{index}]");
                let Some(source) = preset_source(preset, &field, problems) else {
                    continue;
                };
                let name = source.scope.prefix.clone().unwrap_or_default();
                if names.contains(&name) {
                    problems.push(invalid(
                        &field,
                        &format!(
                            "the preset `{name}` is used twice, so the second use needs its own \
                             `name`, such as `{name}({{ ..., name: '{name}-2' }})`, which its \
                             elements, selectors and rules then carry"
                        ),
                    ));
                    continue;
                }
                names.push(name);
                found.push(source);
            }
        }
        Some(_) => problems.push(invalid("extends", "must be a list of presets")),
    }
    found.push(ConfigPart {
        object: root,
        field: String::new(),
        scope: Placement::default(),
        package: None,
    });
    found
}

fn preset_source<'v>(
    preset: &'v Value,
    field: &str,
    problems: &mut Vec<ConfigProblem>,
) -> Option<ConfigPart<'v>> {
    let Some(object) = preset.as_object() else {
        problems.push(invalid(
            field,
            "must be the result of a preset, such as `fsd({ root: 'src' })`",
        ));
        return None;
    };
    let Some(name) = object
        .get("name")
        .and_then(Value::as_str)
        .filter(|n| !n.is_empty() && !n.contains('/'))
    else {
        problems.push(invalid(
            &format!("{field}.name"),
            "a preset needs a name without a slash, which its elements, selectors and rules carry \
             as a prefix",
        ));
        return None;
    };
    let root = object
        .get("root")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim_matches('/')
        .to_owned();
    Some(ConfigPart {
        object,
        field: format!("{field}."),
        scope: Placement {
            prefix: Some(name.to_owned()),
            root,
        },
        package: object
            .get("package")
            .and_then(Value::as_str)
            .map(str::to_owned),
    })
}

/// A preset rule must have a name, because `off`, exceptions and the baseline refer to it; its
/// user cannot fix a missing one, so the message points at the preset's author.
pub(crate) fn preset_rule_name(
    source: &ConfigPart<'_>,
    rule: &Value,
    field: &str,
    problems: &mut Vec<ConfigProblem>,
) -> Option<String> {
    let prefix = source.scope.prefix.as_deref()?;
    if let Some(name) = rule.get("name").and_then(Value::as_str) {
        return Some(source.scope.name(name));
    }
    let package = source
        .package
        .as_deref()
        .map_or_else(String::new, |p| format!(" from the package `{p}`"));
    problems.push(invalid(
        field,
        &format!(
            "a rule of the preset \"{prefix}\"{package} has no name, and every preset rule needs \
             one, because `off`, exceptions and the baseline refer to it; the preset's author \
             must name the rule, so report it to them"
        ),
    ));
    None
}

/// The rules with each config rule that names a preset rule in its place, after refusing a
/// config rule with a preset's prefix that names none of its rules, and a config name with a
/// slash that names no preset.
pub(crate) fn replace(
    preset_rules: Vec<Rule>,
    config_rules: Vec<(Rule, String, bool)>,
    presets: &[Preset],
    problems: &mut Vec<ConfigProblem>,
) -> Vec<Rule> {
    let mut rules = preset_rules;
    for (mut rule, field, named) in config_rules {
        let prefix = rule.name.split_once('/').map(|(prefix, _)| prefix);
        let Some(prefix) = prefix.filter(|_| named) else {
            rules.push(rule);
            continue;
        };
        if !presets.iter().any(|p| p.name == prefix) {
            problems.push(invalid(
                &format!("{field}.name"),
                &format!(
                    "the config's own names cannot contain a slash, and no preset is named \
                     `{prefix}`; a name with a slash replaces the rule of that name in a preset"
                ),
            ));
            continue;
        }
        if let Some(place) = rules.iter().position(|r| r.name == rule.name) {
            rule.replaces_preset_rule = true;
            rules[place] = rule;
            continue;
        }
        let known: Vec<&str> = rules
            .iter()
            .map(|r| r.name.as_str())
            .filter(|name| name.starts_with(&format!("{prefix}/")))
            .collect();
        problems.push(invalid(
            &format!("{field}.name"),
            &format!(
                "the preset `{prefix}` has no rule `{}`, so this rule would replace nothing; {}",
                rule.name,
                suggestion(&rule.name, &known)
            ),
        ));
    }
    rules
}

/// The rules without those `off` names, and the rules it removes, after refusing a name that
/// matches none. A table's name removes every rule of the table.
pub(crate) fn off(
    root: &Map<String, Value>,
    rules: Vec<Rule>,
    tables: &[Table],
    problems: &mut Vec<ConfigProblem>,
) -> (Vec<Rule>, Vec<Rule>) {
    let names = string_list(root.get("off"), "off", problems);
    let mut known: Vec<&str> = rules.iter().map(|r| r.name.as_str()).collect();
    known.extend(tables.iter().map(|table| table.name.as_str()));
    for name in &names {
        if !known.contains(&name.as_str()) {
            problems.push(invalid(
                "off",
                &format!(
                    "no rule is named `{name}`, so `off` turns nothing off; {}",
                    suggestion(name, &known)
                ),
            ));
        }
    }
    rules.into_iter().partition(|rule| {
        let of_a_table = match &rule.kind {
            RuleKind::TableRow { table, .. } => names.contains(&tables[table.index()].name),
            _ => false,
        };
        !of_a_table && !names.contains(&rule.name)
    })
}

fn suggestion(name: &str, known: &[&str]) -> String {
    match nearest(name, known) {
        Some(near) => format!("did you mean `{near}`?"),
        None if known.is_empty() => "there are no such rules".to_owned(),
        None => format!(
            "the rules are {}",
            known
                .iter()
                .map(|k| format!("`{k}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn invalid(field: &str, message: &str) -> ConfigProblem {
    ConfigProblem::new(ProblemCode::ConfigInvalid, message).at(field)
}
