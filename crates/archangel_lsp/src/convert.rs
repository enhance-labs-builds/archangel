//! Conversions between archangel's values and the protocol's: URIs and paths, diagnostics and
//! their ranges, fixes and code actions.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use archangel_engine::{PathEvent, Session};
use archangel_rules::Diagnostic;
use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, CodeDescription,
    DiagnosticSeverity, FileChangeType, FileEvent, NumberOrString, Position, Range, TextEdit, Uri,
    WorkspaceEdit,
};
use serde_json::Value;

/// The workspace folder the editor opened: its first workspace folder, or for an editor that
/// sends none, its root URI.
pub(crate) fn root(params: &Value) -> Option<PathBuf> {
    let folder = params["workspaceFolders"][0]["uri"].as_str();
    uri_path(folder.or_else(|| params["rootUri"].as_str())?)
}

/// The path of a `file:` URI. The `url` crate reads drive letters, UNC hosts and percent
/// escapes as the platform writes them, which a hand-written decoder gets wrong on Windows.
pub(crate) fn uri_path(uri: &str) -> Option<PathBuf> {
    url::Url::parse(uri).ok()?.to_file_path().ok()
}

/// The `file:` URI of an absolute path.
pub(crate) fn uri(path: &Path) -> Option<Uri> {
    Uri::from_str(url::Url::from_file_path(path).ok()?.as_str()).ok()
}

/// The path relative to the config folder, with `/` between folders.
pub(crate) fn workspace_relative(root: &Path, path: &Path) -> Option<String> {
    let real = path
        .parent()
        .and_then(|folder| dunce::canonicalize(folder).ok())
        .and_then(|folder| path.file_name().map(|name| folder.join(name)))
        .unwrap_or_else(|| path.to_path_buf());
    let inside = real.strip_prefix(root).ok()?;
    Some(inside.to_string_lossy().replace('\\', "/"))
}

/// A file event from the editor as the engine's event.
pub(crate) fn file_event(event: &FileEvent) -> Option<PathEvent> {
    let path = uri_path(event.uri.as_str())?;
    Some(match event.typ {
        FileChangeType::CREATED => PathEvent::Created(path),
        FileChangeType::DELETED => PathEvent::Deleted(path),
        _ => PathEvent::Changed(path),
    })
}

/// The config file's path relative to the config folder.
pub(crate) fn config_name(session: &Session) -> String {
    session
        .config_path()
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The file a diagnostic is shown on: its own, or the config for one about no file.
pub(crate) fn file_of(session: &Session, diagnostic: &Diagnostic) -> String {
    diagnostic
        .file
        .clone()
        .unwrap_or_else(|| config_name(session))
}

/// The protocol's diagnostics for one file.
pub(crate) fn diagnostics_of(session: &Session, file: &str) -> Vec<lsp_types::Diagnostic> {
    session
        .report()
        .diagnostics
        .iter()
        .filter(|d| file_of(session, d) == file)
        .map(diagnostic)
        .collect()
}

/// A diagnostic's range, in the UTF-16 code units the protocol counts in: its specifier, or the
/// start of its line when it has none.
fn range(diagnostic: &Diagnostic) -> Range {
    let line = diagnostic.line.saturating_sub(1);
    let (start, end) = diagnostic.utf16.unwrap_or((0, 0));
    Range::new(Position::new(line, start), Position::new(line, end))
}

fn diagnostic(diagnostic: &Diagnostic) -> lsp_types::Diagnostic {
    let message = match &diagnostic.help {
        Some(help) => format!("{}\n{help}", diagnostic.message),
        None => diagnostic.message.clone(),
    };
    lsp_types::Diagnostic {
        range: range(diagnostic),
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String(diagnostic.code.as_str().to_owned())),
        code_description: diagnostic
            .link
            .as_ref()
            .and_then(|link| Uri::from_str(link).ok())
            .map(|href| CodeDescription { href }),
        source: Some("archangel".to_owned()),
        message,
        ..lsp_types::Diagnostic::default()
    }
}

/// A quick fix for each diagnostic with a fix in the requested range.
pub(crate) fn code_actions(session: &Session, file: &str, params: &CodeActionParams) -> Value {
    let Some(uri) = uri(&session.graph().root().join(file)) else {
        return Value::Array(Vec::new());
    };
    let actions: Vec<CodeActionOrCommand> = session
        .report()
        .diagnostics
        .iter()
        .filter(|d| d.file.as_deref() == Some(file))
        .filter(|d| overlaps(&range(d), &params.range))
        .filter_map(|d| {
            let fix = d.fix.as_ref()?;
            // The fix replaces the text between the quotes, one code unit in from each end.
            let quoted = range(d);
            let edit = TextEdit::new(
                Range::new(
                    Position::new(quoted.start.line, quoted.start.character + 1),
                    Position::new(quoted.end.line, quoted.end.character.saturating_sub(1)),
                ),
                fix.specifier.clone(),
            );
            Some(CodeActionOrCommand::CodeAction(CodeAction {
                title: format!("Import through \"{}\"", fix.specifier),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diagnostic(d)]),
                edit: Some(WorkspaceEdit {
                    changes: Some(HashMap::from([(uri.clone(), vec![edit])])),
                    ..WorkspaceEdit::default()
                }),
                is_preferred: Some(true),
                ..CodeAction::default()
            }))
        })
        .collect();
    serde_json::to_value(actions).unwrap_or(Value::Array(Vec::new()))
}

fn overlaps(a: &Range, b: &Range) -> bool {
    a.start <= b.end && b.start <= a.end
}

#[cfg(test)]
mod tests;
