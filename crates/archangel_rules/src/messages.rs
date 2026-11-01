//! The words of each diagnostic: what is wrong, what to do instead, and where.

use archangel_config::{Classification, Condition, ElementId, Pattern, Rule, RuleKind, Sentence};
use archangel_graph::{Import, ImportGraph, Manifests};

use crate::architecture::Architecture;
use crate::diagnostic::{Code, Diagnostic, ViolationId};
use crate::exceptions::Exception;

pub(crate) fn cycle_diagnostic(
    rule: &Rule,
    names: Vec<String>,
    path: Vec<String>,
    step: Option<&Import>,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        Code::NoCycles,
        format!(
            "{} files import each other in a cycle: {}",
            names.len(),
            path.join(" → ")
        ),
    )
    .at(
        &names[0],
        step.map_or(0, |s| s.line),
        step.map_or(0, |s| s.column),
    );
    diagnostic.rule = Some(rule.name.clone());
    diagnostic.help = Some(
        "remove one import of the cycle, or move what both files need into a file that \
         neither of them imports"
            .to_owned(),
    );
    diagnostic.reason.clone_from(&rule.reason);
    diagnostic.link.clone_from(&rule.link);
    diagnostic.id = Some(ViolationId::Group {
        rule: rule.name.clone(),
        files: names.clone(),
    });
    diagnostic.cycle = path;
    diagnostic.group = names;
    diagnostic
}

/// One import in its file.
pub(crate) struct Site<'s> {
    pub(crate) file: &'s str,
    pub(crate) import: &'s Import,
    pub(crate) occurrence: u32,
}

/// What is wrong with an exception before it can permit anything: an unknown rule, or no reason.
pub(crate) fn exception_problem(exception: &Exception, rule_names: &[&str]) -> Option<Diagnostic> {
    if !rule_names.contains(&exception.rule.as_str()) {
        let message = format!(
            "the exception names \"{}\", which is not a rule; the rules are {}",
            exception.rule,
            rule_names.join(", ")
        );
        return Some(Diagnostic::new(Code::ExceptionUnknownRule, message));
    }
    exception.reason.is_empty().then(|| {
        let message = format!(
            "the exception for \"{}\" gives no reason, so it permits nothing; write the reason after the colon",
            exception.rule
        );
        Diagnostic::new(Code::ExceptionWithoutReason, message)
    })
}

pub(crate) fn pattern_list<'p>(patterns: impl Iterator<Item = &'p str>) -> String {
    patterns.collect::<Vec<_>>().join(", ")
}

pub(crate) fn parse_failed(path: &str, failure: &archangel_graph::ParseFailure) -> Diagnostic {
    if failure.too_large {
        return Diagnostic::new(Code::ParseFailed, "the file is larger than the parser accepts (4 GiB); add it to the config's ignore list").at(path, 0, 0);
    }
    if failure.flow {
        return Diagnostic::new(
            Code::ParseFailed,
            "the file uses Flow syntax, which archangel does not read; add it to the config's ignore list to leave it out",
        )
        .at(path, 0, 0);
    }
    let first = failure.errors.first();
    let listed: Vec<String> = failure
        .errors
        .iter()
        .map(|e| format!("line {}, column {}: {}", e.line, e.column, e.message))
        .collect();
    Diagnostic::new(
        Code::ParseFailed,
        format!(
            "the file has syntax errors, so archangel uses none of its imports: {}",
            listed.join("; ")
        ),
    )
    .at(
        path,
        first.map_or(0, |e| e.line),
        first.map_or(0, |e| e.column),
    )
}

/// The two ends of a violating import, and the specifier between them.
#[derive(Clone, Copy)]
pub(crate) struct Parties<'p> {
    pub(crate) from: Option<&'p Classification>,
    pub(crate) to: Option<&'p Classification>,
    pub(crate) specifier: &'p str,
    pub(crate) file: &'p str,
}

/// A violation of an import, at the import.
pub(crate) fn import_diagnostic(
    rule: &Rule,
    site: &Site<'_>,
    (code, message, help): (Code, String, String),
) -> Diagnostic {
    let mut diagnostic =
        Diagnostic::new(code, message).at(site.file, site.import.line, site.import.column);
    diagnostic.rule = Some(rule.name.clone());
    diagnostic.span = Some(site.import.span);
    diagnostic.utf16 = Some(site.import.utf16);
    diagnostic.specifier = Some(site.import.specifier.clone());
    diagnostic.help = Some(help);
    diagnostic.reason.clone_from(&rule.reason);
    diagnostic.link.clone_from(&rule.link);
    diagnostic.id = Some(ViolationId::Import {
        rule: rule.name.clone(),
        file: site.file.to_owned(),
        specifier: site.import.specifier.clone(),
        occurrence: site.occurrence,
    });
    diagnostic
}

/// Writes the message and help of a violation in the terms of its rule.
pub(crate) struct Texts<'a> {
    pub(crate) architecture: &'a Architecture,
    pub(crate) graph: &'a ImportGraph,
    pub(crate) manifests: Manifests<'a>,
}

impl Texts<'_> {
    fn name(&self, id: ElementId) -> &str {
        &self.architecture.config.elements[id.index()].name
    }

    fn instance(&self, class: &Classification) -> String {
        if class.captures.is_empty() {
            format!("\"{}\"", self.name(class.element))
        } else {
            format!(
                "\"{}\" of \"{}\"",
                class.captures.join("/"),
                self.name(class.element)
            )
        }
    }

    fn of(&self, class: Option<&Classification>) -> String {
        class.map_or_else(
            || "a file in no element".to_owned(),
            |c| format!("\"{}\"", self.name(c.element)),
        )
    }

    pub(crate) fn describe(
        &self,
        rule: &Rule,
        sentence: &Sentence,
        parties: &Parties<'_>,
    ) -> (Code, String, String) {
        match &rule.kind {
            RuleKind::Layers { order } => return self.layers(order, parties),
            RuleKind::Isolate { element } => return self.isolate(*element, parties),
            RuleKind::Entry { files, .. } => return self.entry(files, parties),
            RuleKind::Sentence => {}
        }
        let Parties {
            specifier, file, ..
        } = *parties;
        let files = &sentence.files_text;
        let side = sentence
            .condition
            .side()
            .map(|side| self.side(side))
            .unwrap_or_default();
        let area_text = sentence.options.within.as_ref().map(|area| &area.text);
        let area = area_text.map_or_else(String::new, |area| format!(", within \"{area}\""));
        let importers = area_text.map_or_else(String::new, |area| {
            format!(", among the importers in \"{area}\"")
        });
        let manifest = sentence
            .options
            .dependency
            .map(|kind| self.manifest(file, kind))
            .unwrap_or_default();
        match &sentence.condition {
            Condition::MayOnlyImport(_) => {
                let own = if sentence.files.element().is_some_and(|element| {
                    self.architecture.config.elements[element.index()].has_capture()
                }) {
                    "other files in the same instance"
                } else {
                    "each other"
                };
                let may = if side.is_empty() {
                    own.to_owned()
                } else {
                    format!("{side} and {own}")
                };
                (
                    Code::MayOnlyImport,
                    format!(
                        "{file} may not import \"{specifier}\", because files in \"{files}\" may \
                         only import {may}{area}"
                    ),
                    format!("files in \"{files}\" may import only {may}{area}"),
                )
            }
            Condition::MayOnlyBeImportedBy(_) => (
                Code::MayOnlyBeImportedBy,
                format!(
                    "{file} may not import \"{specifier}\", because only {side} and files in \
                     \"{files}\" may import files in \"{files}\"{importers}"
                ),
                format!(
                    "only {side} and files in \"{files}\" may import files in \"{files}\"{importers}"
                ),
            ),
            _ => (
                Code::MayNotImport,
                format!(
                    "{file} may not import \"{specifier}\", because files in \"{files}\" may not \
                     import {side}{manifest}"
                ),
                "remove the import, or move the code to files that may import it".to_owned(),
            ),
        }
    }

    /// The names a side gives, each quoted.
    fn side(&self, side: &archangel_config::Side) -> String {
        let mut names: Vec<String> = side
            .files
            .iter()
            .map(|set| format!("\"{}\"", self.architecture.describe(set)))
            .collect();
        names.extend(
            side.elements
                .iter()
                .map(|e| format!("\"{}\"", self.name(*e))),
        );
        names.extend(
            side.packages
                .iter()
                .map(|p| format!("the package `{}`", p.source())),
        );
        names.join(", ")
    }

    /// How the importing file's nearest manifest lists the package.
    fn manifest(&self, file: &str, kind: archangel_config::DependencyKind) -> String {
        match (self.manifests.nearest(file), kind) {
            (Some(manifest), archangel_config::DependencyKind::Dev) => {
                format!(" that {} lists only under devDependencies", manifest.path)
            }
            (Some(manifest), archangel_config::DependencyKind::Undeclared) => {
                format!(" that {} does not list", manifest.path)
            }
            (None, _) => " that no package.json above the file lists".to_owned(),
        }
    }

    fn layers(&self, order: &[ElementId], parties: &Parties<'_>) -> (Code, String, String) {
        let Parties {
            from,
            to,
            specifier,
            ..
        } = *parties;
        let from_name = self.of(from);
        let position = from
            .and_then(|c| order.iter().position(|e| c.of(*e).is_some()))
            .unwrap_or(0);
        let allowed: Vec<String> = order[position..]
            .iter()
            .map(|e| format!("\"{}\"", self.name(*e)))
            .collect();
        (
            Code::Layers,
            format!(
                "{from_name} may not import {}, an earlier layer, through \"{specifier}\"",
                self.of(to)
            ),
            format!("{from_name} may import only {}", allowed.join(", ")),
        )
    }

    fn isolate(&self, element: ElementId, parties: &Parties<'_>) -> (Code, String, String) {
        let Parties {
            from,
            to,
            specifier,
            ..
        } = *parties;
        let one = from.map(|c| self.instance(c)).unwrap_or_default();
        let other = to.map(|c| self.instance(c)).unwrap_or_default();
        let name = self.name(element);
        (
            Code::Isolate,
            format!(
                "{one} imports {other} through \"{specifier}\", and instances of \"{name}\" may not import each other"
            ),
            format!("move what both need into an element that instances of \"{name}\" may share"),
        )
    }

    fn entry(&self, files: &[Pattern], parties: &Parties<'_>) -> (Code, String, String) {
        let Parties { to, specifier, .. } = *parties;
        let instance = to.map(|c| self.instance(c)).unwrap_or_default();
        let root = to.map(|c| c.root.clone()).unwrap_or_default();
        let entries: Vec<String> = self
            .graph
            .files()
            .iter()
            .filter_map(|f| {
                let inside = f.path.strip_prefix(&format!("{root}/"))?;
                files
                    .iter()
                    .any(|p| p.matches(inside).is_some())
                    .then(|| f.path.clone())
            })
            .collect();
        let through = if entries.is_empty() {
            let patterns: Vec<&str> = files.iter().map(Pattern::source).collect();
            format!("a file in {root} that matches {}", patterns.join(" or "))
        } else {
            entries.join(" or ")
        };
        (
            Code::Entry,
            format!("\"{specifier}\" reaches inside {instance} past its entry files"),
            format!("import it through {through}"),
        )
    }
}
