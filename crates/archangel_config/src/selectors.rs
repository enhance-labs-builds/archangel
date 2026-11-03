//! Reading selectors and rule sides: the names, patterns and combinations that say which files and
//! packages a rule is about.

use std::collections::HashMap;

use serde_json::{Map, Value};

use crate::model::{
    Element, ElementId, ElementKind, FileSet, PatternId, Selector, SelectorId, Side, Term,
};
use crate::pattern::Pattern;
use crate::plugins::LoadedPlugins;
use crate::problem::{ConfigProblem, ProblemCode};

/// The declared names a selector or rule side can use, and the patterns found so far.
pub(crate) struct Names<'a> {
    pub(crate) elements: &'a [Element],
    pub(crate) selectors: Vec<String>,
    pub(crate) patterns: Vec<Pattern>,
    index: HashMap<String, PatternId>,
    /// The plugins the config loads, which plugin rules name.
    pub(crate) plugins: LoadedPlugins<'a>,
    /// Where the names being read were written: in a preset or in the config itself.
    pub(crate) scope: Placement,
    /// Whether the config says parts and groups, so that messages say so too.
    pub(crate) says_parts: bool,
}

/// Where a part of the config was written. A preset's names are its prefix, a slash and the name
/// it wrote, and its paths are relative to its root; the config's own are as written.
#[derive(Debug, Clone, Default)]
pub(crate) struct Placement {
    /// The preset's name, or `None` for the config's own fields.
    pub(crate) prefix: Option<String>,
    /// The preset's root, relative to the config folder; empty for the config folder itself.
    pub(crate) root: String,
}

impl Placement {
    /// A name as the merged config holds it.
    pub(crate) fn name(&self, written: &str) -> String {
        match &self.prefix {
            Some(prefix) => format!("{prefix}/{written}"),
            None => written.to_owned(),
        }
    }

    /// A path pattern relative to the config folder.
    pub(crate) fn path(&self, written: &str) -> String {
        if self.root.is_empty() {
            written.to_owned()
        } else {
            format!("{}/{written}", self.root)
        }
    }
}

/// What a string on a rule side or in a selector names.
enum Named {
    FileElement(ElementId),
    PackageElement(ElementId),
    Selector(SelectorId),
    Pattern(PatternId),
}

/// One thing a rule side names.
enum Item {
    Files(FileSet),
    PackageElement(ElementId),
    Packages(Vec<Pattern>),
}

/// A step of reading a set of files: a value to read, or a combination to close once the values
/// it combines are read.
enum Step<'v> {
    Read(&'v Value, String),
    Close(Term),
}

/// The steps still to take in reading a set of files, and the problems found so far.
struct Reading<'v, 'p> {
    steps: Vec<Step<'v>>,
    problems: &'p mut Vec<ConfigProblem>,
}

impl<'a> Names<'a> {
    pub(crate) fn new(elements: &'a [Element], selectors: Vec<String>) -> Self {
        Self {
            elements,
            selectors,
            patterns: Vec::new(),
            index: HashMap::new(),
            plugins: LoadedPlugins::default(),
            scope: Placement::default(),
            says_parts: false,
        }
    }

    /// What the config calls a part, with its article.
    pub(crate) fn a_part(&self) -> &'static str {
        if self.says_parts {
            "a part"
        } else {
            "an element"
        }
    }

    /// What the config calls a part.
    pub(crate) fn part_word(&self) -> &'static str {
        if self.says_parts { "part" } else { "element" }
    }

    pub(crate) fn intern(&mut self, pattern: Pattern) -> PatternId {
        if let Some(&id) = self.index.get(pattern.source()) {
            return id;
        }
        let id = PatternId(u32::try_from(self.patterns.len()).unwrap_or(u32::MAX));
        self.index.insert(pattern.source().to_owned(), id);
        self.patterns.push(pattern);
        id
    }

    /// The declared selector with this name in the merged config.
    pub(crate) fn selector_id(&self, name: &str) -> Option<SelectorId> {
        self.selectors
            .iter()
            .position(|selector| selector == name)
            .map(|index| SelectorId(u32::try_from(index).unwrap_or(u32::MAX)))
    }

    /// Whether a name, as the scope writes it, is a declared part, group or externals set.
    pub(crate) fn declares(&self, written: &str) -> bool {
        let name = self.scope.name(written);
        self.elements.iter().any(|e| e.name == name) || self.selector_id(&name).is_some()
    }

    /// A string as a declared name first, and then as a path pattern when it has `/` or `*`.
    fn named(
        &mut self,
        text: &str,
        field: &str,
        problems: &mut Vec<ConfigProblem>,
    ) -> Option<Named> {
        let name = self.scope.name(text);
        if let Some(index) = self.elements.iter().position(|e| e.name == name) {
            let id = ElementId(u32::try_from(index).unwrap_or(u32::MAX));
            return Some(match self.elements[index].kind {
                ElementKind::Files(_) => Named::FileElement(id),
                ElementKind::Packages(_) => Named::PackageElement(id),
            });
        }
        if let Some(index) = self.selectors.iter().position(|s| *s == name) {
            return Some(Named::Selector(SelectorId(
                u32::try_from(index).unwrap_or(u32::MAX),
            )));
        }
        if text.contains('/') || text.contains('*') {
            return match Pattern::parse(&self.scope.path(text)) {
                Ok(pattern) => Some(Named::Pattern(self.intern(pattern))),
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
            };
        }
        problems.push(self.unknown(text, field));
        None
    }

    fn unknown(&self, text: &str, field: &str) -> ConfigProblem {
        let mut declared: Vec<&str> = self.elements.iter().map(|e| e.name.as_str()).collect();
        declared.extend(self.selectors.iter().map(String::as_str));
        ConfigProblem::new(
            ProblemCode::RuleUnknownElement,
            format!(
                "the config declares no {} \"{text}\"; {}{}; {}a path pattern has a `/` or a `*`",
                if self.says_parts {
                    "part or group"
                } else {
                    "element or selector"
                },
                crate::parse::nearest(text, &declared)
                    .map(|near| format!("did you mean \"{near}\"? "))
                    .unwrap_or_default(),
                if declared.is_empty() {
                    "the config declares no names".to_owned()
                } else {
                    format!("the declared names are {}", declared.join(", "))
                },
                if self.says_parts && field.rsplit('.').next() == Some("files") {
                    "a kind has a meaning only in its table, and ".to_owned()
                } else {
                    format!("a package is written {{ packages: ['{text}'] }}, and ")
                },
            ),
        )
        .at(field)
    }

    /// A set of files: a name, a pattern, a list of them, or a combination, read with a stack so
    /// that a deeply nested config cannot exhaust the thread's stack.
    pub(crate) fn file_set(
        &mut self,
        value: &Value,
        field: &str,
        problems: &mut Vec<ConfigProblem>,
    ) -> Option<FileSet> {
        let before = problems.len();
        let mut set = FileSet::default();
        let mut reading = Reading {
            steps: vec![Step::Read(value, field.to_owned())],
            problems,
        };
        while let Some(step) = reading.steps.pop() {
            match step {
                Step::Close(term) => set.terms.push(term),
                Step::Read(value, field) => {
                    if let Some(term) = self.read(value, &field, &mut reading) {
                        set.terms.push(term);
                    }
                }
            }
        }
        (problems.len() == before).then_some(set)
    }

    /// Reads one value: a term at once, or a combination whose parts become steps.
    fn read<'v>(
        &mut self,
        value: &'v Value,
        field: &str,
        reading: &mut Reading<'v, '_>,
    ) -> Option<Term> {
        match value {
            Value::String(text) => self.name_term(text, field, reading.problems),
            Value::Array(items) => {
                open(Term::Any(items.len()), items, field, reading);
                None
            }
            Value::Object(object) => self.object_term(object, field, reading),
            _ => {
                reading.problems.push(invalid(field, SET_FORMS));
                None
            }
        }
    }

    fn name_term(
        &mut self,
        text: &str,
        field: &str,
        problems: &mut Vec<ConfigProblem>,
    ) -> Option<Term> {
        match self.named(text, field, problems)? {
            Named::FileElement(id) => Some(Term::Element(id)),
            Named::Selector(id) => Some(Term::Selector(id)),
            Named::Pattern(id) => Some(Term::Pattern(id)),
            Named::PackageElement(_) => {
                problems.push(invalid(
                    field,
                    &format!(
                        "\"{text}\" is a package {}, and this place selects files; name \
                         packages on the other side of an import rule",
                        self.part_word()
                    ),
                ));
                None
            }
        }
    }

    fn object_term<'v>(
        &self,
        object: &'v Map<String, Value>,
        field: &str,
        reading: &mut Reading<'v, '_>,
    ) -> Option<Term> {
        let problems = &mut *reading.problems;
        let keys: Vec<&str> = object.keys().map(String::as_str).collect();
        match keys.as_slice() {
            [key @ ("all" | "any")] => {
                let inner = format!("{field}.{key}");
                if let Some(Value::Array(items)) = object.get(*key) {
                    let close = if *key == "all" {
                        Term::All(items.len())
                    } else {
                        Term::Any(items.len())
                    };
                    open(close, items, &inner, reading);
                } else {
                    problems.push(invalid(&inner, "must be a list"));
                }
                None
            }
            ["not"] => {
                reading.steps.push(Step::Close(Term::Not));
                reading
                    .steps
                    .push(Step::Read(&object["not"], format!("{field}.not")));
                None
            }
            ["directive"] => match &object["directive"] {
                Value::String(text) if !text.is_empty() => Some(Term::Directive(text.clone())),
                _ => {
                    problems.push(invalid(
                        &format!("{field}.directive"),
                        "must be the directive's text, such as 'use client'",
                    ));
                    None
                }
            },
            ["element"] | ["capture" | "element", "element" | "capture"] => {
                self.element_term(object, field, problems)
            }
            ["packages"] => {
                problems.push(invalid(
                    field,
                    "a selector selects files, and packages belong in a package element, such as \
                     `elements: { react: { packages: ['react'] } }`, or inline on the other side \
                     of an import rule",
                ));
                None
            }
            _ => {
                problems.push(invalid(field, SET_FORMS));
                None
            }
        }
    }

    /// `{ element: ... }`, with an optional `capture`.
    fn element_term(
        &self,
        object: &Map<String, Value>,
        field: &str,
        problems: &mut Vec<ConfigProblem>,
    ) -> Option<Term> {
        let Some(Value::String(name)) = object.get("element") else {
            problems.push(invalid(
                &format!("{field}.element"),
                "must name an element, or be '*' for any element",
            ));
            return None;
        };
        if name == "*" && !object.contains_key("capture") {
            return Some(Term::AnyElement);
        }
        let scoped = self.scope.name(name);
        let Some(index) = self
            .elements
            .iter()
            .position(|e| e.name == scoped && matches!(e.kind, ElementKind::Files(_)))
        else {
            problems.push(self.unknown_file_element(name, field));
            return None;
        };
        let element = ElementId(u32::try_from(index).unwrap_or(u32::MAX));
        match object.get("capture") {
            None => Some(Term::Element(element)),
            Some(capture) => {
                let captures = self.captures(index, capture, field, problems)?;
                Some(Term::Instance { element, captures })
            }
        }
    }

    fn unknown_file_element(&self, name: &str, field: &str) -> ConfigProblem {
        let declared: Vec<&str> = self
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Files(_)))
            .map(|e| e.name.as_str())
            .collect();
        ConfigProblem::new(
            ProblemCode::RuleUnknownElement,
            format!(
                "the config declares no file {word} \"{name}\"; the file {word}s are {}",
                declared.join(", "),
                word = self.part_word()
            ),
        )
        .at(format!("{field}.element"))
    }

    /// The captures of one instance, each a capture the element's patterns have.
    fn captures(
        &self,
        element: usize,
        capture: &Value,
        field: &str,
        problems: &mut Vec<ConfigProblem>,
    ) -> Option<Vec<(String, String)>> {
        let name = &self.elements[element].name;
        let known: Vec<&str> = match &self.elements[element].kind {
            ElementKind::Files(patterns) => patterns
                .iter()
                .flat_map(|p| p.captures().iter().map(String::as_str))
                .collect(),
            ElementKind::Packages(_) => Vec::new(),
        };
        let Some(capture) = capture.as_object().filter(|c| !c.is_empty()) else {
            problems.push(invalid(
                &format!("{field}.capture"),
                "must map capture names to values, such as `{ feature: 'login' }`",
            ));
            return None;
        };
        let before = problems.len();
        let mut captures = Vec::new();
        for (key, value) in capture {
            let inner = format!("{field}.capture.{key}");
            match value {
                Value::String(value) if known.contains(&key.as_str()) => {
                    captures.push((key.clone(), value.clone()));
                }
                Value::String(_) => problems.push(
                    ConfigProblem::new(
                        ProblemCode::RuleNeedsCapture,
                        format!(
                            "the {} \"{name}\" has no capture `{{{key}}}`; its captures are {}",
                            self.part_word(),
                            if known.is_empty() {
                                "none".to_owned()
                            } else {
                                known.join(", ")
                            }
                        ),
                    )
                    .at(inner),
                ),
                _ => problems.push(invalid(&inner, "must be the capture's value")),
            }
        }
        (problems.len() == before).then_some(captures)
    }

    /// The other side of an import rule: files, package elements and inline packages.
    pub(crate) fn side(
        &mut self,
        value: &Value,
        field: &str,
        problems: &mut Vec<ConfigProblem>,
    ) -> Option<Side> {
        let mut side = Side {
            text: written(value),
            ..Side::default()
        };
        let before = problems.len();
        let items: Vec<(&Value, String)> = match value {
            Value::Array(items) => items
                .iter()
                .enumerate()
                .map(|(i, item)| (item, format!("{field}[{i}]")))
                .collect(),
            item => vec![(item, field.to_owned())],
        };
        if items.is_empty() {
            problems.push(invalid(field, "lists nothing"));
        }
        for (item, field) in items {
            match self.side_item(item, &field, problems) {
                Some(Item::Files(set)) => side.files.push(set),
                Some(Item::PackageElement(id)) => side.elements.push(id),
                Some(Item::Packages(patterns)) => side.packages.extend(patterns),
                None => {}
            }
        }
        (problems.len() == before).then_some(side)
    }

    fn side_item(
        &mut self,
        value: &Value,
        field: &str,
        problems: &mut Vec<ConfigProblem>,
    ) -> Option<Item> {
        match value {
            Value::String(text) => match self.named(text, field, problems)? {
                Named::PackageElement(id) => Some(Item::PackageElement(id)),
                Named::FileElement(id) => Some(Item::Files(FileSet::of(Term::Element(id)))),
                Named::Selector(id) => Some(Item::Files(FileSet::of(Term::Selector(id)))),
                Named::Pattern(id) => Some(Item::Files(FileSet::of(Term::Pattern(id)))),
            },
            Value::Object(object) if object.contains_key("packages") => {
                packages(object, field, problems).map(Item::Packages)
            }
            other => self.file_set(other, field, problems).map(Item::Files),
        }
    }
}

/// Opens a combination: its closing term waits on the stack below the values it combines, which
/// are read in order.
fn open<'v>(close: Term, items: &'v [Value], field: &str, reading: &mut Reading<'v, '_>) {
    if items.is_empty() {
        reading.problems.push(invalid(field, "lists nothing"));
        return;
    }
    reading.steps.push(Step::Close(close));
    for (i, item) in items.iter().enumerate().rev() {
        reading
            .steps
            .push(Step::Read(item, format!("{field}[{i}]")));
    }
}

/// `{ packages: [...] }` on a rule side.
fn packages(
    object: &Map<String, Value>,
    field: &str,
    problems: &mut Vec<ConfigProblem>,
) -> Option<Vec<Pattern>> {
    crate::parse::unknown_fields(object, &["packages"], field, problems);
    let inner = format!("{field}.packages");
    let names = crate::parse::string_list(object.get("packages"), &inner, problems);
    if names.is_empty() {
        problems.push(invalid(&inner, "lists no packages"));
        return None;
    }
    let before = problems.len();
    let patterns: Vec<Pattern> = names
        .iter()
        .filter_map(|name| match Pattern::parse(name) {
            Ok(pattern) => Some(pattern),
            Err(error) => {
                problems.push(
                    ConfigProblem::new(
                        ProblemCode::PatternInvalid,
                        format!("the package name `{name}` is invalid: {error}"),
                    )
                    .at(&inner),
                );
                None
            }
        })
        .collect();
    (problems.len() == before).then_some(patterns)
}

const SET_FORMS: &str = "must be a name, a pattern, a list of them, `{ all: [...] }`, \
                         `{ any: [...] }`, `{ not: ... }`, `{ element: ... }` or `{ directive: ... }`";

fn invalid(field: &str, message: &str) -> ConfigProblem {
    ConfigProblem::new(ProblemCode::ConfigInvalid, message).at(field)
}

/// A selector, a side or a rule's files as the config wrote them, in a few words.
pub(crate) fn written(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(items) => items.iter().map(compact).collect::<Vec<_>>().join(" | "),
        Value::Object(object) => {
            if let Some(Value::Array(items)) = object.get("all") {
                format!("all of ({})", list(items))
            } else if let Some(Value::Array(items)) = object.get("any") {
                format!("any of ({})", list(items))
            } else if let Some(inner) = object.get("not") {
                format!("not {}", compact(inner))
            } else if let Some(Value::String(directive)) = object.get("directive") {
                format!("'{directive}' files")
            } else if let Some(Value::Array(names)) = object.get("packages") {
                names.iter().map(compact).collect::<Vec<_>>().join(" | ")
            } else if let Some(Value::String(element)) = object.get("element") {
                instance(element, object.get("capture"))
            } else {
                Value::Object(object.clone()).to_string()
            }
        }
        other => other.to_string(),
    }
}

/// A part of what the config wrote, in a few words, reading one level into it.
fn compact(value: &Value) -> String {
    let leaf = |value: &Value| {
        value
            .as_str()
            .map_or_else(|| value.to_string(), str::to_owned)
    };
    match value {
        Value::String(text) => text.clone(),
        Value::Object(object) => match (
            object.get("packages"),
            object.get("not"),
            object.get("element"),
        ) {
            (Some(Value::Array(names)), _, _) => {
                names.iter().map(leaf).collect::<Vec<_>>().join(" | ")
            }
            (_, Some(inner), _) => format!("not {}", leaf(inner)),
            (_, _, Some(Value::String(element))) => instance(element, object.get("capture")),
            _ => value.to_string(),
        },
        other => other.to_string(),
    }
}

fn list(items: &[Value]) -> String {
    items.iter().map(compact).collect::<Vec<_>>().join(", ")
}

fn instance(element: &str, capture: Option<&Value>) -> String {
    let captures: Vec<String> = capture
        .and_then(Value::as_object)
        .map(|c| {
            c.iter()
                .map(|(k, v)| format!("{k} {}", v.as_str().unwrap_or_default()))
                .collect()
        })
        .unwrap_or_default();
    match (element, captures.is_empty()) {
        ("*", _) => "any element".to_owned(),
        (name, true) => name.to_owned(),
        (name, false) => format!("{name} {}", captures.join(", ")),
    }
}

/// A selector as one part of the config declared it: its name in the merged config, its
/// definition, where it is, and the scope it was written in.
pub(crate) struct Declared<'v> {
    pub(crate) name: String,
    pub(crate) definition: &'v Value,
    pub(crate) field: String,
    pub(crate) scope: Placement,
}

/// Reads the declared selectors, refusing a selector that refers to itself through others, and
/// returns them with an order in which each comes after the selectors it refers to.
pub(crate) fn parse_selectors(
    declared: &[Declared<'_>],
    names: &mut Names<'_>,
    problems: &mut Vec<ConfigProblem>,
) -> (Vec<Selector>, Vec<SelectorId>) {
    let mut selectors = Vec::new();
    for Declared {
        name,
        definition,
        field,
        scope,
    } in declared
    {
        if names.elements.iter().any(|e| e.name == *name) {
            problems.push(invalid(
                field,
                &format!(
                    "\"{name}\" is also an element; a rule side looks a name up as either, so give \
                     the selector its own name"
                ),
            ));
            continue;
        }
        names.scope = scope.clone();
        if let Some(definition) = names.file_set(definition, field, problems) {
            selectors.push(Selector {
                name: name.clone(),
                definition,
            });
        }
    }
    names.scope = Placement::default();
    if selectors.len() != declared.len() {
        return (selectors, Vec::new());
    }
    match order(&selectors) {
        Ok(order) => (selectors, order),
        Err(chain) => {
            let shown: Vec<String> = chain.iter().map(|name| format!("\"{name}\"")).collect();
            problems.push(invalid(
                &format!("selectors.{}", chain[0]),
                &format!(
                    "the selectors refer to each other in a loop: {}; a selector cannot be \
                     defined by itself",
                    shown.join(" -> ")
                ),
            ));
            (selectors, Vec::new())
        }
    }
}

/// The selectors with each after those it refers to, or the names of a loop, the first repeated
/// at the end: a depth-first search with an explicit stack.
fn order(selectors: &[Selector]) -> Result<Vec<SelectorId>, Vec<&str>> {
    let mut on_path = vec![false; selectors.len()];
    let mut done = vec![false; selectors.len()];
    let mut order = Vec::new();
    for start in 0..selectors.len() {
        if done[start] {
            continue;
        }
        let mut path: Vec<(usize, Vec<usize>)> = vec![(start, refers(&selectors[start]))];
        on_path[start] = true;
        while let Some((_, next)) = path.last_mut() {
            match next.pop() {
                Some(child) if on_path[child] => {
                    let from = path.iter().position(|(p, _)| *p == child).unwrap_or(0);
                    let mut names: Vec<&str> = path[from..]
                        .iter()
                        .map(|(p, _)| selectors[*p].name.as_str())
                        .collect();
                    names.push(selectors[child].name.as_str());
                    return Err(names);
                }
                Some(child) if !done[child] => {
                    on_path[child] = true;
                    path.push((child, refers(&selectors[child])));
                }
                Some(_) => {}
                None => {
                    if let Some((at, _)) = path.pop() {
                        on_path[at] = false;
                        done[at] = true;
                        order.push(SelectorId(u32::try_from(at).unwrap_or(u32::MAX)));
                    }
                }
            }
        }
    }
    Ok(order)
}

/// The selectors a selector refers to, last first, so the search takes them in order.
fn refers(selector: &Selector) -> Vec<usize> {
    let mut found: Vec<usize> = selector
        .definition
        .selectors()
        .map(|id| id.0 as usize)
        .collect();
    found.reverse();
    found
}
