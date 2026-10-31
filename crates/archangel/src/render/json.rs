//! JSON output: one document with `schemaVersion: 1`. The schema is written out here, field by
//! field, so a change to an internal type cannot change it by accident.

use archangel_engine::{ExampleReport, Timings};
use archangel_graph::Work;
use archangel_rules::{Diagnostic, ExampleKind, Summary};
use serde_json::{Value, json};

/// The schema version of every JSON document archangel prints.
pub(crate) const SCHEMA_VERSION: u32 = 1;

/// One diagnostic.
pub(crate) fn diagnostic(diagnostic: &Diagnostic) -> Value {
    json!({
        "code": diagnostic.code.as_str(),
        "rule": diagnostic.rule,
        "file": diagnostic.file,
        "line": diagnostic.line,
        "column": diagnostic.column,
        "span": diagnostic.span.map(|(start, end)| json!([start, end])),
        "specifier": diagnostic.specifier,
        "message": diagnostic.message,
        "help": diagnostic.help,
        "reason": diagnostic.reason,
        "link": diagnostic.link,
        "id": diagnostic.id.as_ref().map(|id| serde_json::to_value(id).unwrap_or(Value::Null)),
        "cycle": diagnostic.cycle,
        "group": diagnostic.group,
        "chain": diagnostic.chain,
        "fix": diagnostic.fix.as_ref().map(|fix| json!({
            "span": [fix.span.0, fix.span.1],
            "specifier": fix.specifier,
        })),
    })
}

fn durations(timings: Timings) -> Value {
    let ms = |duration: std::time::Duration| duration.as_secs_f64() * 1000.0;
    json!({
        "configMs": ms(timings.config),
        "graphMs": ms(timings.graph),
        "listFilesMs": ms(timings.steps.walk),
        "readAndParseMs": ms(timings.steps.read_and_parse),
        "resolveMs": ms(timings.steps.resolve),
        "rulesMs": ms(timings.rules),
    })
}

/// The document for `check`. Durations are in it only with `--stats`, so that two checks of the
/// same code print the same bytes.
pub(crate) fn check_document(
    diagnostics: &[&Diagnostic],
    summary: &Summary,
    counts: &Work,
    timings: Option<Timings>,
) -> String {
    let mut summary = json!({
        "files": summary.files,
        "imports": summary.imports,
        "exceptions": summary.exceptions,
        "baselined": summary.baselined,
        "permittedUnresolved": summary.permitted_unresolved,
        "unfollowed": summary.unfollowed,
        "linksNotFollowed": summary.links_not_followed,
        "nestedRepositories": summary.nested_repositories,
        "presetNamesMatchingNothing": summary.preset_names_matching_nothing,
        "work": counts.counts_json(),
    });
    if let Some(timings) = timings {
        summary["durations"] = durations(timings);
    }
    document(&json!({
        "schemaVersion": SCHEMA_VERSION,
        "diagnostics": diagnostics.iter().map(|d| diagnostic(d)).collect::<Vec<_>>(),
        "summary": summary,
    }))
}

/// What one check of watch mode shows.
pub(crate) struct WatchLine<'w> {
    pub(crate) new: &'w [Diagnostic],
    pub(crate) gone: &'w [Diagnostic],
    pub(crate) report: &'w archangel_rules::Report,
    pub(crate) work: &'w Work,
    pub(crate) polling: Option<&'w str>,
}

/// One line of `watch --format json`: what is new, what is gone, and the summary of the check
/// with its work, on one line.
pub(crate) fn watch_line(line: &WatchLine<'_>) -> String {
    let WatchLine {
        new,
        gone,
        report,
        work: counts,
        polling,
    } = *line;
    let summary = &report.summary;
    let line = json!({
        "schemaVersion": SCHEMA_VERSION,
        "new": new.iter().map(diagnostic).collect::<Vec<_>>(),
        "gone": gone.iter().map(diagnostic).collect::<Vec<_>>(),
        "summary": {
            "files": summary.files,
            "imports": summary.imports,
            "problems": report.diagnostics.len(),
            "exceptions": summary.exceptions,
            "baselined": summary.baselined,
            "work": counts.counts_json(),
            "polling": polling,
        },
    });
    let mut text = line.to_string();
    text.push('\n');
    text
}

/// The document for diagnostics that stopped a command.
pub(crate) fn stopped_document(diagnostics: &[Diagnostic]) -> String {
    document(&json!({
        "schemaVersion": SCHEMA_VERSION,
        "diagnostics": diagnostics.iter().map(diagnostic).collect::<Vec<_>>(),
    }))
}

/// The document for `test`.
pub(crate) fn test_document(report: &ExampleReport) -> String {
    let examples: Vec<Value> = report
        .runs
        .iter()
        .map(|run| {
            json!({
                "rule": run.rule,
                "kind": match run.kind { ExampleKind::Accepted => "accepted", ExampleKind::Rejected => "rejected" },
                "number": run.number,
                "files": run.files,
                "passed": run.outcome.passed,
                "failure": run.outcome.failure.as_ref().map(diagnostic),
                "diagnostics": run.outcome.diagnostics.iter().map(diagnostic).collect::<Vec<_>>(),
            })
        })
        .collect();
    document(&json!({
        "schemaVersion": SCHEMA_VERSION,
        "examples": examples,
        "unproven": report.unproven.iter().map(diagnostic).collect::<Vec<_>>(),
        "summary": { "passed": report.passed(), "work": report.work.counts_json() },
    }))
}

fn document(value: &Value) -> String {
    let mut text = serde_json::to_string_pretty(value).unwrap_or_default();
    text.push('\n');
    text
}

#[cfg(test)]
mod tests;
