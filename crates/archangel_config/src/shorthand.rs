//! The words and sentences of rule kinds: what the removed kinds became, and what `layers` compiles
//! to.

use serde_json::{Map, Value};

use crate::model::{
    Condition, Element, ElementId, ElementKind, FileSet, ImportOptions, Sentence, Side, Term,
};

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

/// Each later layer may not import any earlier one; a package element imports nothing, so it is
/// only ever the other side.
pub(crate) fn layer_sentences(order: &[ElementId], elements: &[Element]) -> Vec<Sentence> {
    let mut seen: Vec<ElementId> = Vec::new();
    let mut sentences = Vec::new();
    for &element in order {
        if seen.contains(&element) {
            continue;
        }
        if !seen.is_empty() && matches!(elements[element.index()].kind, ElementKind::Files(_)) {
            let mut side = Side {
                text: element_names(&seen, elements).join(", "),
                ..Side::default()
            };
            for &earlier in &seen {
                match elements[earlier.index()].kind {
                    ElementKind::Files(_) => side.files.push(FileSet::of(Term::Element(earlier))),
                    ElementKind::Packages(_) => side.elements.push(earlier),
                }
            }
            sentences.push(Sentence {
                files: FileSet::of(Term::Element(element)),
                folders: None,
                files_text: elements[element.index()].name.clone(),
                condition: Condition::MayNotImport(side),
                options: ImportOptions::default(),
            });
        }
        seen.push(element);
    }
    sentences
}

pub(crate) fn element_names(ids: &[ElementId], elements: &[Element]) -> Vec<String> {
    ids.iter()
        .map(|id| elements[id.index()].name.clone())
        .collect()
}

pub(crate) fn or_list(items: &[String]) -> String {
    match items.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} or {last}", rest.join(", ")),
        _ => items.concat(),
    }
}
