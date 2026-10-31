//! Plugins as the loader describes them, and the rules that use their conditions: the names a
//! rule may use, the schema version, and each rule's options against its condition's schema.

use serde_json::{Map, Value};

use crate::model::{Plugin, PluginCondition, PluginUse, Reads};
use crate::parse::nearest;
use crate::problem::{ConfigProblem, ProblemCode};

/// The version of the view that this archangel sends to plugins.
pub const VIEW_SCHEMA_VERSION: u64 = 1;

/// The facts a condition may read, as the loader names them.
const FACTS: [&str; 3] = ["imports", "importers", "exports"];

/// The plugins of the loader's description, `{ name: { schemaVersion, conditions } }`.
pub(crate) fn parse_plugins(
    value: Option<&Value>,
    problems: &mut Vec<ConfigProblem>,
) -> Vec<Plugin> {
    let Some(value) = value else {
        return Vec::new();
    };
    let Some(map) = value.as_object() else {
        problems.push(invalid(
            "plugins",
            "must be an object of plugin names to plugins",
        ));
        return Vec::new();
    };
    map.iter()
        .filter_map(|(name, plugin)| parse_plugin(name, plugin, problems))
        .collect()
}

fn parse_plugin(name: &str, value: &Value, problems: &mut Vec<ConfigProblem>) -> Option<Plugin> {
    let field = format!("plugins.{name}");
    if name.is_empty() || name.contains('/') {
        problems.push(invalid(
            &field,
            &format!(
                "`{name}` cannot name a plugin, because a rule writes the plugin's name and a slash \
                 before the condition's name; give the plugin a name without a slash"
            ),
        ));
        return None;
    }
    let Some(object) = value.as_object() else {
        problems.push(invalid(
            &field,
            &author(name, "is not an object made with `definePlugin`"),
        ));
        return None;
    };
    let schema_version = object.get("schemaVersion").and_then(Value::as_u64);
    match schema_version {
        None => problems.push(invalid(
            &format!("{field}.schemaVersion"),
            &author(name, "declares no `schemaVersion`, the version of the view it was written for"),
        )),
        Some(version) if version != VIEW_SCHEMA_VERSION => problems.push(invalid(
            &field,
            &format!(
                "the plugin \"{name}\" was written for version {version} of the view, and this \
                 archangel sends version {VIEW_SCHEMA_VERSION}; use a release of the plugin written \
                 for version {VIEW_SCHEMA_VERSION}, or an archangel that sends version {version}"
            ),
        )),
        Some(_) => {}
    }
    let conditions = if let Some(Value::Object(conditions)) = object.get("conditions") {
        conditions
            .iter()
            .filter_map(|(condition, value)| parse_condition(name, condition, value, problems))
            .collect()
    } else {
        problems.push(invalid(
            &format!("{field}.conditions"),
            &author(name, "lists no conditions"),
        ));
        Vec::new()
    };
    Some(Plugin {
        name: name.to_owned(),
        schema_version: schema_version.unwrap_or_default(),
        conditions,
    })
}

fn parse_condition(
    plugin: &str,
    name: &str,
    value: &Value,
    problems: &mut Vec<ConfigProblem>,
) -> Option<PluginCondition> {
    let field = format!("plugins.{plugin}.conditions.{name}");
    let object = value.as_object()?;
    let description = object
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let docs = object
        .get("docs")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let mut reads = Reads::default();
    for fact in object
        .get("reads")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        match fact.as_str() {
            Some("imports") => reads.imports = true,
            Some("importers") => reads.importers = true,
            Some("exports") => reads.exports = true,
            _ => problems.push(invalid(
                &format!("{field}.reads"),
                &author(
                    plugin,
                    &format!(
                        "says its condition `{name}` reads {fact}, and a condition reads {}",
                        FACTS.join(", ")
                    ),
                ),
            )),
        }
    }
    if let Some(schema) = object.get("options").filter(|schema| !schema.is_null())
        && let Err(error) = jsonschema::validator_for(schema)
    {
        problems.push(invalid(
            &format!("{field}.options"),
            &author(
                plugin,
                &format!("gives its condition `{name}` an options schema that is not valid JSON Schema: {error}"),
            ),
        ));
    }
    Some(PluginCondition {
        name: name.to_owned(),
        description,
        docs,
        reads,
    })
}

/// The plugins a config loads, and the loader's description of them, which holds each
/// condition's options schema.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct LoadedPlugins<'a> {
    pub(crate) plugins: &'a [Plugin],
    pub(crate) described: Option<&'a Value>,
}

/// The condition a rule names, `<plugin>/<condition>`, with its options checked against the
/// condition's schema.
pub(crate) fn plugin_use(
    rule: &Map<String, Value>,
    field: &str,
    loaded: LoadedPlugins<'_>,
    problems: &mut Vec<ConfigProblem>,
) -> Option<PluginUse> {
    let LoadedPlugins { plugins, described } = loaded;
    let plugin_field = format!("{field}.plugin");
    let Some(text) = rule.get("plugin").and_then(Value::as_str) else {
        problems.push(invalid(
            &plugin_field,
            "must name a plugin condition, such as `team/one-store-per-entity`",
        ));
        return None;
    };
    let found = text.split_once('/').and_then(|(plugin, condition)| {
        let plugin = plugins.iter().find(|p| p.name == plugin)?;
        let condition = plugin.conditions.iter().find(|c| c.name == condition)?;
        Some((plugin, condition))
    });
    let Some((plugin, condition)) = found else {
        problems.push(invalid(&plugin_field, &missing(text, plugins)));
        return None;
    };
    let options = rule
        .get("options")
        .cloned()
        .unwrap_or_else(|| Value::Object(Map::new()));
    let schema = described
        .and_then(|plugins| plugins.get(&plugin.name))
        .and_then(|plugin| plugin.get("conditions"))
        .and_then(|conditions| conditions.get(&condition.name))
        .and_then(|condition| condition.get("options"))
        .filter(|schema| !schema.is_null());
    let wrong = match schema {
        None => rule
            .contains_key("options")
            .then(|| format!("the condition `{text}` takes no options")),
        Some(schema) => misfits(schema, &options)
            .map(|errors| format!("the options do not fit the schema of `{text}`: {errors}")),
    };
    if let Some(message) = wrong {
        problems.push(invalid(&format!("{field}.options"), &message));
        return None;
    }
    Some(PluginUse {
        plugin: plugin.name.clone(),
        condition: condition.name.clone(),
        options,
        reads: condition.reads,
    })
}

/// Where options break a schema, each with the option's path, or `None` when they fit. A schema
/// that does not compile is the plugin's problem, reported where the plugin is.
fn misfits(schema: &Value, options: &Value) -> Option<String> {
    let validator = jsonschema::validator_for(schema).ok()?;
    let errors: Vec<String> = validator
        .iter_errors(options)
        .map(|error| {
            let path = error.instance_path().to_string();
            let at = path.trim_start_matches('/').replace('/', ".");
            if at.is_empty() {
                format!("the options: {error}")
            } else {
                format!("`{at}`: {error}")
            }
        })
        .collect();
    (!errors.is_empty()).then(|| errors.join("; "))
}

/// The message for a rule that names a condition no loaded plugin has.
fn missing(text: &str, plugins: &[Plugin]) -> String {
    let known: Vec<String> = plugins
        .iter()
        .flat_map(|plugin| {
            plugin
                .conditions
                .iter()
                .map(move |condition| format!("{}/{}", plugin.name, condition.name))
        })
        .collect();
    if known.is_empty() {
        return format!(
            "the config loads no plugin, so no rule can use `{text}`; add the plugin to `plugins`"
        );
    }
    let refs: Vec<&str> = known.iter().map(String::as_str).collect();
    let suggestion = nearest(text, &refs)
        .map(|name| format!("did you mean `{name}`? "))
        .unwrap_or_default();
    format!(
        "no plugin the config loads has the condition `{text}`; {suggestion}The loaded conditions \
         are {}",
        refs.join(", ")
    )
}

/// A problem that only the plugin's author can fix.
fn author(plugin: &str, problem: &str) -> String {
    format!("the plugin \"{plugin}\" {problem}; report it to the plugin's author")
}

fn invalid(field: &str, message: &str) -> ConfigProblem {
    ConfigProblem::new(ProblemCode::ConfigInvalid, message).at(field)
}
