//! `archangel explain <file>`: the file's part, the rules that apply to it, each import with its
//! resolution and the rules it breaks, the imports archangel cannot follow, and the importers. With
//! `<file>:<line>`: every rule that judged or skipped the import on that line.

use std::fmt::Write;
use std::path::Path;

use archangel_config::{Classification, ElementKind, Pattern};
use archangel_engine::Session;
use archangel_graph::{FileFacts, Import, Resolution};
use archangel_rules::{Code, ImportEnd, ImportJudgement, RuleJudgement, Target, Verdict};

use serde_json::{Value, json};

use crate::render::json::SCHEMA_VERSION;
use crate::{Answer, Format, Options, Outcome, paths};

/// The file and the line of an argument: the line is what follows the last colon, when that is a
/// number, so that a Windows path and a file name with a colon stay in one piece.
fn file_and_line(argument: &Path) -> (&Path, Option<u32>) {
    let Some(text) = argument.to_str() else {
        return (argument, None);
    };
    match text.rsplit_once(':') {
        Some((file, line)) if !file.is_empty() => match line.parse() {
            Ok(line) => (Path::new(file), Some(line)),
            Err(_) => (argument, None),
        },
        _ => (argument, None),
    }
}

pub(crate) fn run_explain(options: &Options, file: &Path) -> Answer {
    let session = match super::open_session(options) {
        Ok(session) => session,
        Err(answer) => return answer,
    };
    let (file, line) = file_and_line(file);
    let path = match paths::source(&session, file) {
        Ok(path) => path,
        Err(reason) => return super::not_a_source_file(&reason),
    };
    if let Some(line) = line {
        return explain_line(options, &session, &path, line);
    }
    let architecture = session.architecture();
    let class = architecture.matcher().classify_file(&path);
    if options.format == Format::Json {
        return Answer::out(
            json_document(&session, &path, class.as_ref()),
            Outcome::Clean,
        );
    }
    let mut out = format!("{path}\n");
    let part = architecture.config().part_word();
    match &class {
        Some(class) => {
            for place in class.all() {
                let _ = writeln!(out, "  {part}: {}", instance(&session, place));
            }
        }
        None => out.push_str(&no_element(&session, &path)),
    }
    for (table, kind) in architecture.kinds_of(session.graph(), &path) {
        let _ = writeln!(out, "  kind: {kind} in {table}");
    }
    let selectors = architecture.selectors_of(session.graph(), &path);
    let selectors: Vec<&str> = selectors.iter().map(String::as_str).collect();
    let group = architecture.config().group_word();
    let _ = writeln!(out, "  {group}s: {}", list(&selectors));
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
    let graph = session.graph();
    let facts = graph.file(path).and_then(|f| f.facts.as_ref());
    let imports = facts.map(|facts| imports_json(session, path, facts));
    let document = json!({
        "schemaVersion": SCHEMA_VERSION,
        "file": path,
        "element": class.map(|class| element_json(session, class)),
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
    let mut document = document;
    if !session.architecture().config().tables.is_empty() {
        let kinds = session.architecture().kinds_of(graph, path);
        document["kinds"] = kinds_json(&kinds).into();
    }
    json_text(&document)
}

/// The part of a file or a target, with its instance, and any other part of a preset that has it.
fn element_json(session: &Session, class: &Classification) -> Value {
    let elements = &session.architecture().config().elements;
    let mut found = json!({
        "name": elements[class.element.index()].name,
        "instance": class.captures,
    });
    if !class.also.is_empty() {
        found["also"] = class
            .also
            .iter()
            .map(|place| {
                json!({
                    "name": elements[place.element.index()].name,
                    "instance": place.captures,
                })
            })
            .collect();
    }
    found
}

/// Each import of a file, with its resolution, its target's part and the rules that it breaks.
fn imports_json(session: &Session, path: &str, facts: &FileFacts) -> Vec<Value> {
    let resolutions = session.graph().resolutions(path).unwrap_or_default();
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
                "element": found.class.as_ref().map(|class| element_json(session, class)),
                "breaks": found.broken,
            })
        })
        .collect()
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
    let part = session.architecture().config().part_word();
    let mut out = format!("  {part}: no {part}; no file pattern matches it:\n");
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

/// The answer for a line that holds no import.
fn no_import_on_line(options: &Options, path: &str, line: u32) -> Answer {
    let message = format!("line {line} of {path} has no import");
    let stdout = if options.format == Format::Json {
        json_text(
            &json!({ "schemaVersion": SCHEMA_VERSION, "file": path, "line": line, "imports": [], "message": message }),
        )
    } else {
        format!("{message}\n")
    };
    Answer::out(stdout, Outcome::Found)
}

/// One import, or each import on a line, against every rule.
fn explain_line(options: &Options, session: &Session, path: &str, line: u32) -> Answer {
    let graph = session.graph();
    let (Some(facts), Some(resolutions)) = (
        graph.file(path).and_then(|f| f.facts.as_ref()),
        graph.resolutions(path),
    ) else {
        return does_not_parse(options, session, path);
    };
    let on_line: Vec<(&Import, &Resolution)> = facts
        .imports
        .iter()
        .zip(resolutions)
        .filter(|(import, _)| (import.statement_line..=import.line).contains(&line))
        .collect();
    if on_line.is_empty() {
        return no_import_on_line(options, path, line);
    }
    let blocks: Vec<(&Import, &Resolution, ImportJudgement)> = on_line
        .into_iter()
        .map(|(import, resolution)| {
            let judged = session.architecture().judge_import(
                graph,
                path,
                import,
                resolution,
                session.report(),
            );
            (import, resolution, judged)
        })
        .collect();
    let stdout = if options.format == Format::Json {
        let imports: Vec<Value> = blocks
            .iter()
            .map(|(import, resolution, judged)| import_json(import, resolution, judged))
            .collect();
        json_text(&json!({
            "schemaVersion": SCHEMA_VERSION,
            "file": path,
            "line": line,
            "imports": imports,
        }))
    } else {
        let texts: Vec<String> = blocks
            .iter()
            .map(|(import, resolution, judged)| {
                import_block(session, (path, line), (import, resolution), judged)
            })
            .collect();
        texts.join("\n")
    };
    Answer::out(stdout, Outcome::Clean)
}

/// The answer for a file that does not parse: its `parse-failed` diagnostic, and that archangel
/// cannot explain it.
fn does_not_parse(options: &Options, session: &Session, path: &str) -> Answer {
    let failed: Vec<_> = session
        .report()
        .diagnostics
        .iter()
        .filter(|d| d.code == Code::ParseFailed && d.file.as_deref() == Some(path))
        .cloned()
        .collect();
    let message = "archangel cannot explain a file that does not parse";
    let stdout = if options.format == Format::Json {
        json_text(&json!({
            "schemaVersion": SCHEMA_VERSION,
            "file": path,
            "imports": [],
            "message": message,
            "diagnostics": failed.iter().map(crate::render::json::diagnostic).collect::<Vec<_>>(),
        }))
    } else {
        let mut out = crate::render::text::diagnostics(session.config_dir(), &failed, false);
        let _ = writeln!(out, "{message}");
        out
    };
    Answer::out(stdout, Outcome::Found)
}

fn json_text(value: &Value) -> String {
    let mut text = serde_json::to_string_pretty(value).unwrap_or_default();
    text.push('\n');
    text
}

/// An end of an import in words: its parts, each with its instance, and its kinds.
fn end_text(session: &Session, end: &ImportEnd) -> String {
    let word = session.architecture().config().part_word();
    let mut pieces: Vec<String> = end
        .parts
        .iter()
        .map(|(name, captures)| {
            if captures.is_empty() {
                format!("{word} {name}")
            } else {
                format!("{word} {name} (instance {})", captures.join("/"))
            }
        })
        .collect();
    pieces.extend(
        end.kinds
            .iter()
            .map(|(table, kind)| format!("kind {kind} in {table}")),
    );
    if pieces.is_empty() {
        format!("in no {word}")
    } else {
        pieces.join(", ")
    }
}

fn import_block(
    session: &Session,
    (path, line): (&str, u32),
    (import, resolution): (&Import, &Resolution),
    judged: &ImportJudgement,
) -> String {
    let specifier = &import.specifier;
    let mut out = match resolution {
        Resolution::File(file) => format!("{path}:{line}  '{specifier}' -> {file}\n"),
        Resolution::Package(name) => format!("{path}:{line}  '{specifier}' -> package {name}\n"),
        Resolution::Unresolved(reason) => format!(
            "{path}:{line}  '{specifier}' does not resolve: {}\n",
            reason.message(specifier)
        ),
    };
    let _ = writeln!(out, "  file:    {}", end_text(session, &judged.from));
    match resolution {
        Resolution::File(_) => {
            let _ = writeln!(out, "  target:  {}", end_text(session, &judged.to));
        }
        Resolution::Package(name) => {
            let sets = judged.to.parts.iter().map(|(set, _)| format!(", in {set}"));
            let _ = writeln!(out, "  target:  package {name}{}", sets.collect::<String>());
        }
        Resolution::Unresolved(_) => {}
    }
    let (judging, skipping): (Vec<&RuleJudgement>, Vec<&RuleJudgement>) = judged
        .rules
        .iter()
        .partition(|rule| !matches!(rule.verdict, Verdict::Skips(_)));
    let width = judged.rules.iter().map(|r| r.rule.len()).max().unwrap_or(0);
    for (title, rules) in [("judged by", judging), ("skipped by", skipping)] {
        if rules.is_empty() {
            continue;
        }
        let _ = writeln!(out, "  {title}:");
        for rule in rules {
            let _ = writeln!(out, "    {:width$}  {}", rule.rule, verdict_text(rule));
        }
    }
    out
}

fn verdict_text(rule: &RuleJudgement) -> String {
    let beside = rule
        .permitted_by
        .as_ref()
        .map_or_else(String::new, |by| format!("; permitted by {by}"));
    match &rule.verdict {
        Verdict::Refuses(why) => format!("refuses: {why}{beside}"),
        Verdict::Permits(why) => format!("permits: {why}"),
        Verdict::Skips(why) => why.clone(),
    }
}

fn kinds_json(kinds: &[(String, String)]) -> Vec<Value> {
    kinds
        .iter()
        .map(|(table, kind)| json!({ "table": table, "kind": kind }))
        .collect()
}

fn end_json(end: &ImportEnd) -> Value {
    let (part, instance) = end
        .parts
        .first()
        .map_or((None, Vec::new()), |(name, captures)| {
            (Some(name.clone()), captures.clone())
        });
    json!({ "part": part, "instance": instance, "kinds": kinds_json(&end.kinds) })
}

fn import_json(import: &Import, resolution: &Resolution, judged: &ImportJudgement) -> Value {
    let rules: Vec<Value> = judged
        .rules
        .iter()
        .map(|rule| {
            let (verdict, reason) = match &rule.verdict {
                Verdict::Refuses(why) => ("refuses", why),
                Verdict::Permits(why) => ("permits", why),
                Verdict::Skips(why) => ("skips", why),
            };
            json!({
                "rule": rule.rule,
                "verdict": verdict,
                "reason": reason,
                "permittedBy": rule.permitted_by,
            })
        })
        .collect();
    let target = match resolution {
        Resolution::File(file) => json!({ "file": file }),
        Resolution::Package(name) => json!({ "package": name }),
        Resolution::Unresolved(reason) => {
            json!({ "unresolved": reason.message(&import.specifier) })
        }
    };
    json!({
        "specifier": import.specifier,
        "line": import.line,
        "target": target,
        "from": end_json(&judged.from),
        "to": end_json(&judged.to),
        "rules": rules,
    })
}
