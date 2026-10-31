//! Names: a file's name and full name, the five styles, and name patterns with `{FileName}` and
//! `{FolderName}` in each style.

use archangel_config::{NamePart, NamePattern, Owner, Style};

/// The names of one file or folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileName<'p> {
    /// The last part of the path: `button.view.tsx`.
    pub base: &'p str,
    /// The name up to the first dot: `button`. A leading dot, as in `.eslintrc.js`, is part of
    /// no name, so that file's name is `eslintrc`.
    pub name: &'p str,
    /// The name without the last extension: `button.view`.
    pub full: &'p str,
    /// The last part of the folder that holds it, or empty at the top.
    pub folder: &'p str,
}

impl<'p> FileName<'p> {
    /// The names of the file or folder at a path relative to the config folder.
    #[must_use]
    pub fn of(path: &'p str) -> Self {
        let (parent, base) = path.rsplit_once('/').unwrap_or(("", path));
        let folder = parent.rsplit_once('/').map_or(parent, |(_, last)| last);
        let trimmed = base.trim_start_matches('.');
        let name = trimmed.split_once('.').map_or(trimmed, |(name, _)| name);
        let full = trimmed.rsplit_once('.').map_or(trimmed, |(full, _)| full);
        Self {
            base,
            name,
            full,
            folder,
        }
    }

    /// The file's base name with its name, up to the first dot, written in a style.
    #[must_use]
    pub fn renamed(&self, style: Style) -> String {
        let at = self.base.find(self.name).unwrap_or(0);
        format!(
            "{}{}{}",
            &self.base[..at],
            convert(self.name, style),
            &self.base[at + self.name.len()..]
        )
    }
}

/// A name written in a style, with the words a reference conversion finds: a boundary before an
/// upper-case letter that follows a lower-case one, before the last of a run of upper-case
/// letters that a lower-case one follows, as in `HTTPServer`, and at every character that is
/// neither a letter nor a digit. Digits take the case of what comes before them.
#[must_use]
pub fn convert(name: &str, style: Style) -> String {
    let words = words(name);
    let mut out = String::with_capacity(name.len() + words.len());
    for (index, word) in words.iter().enumerate() {
        if index > 0 {
            match style {
                Style::Kebab => out.push('-'),
                Style::Snake | Style::ScreamingSnake => out.push('_'),
                Style::Camel | Style::Pascal => {}
            }
        }
        match style {
            Style::Kebab | Style::Snake => out.push_str(&lower(word)),
            Style::ScreamingSnake => out.extend(word.chars().flat_map(char::to_uppercase)),
            Style::Camel if index == 0 => out.push_str(&lower(word)),
            Style::Camel | Style::Pascal => out.push_str(&capitalized(word)),
        }
    }
    out
}

/// Whether a name is written in a style: whether writing it in the style changes nothing.
#[must_use]
pub fn is_named(name: &str, style: Style) -> bool {
    convert(name, style) == name
}

/// The words of a name.
fn words(name: &str) -> Vec<&str> {
    #[derive(Clone, Copy, PartialEq)]
    enum Case {
        None,
        Lower,
        Upper,
    }
    let mut found = Vec::new();
    for word in name.split(|c: char| !c.is_alphanumeric()) {
        let mut chars = word.char_indices().peekable();
        let mut start = 0;
        let mut case = Case::None;
        while let Some((at, c)) = chars.next() {
            let Some(&(next_at, next)) = chars.peek() else {
                found.push(&word[start..]);
                break;
            };
            let next_case = if c.is_lowercase() {
                Case::Lower
            } else if c.is_uppercase() {
                Case::Upper
            } else {
                case
            };
            if next_case == Case::Lower && next.is_uppercase() {
                found.push(&word[start..next_at]);
                start = next_at;
                case = Case::None;
            } else if case == Case::Upper && c.is_uppercase() && next.is_lowercase() {
                found.push(&word[start..at]);
                start = at;
                case = Case::None;
            } else {
                case = next_case;
            }
        }
    }
    found
}

/// A word in lower case, with a final capital sigma written as the final form.
fn lower(word: &str) -> String {
    let mut out = String::with_capacity(word.len());
    let mut chars = word.chars().peekable();
    while let Some(c) = chars.next() {
        if c == 'Σ' && chars.peek().is_none() {
            out.push('ς');
        } else {
            out.extend(c.to_lowercase());
        }
    }
    out
}

fn capitalized(word: &str) -> String {
    let mut chars = word.char_indices();
    chars.next().map_or_else(String::new, |(_, first)| {
        let rest = chars.next().map_or("", |(at, _)| &word[at..]);
        first.to_uppercase().collect::<String>() + &lower(rest)
    })
}

/// A name pattern with its placeholders written out for one file: what the pattern asks for,
/// with `*` where any text may stand.
#[must_use]
pub(crate) fn expand(pattern: &NamePattern, names: &FileName<'_>) -> Vec<Piece> {
    pattern
        .parts()
        .iter()
        .map(|part| match part {
            NamePart::Text(text) => Piece::Text(text.clone()),
            NamePart::Any => Piece::Any,
            NamePart::Name(Owner::File, style) => Piece::Text(convert(names.name, *style)),
            NamePart::Name(Owner::Folder, style) => Piece::Text(convert(names.folder, *style)),
        })
        .collect()
}

/// One piece of an expanded name pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Piece {
    Text(String),
    Any,
}

/// The expanded pattern as text, with `*` for each piece that takes any text.
pub(crate) fn spelled(pieces: &[Piece]) -> String {
    pieces
        .iter()
        .map(|piece| match piece {
            Piece::Text(text) => text.as_str(),
            Piece::Any => "*",
        })
        .collect()
}

/// Whether a name matches an expanded pattern, each `*` taking any text, empty included.
pub(crate) fn matches(pieces: &[Piece], name: &str) -> bool {
    // @justified intent
    // The text before the first `*` must start the name and the text after the last must end it.
    // Each text between them is taken at its first place after the one before: that leaves the
    // most room for the rest, so no later place can succeed where the first fails, and the match
    // needs no backtracking.
    let mut texts: Vec<String> = vec![String::new()];
    for piece in pieces {
        match piece {
            Piece::Text(text) => {
                if let Some(last) = texts.last_mut() {
                    last.push_str(text);
                }
            }
            Piece::Any => texts.push(String::new()),
        }
    }
    let (Some(first), Some(last)) = (texts.first(), texts.last()) else {
        return name.is_empty();
    };
    if texts.len() == 1 {
        return name == first;
    }
    if name.len() < first.len() + last.len()
        || !name.starts_with(first.as_str())
        || !name.ends_with(last.as_str())
    {
        return false;
    }
    let mut rest = &name[first.len()..name.len() - last.len()];
    for middle in &texts[1..texts.len() - 1] {
        match rest.find(middle.as_str()) {
            Some(at) => rest = &rest[at + middle.len()..],
            None => return false,
        }
    }
    true
}
