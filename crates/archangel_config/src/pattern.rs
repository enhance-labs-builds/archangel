//! Element patterns: `src/features/{feature}/**`, `src/**/*.model.ts`, `src/{ui,widgets}/**`.
//!
//! A pattern is a list of `/`-separated parts. `**` matches any number of folders, `*` matches
//! within one name, braces with a comma list alternatives, and braces without one capture a part
//! of one name that is not empty. Each pattern becomes one regular expression with a named group
//! per capture, the way globset turns globs into regular expressions.

use std::fmt::{self, Write as _};

use regex::Regex;

/// A compiled pattern.
#[derive(Debug, Clone)]
pub struct Pattern {
    source: String,
    regex: Regex,
    captures: Vec<String>,
    root_parts: usize,
}

/// What a pattern found in a path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternMatch {
    /// The capture values, in the order the captures appear in the pattern.
    pub captures: Vec<String>,
    /// The matched path up to the first part with a wildcard, with captures filled in.
    pub root: String,
}

/// Why a pattern cannot mean what it says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternError {
    /// `a//b`, or a pattern that starts or ends with `/`.
    EmptyName,
    /// `**` that is not a whole folder name, such as `ui**`.
    DoubleStarInName,
    /// `{}`.
    EmptyCapture,
    /// A capture name that is not letters, digits and underscores, or starts with a digit.
    CaptureName(String),
    /// `{a}/{a}`.
    RepeatedCapture(String),
    /// `{a,}`: an alternative with nothing in it.
    EmptyAlternative,
    /// `{a` or `a}`, or braces inside braces.
    UnbalancedBraces,
}

impl fmt::Display for PatternError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => f.write_str("it has an empty folder name"),
            Self::DoubleStarInName => {
                f.write_str("`**` must be a whole folder name, as in `src/**/x.ts`")
            }
            Self::EmptyCapture => f.write_str("`{}` captures nothing; give the capture a name"),
            Self::CaptureName(name) => write!(
                f,
                "the capture `{{{name}}}` needs a name of letters, digits and underscores that \
                 does not start with a digit"
            ),
            Self::RepeatedCapture(name) => write!(f, "it captures `{name}` two times"),
            Self::EmptyAlternative => f.write_str("one of the alternatives in braces is empty"),
            Self::UnbalancedBraces => f.write_str("its braces do not pair up"),
        }
    }
}

enum Token {
    Literal(String),
    Star,
    Capture(String),
    Alternatives(Vec<String>),
}

impl Pattern {
    /// Compiles a pattern, or says why it cannot mean what it says.
    ///
    /// # Errors
    ///
    /// Returns the first problem in the pattern.
    pub fn parse(source: &str) -> Result<Self, PatternError> {
        let parts: Vec<&str> = source.split('/').collect();
        if parts.iter().any(|part| part.is_empty()) {
            return Err(PatternError::EmptyName);
        }
        let mut captures = Vec::new();
        let mut expression = String::from("^");
        let mut root_parts = None;
        for (index, part) in parts.iter().enumerate() {
            let last = index + 1 == parts.len();
            if *part == "**" {
                root_parts.get_or_insert(index);
                expression.push_str(if last {
                    "(?:[^/]+/)*[^/]+"
                } else {
                    "(?:[^/]+/)*"
                });
                continue;
            }
            let tokens = tokenize(part)?;
            if tokens.iter().any(|token| matches!(token, Token::Star)) {
                root_parts.get_or_insert(index);
            }
            for token in tokens {
                match token {
                    Token::Literal(text) => expression.push_str(&regex::escape(&text)),
                    Token::Star => expression.push_str("[^/]*"),
                    Token::Capture(name) => {
                        if captures.contains(&name) {
                            return Err(PatternError::RepeatedCapture(name));
                        }
                        let _ = write!(expression, "(?P<{name}>[^/]+)");
                        captures.push(name);
                    }
                    Token::Alternatives(options) => {
                        let options: Vec<String> =
                            options.iter().map(|option| glob_text(option)).collect();
                        let _ = write!(expression, "(?:{})", options.join("|"));
                    }
                }
            }
            if !last {
                expression.push('/');
            }
        }
        expression.push('$');
        let regex = Regex::new(&expression).map_err(|_| PatternError::UnbalancedBraces)?;
        Ok(Self {
            source: source.to_owned(),
            regex,
            captures,
            root_parts: root_parts.unwrap_or(parts.len()),
        })
    }

    /// The pattern as the developer wrote it.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Whether the pattern names one exact path: no wildcard, capture or alternative.
    #[must_use]
    pub fn is_exact(&self) -> bool {
        !self.source.contains(['*', '?', '{'])
    }

    /// The capture names, in the order they appear.
    #[must_use]
    pub fn captures(&self) -> &[String] {
        &self.captures
    }

    /// The regular expression that the pattern compiles to.
    #[must_use]
    pub fn regex(&self) -> &Regex {
        &self.regex
    }

    /// Matches a path relative to the config folder, with `/` or `\` between folders.
    #[must_use]
    pub fn matches(&self, path: &str) -> Option<PatternMatch> {
        let path = normalize_path(path);
        let found = self.regex.captures(&path)?;
        let captures = self
            .captures
            .iter()
            .map(|name| {
                found
                    .name(name)
                    .map_or_else(String::new, |m| m.as_str().to_owned())
            })
            .collect();
        let root = path
            .split('/')
            .take(self.root_parts)
            .collect::<Vec<_>>()
            .join("/");
        Some(PatternMatch { captures, root })
    }
}

/// Turns `\` into `/`, so paths from Windows match the same patterns.
#[must_use]
pub fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn tokenize(part: &str) -> Result<Vec<Token>, PatternError> {
    let mut tokens = Vec::new();
    let mut literal = String::new();
    let mut chars = part.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' => {
                if chars.peek() == Some(&'*') {
                    return Err(PatternError::DoubleStarInName);
                }
                flush(&mut literal, &mut tokens);
                tokens.push(Token::Star);
            }
            '{' => {
                flush(&mut literal, &mut tokens);
                let mut inside = String::new();
                let mut closed = false;
                for next in chars.by_ref() {
                    match next {
                        '}' => {
                            closed = true;
                            break;
                        }
                        '{' => return Err(PatternError::UnbalancedBraces),
                        other => inside.push(other),
                    }
                }
                if !closed {
                    return Err(PatternError::UnbalancedBraces);
                }
                tokens.push(brace_token(&inside)?);
            }
            '}' => return Err(PatternError::UnbalancedBraces),
            other => literal.push(other),
        }
    }
    flush(&mut literal, &mut tokens);
    Ok(tokens)
}

fn brace_token(inside: &str) -> Result<Token, PatternError> {
    if inside.contains(',') {
        let options: Vec<String> = inside.split(',').map(str::to_owned).collect();
        if options.iter().any(String::is_empty) {
            return Err(PatternError::EmptyAlternative);
        }
        return Ok(Token::Alternatives(options));
    }
    if inside.is_empty() {
        return Err(PatternError::EmptyCapture);
    }
    let valid = inside
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !inside.starts_with(|c: char| c.is_ascii_digit());
    if !valid {
        return Err(PatternError::CaptureName(inside.to_owned()));
    }
    Ok(Token::Capture(inside.to_owned()))
}

fn flush(literal: &mut String, tokens: &mut Vec<Token>) {
    if !literal.is_empty() {
        tokens.push(Token::Literal(std::mem::take(literal)));
    }
}

/// An alternative may hold `*`; everything else in it is literal.
fn glob_text(text: &str) -> String {
    text.split('*')
        .map(regex::escape)
        .collect::<Vec<_>>()
        .join("[^/]*")
}
