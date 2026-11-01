//! `archangel rules [path]`: every rule and table row as a sentence, with what it covers in this
//! repository, and what no rule judges. It describes, so it exits with status 0 whatever a check
//! finds.

use std::fmt::Write;
use std::path::Path;

use archangel_config::{Condition, Rule, RuleKind, TableId};
use archangel_engine::Session;
use archangel_rules::{Reach, RuleReach};
use serde_json::{Value, json};

use crate::render::json::SCHEMA_VERSION;
use crate::{Answer, Format, Options, Outcome, paths};

pub(crate) fn run_rules(options: &Options, path: Option<&Path>) -> Answer {
    let session = match super::open_session(options) {
        Ok(session) => session,
        Err(answer) => return answer,
    };
    let written = path.map(|path| paths::config_relative(&session, path));
    let under = written.as_deref().filter(|under| !under.is_empty());
    if under.is_some_and(|folder| !holds_a_file(&session, folder)) {
        let shown = path.map_or_else(String::new, |path| path.display().to_string());
        return Answer::stopped(format!("error: no source file is under {shown}\n"));
    }
    let config_file = paths::config_relative(&session, session.config_path());
    let reach =
        session
            .architecture()
            .reach(session.graph(), session.report(), under, &config_file);
    let stdout = if options.format == Format::Json {
        json_document(&session, &reach)
    } else {
        text(&session, &reach)
    };
    Answer::out(stdout, Outcome::Clean)
}

fn holds_a_file(session: &Session, folder: &str) -> bool {
    let prefix = format!("{folder}/");
    session
        .graph()
        .files()
        .iter()
        .any(|file| file.path == folder || file.path.starts_with(&prefix))
}

/// Where a rule is written: the file of its builder call with the line, or the config file.
fn location(session: &Session, rule: &Rule) -> (String, Option<u32>) {
    if let Some(source) = &rule.source {
        let file = Path::new(&source.file);
        let shown = file.strip_prefix(session.config_dir()).unwrap_or(file);
        return (
            shown.to_string_lossy().replace('\\', "/"),
            Some(source.line),
        );
    }
    let name = session
        .config_path()
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    (name, None)
}

fn location_text(session: &Session, rule: &Rule) -> String {
    match location(session, rule) {
        (file, Some(line)) => format!("{file}:{line}"),
        (file, None) => file,
    }
}

/// The preset that declared a rule, and whether the config replaced its rule.
fn origin(session: &Session, rule: &Rule) -> Option<String> {
    let (preset, name) = rule.name.split_once('/')?;
    if !session.architecture().from_preset(&rule.name) {
        return None;
    }
    Some(if rule.replaces_preset_rule {
        format!("replaces the rule {name} of the preset {preset}")
    } else {
        format!("from the preset {preset}")
    })
}

/// Every sentence of a rule, in words, with which imports it counts when that is not plain.
fn sentences(session: &Session, rule: &Rule) -> Vec<String> {
    let architecture = session.architecture();
    (0..rule.sentences.len())
        .map(|index| {
            let mut text = architecture
                .sentence_of_rule(rule, index)
                .replace(" | ", ", ");
            let sentence = &rule.sentences[index];
            let by_default =
                matches!(sentence.condition, Condition::NoCycles) || sentence.options.transitively;
            match rule.imports {
                archangel_config::ImportKinds::Types => text.push_str(", type imports only"),
                archangel_config::ImportKinds::Runtime if !by_default => {
                    text.push_str(", runtime imports only");
                }
                _ => {}
            }
            text
        })
        .collect()
}

fn checks_imports(rule: &Rule) -> bool {
    rule.sentences.iter().any(|sentence| {
        matches!(
            sentence.condition,
            Condition::MayNotImport(_)
                | Condition::MayOnlyImport(_)
                | Condition::MayOnlyBeImportedBy(_)
                | Condition::Entry { .. }
        )
    })
}

fn counted(count: u64, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// What no file can give a rule: the name that has no file.
fn checks_nothing(session: &Session, rule: &Rule) -> String {
    let name = match &rule.kind {
        RuleKind::TableRow { row: Some(row), .. } => row.as_str(),
        RuleKind::TableRow { table, row: None } => session.architecture().config().tables
            [table.index()]
        .name
        .as_str(),
        _ => rule
            .sentences
            .first()
            .map_or("", |sentence| sentence.files_text.as_str()),
    };
    format!("checks nothing: no file is in {name}")
}

/// The line under a rule: what it covered.
fn reach_text(session: &Session, rule: &Rule, reach: &RuleReach) -> String {
    let mut pieces = Vec::new();
    match reach.folders {
        Some(folders) => pieces.push(counted(folders, "folder", "folders")),
        None if reach.files == 0 => return checks_nothing(session, rule),
        None => pieces.push(counted(reach.files, "file", "files")),
    }
    let imports = checks_imports(rule);
    if imports {
        pieces.push(counted(
            reach.imports_judged,
            "import judged",
            "imports judged",
        ));
    }
    pieces.push(counted(reach.problems, "problem", "problems"));
    if imports {
        pieces.push(counted(reach.exceptions, "exception", "exceptions"));
    }
    if reach.baseline_matches > 0 {
        pieces.push(counted(
            reach.baseline_matches,
            "baseline match",
            "baseline matches",
        ));
    }
    pieces.join(" · ")
}

/// The lines that a rule and a table share: its reason, its link and its preset.
fn notes(session: &Session, rule: &Rule, out: &mut String) {
    if let Some(reason) = &rule.reason {
        let _ = writeln!(out, "  because: {reason}");
    }
    if let Some(link) = &rule.link {
        let _ = writeln!(out, "  link: {link}");
    }
    if let Some(origin) = origin(session, rule) {
        let _ = writeln!(out, "  {origin}");
    }
}

fn text(session: &Session, reach: &Reach) -> String {
    let rules = &session.architecture().config().rules;
    let mut blocks: Vec<String> = Vec::new();
    let mut shown: Vec<TableId> = Vec::new();
    for first in &reach.rules {
        let rule = &rules[first.rule];
        let RuleKind::TableRow { table, .. } = &rule.kind else {
            blocks.push(plain_rule(session, rule, first));
            continue;
        };
        if shown.contains(table) {
            continue;
        }
        shown.push(*table);
        let group: Vec<&RuleReach> = reach
            .rules
            .iter()
            .filter(|other| {
                matches!(&rules[other.rule].kind, RuleKind::TableRow { table: id, .. } if id == table)
            })
            .collect();
        blocks.push(table_block(session, *table, &group));
    }
    blocks.extend(off_block(session));
    blocks.push(footer(reach));
    blocks.join("\n")
}

fn plain_rule(session: &Session, rule: &Rule, reach: &RuleReach) -> String {
    let mut out = format!("{}  {}\n", rule.name, location_text(session, rule));
    notes(session, rule, &mut out);
    for sentence in sentences(session, rule) {
        let _ = writeln!(out, "  {sentence}");
    }
    let _ = writeln!(out, "    {}", reach_text(session, rule, reach));
    out
}

fn table_block(session: &Session, table: TableId, group: &[&RuleReach]) -> String {
    let config = session.architecture().config();
    let rules = &config.rules;
    let rows = config.tables[table.index()].rows.len() as u64;
    let first = &rules[group[0].rule];
    let mut out = format!(
        "{}  {}  {}\n",
        config.tables[table.index()].name,
        counted(rows, "row", "rows"),
        location_text(session, first)
    );
    notes(session, first, &mut out);
    out.push('\n');
    for reach in group {
        let rule = &rules[reach.rule];
        let _ = writeln!(
            out,
            "  {}: {}",
            rule.name,
            sentences(session, rule).join("; ")
        );
        let _ = writeln!(out, "    {}", reach_text(session, rule, reach));
    }
    out
}

fn off_block(session: &Session) -> Option<String> {
    let off = &session.architecture().config().off;
    if off.is_empty() {
        return None;
    }
    let mut out = format!("Rules that `off` removes: {}\n", off.len());
    for rule in off {
        let _ = writeln!(
            out,
            "  {}: {}",
            rule.name,
            sentences(session, rule).join("; ")
        );
    }
    Some(out)
}

fn footer(reach: &Reach) -> String {
    let mut out = String::new();
    let (files, imports) = (&reach.files_not_judged, &reach.imports_not_judged);
    if files.is_empty() && imports.is_empty() {
        out.push_str("Not judged by any rule: 0 files, 0 imports between repository files.\n");
    }
    if !files.is_empty() {
        let _ = writeln!(out, "Files that no rule judges: {}", files.len());
        for file in files {
            let _ = writeln!(out, "  {file}");
        }
    }
    if !imports.is_empty() {
        let _ = writeln!(
            out,
            "Imports between repository files that no rule judges: {}",
            imports.len()
        );
        for import in imports {
            let _ = writeln!(
                out,
                "  {}:{} imports {}",
                import.from, import.line, import.to
            );
        }
    }
    let packages: Vec<String> = reach
        .packages_not_named
        .iter()
        .map(|(name, count)| format!("{name} ({})", counted(*count, "import", "imports")))
        .collect();
    let named = if packages.is_empty() {
        "none".to_owned()
    } else {
        packages.join(", ")
    };
    let _ = writeln!(out, "Outside packages that no rule names: {named}.");
    out
}

fn rule_json(session: &Session, rule: &Rule, reach: Option<&RuleReach>) -> Value {
    let (file, line) = location(session, rule);
    let mut entry = json!({
        "name": rule.name,
        "sentences": sentences(session, rule),
        "because": rule.reason,
        "link": rule.link,
        "file": file,
        "line": line,
        "origin": origin(session, rule),
        "off": reach.is_none(),
    });
    if let RuleKind::TableRow { table, row } = &rule.kind {
        let tables = &session.architecture().config().tables;
        entry["table"] = json!(tables[table.index()].name);
        entry["row"] = json!(row);
    }
    if let Some(reach) = reach {
        let mut counts = json!({
            "files": reach.files,
            "importsJudged": reach.imports_judged,
            "problems": reach.problems,
            "exceptions": reach.exceptions,
            "baselineMatches": reach.baseline_matches,
        });
        if let Some(folders) = reach.folders {
            counts["folders"] = json!(folders);
        }
        entry["reach"] = counts;
        if reach.files == 0 && reach.folders.is_none() {
            entry["checksNothing"] = json!(checks_nothing(session, rule));
        }
    }
    entry
}

fn json_document(session: &Session, reach: &Reach) -> String {
    let config = session.architecture().config();
    let mut rules: Vec<Value> = reach
        .rules
        .iter()
        .map(|found| rule_json(session, &config.rules[found.rule], Some(found)))
        .collect();
    rules.extend(config.off.iter().map(|rule| rule_json(session, rule, None)));
    let imports: Vec<Value> = reach
        .imports_not_judged
        .iter()
        .map(|import| json!({ "from": import.from, "line": import.line, "to": import.to }))
        .collect();
    let packages: Vec<Value> = reach
        .packages_not_named
        .iter()
        .map(|(name, count)| json!({ "name": name, "imports": count }))
        .collect();
    let document = json!({
        "schemaVersion": SCHEMA_VERSION,
        "rules": rules,
        "notJudged": { "files": reach.files_not_judged, "imports": imports },
        "outsidePackages": packages,
    });
    let mut text = serde_json::to_string_pretty(&document).unwrap_or_default();
    text.push('\n');
    text
}
