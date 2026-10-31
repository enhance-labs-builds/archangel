//! Text output, drawn by annotate-snippets as rustc draws its errors: the source line, and a mark
//! under the specifier.

use std::fmt::Write;
use std::path::Path;

use annotate_snippets::{AnnotationKind, Group, Level, Origin, Renderer, Snippet};
use archangel_engine::Timings;
use archangel_graph::Work;
use archangel_rules::{Diagnostic, Summary};

/// Every diagnostic, each with its source line when the file can be read from `dir`.
pub(crate) fn diagnostics(dir: &Path, diagnostics: &[Diagnostic], color: bool) -> String {
    let renderer = if color {
        Renderer::styled()
    } else {
        Renderer::plain()
    };
    let mut out = String::new();
    for diagnostic in diagnostics {
        let source = diagnostic
            .file
            .as_ref()
            .filter(|_| diagnostic.span.is_some())
            .and_then(|file| std::fs::read_to_string(dir.join(file)).ok());
        out.push_str(&renderer.render(&[group(diagnostic, source.as_deref())]));
        out.push_str("\n\n");
    }
    out
}

fn group<'a>(diagnostic: &'a Diagnostic, source: Option<&'a str>) -> Group<'a> {
    let title = Level::ERROR
        .primary_title(diagnostic.message.as_str())
        .id(diagnostic.code.as_str());
    let mut group = match (&diagnostic.file, diagnostic.span, source) {
        (Some(file), Some((start, end)), Some(text)) => {
            let mut annotation = AnnotationKind::Primary.span(start as usize..end as usize);
            if let Some(help) = &diagnostic.help {
                annotation = annotation.label(help.as_str());
            }
            title.element(
                Snippet::source(text)
                    .path(file.as_str())
                    .fold(true)
                    .annotation(annotation),
            )
        }
        (Some(file), _, _) => {
            let mut origin = Origin::path(file.as_str());
            if diagnostic.line > 0 {
                origin = origin
                    .line(diagnostic.line as usize)
                    .char_column(diagnostic.column as usize);
            }
            let group = title.element(origin);
            note(group, Level::HELP, diagnostic.help.as_deref())
        }
        (None, _, _) => note(
            Group::with_title(title),
            Level::HELP,
            diagnostic.help.as_deref(),
        ),
    };
    if let Some(rule) = &diagnostic.rule
        && rule != diagnostic.code.as_str()
    {
        group = group.element(Level::NOTE.message(format!("rule: {rule}")));
    }
    if let Some(reason) = &diagnostic.reason {
        group = group.element(Level::NOTE.message(format!("reason: {reason}")));
    }
    if let Some(link) = &diagnostic.link {
        group = group.element(Level::NOTE.message(format!("see: {link}")));
    }
    group
}

fn note<'a>(group: Group<'a>, level: Level<'a>, text: Option<&'a str>) -> Group<'a> {
    match text {
        Some(text) => group.element(level.message(text)),
        None => group,
    }
}

/// One line for a diagnostic that a change removed.
pub(crate) fn gone(diagnostic: &Diagnostic) -> String {
    let place = match &diagnostic.file {
        Some(file) if diagnostic.line > 0 => format!("{file}:{}", diagnostic.line),
        Some(file) => file.clone(),
        None => "the config".to_owned(),
    };
    format!(
        "gone: [{}] {place}: {}\n",
        diagnostic.code.as_str(),
        diagnostic.message
    )
}

/// The closing summary: what was checked, what the config permitted, and how many problems.
pub(crate) fn summary(summary: &Summary, problems: usize) -> String {
    let mut out = format!(
        "checked {} files and {} imports: {}\n",
        summary.files,
        summary.imports,
        match problems {
            0 => "no problems".to_owned(),
            1 => "1 problem".to_owned(),
            n => format!("{n} problems"),
        }
    );
    let rules: std::collections::BTreeSet<&String> = summary
        .exceptions
        .keys()
        .chain(summary.baselined.keys())
        .collect();
    for rule in rules {
        let exceptions = summary.exceptions.get(rule).copied().unwrap_or(0);
        let baselined = summary.baselined.get(rule).copied().unwrap_or(0);
        let _ = writeln!(
            out,
            "  {rule}: {exceptions} permitted by exceptions, {baselined} by the baseline"
        );
    }
    if summary.permitted_unresolved > 0 {
        let _ = writeln!(
            out,
            "  {} unresolved imports permitted by allowUnresolved",
            summary.permitted_unresolved
        );
    }
    if summary.unfollowed > 0 {
        let _ = writeln!(
            out,
            "  {} imports with a computed specifier, which archangel cannot follow; `archangel explain` lists them",
            summary.unfollowed
        );
    }
    for link in &summary.links_not_followed {
        let _ = writeln!(
            out,
            "  {link} is a symbolic link, which archangel does not follow"
        );
    }
    for repository in &summary.nested_repositories {
        let _ = writeln!(
            out,
            "  {repository} is a repository of its own, checked as part of this one"
        );
    }
    for name in &summary.preset_names_matching_nothing {
        let _ = writeln!(out, "  {name} matches nothing in this repository");
    }
    out
}

/// Where to read about the codes of the diagnostics printed, or nothing when there are none.
pub(crate) fn docs_hint(diagnostics: &[Diagnostic]) -> String {
    let mut codes: Vec<&str> = Vec::new();
    for diagnostic in diagnostics {
        if !codes.contains(&diagnostic.code.as_str()) {
            codes.push(diagnostic.code.as_str());
        }
    }
    let commands: Vec<String> = codes
        .iter()
        .map(|code| format!("`archangel docs {code}`"))
        .collect();
    match commands.split_last() {
        None => String::new(),
        Some((last, [])) => format!("For what a code means, run {last}.\n"),
        Some((last, rest)) => format!(
            "For what a code means, run {} or {last}.\n",
            rest.join(", ")
        ),
    }
}

/// The work counts and the duration of each stage, for `--stats`.
pub(crate) fn stats(work: &Work, timings: Option<Timings>) -> String {
    let mut out = String::from("work:\n");
    for (name, count) in work.counts() {
        let _ = writeln!(out, "  {}: {count}", words(name));
    }
    if let Some(timings) = timings {
        out.push_str("stages:\n");
        for (stage, duration) in timings.stages() {
            let _ = writeln!(out, "  {stage}: {:.1} ms", duration.as_secs_f64() * 1000.0);
        }
    }
    out
}

/// A camelCase counter name in words: `filesRead` is "files read".
fn words(name: &str) -> String {
    let mut out = String::new();
    for character in name.chars() {
        if character.is_ascii_uppercase() {
            out.push(' ');
            out.push(character.to_ascii_lowercase());
        } else {
            out.push(character);
        }
    }
    out
}
