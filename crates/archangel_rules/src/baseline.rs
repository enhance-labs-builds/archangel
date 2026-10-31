//! The baseline: violations that existed when a team turned a rule on, recorded so that a check
//! fails only for new ones. This module holds the data and its matching; reading and writing the
//! file is the engine's.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::diagnostic::{Diagnostic, ViolationId};

/// The recorded violations.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Baseline {
    /// The entries, sorted.
    pub entries: Vec<BaselineEntry>,
}

/// One recorded violation, or several of the same import, with a count.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineEntry {
    /// The rule's name.
    pub rule: String,
    /// The importing file, for a violation of an import.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// The specifier, for a violation of an import.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub specifier: Option<String>,
    /// Every file of a cycle group, sorted, for a no-cycles violation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
    /// The file, folder or instance root, for a violation about a path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// How many violations the entry permits.
    pub count: u32,
}

/// A baseline file that does not parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaselineError {
    /// The line, from 1.
    pub line: usize,
    /// The column, from 1.
    pub column: usize,
    /// What is wrong.
    pub message: String,
}

/// The part of a violation's identity that a baseline entry records: no line, no occurrence.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Key {
    rule: String,
    file: Option<String>,
    specifier: Option<String>,
    files: Vec<String>,
    path: Option<String>,
}

impl ViolationId {
    pub(crate) fn baseline_key(&self) -> Key {
        match self {
            Self::Import {
                rule,
                file,
                specifier,
                ..
            } => Key {
                rule: rule.clone(),
                file: Some(file.clone()),
                specifier: Some(specifier.clone()),
                files: Vec::new(),
                path: None,
            },
            Self::Group { rule, files } => Key {
                rule: rule.clone(),
                file: None,
                specifier: None,
                files: files.clone(),
                path: None,
            },
            Self::Path { rule, path } => Key {
                rule: rule.clone(),
                file: None,
                specifier: None,
                files: Vec::new(),
                path: Some(path.clone()),
            },
        }
    }
}

impl BaselineEntry {
    pub(crate) fn key(&self) -> Key {
        Key {
            rule: self.rule.clone(),
            file: self.file.clone(),
            specifier: self.specifier.clone(),
            files: self.files.clone(),
            path: self.path.clone(),
        }
    }
}

impl Baseline {
    /// Reads a baseline from its JSON text.
    ///
    /// # Errors
    ///
    /// Returns the line, column and reason when the text does not parse.
    pub fn parse(text: &str) -> Result<Self, BaselineError> {
        serde_json::from_str(text).map_err(|error| BaselineError {
            line: error.line(),
            column: error.column(),
            message: error.to_string(),
        })
    }

    /// A baseline that records every violation in diagnostics.
    #[must_use]
    pub fn from_diagnostics(diagnostics: &[Diagnostic]) -> Self {
        let mut counts: BTreeMap<Key, u32> = BTreeMap::new();
        for id in diagnostics.iter().filter_map(|d| d.id.as_ref()) {
            *counts.entry(id.baseline_key()).or_default() += 1;
        }
        let entries = counts
            .into_iter()
            .map(|(key, count)| BaselineEntry {
                rule: key.rule,
                file: key.file,
                specifier: key.specifier,
                files: key.files,
                path: key.path,
                count,
            })
            .collect();
        Self { entries }
    }

    /// A baseline that records every violation of a report.
    #[must_use]
    pub fn from_report(report: &crate::Report) -> Self {
        Self::from_diagnostics(&report.diagnostics)
    }

    /// This baseline with each entry lowered to the violations it matches in `current`, and the
    /// entries that match none removed. It never adds an entry or raises a count.
    #[must_use]
    pub fn pruned(&self, current: &Self) -> Self {
        let now: BTreeMap<Key, u32> = current
            .entries
            .iter()
            .map(|entry| (entry.key(), entry.count))
            .collect();
        let entries = self
            .entries
            .iter()
            .filter_map(|entry| {
                let count = entry.count.min(now.get(&entry.key()).copied().unwrap_or(0));
                (count > 0).then(|| BaselineEntry {
                    count,
                    ..entry.clone()
                })
            })
            .collect();
        Self { entries }
    }

    /// The baseline as sorted, formatted JSON, so two runs on the same code write the same bytes.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut sorted = self.clone();
        sorted.entries.sort();
        let mut text = serde_json::to_string_pretty(&sorted).unwrap_or_default();
        text.push('\n');
        text
    }
}
