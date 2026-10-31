//! GitHub Actions workflow commands: one `::error` line per diagnostic, which GitHub shows on the
//! line of the pull request's diff.

use std::fmt::Write as _;
use std::path::Path;

use archangel_rules::Diagnostic;

/// One annotation per diagnostic.
pub(crate) fn annotations(dir: &Path, diagnostics: &[Diagnostic]) -> String {
    let mut out = String::new();
    for diagnostic in diagnostics {
        out.push_str(&annotation(dir, diagnostic));
    }
    out
}

fn annotation(dir: &Path, diagnostic: &Diagnostic) -> String {
    let mut properties = Vec::new();
    if let Some(file) = diagnostic.file.as_deref() {
        properties.push(format!("file={}", property(file)));
    }
    if diagnostic.line > 0 {
        properties.push(format!("line={}", diagnostic.line));
        properties.push(format!("col={}", diagnostic.column));
        if let Some(end) = super::end_column(dir, diagnostic) {
            properties.push(format!("endColumn={end}"));
        }
    }
    let title = diagnostic
        .rule
        .as_deref()
        .unwrap_or(diagnostic.code.as_str());
    properties.push(format!("title=archangel {}", property(title)));
    let mut message = diagnostic.message.clone();
    for (label, text) in [
        ("help", &diagnostic.help),
        ("reason", &diagnostic.reason),
        ("see", &diagnostic.link),
    ] {
        if let Some(text) = text {
            let _ = write!(message, "\n{label}: {text}");
        }
    }
    format!("::error {}::{}\n", properties.join(","), data(&message))
}

/// The escaping GitHub's runner undoes in a command's message.
fn data(text: &str) -> String {
    text.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

/// The escaping GitHub's runner undoes in a command's property, where `,` and `:` end the value.
fn property(text: &str) -> String {
    data(text).replace(':', "%3A").replace(',', "%2C")
}
