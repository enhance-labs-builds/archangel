//! Exception comments: `// archangel-allow <rule name>: <reason>` above an import.

use archangel_graph::{CommentText, Import};

/// One exception, read from a comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exception {
    /// The rule it names.
    pub rule: String,
    /// Why, or empty when the comment gives none.
    pub reason: String,
    /// The line of the comment.
    pub line: u32,
    /// The line of the import it applies to.
    pub target_line: u32,
}

const MARKER: &str = "archangel-allow";

/// The exceptions in a file's `archangel-allow` comments. A run of comments on consecutive lines
/// applies to the import on the line after the run: the import whose specifier is on that line, or
/// whose statement starts there.
#[must_use]
pub fn read(comments: &[CommentText], imports: &[Import]) -> Vec<Exception> {
    let mut found: Vec<Exception> = Vec::new();
    let mut run_start = 0;
    for (index, comment) in comments.iter().enumerate() {
        let continues = index > 0 && comments[index - 1].end_line + 1 == comment.line;
        if !continues {
            run_start = found.len();
        }
        for exception in &mut found[run_start..] {
            exception.target_line = comment.end_line + 1;
        }
        if let Some(exception) = parse(comment) {
            found.push(exception);
        }
    }
    for exception in &mut found {
        if let Some(import) = imports
            .iter()
            .find(|import| import.statement_line == exception.target_line)
        {
            exception.target_line = import.line;
        }
    }
    found
}

fn parse(comment: &CommentText) -> Option<Exception> {
    let start = comment.text.find(MARKER)? + MARKER.len();
    let rest = comment.text[start..].trim();
    let (rule, reason) = rest.split_once(':').unwrap_or((rest, ""));
    Some(Exception {
        rule: rule.trim().to_owned(),
        reason: reason.trim().to_owned(),
        line: comment.line,
        target_line: comment.end_line + 1,
    })
}
