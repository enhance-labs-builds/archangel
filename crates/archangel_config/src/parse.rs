//! Turns the config's JSON into a validated [`Config`], with every problem it finds.
//!
//! The walk is by hand rather than through serde's derive, because serde stops at the first
//! problem and the developer should see every problem in one run.

use serde_json::{Map, Value};

use crate::merge::{self, ConfigPart};
use crate::model::{
    AllowUnresolved, Config, Element, ElementId, ElementKind, ExampleFiles, Examples, Preset, Rule,
    RuleSource, SelectorId, Table,
};
use crate::pattern::Pattern;
use crate::plugins::{LoadedPlugins, parse_plugins};
use crate::problem::{ConfigProblem, ProblemCode};
use crate::selectors::{Declared, Names, Placement, parse_selectors};
use crate::tables::TableReader;

/// Parses config text in JSON.
///
/// # Errors
///
/// Returns every problem in the config, or the one parse error with its line and column.
pub fn parse_json(text: &str) -> Result<Config, Vec<ConfigProblem>> {
    let value: Value = serde_json::from_str(text).map_err(|error| {
        let mut problem = ConfigProblem::new(
            ProblemCode::ConfigInvalid,
            format!("the config is not valid JSON: {}", plain_json_error(&error)),
        );
        // serde_json counts columns from 1, except at the start of a line, where it says 0.
        problem.position = Some((error.line(), error.column().max(1)));
        vec![problem]
    })?;
    let (value, refused) = without_code(value);
    let checked = match (from_value(&value), refused) {
        (Ok(config), refused) if refused.is_empty() => Ok(config),
        (Ok(_), refused) => Err(refused),
        (Err(mut problems), refused) => {
            problems.splice(0..0, refused);
            Err(problems)
        }
    };
    checked.map_err(|mut problems| {
        let positions = crate::locate::positions(text);
        for problem in &mut problems {
            if let Some(field) = &problem.field {
                problem.position = positions.of(field);
            }
        }
        problems
    })
}

/// A JSON config without its `plugins` and `extends`, which only code can load, and the
/// problems that say so.
fn without_code(mut value: Value) -> (Value, Vec<ConfigProblem>) {
    let mut refused = Vec::new();
    if let Some(root) = value.as_object_mut() {
        if root.remove("plugins").is_some() {
            refused.push(
                ConfigProblem::new(
                    ProblemCode::ConfigInvalid,
                    "a plugin is code, so only a TypeScript or JavaScript config can load one; \
                     rename the config to archangel.config.ts and import each plugin into \
                     `plugins`",
                )
                .at("plugins"),
            );
        }
        if root.remove("extends").is_some() {
            refused.push(
                ConfigProblem::new(
                    ProblemCode::ConfigInvalid,
                    "a preset is a function, so presets need a TypeScript or JavaScript config; \
                     rename the config to archangel.config.ts and call each preset in `extends`",
                )
                .at("extends"),
            );
        }
    }
    (value, refused)
}

/// Validates a config that is already JSON data, such as the output of the Node loader.
///
/// # Errors
///
/// Returns every problem in the config.
pub fn from_value(value: &Value) -> Result<Config, Vec<ConfigProblem>> {
    let mut problems = Vec::new();
    let Some(root) = value.as_object() else {
        return Err(vec![ConfigProblem::new(
            ProblemCode::ConfigInvalid,
            "the config must be an object with `elements` and `rules`",
        )]);
    };
    unknown_fields(root, &ROOT_FIELDS, "", &mut problems);
    schema_field(root, &mut problems);

    let parts = merge::config_parts(root, &mut problems);
    let (elements, element_ranges) = read_elements(root, &parts, &mut problems);
    let (declared, selector_ranges) = collect(&parts, &mut problems, declared_selectors);
    let described = merged_plugins(&parts, &mut problems);
    let plugins = parse_plugins(described.as_ref(), &mut problems);
    let mut names = Names::new(&elements, declared.iter().map(|d| d.name.clone()).collect());
    names.says_parts = says_parts(root);
    names.plugins = LoadedPlugins {
        plugins: &plugins,
        described: described.as_ref(),
    };
    let (selectors, selector_order) = parse_selectors(&declared, &mut names, &mut problems);
    let presets = presets_of(&parts, &element_ranges, &selector_ranges);
    let Found {
        preset: preset_rules,
        config: config_rules,
        tables,
    } = read_rules(&parts, &mut names, &mut problems);
    let rules = merge::replace(preset_rules, config_rules, &presets, &mut problems);
    let (rules, off) = merge::off(root, rules, &tables, &mut problems);
    let patterns = names.patterns;
    let config = Config {
        selectors,
        selector_order,
        patterns,
        elements,
        ignore: string_list(root.get("ignore"), "ignore", &mut problems),
        allow_unresolved: parse_allow_unresolved(root.get("allowUnresolved"), &mut problems),
        rules,
        plugins,
        presets,
        off,
        tables,
        says_parts: says_parts(root),
    };
    crate::examples::one_liners_belong(&config, &mut problems);
    finish(config, problems)
}

/// The parts or elements of the config and its presets. Under `parts`, each preset has its own set.
fn read_elements(
    root: &Map<String, Value>,
    parts: &[ConfigPart<'_>],
    problems: &mut Vec<ConfigProblem>,
) -> (Vec<Element>, Vec<std::ops::Range<usize>>) {
    let own_parts = root.contains_key("parts");
    let mut presets_read = 0;
    collect(parts, problems, |part, problems| {
        let set = if part.scope.prefix.is_some() && own_parts {
            presets_read += 1;
            presets_read
        } else {
            0
        };
        elements_field(part, parts.len() > 1, set, problems)
    })
}

/// The rules of the config and its presets, each list with unique names.
fn read_rules(
    parts: &[ConfigPart<'_>],
    names: &mut Names<'_>,
    problems: &mut Vec<ConfigProblem>,
) -> Found {
    let mut found = Found::default();
    for part in parts {
        names.scope = part.scope.clone();
        rules_field(part, names, &mut found, problems);
    }
    names.scope = Placement::default();
    check_unique_names(&found.preset, problems);
    let own: Vec<Rule> = found
        .config
        .iter()
        .map(|(rule, _, _)| rule.clone())
        .collect();
    check_unique_names(&own, problems);
    found
}

fn schema_field(root: &Map<String, Value>, problems: &mut Vec<ConfigProblem>) {
    if root
        .get("$schema")
        .is_some_and(|schema| !schema.is_string())
    {
        problems.push(invalid(
            "$schema",
            "must be the path or URL of archangel's JSON schema",
        ));
    }
}

/// What `read` gives for each part of the config, together, with the range each part's items
/// take.
fn collect<'v, T>(
    parts: &[ConfigPart<'v>],
    problems: &mut Vec<ConfigProblem>,
    mut read: impl FnMut(&ConfigPart<'v>, &mut Vec<ConfigProblem>) -> Vec<T>,
) -> (Vec<T>, Vec<std::ops::Range<usize>>) {
    let mut items = Vec::new();
    let mut ranges = Vec::new();
    for part in parts {
        let start = items.len();
        items.extend(read(part, problems));
        ranges.push(start..items.len());
    }
    (items, ranges)
}

/// The presets, each with the elements and selectors it declared.
fn presets_of(
    parts: &[ConfigPart<'_>],
    elements: &[std::ops::Range<usize>],
    selectors: &[std::ops::Range<usize>],
) -> Vec<Preset> {
    parts
        .iter()
        .zip(elements.iter().zip(selectors))
        .filter_map(|(part, (elements, selectors))| {
            Some(Preset {
                name: part.scope.prefix.clone()?,
                root: part.scope.root.clone(),
                elements: elements.clone().map(|i| ElementId(id(i))).collect(),
                selectors: selectors.clone().map(|i| SelectorId(id(i))).collect(),
            })
        })
        .collect()
}

/// The rules read so far: the presets', with their prefixed names, and the config's own, each
/// with its field and whether the config gave its name.
#[derive(Default)]
struct Found {
    preset: Vec<Rule>,
    config: Vec<(Rule, String, bool)>,
    tables: Vec<Table>,
}

fn id(index: usize) -> u32 {
    u32::try_from(index).unwrap_or(u32::MAX)
}

/// The key of one of two spellings that the part uses, and its value. A part with both is refused
/// at the second, and the first is read.
pub(crate) fn one_spelling<'v>(
    object: &'v Map<String, Value>,
    spellings: [&'static str; 2],
    at: impl Fn(&str) -> String,
    problems: &mut Vec<ConfigProblem>,
) -> Option<(&'static str, &'v Value)> {
    let mut found = None;
    for key in object.keys() {
        let Some(&spelling) = spellings.iter().find(|spelling| **spelling == key.as_str()) else {
            continue;
        };
        match found {
            None => found = Some((spelling, &object[key])),
            Some((first, _)) => problems.push(invalid(
                &at(key),
                &format!(
                    "a config uses one spelling for each key, and `{first}` and `{key}` mean the \
                     same; keep one of them"
                ),
            )),
        }
    }
    found
}

fn elements_field(
    source: &ConfigPart<'_>,
    has_presets: bool,
    set: u32,
    problems: &mut Vec<ConfigProblem>,
) -> Vec<Element> {
    let spelled = one_spelling(
        source.object,
        ["parts", "elements"],
        |k| source.at(k),
        problems,
    );
    let mut elements = match spelled {
        Some((key, Value::Object(map))) => parse_elements(map, source, (key, set), problems),
        Some((key, _)) => {
            problems.push(invalid(
                &source.at(key),
                &format!("must be an object of {} names to patterns", part_word(key)),
            ));
            Vec::new()
        }
        None if source.scope.prefix.is_none() && !has_presets && !has_rules(source.object) => {
            problems.push(invalid(
                "parts",
                "is missing, and the config has no rule; declare a part, such as \
                 `parts: { app: 'src/**' }`, or write a rule",
            ));
            Vec::new()
        }
        None => Vec::new(),
    };
    elements.extend(externals_field(source, set, problems));
    elements
}

/// Whether the config's own words for its parts are the new ones: it writes them, or it writes
/// neither spelling and no preset.
fn says_parts(root: &Map<String, Value>) -> bool {
    let has = |key: &str| root.contains_key(key);
    has("parts") || has("groups") || !(has("elements") || has("selectors") || has("extends"))
}

/// What a key of the config calls the things it declares.
fn part_word(key: &str) -> &'static str {
    if key == "parts" { "part" } else { "element" }
}

/// Whether the config writes a rule.
fn has_rules(root: &Map<String, Value>) -> bool {
    match root.get("rules") {
        None => false,
        Some(Value::Array(rules)) => !rules.is_empty(),
        // Not a list: `rules_field` refuses it, and this problem would only add noise.
        Some(_) => true,
    }
}

/// The externals sets of a part, as package elements.
fn externals_field(
    source: &ConfigPart<'_>,
    set: u32,
    problems: &mut Vec<ConfigProblem>,
) -> Vec<Element> {
    let Some(value) = source.object.get("externals") else {
        return Vec::new();
    };
    let Some(map) = value.as_object() else {
        problems.push(invalid(
            &source.at("externals"),
            "must be an object of set names to lists of package names",
        ));
        return Vec::new();
    };
    let mut elements = Vec::new();
    for (written, names) in map {
        if !own_name(source, written, "externals", problems) {
            continue;
        }
        let field = source.at(&format!("externals.{written}"));
        if let Some(other) = declared_as(source, written) {
            problems.push(invalid(
                &field,
                &format!(
                    "\"{written}\" is also a {other}; a rule side looks a name up as either, so \
                     give the externals set its own name"
                ),
            ));
            continue;
        }
        let Value::Array(items) = names else {
            problems.push(invalid(&field, "must be a list of package names"));
            continue;
        };
        let names = strings(items, &field, problems);
        if names.is_empty() {
            problems.push(invalid(&field, "lists no packages"));
        }
        for name in &names {
            if let Some(reason) = never_matches(name) {
                problems.push(invalid(&field, &reason));
            }
        }
        let texts: Vec<&str> = names.iter().map(String::as_str).collect();
        let name = source.scope.name(written);
        elements.push(Element {
            kind: ElementKind::Packages(patterns(&name, &field, &texts, problems)),
            name,
            set,
            part: false,
            externals: true,
        });
    }
    elements
}

/// What else the part declares under a name, for an externals set of that name.
fn declared_as(source: &ConfigPart<'_>, name: &str) -> Option<&'static str> {
    let has = |key: &str| {
        source
            .object
            .get(key)
            .and_then(Value::as_object)
            .is_some_and(|map| map.contains_key(name))
    };
    if has("parts") || has("elements") {
        Some("part")
    } else if has("groups") || has("selectors") {
        Some("group")
    } else {
        None
    }
}

/// Why no import can ever count as this name of an externals set: archangel records the package
/// that an import resolves to, under the `node:` name for a built-in module.
fn never_matches(name: &str) -> Option<String> {
    if name.starts_with("node:") {
        return None;
    }
    if nodejs_built_in_modules::BUILTINS.contains(&name) {
        return Some(format!(
            "`{name}` is a built-in module, which archangel records with its prefix; write \
             `node:{name}`"
        ));
    }
    if let Some(types) = name.strip_prefix("@types/") {
        return Some(format!(
            "archangel judges the package that an import names, such as `{types}`, and no import \
             names `{name}`"
        ));
    }
    let folders = if name.starts_with('@') { 2 } else { 1 };
    let package = name.split('/').take(folders).collect::<Vec<_>>().join("/");
    (package.len() < name.len()).then(|| {
        format!(
            "archangel records the package that an import resolves to, not its subpath, so write \
             `{package}`, which also counts every subpath"
        )
    })
}

fn declared_selectors<'v>(
    source: &ConfigPart<'v>,
    problems: &mut Vec<ConfigProblem>,
) -> Vec<Declared<'v>> {
    match one_spelling(
        source.object,
        ["groups", "selectors"],
        |k| source.at(k),
        problems,
    ) {
        None => Vec::new(),
        Some((key, Value::Object(map))) => map
            .iter()
            .filter(|(name, _)| own_name(source, name, key, problems))
            .map(|(name, definition)| Declared {
                name: source.scope.name(name),
                definition,
                field: source.at(&format!("{key}.{name}")),
                scope: source.scope.clone(),
            })
            .collect(),
        Some((key, _)) => {
            problems.push(invalid(
                &source.at(key),
                &format!(
                    "must be an object of {} names to definitions",
                    if key == "groups" { "group" } else { "selector" }
                ),
            ));
            Vec::new()
        }
    }
}

/// Whether a name the config or a preset declares is free of slashes, which only the prefix of a
/// preset's names holds.
fn own_name(
    source: &ConfigPart<'_>,
    name: &str,
    kind: &str,
    problems: &mut Vec<ConfigProblem>,
) -> bool {
    if !name.contains('/') {
        return true;
    }
    problems.push(invalid(
        &source.at(&format!("{kind}.{name}")),
        "a name cannot contain a slash, because a slash separates a preset's name from the names \
         it declares, as in `fsd/shared`",
    ));
    false
}

/// The plugins of every preset and of the config, by name: a preset brings the plugins its rules
/// use, and a later part's plugin replaces an earlier one of the same name, as the loader hosts
/// them. A preset's `plugins` that are not an object are refused, for its author to fix.
fn merged_plugins(sources: &[ConfigPart<'_>], problems: &mut Vec<ConfigProblem>) -> Option<Value> {
    let mut merged = Map::new();
    let mut any = false;
    for source in sources {
        match source.object.get("plugins") {
            Some(Value::Object(map)) => {
                any = true;
                for (name, plugin) in map {
                    merged.insert(name.clone(), plugin.clone());
                }
            }
            Some(other) => match &source.scope.prefix {
                None => return Some(other.clone()),
                Some(prefix) => problems.push(invalid(
                    &source.at("plugins"),
                    &format!(
                        "the preset \"{prefix}\"{} gives `plugins` that are not an object of \
                         plugin names to plugins, so archangel cannot load them; the preset's \
                         author must fix it, so report it to them",
                        source
                            .package
                            .as_deref()
                            .map_or_else(String::new, |p| format!(" from the package `{p}`"))
                    ),
                )),
            },
            None => {}
        }
    }
    any.then_some(Value::Object(merged))
}

fn rules_field(
    source: &ConfigPart<'_>,
    names: &mut Names<'_>,
    found: &mut Found,
    problems: &mut Vec<ConfigProblem>,
) {
    let items = match source.object.get("rules") {
        None => return,
        Some(Value::Array(items)) => items,
        Some(_) => {
            problems.push(invalid(&source.at("rules"), "must be a list of rules"));
            return;
        }
    };
    for (index, item) in items.iter().enumerate() {
        let field = source.at(&format!("rules[{index}]"));
        if item.get("kind").and_then(Value::as_str) == Some("mayImport") {
            let rules = TableReader {
                source,
                names: &mut *names,
                tables: &mut found.tables,
                problems: &mut *problems,
            }
            .read(item, &field);
            if source.scope.prefix.is_some() {
                found.preset.extend(rules);
            } else {
                found
                    .config
                    .extend(rules.into_iter().map(|rule| (rule, field.clone(), true)));
            }
        } else if source.scope.prefix.is_some() {
            let Some(name) = merge::preset_rule_name(source, item, &field, problems) else {
                continue;
            };
            if let Some(mut rule) = crate::rules::parse_rule(item, field, names, problems) {
                rule.name = name;
                rule.examples = rooted(rule.examples, &source.scope);
                found.preset.push(rule);
            }
        } else if let Some(rule) = crate::rules::parse_rule(item, field.clone(), names, problems) {
            found.config.push((rule, field, item.get("name").is_some()));
        }
    }
}

/// Where the loader says that a builder made this rule.
pub(crate) fn rule_source(object: &Map<String, Value>) -> Option<RuleSource> {
    let at = object.get("$at")?;
    Some(RuleSource {
        file: at.get("file")?.as_str()?.to_owned(),
        line: u32::try_from(at.get("line")?.as_u64()?).ok()?,
    })
}

/// A preset's examples, with their paths moved from its root to the config folder.
pub(crate) fn rooted(examples: Examples, scope: &Placement) -> Examples {
    let move_set = |files: ExampleFiles| -> ExampleFiles {
        files
            .into_iter()
            .map(|(path, text)| (scope.path(&path), text))
            .collect()
    };
    Examples {
        accepted: examples.accepted.into_iter().map(move_set).collect(),
        rejected: examples.rejected.into_iter().map(move_set).collect(),
        one_line: examples.one_line,
    }
}

/// The config, unless the walk found a problem with it.
fn finish(config: Config, problems: Vec<ConfigProblem>) -> Result<Config, Vec<ConfigProblem>> {
    if problems.is_empty() {
        Ok(config)
    } else {
        Err(problems)
    }
}

/// What one entry of `parts` or `elements` finds, or `None` with a problem.
fn element_kind(
    (source, key): (&ConfigPart<'_>, &str),
    (name, field): (&str, &str),
    spec: &Value,
    problems: &mut Vec<ConfigProblem>,
) -> Option<ElementKind> {
    match spec {
        Value::String(pattern) => Some(ElementKind::Files(patterns(
            name,
            field,
            &[source.scope.path(pattern).as_str()],
            problems,
        ))),
        Value::Array(items) => {
            let texts = strings(items, field, problems);
            if texts.is_empty() {
                problems.push(invalid(field, "lists no patterns"));
            }
            let texts: Vec<String> = texts.iter().map(|t| source.scope.path(t)).collect();
            let texts: Vec<&str> = texts.iter().map(String::as_str).collect();
            Some(ElementKind::Files(patterns(name, field, &texts, problems)))
        }
        Value::Object(_) if key == "parts" => {
            problems.push(invalid(
                field,
                "a part is a set of files, so move these packages to `externals`, a list of \
                 named sets of packages",
            ));
            None
        }
        Value::Object(object) => {
            unknown_fields(object, &["packages"], field, problems);
            let packages = string_list(
                object.get("packages"),
                &format!("{field}.packages"),
                problems,
            );
            if packages.is_empty() {
                problems.push(invalid(&format!("{field}.packages"), "lists no packages"));
            }
            let texts: Vec<&str> = packages.iter().map(String::as_str).collect();
            Some(ElementKind::Packages(patterns(
                name, field, &texts, problems,
            )))
        }
        _ => {
            problems.push(invalid(
                field,
                "must be a pattern, a list of patterns, or `{ packages: [...] }`",
            ));
            None
        }
    }
}

fn parse_elements(
    map: &Map<String, Value>,
    source: &ConfigPart<'_>,
    (key, set): (&str, u32),
    problems: &mut Vec<ConfigProblem>,
) -> Vec<Element> {
    let mut elements = Vec::new();
    for (written, spec) in map {
        if !own_name(source, written, key, problems) {
            continue;
        }
        let name = &source.scope.name(written);
        let field = source.at(&format!("{key}.{written}"));
        let Some(kind) = element_kind((source, key), (name, &field), spec, problems) else {
            continue;
        };
        elements.push(Element {
            name: name.clone(),
            kind,
            set,
            part: key == "parts" && source.scope.prefix.is_none(),
            externals: false,
        });
    }
    elements
}

pub(crate) fn patterns(
    element: &str,
    field: &str,
    texts: &[&str],
    problems: &mut Vec<ConfigProblem>,
) -> Vec<Pattern> {
    texts
        .iter()
        .filter_map(|text| match Pattern::parse(text) {
            Ok(pattern) => Some(pattern),
            Err(error) => {
                problems.push(
                    ConfigProblem::new(
                        ProblemCode::PatternInvalid,
                        format!("the pattern `{text}` of \"{element}\" is invalid: {error}"),
                    )
                    .at(field),
                );
                None
            }
        })
        .collect()
}

fn parse_allow_unresolved(
    value: Option<&Value>,
    problems: &mut Vec<ConfigProblem>,
) -> Vec<AllowUnresolved> {
    match value {
        None => Vec::new(),
        Some(Value::Object(map)) => map
            .iter()
            .filter_map(|(specifier, reason)| match reason {
                Value::String(reason) if !reason.trim().is_empty() => Some(AllowUnresolved {
                    specifier: specifier.clone(),
                    reason: reason.clone(),
                }),
                _ => {
                    problems.push(invalid(
                        &format!("allowUnresolved.{specifier}"),
                        "must give the reason the specifier cannot be resolved",
                    ));
                    None
                }
            })
            .collect(),
        Some(_) => {
            problems.push(invalid("allowUnresolved", "must map specifiers to reasons"));
            Vec::new()
        }
    }
}

const ROOT_FIELDS: [&str; 12] = [
    "$schema",
    "parts",
    "elements",
    "groups",
    "selectors",
    "externals",
    "rules",
    "ignore",
    "allowUnresolved",
    "plugins",
    "extends",
    "off",
];

/// The known name nearest to a misspelt one, when one is near enough to be the likely intent.
#[must_use]
pub fn nearest<'n>(name: &str, known: &[&'n str]) -> Option<&'n str> {
    known
        .iter()
        .map(|candidate| (strsim::damerau_levenshtein(name, candidate), *candidate))
        .min()
        .filter(|(distance, candidate)| *distance <= 2.max(candidate.len() / 4))
        .map(|(_, candidate)| candidate)
}

fn check_unique_names(rules: &[Rule], problems: &mut Vec<ConfigProblem>) {
    let mut seen: Vec<&str> = Vec::new();
    for rule in rules {
        if seen.contains(&rule.name.as_str()) {
            problems.push(ConfigProblem::new(
                ProblemCode::RuleNameDuplicate,
                format!(
                    "two rules are named `{}`; give each rule its own `name`, because exceptions \
                     and the baseline refer to rules by name",
                    rule.name
                ),
            ));
        } else {
            seen.push(&rule.name);
        }
    }
}

pub(crate) fn parse_examples(
    value: Option<&Value>,
    field: &str,
    problems: &mut Vec<ConfigProblem>,
) -> Examples {
    let field = format!("{field}.examples");
    let Some(value) = value else {
        return Examples::default();
    };
    let Some(object) = value.as_object() else {
        problems.push(invalid(
            &field,
            "must be `{ accepted: [...], rejected: [...] }`",
        ));
        return Examples::default();
    };
    unknown_fields(object, &["accepted", "rejected"], &field, problems);
    let one_line = ["accepted", "rejected"].iter().any(|key| {
        object
            .get(*key)
            .and_then(Value::as_array)
            .is_some_and(|items| items.iter().any(Value::is_string))
    });
    let sets = |key: &str, problems: &mut Vec<ConfigProblem>| -> Vec<ExampleFiles> {
        let field = format!("{field}.{key}");
        match object.get(key) {
            None => Vec::new(),
            Some(Value::Array(items)) => items
                .iter()
                .enumerate()
                .filter_map(|(index, item)| {
                    example_files(item, &format!("{field}[{index}]"), problems)
                })
                .collect(),
            Some(_) => {
                problems.push(invalid(&field, "must be a list of file sets"));
                Vec::new()
            }
        }
    };
    Examples {
        accepted: sets("accepted", problems),
        rejected: sets("rejected", problems),
        one_line,
    }
}

fn example_files(
    value: &Value,
    field: &str,
    problems: &mut Vec<ConfigProblem>,
) -> Option<ExampleFiles> {
    if let Value::String(line) = value {
        return match crate::examples::one_line(line) {
            Ok(files) => Some(files),
            Err(reason) => {
                problems.push(invalid(field, &reason));
                None
            }
        };
    }
    let Some(object) = value.as_object() else {
        problems.push(invalid(field, "must map file paths to their text"));
        return None;
    };
    let mut files = ExampleFiles::new();
    for (path, text) in object {
        if !crate::examples::path_is_inside(path) {
            problems.push(invalid(
                field,
                &format!(
                    "`{path}` must be a path inside the repository, relative to the config folder"
                ),
            ));
            continue;
        }
        let Value::String(text) = text else {
            problems.push(invalid(
                &format!("{field}.{path}"),
                "must be the file's text",
            ));
            continue;
        };
        files.insert(path.clone(), text.clone());
    }
    Some(files)
}

pub(crate) fn unknown_fields(
    object: &Map<String, Value>,
    allowed: &[&str],
    field: &str,
    problems: &mut Vec<ConfigProblem>,
) {
    for key in object.keys() {
        // The loader writes `$name` for a name given two times, which `parse_rule` refuses, and
        // `$at` for where a builder was called.
        if !allowed.contains(&key.as_str()) && key != "$name" && key != "$at" {
            let location = if field.is_empty() {
                key.clone()
            } else {
                format!("{field}.{key}")
            };
            let known = allowed.join(", ");
            let message = match nearest(key, allowed) {
                Some(name) => format!(
                    "archangel does not know this field; did you mean `{name}`? The known fields are {known}"
                ),
                None => format!("archangel does not know this field; the known fields are {known}"),
            };
            problems.push(invalid(&location, &message));
        }
    }
}

pub(crate) fn string_list(
    value: Option<&Value>,
    field: &str,
    problems: &mut Vec<ConfigProblem>,
) -> Vec<String> {
    match value {
        None => Vec::new(),
        Some(Value::String(text)) => vec![text.clone()],
        Some(Value::Array(items)) => strings(items, field, problems),
        Some(_) => {
            problems.push(invalid(field, "must be a list of strings"));
            Vec::new()
        }
    }
}

fn strings(items: &[Value], field: &str, problems: &mut Vec<ConfigProblem>) -> Vec<String> {
    items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            if let Value::String(text) = item {
                Some(text.clone())
            } else {
                problems.push(invalid(&format!("{field}[{index}]"), "must be a string"));
                None
            }
        })
        .collect()
}

pub(crate) fn optional_string(
    object: &Map<String, Value>,
    key: &str,
    field: &str,
    problems: &mut Vec<ConfigProblem>,
) -> Option<String> {
    match object.get(key) {
        None => None,
        Some(Value::String(text)) => Some(text.clone()),
        Some(_) => {
            problems.push(invalid(&format!("{field}.{key}"), "must be a string"));
            None
        }
    }
}

fn invalid(field: &str, message: &str) -> ConfigProblem {
    ConfigProblem::new(ProblemCode::ConfigInvalid, message).at(field)
}

/// `serde_json`'s message without its own "at line X column Y" suffix, which the position holds.
fn plain_json_error(error: &serde_json::Error) -> String {
    let text = error.to_string();
    text.split(" at line ").next().unwrap_or(&text).to_owned()
}
