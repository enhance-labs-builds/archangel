//! Where each field of a JSON config starts, so a problem with a field can name its line and
//! column. The text has already parsed, so the scan trusts its syntax.

use std::collections::HashMap;

/// A container the scan is inside: an object with the key whose value comes next, or an array
/// with the index of its next item.
enum Frame {
    Object { path: String, key: Option<String> },
    Array { path: String, next: usize },
}

/// Where each field's key and each array item starts in a config's text, by the field's path as
/// the parser names it, such as `rules[0].kind`.
pub(crate) struct Positions<'t> {
    text: &'t str,
    /// Byte offsets into `text`.
    offsets: HashMap<String, usize>,
}

impl Positions<'_> {
    /// The line and column, from 1, of a field, or of the nearest enclosing field or item the
    /// text has, for a field that is missing.
    pub(crate) fn of(&self, field: &str) -> Option<(usize, usize)> {
        std::iter::successors(Some(field), |path| {
            path.rfind(['.', '[']).map(|end| &path[..end])
        })
        .find_map(|path| self.offsets.get(path))
        .map(|&offset| line_and_column(self.text, offset))
    }
}

fn line_and_column(text: &str, offset: usize) -> (usize, usize) {
    let before = &text[..offset];
    let line_start = before.rfind('\n').map_or(0, |newline| newline + 1);
    (
        before.matches('\n').count() + 1,
        before[line_start..].chars().count() + 1,
    )
}

pub(crate) fn positions(text: &str) -> Positions<'_> {
    let mut scan = Scan {
        text,
        chars: text.char_indices().peekable(),
        stack: Vec::new(),
        found: HashMap::new(),
    };
    while let Some((offset, c)) = scan.chars.next() {
        scan.token(offset, c);
    }
    Positions {
        text,
        offsets: scan.found,
    }
}

/// The scan's place in the text, the containers it is inside, and what it found.
struct Scan<'t> {
    text: &'t str,
    chars: std::iter::Peekable<std::str::CharIndices<'t>>,
    stack: Vec<Frame>,
    found: HashMap<String, usize>,
}

impl Scan<'_> {
    /// Handles the token that starts with `c`, at `offset`, which the scan has just read.
    fn token(&mut self, offset: usize, c: char) {
        if c.is_whitespace() || c == ':' || c == ',' {
            return;
        }
        if c == '"' {
            let end = self.string_end();
            if self.key(&self.text[offset..end], offset) {
                return;
            }
        }
        let path = value_path(&mut self.stack, &mut self.found, offset);
        match c {
            '{' => self.stack.push(Frame::Object { path, key: None }),
            '[' => self.stack.push(Frame::Array { path, next: 0 }),
            '}' | ']' => {
                self.stack.pop();
                value_done(&mut self.stack);
            }
            // A string value, already read to its closing quote, or a number, `true`, `false` or
            // `null`, read here to its end.
            _ => {
                while self.chars.peek().is_some_and(|(_, next)| {
                    !matches!(next, ',' | '}' | ']') && !next.is_whitespace()
                }) {
                    self.chars.next();
                }
                value_done(&mut self.stack);
            }
        }
    }

    /// Records a string as the next key, when an object waits for one; false for a value.
    fn key(&mut self, quoted: &str, offset: usize) -> bool {
        let Some(Frame::Object {
            path,
            key: key @ None,
        }) = self.stack.last_mut()
        else {
            return false;
        };
        let name: String = serde_json::from_str(quoted).unwrap_or_default();
        self.found.insert(join(path, &name), offset);
        *key = Some(name);
        true
    }

    /// Reads to the end of a string whose opening quote the scan has read; the byte offset after
    /// its closing quote.
    fn string_end(&mut self) -> usize {
        let mut escaped = false;
        for (at, inner) in self.chars.by_ref() {
            if escaped {
                escaped = false;
            } else if inner == '\\' {
                escaped = true;
            } else if inner == '"' {
                return at + 1;
            }
        }
        self.text.len()
    }
}

/// The path of the value that starts at `offset`, recording an array item's position.
fn value_path(stack: &mut [Frame], found: &mut HashMap<String, usize>, offset: usize) -> String {
    match stack.last_mut() {
        None => String::new(),
        Some(Frame::Object { path, key }) => join(path, key.as_deref().unwrap_or_default()),
        Some(Frame::Array { path, next }) => {
            let item = format!("{path}[{next}]");
            found.insert(item.clone(), offset);
            item
        }
    }
}

/// Moves the container on past a value that ended, unless the value was the container itself.
fn value_done(stack: &mut [Frame]) {
    match stack.last_mut() {
        Some(Frame::Object { key, .. }) => *key = None,
        Some(Frame::Array { next, .. }) => *next += 1,
        None => {}
    }
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_owned()
    } else {
        format!("{path}.{key}")
    }
}
