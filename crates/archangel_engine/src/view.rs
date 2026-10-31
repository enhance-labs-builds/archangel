//! The view a plugin condition receives: one record for each file in its rule's `files`, with the
//! facts the condition reads. The npm package declares the same types by hand in `plugin.d.ts`,
//! and a test compares the two.
//!
//! A new field never changes the schema version, because a plugin reads only the fields it
//! knows; a field whose meaning changes needs a new version.

use std::collections::BTreeMap;

use archangel_config::Reads;
use archangel_graph::{ImportGraph, ImportKind, Resolution, SourceFile};
use archangel_rules::PluginPlace;
use serde::Serialize;

/// The version of the view that this archangel sends.
pub const SCHEMA_VERSION: u64 = archangel_config::VIEW_SCHEMA_VERSION;

/// What a condition's `check` receives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    /// The version of the view's schema.
    pub schema_version: u64,
    /// The files in the rule's `files`, by path.
    pub files: Vec<FileRecord>,
}

/// One file of the view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileRecord {
    /// The path, relative to the config folder.
    pub path: String,
    /// The file's element, if it is in one.
    pub element: Option<String>,
    /// The element's captures, by name.
    pub captures: BTreeMap<String, String>,
    /// The declared selectors the file is in.
    pub selectors: Vec<String>,
    /// The file's imports, for a condition that reads them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imports: Option<Vec<ImportRecord>>,
    /// The paths of the files that import it, for a condition that reads them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub importers: Option<Vec<String>>,
    /// What the file exports, for a condition that reads it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exports: Option<ExportsRecord>,
}

/// One import of a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportRecord {
    /// The specifier, as written.
    pub specifier: String,
    /// Which import of that specifier in the file, from 0, which a violation names with it.
    pub occurrence: u32,
    /// The line, from 1.
    pub line: u32,
    /// The column, from 1, in characters.
    pub column: u32,
    /// `runtime`, or `type` for an import the compiler removes.
    pub kind: &'static str,
    /// The names it brings in: `default`, `*` or the name.
    pub names: Vec<String>,
    /// The repository file it resolves to.
    pub file: Option<String>,
    /// The package it resolves to.
    pub package: Option<String>,
}

/// What a file exports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportsRecord {
    /// The names it exports, `default` included.
    pub names: Vec<String>,
    /// Whether it has a default export.
    pub default: bool,
    /// Whether it has an `export *`.
    pub star: bool,
}

/// The record of a source file of the graph, with the facts in `reads`.
pub(crate) fn record(
    graph: &ImportGraph,
    path: &str,
    place: PluginPlace,
    reads: Reads,
) -> Option<FileRecord> {
    let file = graph.file(path)?;
    let resolutions = graph.resolutions(path).unwrap_or(&[]);
    let importers = || {
        graph
            .importers(path)
            .into_iter()
            .map(str::to_owned)
            .collect()
    };
    Some(record_of(
        Source { file, resolutions },
        importers,
        place,
        reads,
    ))
}

/// A source file and where its imports resolve.
#[derive(Clone, Copy)]
pub(crate) struct Source<'a> {
    pub(crate) file: &'a SourceFile,
    pub(crate) resolutions: &'a [Resolution],
}

/// The record of a source file, with its importers when the facts need them.
pub(crate) fn record_of(
    source: Source<'_>,
    importers: impl FnOnce() -> Vec<String>,
    place: PluginPlace,
    reads: Reads,
) -> FileRecord {
    let facts = source.file.facts.as_ref();
    let imports = reads.imports.then(|| import_records(source));
    let importers = reads.importers.then(|| {
        let mut importers = importers();
        importers.sort();
        importers.dedup();
        importers
    });
    let exports = reads.exports.then(|| {
        let names = facts
            .map(|facts| facts.exports.names.clone())
            .unwrap_or_default();
        ExportsRecord {
            default: names.iter().any(|name| name == "default"),
            star: facts.is_some_and(|facts| facts.exports.entries.iter().any(|e| e.star)),
            names,
        }
    });
    FileRecord {
        path: source.file.path.clone(),
        element: place.element,
        captures: place.captures,
        selectors: place.selectors,
        imports,
        importers,
        exports,
    }
}

/// A file's imports, each with its occurrence and where it resolves.
fn import_records(source: Source<'_>) -> Vec<ImportRecord> {
    let mut seen: BTreeMap<&str, u32> = BTreeMap::new();
    let imports = source
        .file
        .facts
        .as_ref()
        .map(|facts| facts.imports.as_slice())
        .unwrap_or_default();
    imports
        .iter()
        .enumerate()
        .map(|(index, import)| {
            let occurrence = seen.entry(import.specifier.as_str()).or_default();
            let resolution = source.resolutions.get(index);
            let record = ImportRecord {
                specifier: import.specifier.clone(),
                occurrence: *occurrence,
                line: import.line,
                column: import.column,
                kind: match import.kind {
                    ImportKind::Runtime => "runtime",
                    ImportKind::Type => "type",
                },
                names: import.names.clone(),
                file: match resolution {
                    Some(Resolution::File(target)) => Some(target.clone()),
                    _ => None,
                },
                package: match resolution {
                    Some(Resolution::Package(name)) => Some(name.clone()),
                    _ => None,
                },
            };
            *occurrence += 1;
            record
        })
        .collect()
}
