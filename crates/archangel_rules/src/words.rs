//! Rules, sentences and sets of files in words, for `archangel explain` and for diagnostics.

use std::fmt::Write as _;

use archangel_config::{
    Condition, DependencyKind, ExportKind, FileSet, ImportOptions, InstanceMatch, NameRule,
    Pattern, Rule, RuleKind, SpecifierForm, Term,
};

use crate::architecture::Architecture;
use crate::messages::row_permits;

/// The area of a table without `within`, which a sentence does not need to say.
pub(crate) const DEFAULT_AREA: &str = "the repository";

impl Architecture {
    /// A sentence of a rule, in words.
    #[must_use]
    pub fn sentence_text(&self, rule: usize, sentence: usize) -> String {
        self.sentence_of_rule(&self.config.rules[rule], sentence)
    }

    /// A sentence of a rule, in words, for a rule that `off` removed too.
    #[must_use]
    pub fn sentence_of_rule(&self, rule: &Rule, sentence: usize) -> String {
        if matches!(rule.kind, RuleKind::TableRow { .. }) {
            return self.row_text(rule, sentence);
        }
        let sentence = &rule.sentences[sentence];
        let mut text = if sentence.folders.is_some() {
            folder_text(&sentence.files_text, &sentence.condition)
        } else {
            condition_text(&sentence.files_text, &sentence.condition)
        };
        self.options_text(&sentence.options, &mut text);
        text
    }

    /// A sentence of a row: what it may import. For a table's own rule: that each file has a row.
    fn row_text(&self, rule: &Rule, sentence: usize) -> String {
        let RuleKind::TableRow { table, row } = &rule.kind else {
            return String::new();
        };
        let Some(row) = row else {
            let table = &self.config.tables[table.index()].name;
            return format!("every file of {table} has a row");
        };
        let compiled = &rule.sentences[sentence];
        if matches!(compiled.condition, Condition::MayNotImport(_)) {
            return format!("{row} may not import itself");
        }
        let permits = row_permits(rule);
        let mut text = if permits == "nothing" {
            format!("{row} may import nothing")
        } else {
            format!("{row} may import only {permits}")
        };
        let mut options = compiled.options.clone();
        options.within = options.within.filter(|area| area.text != DEFAULT_AREA);
        self.options_text(&options, &mut text);
        text
    }

    fn options_text(&self, options: &ImportOptions, text: &mut String) {
        limits_text(options, text);
        if let Some(area) = &options.within {
            let _ = write!(text, " within {}", area.text);
        }
        if options.transitively {
            text.push_str(", directly or through other files");
        }
        if let Some(stop) = &options.stop_at {
            let _ = write!(text, ", stopping at {}", self.describe(stop));
        }
    }

    /// The patterns, elements and selectors of a set of files, as the config names them: its
    /// postfix terms, read with a stack of phrases.
    pub(crate) fn describe(&self, set: &FileSet) -> String {
        let mut phrases: Vec<String> = Vec::with_capacity(set.terms.len());
        for term in &set.terms {
            let phrase = match term {
                Term::All(count) => {
                    let from = phrases.len().saturating_sub(*count);
                    format!("all of ({})", phrases.split_off(from).join(", "))
                }
                Term::Any(count) => {
                    let from = phrases.len().saturating_sub(*count);
                    phrases.split_off(from).join(" | ")
                }
                Term::Not => format!("not {}", phrases.pop().unwrap_or_default()),
                leaf => self.leaf(leaf),
            };
            phrases.push(phrase);
        }
        phrases.pop().unwrap_or_default()
    }

    fn leaf(&self, term: &Term) -> String {
        match term {
            Term::Pattern(id) => self.config.patterns[id.0 as usize].source().to_owned(),
            Term::Element(id) => self.config.elements[id.index()].name.clone(),
            Term::Instance { element, captures } => {
                let values: Vec<String> =
                    captures.iter().map(|(k, v)| format!("{k} {v}")).collect();
                format!(
                    "{} {}",
                    self.config.elements[element.index()].name,
                    values.join(", ")
                )
            }
            Term::AnyElement => "any element".to_owned(),
            Term::Directive(text) => format!("'{text}' files"),
            Term::Selector(id) => self.config.selectors[id.0 as usize].name.clone(),
            Term::Kind { kind, .. } => kind.clone(),
            Term::AnyKind(table) => {
                format!(
                    "the files with a kind in {}",
                    self.config.tables[table.index()].name
                )
            }
            Term::All(_) | Term::Any(_) | Term::Not => String::new(),
        }
    }

    /// What a set that must match a file is: a declared selector's definition, or a pattern.
    pub(crate) fn definition(&self, set: usize) -> String {
        match self.sets[set].terms.as_slice() {
            [Term::Selector(id)] => self.describe(&self.config.selectors[id.0 as usize].definition),
            _ => self.describe(&self.sets[set]),
        }
    }
}

fn condition_text(files: &str, condition: &Condition) -> String {
    let side = condition
        .side()
        .map(|side| side.text.as_str())
        .unwrap_or_default();
    match condition {
        Condition::MayNotImport(_) => format!("files in {files} may not import {side}"),
        Condition::MayOnlyImport(_) => format!("files in {files} may only import {side}"),
        Condition::MayOnlyBeImportedBy(_) => {
            format!("files in {files} may only be imported by {side}")
        }
        Condition::MustNotExist => format!("files in {files} must not exist"),
        Condition::MustBeNamed(_)
        | Condition::MustContain(_)
        | Condition::MustExport(_)
        | Condition::MayNotExport(_) => valued_text(files, condition),
        Condition::MustOnlyReExport => {
            format!("files in {files} may only pass on what other files export")
        }
        Condition::NoCycles => format!("files in {files} may not import each other in a cycle"),
        Condition::Plugin(used) => format!(
            "files in {files} must meet the plugin condition {}",
            used.full_name()
        ),
        Condition::Entry { files: entries, .. } => {
            let entries: Vec<&str> = entries.iter().map(Pattern::source).collect();
            format!(
                "files in {files} may be imported from outside their instance only through its \
                 entry files {}",
                entries.join(", ")
            )
        }
    }
}

/// The words of a condition whose value is a name, a pattern or a kind of export.
fn valued_text(files: &str, condition: &Condition) -> String {
    match condition {
        Condition::MustBeNamed(NameRule::Style(style)) => {
            format!("files in {files} must have names in {style}")
        }
        Condition::MustBeNamed(NameRule::Pattern(pattern)) => {
            format!(
                "files in {files} must have full names that match {}",
                pattern.source()
            )
        }
        Condition::MustContain(patterns) => {
            format!(
                "each instance of {files} must have a file that matches {}",
                either(patterns)
            )
        }
        Condition::MustExport(pattern) => {
            format!(
                "files in {files} must export a name that matches {}",
                pattern.source()
            )
        }
        Condition::MayNotExport(ExportKind::Default) => {
            format!("files in {files} may not have a default export")
        }
        Condition::MayNotExport(ExportKind::Star) => {
            format!("files in {files} may not have an export *")
        }
        Condition::MayNotExport(ExportKind::Names(pattern)) => {
            format!(
                "files in {files} may not export a name that matches {}",
                pattern.source()
            )
        }
        other => format!("files in {files} must meet {}", other.name()),
    }
}

fn folder_text(folders: &str, condition: &Condition) -> String {
    match condition {
        Condition::MustBeNamed(NameRule::Style(style)) => {
            format!("folders in {folders} must have names in {style}")
        }
        Condition::MustBeNamed(NameRule::Pattern(pattern)) => {
            format!(
                "folders in {folders} must have names that match {}",
                pattern.source()
            )
        }
        Condition::MustContain(patterns) => {
            format!(
                "each folder in {folders} must have a file that matches {}",
                either(patterns)
            )
        }
        Condition::MustNotExist => format!("no folder may be in {folders}"),
        other => condition_text(folders, other),
    }
}

/// Patterns any one of which will do, as a sentence names them.
pub(crate) fn either(patterns: &[Pattern]) -> String {
    patterns
        .iter()
        .map(Pattern::source)
        .collect::<Vec<_>>()
        .join(" or ")
}

/// The limits of an import condition, in words.
fn limits_text(options: &ImportOptions, text: &mut String) {
    match options.instance {
        Some(InstanceMatch::Different) => text.push_str(" in a different instance"),
        Some(InstanceMatch::Same) => text.push_str(" in the same instance"),
        None => {}
    }
    if let Some(names) = &options.names {
        let _ = write!(text, " when the import brings in {}", names.join(", "));
    }
    if let Some(form) = options.specifier {
        let form = match form {
            SpecifierForm::Relative => "relative",
            SpecifierForm::Parent => "parent",
            SpecifierForm::Alias => "alias",
            SpecifierForm::Package => "package",
        };
        let _ = write!(text, " through a {form} specifier");
    }
    match options.dependency {
        Some(DependencyKind::Dev) => {
            text.push_str(" that the nearest package.json lists only for development");
        }
        Some(DependencyKind::Undeclared) => {
            text.push_str(" that the nearest package.json does not list");
        }
        None => {}
    }
}
