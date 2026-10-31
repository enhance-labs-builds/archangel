//! What a file exports, following `export *` chains through the graph, and the export rules:
//! `mustExport`, `mayNotExport` and `mustOnlyReExport`. The rules read the facts the one parse of
//! each file kept, so they parse nothing.

use std::collections::BTreeSet;

use archangel_config::{Condition, ExportKind, NamePattern, Rule};
use archangel_graph::{FileFacts, ImportForm, ImportGraph, OverlayFiles, Resolution};

use crate::diagnostic::{Code, Diagnostic, ViolationId};
use crate::naming::{FileName, expand, matches, spelled};

/// Whether a file exports a name, itself or through the files its `export *` statements pass on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Exports {
    /// It does.
    Yes,
    /// It does not, and every chain could be followed.
    No,
    /// It may, through `export *` statements whose names archangel cannot know: each from a
    /// package or a specifier that does not resolve. The specifiers, in the order found.
    Unknown(Vec<String>),
}

/// Every name a file exports, through its `export *` chains, and the `export *` specifiers on
/// the way whose names archangel cannot know.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExportedNames {
    /// The names, sorted.
    pub names: BTreeSet<String>,
    /// The specifiers of `export *` statements that come from a package or do not resolve.
    pub unknown: Vec<String>,
}

/// The parsed files whose exports the walk reads: the graph's, with an example's files over them.
#[derive(Clone, Copy)]
pub(crate) struct Sources<'a> {
    pub(crate) graph: &'a ImportGraph,
    pub(crate) overlay: Option<&'a OverlayFiles>,
}

impl<'a> Sources<'a> {
    /// A file's facts and where its imports resolve, from the example first.
    pub(crate) fn facts(&self, path: &str) -> Option<(&'a FileFacts, &'a [Resolution])> {
        let example = self.overlay.and_then(|overlay| {
            let at = overlay.files.iter().position(|f| f.path == path)?;
            Some((&overlay.files[at], overlay.resolutions[at].as_slice()))
        });
        if let Some((file, resolutions)) = example {
            return file.facts.as_ref().map(|facts| (facts, resolutions));
        }
        let facts = self.graph.file(path)?.facts.as_ref()?;
        Some((facts, self.graph.resolutions(path).unwrap_or(&[])))
    }
}

/// Every name the file at `path` exports. `export *` never passes on `default`.
#[must_use]
pub fn exported_names(graph: &ImportGraph, path: &str) -> ExportedNames {
    names_through(
        Sources {
            graph,
            overlay: None,
        },
        path,
    )
}

fn names_through(sources: Sources<'_>, path: &str) -> ExportedNames {
    let mut found = ExportedNames::default();
    let mut seen = BTreeSet::new();
    let mut stack = vec![(path.to_owned(), true)];
    while let Some((file, own)) = stack.pop() {
        if !seen.insert(file.clone()) {
            continue;
        }
        let Some((facts, resolutions)) = sources.facts(&file) else {
            continue;
        };
        found.names.extend(
            facts
                .exports
                .names
                .iter()
                .filter(|name| own || name.as_str() != "default")
                .cloned(),
        );
        for (specifier, resolution) in star_sources(facts, resolutions) {
            match resolution {
                Resolution::File(next) => stack.push((next.clone(), false)),
                Resolution::Package(_) | Resolution::Unresolved(_) => {
                    found.unknown.push(specifier.to_owned());
                }
            }
        }
    }
    found
}

/// Whether the file at `path` exports `name`. A namespace import, `*`, needs no name in
/// particular.
#[must_use]
pub fn exports_name(graph: &ImportGraph, path: &str, name: &str) -> Exports {
    if name == "*" {
        return Exports::Yes;
    }
    let found = exported_names(graph, path);
    if found.names.contains(name) {
        Exports::Yes
    } else if found.unknown.is_empty() || name == "default" {
        Exports::No
    } else {
        Exports::Unknown(found.unknown)
    }
}

/// The specifier and resolution of each `export *` statement of a file, `export * as` aside,
/// which names what it passes on.
fn star_sources<'g>(
    facts: &'g FileFacts,
    resolutions: &'g [Resolution],
) -> impl Iterator<Item = (&'g str, &'g Resolution)> {
    facts
        .imports
        .iter()
        .zip(resolutions)
        .filter(move |(import, _)| {
            import.form == ImportForm::ExportFrom
                && import.names.is_empty()
                && facts.exports.star_from.contains(&import.specifier)
        })
        .map(|(import, resolution)| (import.specifier.as_str(), resolution))
}

/// The files that pass on the names of a file through `export *`, directly or through other
/// such files: the files whose export rules an edit to it can change.
#[must_use]
pub fn star_importers(graph: &ImportGraph, paths: &[&str]) -> Vec<String> {
    let mut found: BTreeSet<String> = BTreeSet::new();
    let mut stack: Vec<String> = paths.iter().map(|p| (*p).to_owned()).collect();
    while let Some(file) = stack.pop() {
        for importer in graph.importers(&file) {
            let passes_on = graph
                .file(importer)
                .and_then(|f| f.facts.as_ref())
                .is_some_and(|facts| {
                    star_sources(facts, graph.resolutions(importer).unwrap_or(&[])).any(
                        |(_, resolution)| matches!(resolution, Resolution::File(p) if *p == file),
                    )
                });
            if passes_on && found.insert(importer.to_owned()) {
                stack.push(importer.to_owned());
            }
        }
    }
    found.into_iter().collect()
}

/// A rule, and the path of the file, folder or instance root that it looks at.
#[derive(Clone, Copy)]
pub(crate) struct About<'a> {
    pub(crate) rule: &'a Rule,
    pub(crate) path: &'a str,
}

impl About<'_> {
    /// A violation of the rule at the path, identified by the two.
    pub(crate) fn violation(self, code: Code, message: String, help: &str) -> Diagnostic {
        let mut diagnostic = Diagnostic::new(code, message).at(self.path, 0, 0);
        diagnostic.rule = Some(self.rule.name.clone());
        diagnostic.help = Some(help.to_owned());
        diagnostic.reason.clone_from(&self.rule.reason);
        diagnostic.link.clone_from(&self.rule.link);
        diagnostic.id = Some(ViolationId::Path {
            rule: self.rule.name.clone(),
            path: self.path.to_owned(),
        });
        diagnostic
    }
}

/// The diagnostics of one export condition for one parsed file.
pub(crate) fn evaluate(
    sources: Sources<'_>,
    about: About<'_>,
    condition: &Condition,
    facts: &FileFacts,
) -> Vec<Diagnostic> {
    match condition {
        Condition::MustExport(pattern) => must_export(sources, about, pattern),
        Condition::MayNotExport(kind) => may_not_export(about, kind, facts),
        Condition::MustOnlyReExport => facts
            .own_statement
            .iter()
            .map(|own| {
                let what = own
                    .name
                    .as_ref()
                    .map_or_else(|| "code".to_owned(), |name| format!("`{name}`"));
                let mut diagnostic = about.violation(
                    Code::MustOnlyReExport,
                    format!(
                        "{} may only pass on what other files export, and it declares {what} of \
                         its own",
                        about.path
                    ),
                    "move the declaration into a file of its own, and re-export it from there",
                );
                diagnostic.line = own.line;
                diagnostic.column = own.column;
                diagnostic
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// A violation at each export of the kind the rule refuses.
fn may_not_export(about: About<'_>, kind: &ExportKind, facts: &FileFacts) -> Vec<Diagnostic> {
    let names = FileName::of(about.path);
    let pieces = match kind {
        ExportKind::Names(pattern) => expand(pattern, &names),
        ExportKind::Default | ExportKind::Star => Vec::new(),
    };
    let help = match kind {
        ExportKind::Default => "export the value by name instead",
        ExportKind::Star => "pass the names on one by one instead",
        ExportKind::Names(_) => "stop exporting the name, or rename it",
    };
    facts
        .exports
        .entries
        .iter()
        .filter_map(|export| {
            let what = match kind {
                ExportKind::Default if export.name == "default" && !export.star => {
                    "a default export".to_owned()
                }
                ExportKind::Star if export.star && export.name == "*" => "an `export *`".to_owned(),
                ExportKind::Star if export.star => format!("`export * as {}`", export.name),
                ExportKind::Names(pattern)
                    if export.name != "*" && matches(&pieces, &export.name) =>
                {
                    format!("`{}`, which matches `{}`", export.name, pattern.source())
                }
                _ => return None,
            };
            let mut diagnostic = about.violation(
                Code::MayNotExport,
                format!("{} may not have {what}", about.path),
                help,
            );
            diagnostic.line = export.line;
            diagnostic.column = export.column;
            Some(diagnostic)
        })
        .collect()
}

fn must_export(sources: Sources<'_>, about: About<'_>, pattern: &NamePattern) -> Vec<Diagnostic> {
    let path = about.path;
    let pieces = expand(pattern, &FileName::of(path));
    let found = names_through(sources, path);
    if found.names.iter().any(|name| matches(&pieces, name)) {
        return Vec::new();
    }
    let wanted = spelled(&pieces);
    if !found.unknown.is_empty() {
        let sources: Vec<String> = found.unknown.iter().map(|s| format!("`{s}`")).collect();
        let mut diagnostic = Diagnostic::new(
            Code::ExportsUnknown,
            format!(
                "\"{}\" needs {path} to export `{wanted}`, and archangel cannot know the names it \
                 passes on through `export *` from {}, a package or a specifier that does not \
                 resolve",
                about.rule.name,
                sources.join(", ")
            ),
        )
        .at(path, 0, 0);
        diagnostic.rule = Some(about.rule.name.clone());
        diagnostic.help = Some(format!(
            "export `{wanted}` by name, as in `export {{ {wanted} }} from '{}'`",
            found.unknown[0]
        ));
        return vec![diagnostic];
    }
    let has = if found.names.is_empty() {
        "it exports nothing".to_owned()
    } else {
        let listed: Vec<String> = found.names.iter().map(|n| format!("`{n}`")).collect();
        format!("it exports {}", listed.join(", "))
    };
    vec![about.violation(
        Code::MustExport,
        format!("{path} must export `{wanted}`, and {has}"),
        &format!("export `{wanted}`, or rename the export that should be it"),
    )]
}
