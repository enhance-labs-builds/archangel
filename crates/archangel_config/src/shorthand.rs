//! The words and sentences of rule kinds: what the removed kinds became, and what `layers` compiles
//! to.

use serde_json::{Map, Value};

use crate::model::{
    Condition, Element, ElementId, ElementKind, FileSet, ImportOptions, Layer, SelectorId,
    Sentence, Side, Term,
};
use crate::parse::{nearest, string_list};
use crate::problem::{ConfigProblem, ProblemCode};
use crate::selectors::Names;

pub(crate) const KIND_LIST: &str = "must be one of layers, isolate or entry; any other rule is a sentence of \
                         `files` and one condition, without `kind`";

/// The sentence that replaces a removed rule kind, written from the rule's own fields.
pub(crate) fn removed_kind(kind: &str, object: &Map<String, Value>) -> Option<String> {
    let field = |key: &str| object.get(key).map_or_else(|| "...".to_owned(), compact);
    match kind {
        "only" => Some(format!(
            "{{ \"files\": {}, \"mayOnlyBeImportedBy\": {} }}",
            field("element"),
            field("importedBy")
        )),
        "forbid" => Some(format!(
            "{{ \"files\": {}, \"mayNotImport\": {} }}",
            field("from"),
            field("to")
        )),
        "noCycles" => Some("{ \"files\": \"**\", \"noCycles\": true }".to_owned()),
        _ => None,
    }
}

/// JSON with a space after each comma, as a person writes it.
fn compact(value: &Value) -> String {
    match value {
        Value::Array(items) => format!(
            "[{}]",
            items.iter().map(compact).collect::<Vec<_>>().join(", ")
        ),
        other => other.to_string(),
    }
}

/// The layers of a `layers` rule's `order`, with their names: parts, package parts or groups.
pub(crate) fn layer_list(
    value: Option<&Value>,
    field: &str,
    names: &Names<'_>,
    problems: &mut Vec<ConfigProblem>,
) -> (Vec<Layer>, Vec<String>) {
    let found = string_list(value, field, problems);
    if found.is_empty() && value.is_none_or(Value::is_array) {
        problems.push(
            ConfigProblem::new(ProblemCode::ConfigInvalid, "must name at least one element")
                .at(field),
        );
    }
    let mut layers = Vec::new();
    let mut layer_names = Vec::new();
    for written in &found {
        let name = names.scope.name(written);
        if let Some(layer) = layer_named(&name, names) {
            layers.push(layer);
            layer_names.push(name);
        } else {
            problems.push(unknown_layer(&name, names, field));
        }
    }
    (layers, layer_names)
}

/// The part or group of a name.
fn layer_named(name: &str, names: &Names<'_>) -> Option<Layer> {
    if let Some(index) = names.elements.iter().position(|e| e.name == name) {
        return u32::try_from(index)
            .ok()
            .map(|i| Layer::Element(ElementId(i)));
    }
    let index = names.selectors.iter().position(|s| s == name)?;
    u32::try_from(index)
        .ok()
        .map(|i| Layer::Selector(SelectorId(i)))
}

/// A layer that names no declared part or group.
fn unknown_layer(name: &str, names: &Names<'_>, field: &str) -> ConfigProblem {
    let declared: Vec<&str> = names
        .elements
        .iter()
        .map(|e| e.name.as_str())
        .chain(names.selectors.iter().map(String::as_str))
        .collect();
    let word = if names.says_parts {
        "part or group"
    } else {
        "element or selector"
    };
    ConfigProblem::new(
        ProblemCode::RuleUnknownElement,
        format!(
            "the config declares no {word} \"{name}\"; {}the declared names are {}",
            nearest(name, &declared)
                .map(|near| format!("did you mean \"{near}\"? "))
                .unwrap_or_default(),
            declared.join(", ")
        ),
    )
    .at(field)
}

/// The files of a layer, or `None` for a package element, which imports nothing.
fn layer_files(layer: Layer, elements: &[Element]) -> Option<FileSet> {
    match layer {
        Layer::Element(id) => match elements[id.index()].kind {
            ElementKind::Files(_) => Some(FileSet::of(Term::Element(id))),
            ElementKind::Packages(_) => None,
        },
        Layer::Selector(id) => Some(FileSet::of(Term::Selector(id))),
    }
}

/// Each later layer may not import any earlier one; a package element imports nothing, so it is
/// only ever the other side.
pub(crate) fn layer_sentences(
    order: &[Layer],
    names: &[String],
    elements: &[Element],
) -> Vec<Sentence> {
    let mut seen: Vec<usize> = Vec::new();
    let mut sentences = Vec::new();
    for (index, layer) in order.iter().enumerate() {
        if seen.iter().any(|&earlier| order[earlier] == *layer) {
            continue;
        }
        if let (Some(files), false) = (layer_files(*layer, elements), seen.is_empty()) {
            let mut side = Side {
                text: seen
                    .iter()
                    .map(|&earlier| names[earlier].clone())
                    .collect::<Vec<_>>()
                    .join(", "),
                ..Side::default()
            };
            for &earlier in &seen {
                match (order[earlier], layer_files(order[earlier], elements)) {
                    (_, Some(files)) => side.files.push(files),
                    (Layer::Element(id), None) => side.elements.push(id),
                    (Layer::Selector(_), None) => {}
                }
            }
            sentences.push(Sentence {
                files,
                folders: None,
                files_text: names[index].clone(),
                condition: Condition::MayNotImport(side),
                options: ImportOptions::default(),
            });
        }
        seen.push(index);
    }
    sentences
}

/// The items in words: `a, b or c`.
#[must_use]
pub fn or_list(items: &[String]) -> String {
    match items.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} or {last}", rest.join(", ")),
        _ => items.concat(),
    }
}
