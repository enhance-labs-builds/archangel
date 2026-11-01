//! Reading rules: the three shorthands, which compile to sentences here, and sentences written as
//! one selector and one condition.

use serde_json::{Map, Value};

use crate::model::{
    Condition, DependencyKind, Element, ElementId, ElementKind, ExportKind, FileSet, Folders,
    ImportKinds, ImportOptions, InstanceMatch, NameRule, Rule, RuleKind, Sentence, Side,
    SpecifierForm, Term,
};
use crate::names::{NamePattern, Owner, Style};
use crate::parse::{
    nearest, one_spelling, optional_string, parse_examples, patterns, string_list, unknown_fields,
};
use crate::pattern::Pattern;
use crate::plugins::plugin_use;
use crate::problem::{ConfigProblem, ProblemCode};
use crate::selectors::{Names, Placement, written};
use crate::shorthand::{KIND_LIST, element_names, layer_sentences, or_list, removed_kind};

const COMMON_FIELDS: [&str; 6] = ["name", "reason", "because", "link", "imports", "examples"];

const SHORTHANDS: [&str; 3] = ["layers", "isolate", "entry"];

/// The fields of a rule that uses a plugin condition.
const PLUGIN_FIELDS: [&str; 8] = [
    "files", "plugin", "options", "name", "reason", "because", "link", "examples",
];

/// The conditions about imports, which the import options limit.
const IMPORT_CONDITIONS: [&str; 3] = ["mayNotImport", "mayOnlyImport", "mayOnlyBeImportedBy"];

/// The conditions a rule about folders takes.
const FOLDER_CONDITIONS: [&str; 3] = ["mustBeNamed", "mustContain", "mustNotExist"];

/// The fields of a rule written as several sentences, which belong to the rule, not to one of
/// its sentences.
const SENTENCES_FIELDS: [&str; 7] = [
    "name",
    "reason",
    "because",
    "link",
    "imports",
    "examples",
    "sentences",
];

/// Every condition the rule language names, so that a rule with two of them is refused as one
/// mistake, whichever two they are.
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

const SPECIFIERS: [(&str, SpecifierForm); 4] = [
    ("relative", SpecifierForm::Relative),
    ("parent", SpecifierForm::Parent),
    ("alias", SpecifierForm::Alias),
    ("package", SpecifierForm::Package),
];

const INSTANCES: [(&str, InstanceMatch); 2] = [
    ("same", InstanceMatch::Same),
    ("different", InstanceMatch::Different),
];

const DEPENDENCIES: [(&str, DependencyKind); 2] = [
    ("dev", DependencyKind::Dev),
    ("undeclared", DependencyKind::Undeclared),
];

const OPTIONS: [&str; 7] = [
    "names",
    "specifier",
    "instance",
    "dependency",
    "transitively",
    "stopAt",
    "within",
];

/// One rule object, with its location in the config.
pub(crate) struct Input<'a> {
    pub(crate) object: &'a Map<String, Value>,
    pub(crate) field: String,
}

pub(crate) fn parse_rule(
    value: &Value,
    field: String,
    names: &mut Names<'_>,
    problems: &mut Vec<ConfigProblem>,
) -> Option<Rule> {
    let Some(object) = value.as_object() else {
        problems.push(invalid(&field, "must be a rule object"));
        return None;
    };
    let input = Input { object, field };
    let before = problems.len();
    let parsed = match object.get("kind") {
        Some(kind) => input.shorthand(kind, names, problems),
        None if object.contains_key("sentences") => input.sentences(names, problems),
        None if object.contains_key("plugin") => input.plugin(names, problems),
        None => input.sentence(names, problems),
    };
    let given_name = optional_string(object, "name", &input.field, problems);
    if object.contains_key("$name") {
        problems.push(invalid(
            &input.at("name"),
            "a config uses one spelling for each key, and this call names the rule two times; \
             keep the name as the first argument and remove the `name` key",
        ));
    }
    let reason_key = one_spelling(object, ["because", "reason"], |key| input.at(key), problems)
        .map_or("reason", |(key, _)| key);
    let reason = optional_string(object, reason_key, &input.field, problems);
    let link = optional_string(object, "link", &input.field, problems);
    let examples = parse_examples(object.get("examples"), &input.field, problems);
    let (kind, sentences, default_name, transitive) = parsed?;
    let imports = input.imports(&sentences, transitive, problems);
    if problems.len() != before {
        return None;
    }
    Some(Rule {
        name: given_name.unwrap_or(default_name),
        kind,
        sentences,
        reason,
        because: reason_key == "because",
        link,
        imports,
        examples,
        source: crate::parse::rule_source(object),
        replaces_preset_rule: false,
    })
}

type Parsed = (RuleKind, Vec<Sentence>, String, bool);

impl Input<'_> {
    pub(crate) fn at(&self, key: &str) -> String {
        format!("{}.{key}", self.field)
    }

    fn shorthand(
        &self,
        kind: &Value,
        names: &mut Names<'_>,
        problems: &mut Vec<ConfigProblem>,
    ) -> Option<Parsed> {
        let kind = self.shorthand_kind(kind, problems)?;
        match kind {
            "layers" => {
                let elements = names.elements;
                let order = self.element_list("order", names, problems);
                let name = format!("layers({})", element_names(&order, elements).join(", "));
                Some((
                    RuleKind::Layers {
                        order: order.clone(),
                    },
                    layer_sentences(&order, elements),
                    name,
                    false,
                ))
            }
            "isolate" => self.isolate(names, problems),
            _ => self.entry(names, problems),
        }
    }

    /// The shorthand's kind, after refusing a removed or unknown kind and the fields it does not
    /// take.
    fn shorthand_kind<'k>(
        &self,
        kind: &'k Value,
        problems: &mut Vec<ConfigProblem>,
    ) -> Option<&'k str> {
        let kind_field = self.at("kind");
        let Value::String(kind) = kind else {
            problems.push(invalid(&kind_field, KIND_LIST));
            return None;
        };
        if let Some(sentence) = removed_kind(kind, self.object) {
            problems.push(invalid(
                &kind_field,
                &format!(
                    "the rule kind `{kind}` is now a sentence of one selector and one condition; \
                     write `{sentence}`"
                ),
            ));
            return None;
        }
        let specific: &[&str] = match kind.as_str() {
            "layers" => &["order"],
            "isolate" => &["element"],
            "entry" => &["element", "files"],
            _ => {
                problems.push(invalid(
                    &kind_field,
                    &format!(
                        "`{kind}` is not a rule kind; {}{KIND_LIST}",
                        nearest(kind, &SHORTHANDS)
                            .map(|name| format!("did you mean `{name}`? It "))
                            .unwrap_or_default()
                    ),
                ));
                return None;
            }
        };
        let allowed: Vec<&str> = ["kind"]
            .iter()
            .chain(&COMMON_FIELDS)
            .chain(specific)
            .copied()
            .collect();
        unknown_fields(self.object, &allowed, &self.field, problems);
        Some(kind.as_str())
    }

    fn isolate(&self, names: &Names<'_>, problems: &mut Vec<ConfigProblem>) -> Option<Parsed> {
        let elements = names.elements;
        let element = self.one_element("element", names, problems)?;
        let name = elements[element.index()].name.clone();
        if !elements[element.index()].has_capture() {
            problems.push(
                ConfigProblem::new(
                    ProblemCode::RuleNeedsCapture,
                    format!(
                        "isolate needs {} whose pattern captures a name, such as \
                         `src/features/{{feature}}/**`, and \"{name}\" has none",
                        names.a_part()
                    ),
                )
                .at(self.at("element")),
            );
            return None;
        }
        let sentence = Sentence {
            files: FileSet::of(Term::Element(element)),
            folders: None,
            files_text: name.clone(),
            condition: Condition::MayNotImport(Side {
                files: vec![FileSet::of(Term::Element(element))],
                text: name.clone(),
                ..Side::default()
            }),
            options: ImportOptions {
                instance: Some(InstanceMatch::Different),
                ..ImportOptions::default()
            },
        };
        Some((
            RuleKind::Isolate { element },
            vec![sentence],
            format!("isolate({name})"),
            false,
        ))
    }

    fn entry(&self, names: &Names<'_>, problems: &mut Vec<ConfigProblem>) -> Option<Parsed> {
        let elements = names.elements;
        let element = self.one_element("element", names, problems);
        if let Some(id) = element
            && matches!(elements[id.index()].kind, ElementKind::Packages(_))
        {
            let word = names.part_word();
            problems.push(invalid(
                &self.at("element"),
                &format!(
                    "\"{}\" is a package {word}; an entry rule needs a file {word}",
                    elements[id.index()].name
                ),
            ));
        }
        let files_field = self.at("files");
        let texts = string_list(self.object.get("files"), &files_field, problems);
        if texts.is_empty() {
            problems.push(invalid(&files_field, "lists no entry files"));
        }
        let element = element?;
        let name = elements[element.index()].name.clone();
        let text_refs: Vec<&str> = texts.iter().map(String::as_str).collect();
        let files = patterns(&name, &files_field, &text_refs, problems);
        let sentence = Sentence {
            files: FileSet::of(Term::Element(element)),
            folders: None,
            files_text: name.clone(),
            condition: Condition::Entry {
                element,
                files: files.clone(),
            },
            options: ImportOptions::default(),
        };
        Some((
            RuleKind::Entry { element, files },
            vec![sentence],
            format!("entry({name})"),
            false,
        ))
    }

    /// A rule of several sentences, broken when any of them is: `sentences` and a `name`, with
    /// the reason, link, imports and examples of the whole rule.
    fn sentences(
        &self,
        names: &mut Names<'_>,
        problems: &mut Vec<ConfigProblem>,
    ) -> Option<Parsed> {
        unknown_fields(self.object, &SENTENCES_FIELDS, &self.field, problems);
        let field = self.at("sentences");
        let Some(Value::Array(items)) = self.object.get("sentences") else {
            problems.push(invalid(&field, "must be a list of sentences"));
            return None;
        };
        if items.is_empty() {
            problems.push(invalid(&field, "lists no sentences"));
        }
        if !self.object.contains_key("name") {
            problems.push(invalid(
                &self.at("name"),
                "a rule of several sentences needs a `name`, because exceptions and the \
                 baseline refer to it by name",
            ));
        }
        let before = problems.len();
        let mut sentences = Vec::new();
        let mut transitive = false;
        for (index, item) in items.iter().enumerate() {
            let field = format!("{field}[{index}]");
            let Some(object) = item.as_object() else {
                problems.push(invalid(&field, "must be a sentence object"));
                continue;
            };
            for key in [
                "name", "reason", "because", "link", "imports", "examples", "plugin",
            ] {
                if object.contains_key(key) {
                    problems.push(invalid(
                        &format!("{field}.{key}"),
                        &format!(
                            "a sentence of a rule takes no `{key}`; give it to the rule, or write \
                             the sentence as a rule of its own"
                        ),
                    ));
                }
            }
            let inner = Input { object, field };
            let parsed = match object.get("kind") {
                Some(kind) => inner.shorthand(kind, names, problems),
                None => inner.sentence(names, problems),
            };
            if let Some((_, found, _, chained)) = parsed {
                sentences.extend(found);
                transitive |= chained;
            }
        }
        (problems.len() == before)
            .then(|| (RuleKind::Sentence, sentences, String::new(), transitive))
    }

    /// A rule that uses a plugin's condition: `files`, `plugin` and `options`.
    fn plugin(&self, names: &mut Names<'_>, problems: &mut Vec<ConfigProblem>) -> Option<Parsed> {
        unknown_fields(self.object, &PLUGIN_FIELDS, &self.field, problems);
        let files_field = self.at("files");
        let files = if let Some(files) = self.object.get("files") {
            names
                .file_set(files, &files_field, problems)
                .map(|set| (set, written(files)))
        } else {
            problems.push(invalid(
                &files_field,
                "the rule needs `files`, the files whose records the plugin condition sees",
            ));
            None
        };
        let used = plugin_use(self.object, &self.field, names.plugins, problems);
        let ((files, files_text), used) = (files?, used?);
        let name = format!("{}({files_text})", used.full_name());
        Some((
            RuleKind::Sentence,
            vec![Sentence {
                files,
                folders: None,
                files_text,
                condition: Condition::Plugin(used),
                options: ImportOptions::default(),
            }],
            name,
            false,
        ))
    }

    fn sentence(&self, names: &mut Names<'_>, problems: &mut Vec<ConfigProblem>) -> Option<Parsed> {
        let condition_name = self.condition_name(problems)?;
        let (files, folders, files_text) = self.scope(condition_name, names, problems)?;
        let folder_rule = folders.is_some();
        let condition = self.condition(condition_name, folder_rule, names, problems);
        let options = self.options(condition_name, files.as_ref(), names, problems);
        let (files, condition, options) = (files?, condition?, options?);
        if let (Condition::MustContain(_), None) = (&condition, &folders)
            && !captured_element(&files, names.elements)
        {
            problems.push(
                ConfigProblem::new(
                    ProblemCode::RuleNeedsCapture,
                    format!(
                        "`mustContain` looks in each instance, so `files` must name {} whose \
                         pattern captures a name, such as `src/features/{{feature}}/**`; to look \
                         in each folder, write `folders` instead",
                        names.a_part()
                    ),
                )
                .at(self.at("files")),
            );
            return None;
        }
        let name = match (condition.side(), value_text(&condition)) {
            (Some(side), _) => format!("{condition_name}({files_text}, {})", side.text),
            (None, Some(value)) => format!("{condition_name}({files_text}, {value})"),
            (None, None) => format!("{condition_name}({files_text})"),
        };
        let transitive = options.transitively;
        Some((
            RuleKind::Sentence,
            vec![Sentence {
                files,
                folders,
                files_text,
                condition,
                options,
            }],
            name,
            transitive,
        ))
    }

    /// What the sentence is about: `files`, a set of files, or `folders`, patterns of folders.
    fn scope(
        &self,
        condition: &str,
        names: &mut Names<'_>,
        problems: &mut Vec<ConfigProblem>,
    ) -> Option<(Option<FileSet>, Option<Folders>, String)> {
        match (self.object.get("files"), self.object.get("folders")) {
            (Some(files), None) => Some((
                names.file_set(files, &self.at("files"), problems),
                None,
                written(files),
            )),
            (None, Some(folders)) => {
                if !FOLDER_CONDITIONS.contains(&condition) {
                    problems.push(invalid(
                        &self.at("folders"),
                        &format!(
                            "a rule about folders takes `mustBeNamed`, `mustContain` or \
                             `mustNotExist`, and `{condition}` is about files; write `files` \
                             instead"
                        ),
                    ));
                    return None;
                }
                let found = self.folder_patterns(folders, &names.scope, problems);
                Some((Some(FileSet::default()), Some(found), written(folders)))
            }
            (Some(_), Some(_)) => {
                problems.push(invalid(
                    &self.field,
                    "the rule has both `files` and `folders`; a rule is about files or about \
                     folders, so write two rules",
                ));
                None
            }
            (None, None) => {
                problems.push(invalid(
                    &self.at("files"),
                    "the rule needs `files`, the files it is about, or `folders` for a rule about \
                     folders",
                ));
                None
            }
        }
    }

    /// The sentence's one condition, refusing a sentence with none or with several, and the
    /// fields a sentence does not take.
    fn condition_name(&self, problems: &mut Vec<ConfigProblem>) -> Option<&'static str> {
        let found: Vec<&'static str> = CONDITIONS
            .iter()
            .copied()
            .filter(|c| self.object.contains_key(*c))
            .collect();
        if found.len() > 1 {
            let listed: Vec<String> = found.iter().map(|c| format!("`{c}`")).collect();
            problems.push(invalid(
                &self.field,
                &format!(
                    "the rule has {} conditions, {}; a rule is one selector and one condition, so \
                     write {} rules",
                    found.len(),
                    listed.join(" and "),
                    if found.len() == 2 { "two" } else { "separate" }
                ),
            ));
            return None;
        }
        let allowed: Vec<&str> = ["files", "folders"]
            .iter()
            .chain(&COMMON_FIELDS)
            .chain(&CONDITIONS)
            .chain(&OPTIONS)
            .copied()
            .collect();
        unknown_fields(self.object, &allowed, &self.field, problems);
        let known = found.first().copied();
        if found.is_empty() {
            problems.push(invalid(
                &self.field,
                &format!(
                    "the rule has no condition; give it one of {}",
                    CONDITIONS.join(", ")
                ),
            ));
        }
        known
    }

    fn condition(
        &self,
        name: &str,
        folder_rule: bool,
        names: &mut Names<'_>,
        problems: &mut Vec<ConfigProblem>,
    ) -> Option<Condition> {
        let value = &self.object[name];
        let field = self.at(name);
        match name {
            "mustBeNamed" | "mustContain" | "mustExport" | "mayNotExport" => {
                self.named_condition(name, folder_rule, problems)
            }
            "mustOnlyReExport" => self.must_be_true(name, Condition::MustOnlyReExport, problems),
            "mayNotImport" => names
                .side(value, &field, problems)
                .map(Condition::MayNotImport),
            "mayOnlyImport" if value.as_array().is_some_and(Vec::is_empty) => {
                if self.object.contains_key("within") {
                    Some(Condition::MayOnlyImport(Side {
                        text: "nothing".to_owned(),
                        ..Side::default()
                    }))
                } else {
                    problems.push(invalid(
                        &field,
                        "lists nothing; to refuse every import into an area, add `within`",
                    ));
                    None
                }
            }
            "mayOnlyImport" => names
                .side(value, &field, problems)
                .map(Condition::MayOnlyImport),
            "mayOnlyBeImportedBy" => names
                .side(value, &field, problems)
                .map(Condition::MayOnlyBeImportedBy),
            "mustNotExist" => self.must_be_true(name, Condition::MustNotExist, problems),
            _ => self.must_be_true(name, Condition::NoCycles, problems),
        }
    }

    /// A condition whose value is a name, a pattern or a kind of export.
    fn named_condition(
        &self,
        name: &str,
        folder_rule: bool,
        problems: &mut Vec<ConfigProblem>,
    ) -> Option<Condition> {
        let field = self.at(name);
        if name == "mustContain" {
            return self.contained(&field, problems);
        }
        let text = self.text(name, problems)?;
        match name {
            "mustBeNamed" => {
                let rule = match Style::from_name(text) {
                    Some(style) => NameRule::Style(style),
                    None => NameRule::Pattern(self.name_pattern(name, text, problems)?),
                };
                if let (true, NameRule::Pattern(pattern)) = (folder_rule, &rule)
                    && pattern.uses(Owner::File)
                {
                    problems.push(invalid(
                        &field,
                        "a folder has no file name; a rule about folders can use the spellings of \
                         `{FolderName}`, the name of the folder that holds it",
                    ));
                    return None;
                }
                Some(Condition::MustBeNamed(rule))
            }
            "mustExport" => self
                .name_pattern(name, text, problems)
                .map(Condition::MustExport),
            _ => {
                let kind = match text {
                    "default" => ExportKind::Default,
                    "star" => ExportKind::Star,
                    _ => ExportKind::Names(self.name_pattern(name, text, problems)?),
                };
                Some(Condition::MayNotExport(kind))
            }
        }
    }

    /// The patterns of `folders`: those with a leading `!` leave folders out.
    fn folder_patterns(
        &self,
        folders: &Value,
        scope: &Placement,
        problems: &mut Vec<ConfigProblem>,
    ) -> Folders {
        let field = self.at("folders");
        let texts = string_list(Some(folders), &field, problems);
        let (except, patterns): (Vec<&String>, Vec<&String>) =
            texts.iter().partition(|text| text.starts_with('!'));
        if patterns.is_empty() {
            problems.push(invalid(&field, "lists no folder patterns"));
        }
        let compile = |texts: &[&String], problems: &mut Vec<ConfigProblem>| {
            texts
                .iter()
                .filter_map(|text| {
                    self.folder_pattern(text.trim_start_matches('!'), scope, problems)
                })
                .collect::<Vec<Pattern>>()
        };
        Folders {
            patterns: compile(&patterns, problems),
            except: compile(&except, problems),
        }
    }

    fn folder_pattern(
        &self,
        text: &str,
        scope: &Placement,
        problems: &mut Vec<ConfigProblem>,
    ) -> Option<Pattern> {
        let text = scope.path(text);
        match Pattern::parse(&text) {
            Ok(pattern) => Some(pattern),
            Err(error) => {
                problems.push(
                    ConfigProblem::new(
                        ProblemCode::PatternInvalid,
                        format!("the folder pattern `{text}` is invalid: {error}"),
                    )
                    .at(self.at("folders")),
                );
                None
            }
        }
    }

    /// The patterns of `mustContain`, one or a list, any of which an instance or folder must
    /// hold.
    fn contained(&self, field: &str, problems: &mut Vec<ConfigProblem>) -> Option<Condition> {
        let before = problems.len();
        let texts = string_list(self.object.get("mustContain"), field, problems);
        if texts.is_empty() && problems.len() == before {
            problems.push(invalid(field, "must be a pattern or a list of patterns"));
        }
        let patterns: Vec<Pattern> = texts
            .iter()
            .filter_map(|text| match Pattern::parse(text) {
                Ok(pattern) => Some(pattern),
                Err(error) => {
                    problems.push(
                        ConfigProblem::new(
                            ProblemCode::PatternInvalid,
                            format!("the pattern `{text}` is invalid: {error}"),
                        )
                        .at(field),
                    );
                    None
                }
            })
            .collect();
        (problems.len() == before).then_some(Condition::MustContain(patterns))
    }

    /// A condition's value that must be a string.
    fn text(&self, key: &str, problems: &mut Vec<ConfigProblem>) -> Option<&str> {
        let found = self.object.get(key).and_then(Value::as_str);
        if found.is_none() {
            problems.push(invalid(&self.at(key), "must be a string"));
        }
        found
    }

    fn name_pattern(
        &self,
        key: &str,
        text: &str,
        problems: &mut Vec<ConfigProblem>,
    ) -> Option<NamePattern> {
        NamePattern::parse(text)
            .map_err(|message| {
                problems.push(
                    ConfigProblem::new(ProblemCode::PatternInvalid, message).at(self.at(key)),
                );
            })
            .ok()
    }

    fn must_be_true(
        &self,
        key: &str,
        condition: Condition,
        problems: &mut Vec<ConfigProblem>,
    ) -> Option<Condition> {
        if self.object.get(key) == Some(&Value::Bool(true)) {
            Some(condition)
        } else {
            problems.push(invalid(&self.at(key), "must be `true`"));
            None
        }
    }

    fn options(
        &self,
        condition: &str,
        files: Option<&FileSet>,
        names: &mut Names<'_>,
        problems: &mut Vec<ConfigProblem>,
    ) -> Option<ImportOptions> {
        let before = problems.len();
        self.options_belong(condition, problems);
        if problems.len() != before {
            return None;
        }
        let options = self.read_options(names, problems);
        let captured = files.is_some_and(|files| captured_element(files, names.elements));
        self.options_agree(&options, (captured, names.a_part()), problems);
        (problems.len() == before).then_some(options)
    }

    /// Refuses an option on a condition it does not limit.
    fn options_belong(&self, condition: &str, problems: &mut Vec<ConfigProblem>) {
        let import_condition = IMPORT_CONDITIONS.contains(&condition);
        for option in OPTIONS.iter().filter(|o| self.object.contains_key(**o)) {
            if *option == "within" {
                self.area_belongs(condition, problems);
            } else if !import_condition {
                problems.push(invalid(
                    &self.at(option),
                    &format!("`{option}` limits an import condition, and `{condition}` is not one"),
                ));
            } else if matches!(*option, "transitively" | "stopAt") && condition != "mayNotImport" {
                problems.push(invalid(
                    &self.at(option),
                    &format!("`{option}` applies to `mayNotImport` only"),
                ));
            }
        }
    }

    /// Refuses an area on a condition that it does not limit. `mayNotImport` already writes what
    /// an area would mean there, so the message gives that form.
    fn area_belongs(&self, condition: &str, problems: &mut Vec<ConfigProblem>) {
        let field = self.at("within");
        if condition == "mayNotImport" {
            let side = self.object.get(condition).map_or_else(String::new, source);
            let area = self.object.get("within").map_or_else(String::new, source);
            problems.push(invalid(
                &field,
                &format!(
                    "`mayNotImport` already names what it refuses, so it takes no area; write \
                     `mayNotImport: {{ all: [{side}, {area}] }}`"
                ),
            ));
        } else if !IMPORT_CONDITIONS.contains(&condition) {
            problems.push(invalid(
                &field,
                &format!(
                    "`within` limits the imports that a rule judges, and `{condition}` checks no \
                     import"
                ),
            ));
        }
    }

    /// The area of a rule, as a rule side.
    fn area(&self, names: &mut Names<'_>, problems: &mut Vec<ConfigProblem>) -> Option<Side> {
        let value = self.object.get("within")?;
        let field = self.at("within");
        if lists_nothing(value) {
            problems.push(invalid(&field, "an area needs at least one value"));
            return None;
        }
        names.side(value, &field, problems)
    }

    pub(crate) fn read_options(
        &self,
        names: &mut Names<'_>,
        problems: &mut Vec<ConfigProblem>,
    ) -> ImportOptions {
        let transitively = match self.object.get("transitively") {
            None => false,
            Some(Value::Bool(value)) => *value,
            Some(_) => {
                problems.push(invalid(
                    &self.at("transitively"),
                    "must be `true` or `false`",
                ));
                false
            }
        };
        ImportOptions {
            names: self.object.get("names").map(|value| {
                let found = string_list(Some(value), &self.at("names"), problems);
                if found.is_empty() {
                    problems.push(invalid(&self.at("names"), "lists no names"));
                }
                found
            }),
            specifier: self.choice("specifier", &SPECIFIERS, problems),
            instance: self.choice("instance", &INSTANCES, problems),
            dependency: self.choice("dependency", &DEPENDENCIES, problems),
            transitively,
            stop_at: self
                .object
                .get("stopAt")
                .and_then(|value| names.file_set(value, &self.at("stopAt"), problems)),
            within: self.area(names, problems),
            own: None,
        }
    }

    /// Refuses options that cannot hold together, or that need what `files` does not give.
    fn options_agree(
        &self,
        options: &ImportOptions,
        (captured, a_part): (bool, &str),
        problems: &mut Vec<ConfigProblem>,
    ) {
        if options.stop_at.is_some() && !options.transitively {
            problems.push(invalid(
                &self.at("stopAt"),
                "ends a chain of imports, so it needs `transitively: true`",
            ));
        }
        if options.transitively && options.needs_import() {
            problems.push(invalid(
                &self.at("transitively"),
                "a transitive rule follows chains of imports, and `names`, `specifier`, `instance` \
                 and `dependency` limit one import; write the limited rule as a rule of its own",
            ));
        }
        if options.instance.is_some() && !captured {
            problems.push(
                ConfigProblem::new(
                    ProblemCode::RuleNeedsCapture,
                    format!(
                        "`instance` compares the captures at both ends, so `files` must name \
                         {a_part} whose pattern captures a name, such as \
                         `src/features/{{feature}}/**`"
                    ),
                )
                .at(self.at("instance")),
            );
        }
    }

    fn choice<T: Copy>(
        &self,
        key: &str,
        values: &[(&str, T)],
        problems: &mut Vec<ConfigProblem>,
    ) -> Option<T> {
        let given = self.object.get(key)?;
        let found = values
            .iter()
            .find(|(name, _)| given.as_str() == Some(*name))
            .map(|(_, value)| *value);
        if found.is_none() {
            let listed: Vec<String> = values.iter().map(|(name, _)| format!("'{name}'")).collect();
            problems.push(invalid(
                &self.at(key),
                &format!("must be {}", or_list(&listed)),
            ));
        }
        found
    }

    pub(crate) fn imports(
        &self,
        sentences: &[Sentence],
        transitive: bool,
        problems: &mut Vec<ConfigProblem>,
    ) -> ImportKinds {
        let runtime_default = transitive
            || sentences
                .iter()
                .any(|s| matches!(s.condition, Condition::NoCycles));
        if self.object.contains_key("imports")
            && let Some(sentence) = sentences.iter().find(|s| {
                s.condition.side().is_none()
                    && !matches!(s.condition, Condition::NoCycles | Condition::Entry { .. })
            })
        {
            problems.push(invalid(
                &self.at("imports"),
                &format!(
                    "`imports` chooses which imports a rule checks, and `{}` checks no import",
                    sentence.condition.name()
                ),
            ));
            return default_kinds(runtime_default);
        }
        let default = default_kinds(runtime_default);
        let parsed = match self.object.get("imports") {
            None => return default,
            Some(Value::String(value)) => match value.as_str() {
                "all" => Some(ImportKinds::All),
                "runtime" => Some(ImportKinds::Runtime),
                "types" => Some(ImportKinds::Types),
                _ => None,
            },
            Some(_) => None,
        };
        match parsed {
            Some(ImportKinds::Types) if transitive => {
                problems.push(invalid(
                    &self.at("imports"),
                    "a transitive rule follows chains that run, and a chain of type imports never \
                     runs, because TypeScript removes type imports; use 'runtime' or 'all'",
                ));
                default
            }
            Some(kinds) => kinds,
            None => {
                problems.push(invalid(
                    &self.at("imports"),
                    "must be 'all', 'runtime' or 'types'",
                ));
                default
            }
        }
    }

    fn one_element(
        &self,
        key: &str,
        names: &Names<'_>,
        problems: &mut Vec<ConfigProblem>,
    ) -> Option<ElementId> {
        let field = self.at(key);
        if let Some(Value::String(name)) = self.object.get(key) {
            resolve(names.elements, &names.scope.name(name), &field, problems)
        } else {
            problems.push(invalid(&field, "must name one element"));
            None
        }
    }

    fn element_list(
        &self,
        key: &str,
        names: &Names<'_>,
        problems: &mut Vec<ConfigProblem>,
    ) -> Vec<ElementId> {
        let field = self.at(key);
        let found = string_list(self.object.get(key), &field, problems);
        if found.is_empty() && self.object.get(key).is_none_or(Value::is_array) {
            problems.push(invalid(&field, "must name at least one element"));
        }
        found
            .iter()
            .filter_map(|name| resolve(names.elements, &names.scope.name(name), &field, problems))
            .collect()
    }
}

fn default_kinds(runtime: bool) -> ImportKinds {
    if runtime {
        ImportKinds::Runtime
    } else {
        ImportKinds::All
    }
}

/// The value of a condition that has one besides its side, as the default name writes it.
fn value_text(condition: &Condition) -> Option<String> {
    match condition {
        Condition::MustBeNamed(rule) => Some(rule.source().to_owned()),
        Condition::MustContain(patterns) => Some(
            patterns
                .iter()
                .map(Pattern::source)
                .collect::<Vec<_>>()
                .join(", "),
        ),
        Condition::MustExport(pattern) => Some(pattern.source().to_owned()),
        Condition::MayNotExport(kind) => Some(kind.source().to_owned()),
        _ => None,
    }
}

/// Whether a set of files is exactly one element whose pattern has a capture.
/// Whether a set of files or packages holds no value: an empty list, or an empty `any` or `all`.
fn lists_nothing(value: &Value) -> bool {
    let empty = |items: Option<&Value>| items.and_then(Value::as_array).is_some_and(Vec::is_empty);
    empty(Some(value)) || empty(value.get("any")) || empty(value.get("all"))
}

/// A value of the config as the config's own text writes it.
fn source(value: &Value) -> String {
    match value {
        Value::String(text) => format!("'{text}'"),
        Value::Array(items) => {
            let items: Vec<String> = items.iter().map(source).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Object(object) => {
            let fields: Vec<String> = object
                .iter()
                .map(|(key, value)| format!("{key}: {}", source(value)))
                .collect();
            format!("{{ {} }}", fields.join(", "))
        }
        other => other.to_string(),
    }
}

fn captured_element(files: &FileSet, elements: &[Element]) -> bool {
    files
        .element()
        .is_some_and(|id| elements[id.index()].has_capture())
}

/// An element by name, for a shorthand.
fn resolve(
    elements: &[Element],
    name: &str,
    field: &str,
    problems: &mut Vec<ConfigProblem>,
) -> Option<ElementId> {
    let found = elements.iter().position(|element| element.name == name);
    if let Some(index) = found.and_then(|index| u32::try_from(index).ok()) {
        return Some(ElementId(index));
    }
    let declared: Vec<&str> = elements.iter().map(|e| e.name.as_str()).collect();
    problems.push(
        ConfigProblem::new(
            ProblemCode::RuleUnknownElement,
            format!(
                "the config declares no element \"{name}\"; {}the declared elements are {}",
                nearest(name, &declared)
                    .map(|near| format!("did you mean \"{near}\"? "))
                    .unwrap_or_default(),
                declared.join(", ")
            ),
        )
        .at(field),
    );
    None
}

fn invalid(field: &str, message: &str) -> ConfigProblem {
    ConfigProblem::new(ProblemCode::ConfigInvalid, message).at(field)
}
