//! Output: text for people, JSON for programs, and the formats CI reads. Each takes diagnostics as
//! data and decides nothing.

pub(crate) mod github;
pub(crate) mod json;
pub(crate) mod sarif;
pub(crate) mod text;

use std::path::Path;

use archangel_rules::Diagnostic;

use crate::Color;

/// Whether to print color. `auto` colors only a terminal, and only when `NO_COLOR` is unset or
/// empty.
pub(crate) fn color(choice: Color) -> bool {
    use std::io::IsTerminal;
    match choice {
        Color::Always => true,
        Color::Never => false,
        Color::Auto => {
            let no_color = std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
            std::io::stdout().is_terminal() && !no_color
        }
    }
}

/// The column, from 1, just past a diagnostic's specifier, when its file can be read from `dir`.
pub(crate) fn end_column(dir: &Path, diagnostic: &Diagnostic) -> Option<u32> {
    let (start, end) = diagnostic.span?;
    let text = std::fs::read_to_string(dir.join(diagnostic.file.as_ref()?)).ok()?;
    let marked = text.get(start as usize..end as usize)?;
    u32::try_from(marked.chars().count())
        .ok()
        .map(|width| diagnostic.column + width)
}
