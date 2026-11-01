//! `archangel explain <file>`: the file's element, the rules that apply to it, each import with its
//! resolution and the rules it breaks, the imports archangel cannot follow, and the importers.

use std::fmt::Write;
use std::path::Path;

use archangel_config::{Classification, ElementKind, Pattern};
use archangel_engine::Session;
use archangel_graph::{Import, Resolution};
use archangel_rules::Target;

use serde_json::{Value, json};

use crate::render::json::SCHEMA_VERSION;
use crate::{Answer, Format, Options, Outcome, paths};

pub(crate) fn run_explain(options: &Options, file: &Path) -> Answer {
    let session = match super::open_session(options) {
        Ok(session) => session,
        Err(answer) => return answer,
    };
    let path = match paths::source(&session, file) {
        Ok(path) => path,
        Err(reason) => return super::not_a_source_file(&reason),
    };
    let architecture = session.architecture();
    let class = architecture.matcher().classify_file(&path);
    if options.format == Format::Json {
        return Answer::out(
            json_document(&session, &path, class.as_ref()),
            Outcome::Clean,
        );
    }
    let mut out = format!("{path}\n");
    match &class {
        Some(class) => {
            for place in class.all() {
                let _ = writeln!(out, "  element: {}", instance(&session, place));
            }
        }
        None => out.push_str(&no_element(&session, &path)),
    }
    let selectors = architecture.selectors_of(session.graph(), &path);
    let selectors: Vec<&str> = selectors.iter().map(String::as_str).collect();
    let _ = writeln!(out, "  selectors: {}", list(&selectors));
    let names = rules_that_apply(&session, &path);
    let _ = writeln!(out, "  rules that apply: {}", list(&names));
    for (rule, sentence) in architecture.sentences_about(session.graph(), &path) {
        let _ = writeln!(
            out,
            "    {}: {}",
            architecture.config().rules[rule].name,
            architecture.sentence_text(rule, sentence)
        );
    }
    out.push_str(&imports(&session, &path));
    let importers = session.graph().importers(&path);
    let _ = writeln!(out, "  imported by: {}", list(&importers));
    Answer::out(out, Outcome::Clean)
}

/// The names of the rules with a sentence about the file, in config order.
fn rules_that_apply<'s>(session: &'s Session, path: &str) -> Vec<&'s str> {
    let architecture = session.architecture();
    let mut rules: Vec<usize> = architecture
        .sentences_about(session.graph(), path)
        .into_iter()
        .map(|(rule, _)| rule)
        .collect();
    rules.dedup();
    rules
        .into_iter()
        .map(|rule| architecture.config().rules[rule].name.as_str())
        .collect()
}

/// The same answer as the text, as one JSON document.
fn json_document(session: &Session, path: &str, class: Option<&Classification>) -> String {
    let element = |class: &Classification| {
        let mut found = json!({
            "name": session.architecture().config().elements[class.element.index()].name,
            "instance": class.captures,
        });
        if !class.also.is_empty() {
            found["also"] = class
                .also
                .iter()
                .map(|place| {
                    json!({
                        "name": session.architecture().config().elements[place.element.index()].name,
                        "instance": place.captures,
                    })
                })
                .collect();
        }
        found
    };
    let graph = session.graph();
    let facts = graph.file(path).and_then(|f| f.facts.as_ref());
    let imports: Option<Vec<Value>> = facts.zip(graph.resolutions(path)).map(|(facts, resolutions)| {
        facts
            .imports
            .iter()
            .zip(resolutions)
            .map(|(import, resolution)| {
                let found = resolved(session, path, import, resolution);
                json!({
                    "line": import.line,
                    "specifier": import.specifier,
                    "resolution": match resolution {
                        Resolution::File(file) => json!({ "file": file }),
                        Resolution::Package(name) => json!({ "package": name }),
                        Resolution::Unresolved(reason) => json!({ "unresolved": reason.message(&import.specifier) }),
                    },
                    "element": found.class.as_ref().map(element),
                    "breaks": found.broken,
                })
            })
            .collect()
    });
    let document = json!({
        "schemaVersion": SCHEMA_VERSION,
        "file": path,
        "element": class.map(element),
        "selectors": session.architecture().selectors_of(graph, path),
        "rules": rules_that_apply(session, path),
        "sentences": session
            .architecture()
            .sentences_about(graph, path)
            .into_iter()
            .map(|(rule, sentence)| json!({
                "rule": session.architecture().config().rules[rule].name,
                "sentence": session.architecture().sentence_text(rule, sentence),
            }))
            .collect::<Vec<Value>>(),
        "imports": imports,
        "unfollowedLines": facts.map(|facts| facts.unfollowed.clone()),
        "importedBy": graph.importers(path),
    });
    let mut text = serde_json::to_string_pretty(&document).unwrap_or_default();
    text.push('\n');
    text
}

fn list(items: &[&str]) -> String {
    if items.is_empty() {
        "none".to_owned()
    } else {
        items.join(", ")
    }
}

fn instance(session: &Session, class: &Classification) -> String {
    let name = &session.architecture().config().elements[class.element.index()].name;
    if class.captures.is_empty() {
        format!("\"{name}\"")
    } else {
        format!("\"{name}\", instance \"{}\"", class.captures.join("/"))
    }
}

/// Each element pattern, with why it does not match the path.
fn no_element(session: &Session, path: &str) -> String {
    let mut out = String::from("  element: no element; no file pattern matches it:\n");
    for element in &session.architecture().config().elements {
        if let ElementKind::Files(patterns) = &element.kind {
            for pattern in patterns {
                let _ = writeln!(
                    out,
                    "    \"{}\" {}: {}",
                    element.name,
                    pattern.source(),
                    mismatch(pattern, path)
                );
            }
        }
    }
    out
}

fn mismatch(pattern: &Pattern, path: &str) -> String {
    let source = pattern.source();
    let literal = source
        .find(['*', '{', '?'])
        .map_or(source, |end| &source[..end]);
    if path.starts_with(literal) {
        format!("the path starts with \"{literal}\", but the rest does not match")
    } else {
        format!("the path does not start with \"{literal}\"")
    }
}

/// The element instance an import points into, and the rules the import breaks.
struct Resolved<'s> {
    class: Option<Classification>,
    broken: Vec<&'s str>,
}

fn resolved<'s>(
    session: &'s Session,
    from: &str,
    import: &Import,
    resolution: &'s Resolution,
) -> Resolved<'s> {
    let architecture = session.architecture();
    let (target, class) = match resolution {
        Resolution::File(file) => (
            Some(Target::File(file)),
            architecture.matcher().classify_file(file),
        ),
        Resolution::Package(name) => (
            Some(Target::Package(name)),
            architecture.matcher().package_place(name),
        ),
        Resolution::Unresolved(_) => (None, None),
    };
    let broken = target.map_or_else(Vec::new, |target| {
        architecture
            .broken_rules_in(Some(session.graph()), from, target, import)
            .into_iter()
            .map(|index| architecture.config().rules[index].name.as_str())
            .collect()
    });
    Resolved { class, broken }
}

/// One import: where it resolves, the element instance there, and the rules it breaks.
fn import_line(session: &Session, from: &str, import: &Import, resolution: &Resolution) -> String {
    let found = resolved(session, from, import, resolution);
    let place = match resolution {
        Resolution::File(file) => file.clone(),
        Resolution::Package(name) => format!("package {name}"),
        Resolution::Unresolved(reason) => {
            format!("unresolved: {}", reason.message(&import.specifier))
        }
    };
    let shown = found
        .class
        .as_ref()
        .map(|c| format!(" ({})", instance(session, c)))
        .unwrap_or_default();
    let marks = if found.broken.is_empty() {
        String::new()
    } else {
        format!("  breaks {}", found.broken.join(", "))
    };
    format!(
        "    line {}  '{}' -> {place}{shown}{marks}\n",
        import.line, import.specifier
    )
}

fn imports(session: &Session, path: &str) -> String {
    let graph = session.graph();
    let mut out = String::from("  imports:\n");
    let (Some(facts), Some(resolutions)) = (
        graph.file(path).and_then(|f| f.facts.as_ref()),
        graph.resolutions(path),
    ) else {
        out.push_str("    none: the file could not be parsed\n");
        return out;
    };
    for (import, resolution) in facts.imports.iter().zip(resolutions) {
        out.push_str(&import_line(session, path, import, resolution));
    }
    if !facts.unfollowed.is_empty() {
        let lines: Vec<String> = facts
            .unfollowed
            .iter()
            .map(|line| format!("line {line}"))
            .collect();
        let _ = writeln!(
            out,
            "  imports archangel cannot follow, because the specifier is computed: {}",
            lines.join(", ")
        );
    }
    out
}
