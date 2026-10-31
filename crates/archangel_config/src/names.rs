//! Name styles, such as `kebab-case`, and name patterns, such as `{folder-name}.test` or
//! `internal*`, which naming and export rules match names against.

use std::fmt;

/// A way of writing the words of a name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Style {
    /// `user-profile`.
    Kebab,
    /// `userProfile`.
    Camel,
    /// `UserProfile`.
    Pascal,
    /// `user_profile`.
    Snake,
    /// `USER_PROFILE`.
    ScreamingSnake,
}

impl Style {
    /// Every style, in the order the config's documentation lists them.
    pub const ALL: [Self; 5] = [
        Self::Kebab,
        Self::Camel,
        Self::Pascal,
        Self::Snake,
        Self::ScreamingSnake,
    ];

    /// The style as the config writes it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Kebab => "kebab-case",
            Self::Camel => "camelCase",
            Self::Pascal => "PascalCase",
            Self::Snake => "snake_case",
            Self::ScreamingSnake => "SCREAMING_SNAKE_CASE",
        }
    }

    /// The style a config names.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|style| style.as_str() == name)
    }

    /// How `{FileName}` is spelt in this style: the spellings of the placeholder are the name
    /// `file name` written in each style.
    fn spelling(self, words: [&str; 2]) -> String {
        let [first, second] = words;
        let capital = |word: &str| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |c| {
                c.to_uppercase().chain(chars).collect::<String>()
            })
        };
        match self {
            Self::Kebab => format!("{first}-{second}"),
            Self::Camel => format!("{first}{}", capital(second)),
            Self::Pascal => format!("{}{}", capital(first), capital(second)),
            Self::Snake => format!("{first}_{second}"),
            Self::ScreamingSnake => {
                format!("{}_{}", first.to_uppercase(), second.to_uppercase())
            }
        }
    }
}

impl fmt::Display for Style {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Whose name a placeholder stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    /// The file's own name, up to its first dot.
    File,
    /// The name of the folder that holds the file.
    Folder,
}

/// One part of a name pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamePart {
    /// Text that must appear as written.
    Text(String),
    /// `*`: any text, empty included.
    Any,
    /// `{FileName}` and its other spellings: a name, written in a style.
    Name(Owner, Style),
}

/// A pattern for one name: text, `*`, and the ten placeholders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamePattern {
    source: String,
    parts: Vec<NamePart>,
}

/// Every placeholder a name pattern accepts, as the config writes it.
#[must_use]
pub fn placeholders() -> Vec<String> {
    [
        (Owner::File, ["file", "name"]),
        (Owner::Folder, ["folder", "name"]),
    ]
    .into_iter()
    .flat_map(|(_, words)| Style::ALL.map(|style| format!("{{{}}}", style.spelling(words))))
    .collect()
}

impl NamePattern {
    /// Reads a name pattern, or gives the placeholder it does not know.
    ///
    /// # Errors
    ///
    /// Returns a sentence that names the unknown placeholder or the unclosed brace.
    pub fn parse(source: &str) -> Result<Self, String> {
        let mut parts = Vec::new();
        let mut text = String::new();
        let mut rest = source;
        while let Some(c) = rest.chars().next() {
            match c {
                '*' => {
                    flush(&mut text, &mut parts);
                    parts.push(NamePart::Any);
                    rest = &rest[1..];
                }
                '{' => {
                    let end = rest.find('}').ok_or_else(|| {
                        format!("the name pattern `{source}` opens a brace it does not close")
                    })?;
                    let inner = &rest[1..end];
                    let part = placeholder(inner).ok_or_else(|| {
                        format!(
                            "the name pattern `{source}` has `{{{inner}}}`, which is not a \
                             placeholder; the placeholders are {}",
                            placeholders().join(", ")
                        )
                    })?;
                    flush(&mut text, &mut parts);
                    parts.push(part);
                    rest = &rest[end + 1..];
                }
                other => {
                    text.push(other);
                    rest = &rest[other.len_utf8()..];
                }
            }
        }
        flush(&mut text, &mut parts);
        Ok(Self {
            source: source.to_owned(),
            parts,
        })
    }

    /// The pattern as the config wrote it.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The parts, in order.
    #[must_use]
    pub fn parts(&self) -> &[NamePart] {
        &self.parts
    }

    /// Whether the pattern names the file's own name, which a folder has no use for.
    #[must_use]
    pub fn uses(&self, owner: Owner) -> bool {
        self.parts
            .iter()
            .any(|part| matches!(part, NamePart::Name(o, _) if *o == owner))
    }
}

fn flush(text: &mut String, parts: &mut Vec<NamePart>) {
    if !text.is_empty() {
        parts.push(NamePart::Text(std::mem::take(text)));
    }
}

fn placeholder(inner: &str) -> Option<NamePart> {
    [
        (Owner::File, ["file", "name"]),
        (Owner::Folder, ["folder", "name"]),
    ]
    .into_iter()
    .find_map(|(owner, words)| {
        Style::ALL
            .into_iter()
            .find(|style| style.spelling(words) == inner)
            .map(|style| NamePart::Name(owner, style))
    })
}
