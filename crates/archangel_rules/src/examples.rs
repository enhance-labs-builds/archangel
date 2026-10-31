//! Rule examples: the files of an example, judged by their own rule against the repository,
//! without a change to the repository's graph.

use std::collections::BTreeSet;

use archangel_graph::{ImportGraph, OverlayFiles, Work};

use crate::architecture::Architecture;
use crate::check::{Entry, Run, Scope};
use crate::classes::Classes;
use crate::diagnostic::{Code, Diagnostic};
use crate::plugin::PluginFindings;
use crate::structure::Example;

/// Whether the rule must accept or reject an example.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExampleKind {
    /// The rule must accept it, and must have looked at one of its imports.
    Accepted,
    /// The rule itself must reject it.
    Rejected,
}

/// How an example fared.
#[derive(Debug, Clone)]
pub struct ExampleOutcome {
    /// Whether the example passed.
    pub passed: bool,
    /// Why it failed: `example-failed` or `example-proves-nothing`.
    pub failure: Option<Diagnostic>,
    /// What the example's files gave: violations of every rule, and parse and resolution problems.
    pub diagnostics: Vec<Diagnostic>,
}

impl Architecture {
    /// Checks one example of a rule: the example's files are `overlay`, laid over `graph`.
    #[must_use]
    pub fn check_example(
        &self,
        graph: &ImportGraph,
        overlay: &OverlayFiles,
        rule: usize,
        kind: ExampleKind,
        work: &mut Work,
    ) -> ExampleOutcome {
        let plugin = PluginFindings::default();
        self.check_example_with(graph, overlay, (rule, kind), &plugin, work)
    }

    /// Checks one example of a rule, with what the plugin process found in the example's files
    /// for a plugin rule.
    #[must_use]
    pub fn check_example_with(
        &self,
        graph: &ImportGraph,
        overlay: &OverlayFiles,
        (rule, kind): (usize, ExampleKind),
        plugin: &PluginFindings,
        work: &mut Work,
    ) -> ExampleOutcome {
        let mut classes = Classes::new();
        let mut run = Run::new(self, graph, &mut classes, example_scope(graph, overlay));
        run.manifests = graph.manifests_with(Some(overlay));
        run.watched = Some((rule, false));
        run.imports(work);
        let checks = std::mem::take(&mut run.checks);
        run.absorb(checks.values());
        run.cycles(work);
        run.transitive(work);
        run.problems(&overlay.problems);
        let example = Example {
            graph,
            overlay,
            rule,
        };
        let (structure, looked) = self.example_structure(example, run.classes, work);
        for diagnostic in structure
            .into_iter()
            .chain(plugin.diagnostics.iter().cloned())
        {
            if diagnostic.code.is_rule() {
                run.violations.push(diagnostic);
            } else {
                run.other.push(diagnostic);
            }
        }
        let considered =
            looked || plugin.considered || run.watched.is_some_and(|(_, considered)| considered);
        let report = run.finish();
        let paths: BTreeSet<&str> = overlay
            .files
            .iter()
            .map(|f| f.path.as_str())
            .chain(overlay.others.iter().map(String::as_str))
            .collect();
        let judge = Judge {
            architecture: self,
            rule,
            paths: &paths,
        };
        let failure = judge.failure(&report.diagnostics, kind, considered);
        ExampleOutcome {
            passed: failure.is_none(),
            failure,
            diagnostics: report.diagnostics,
        }
    }

    /// A `rule-unproven` diagnostic for each rule without a rejected example.
    #[must_use]
    pub fn unproven_rules(&self) -> Vec<Diagnostic> {
        self.config
            .rules
            .iter()
            .filter(|rule| rule.examples.rejected.is_empty())
            .map(|rule| {
                let message = format!(
                    "the rule \"{}\" has no rejected example, so nothing shows that it can find a \
                     violation; add one under examples.rejected",
                    rule.name
                );
                let mut diagnostic = Diagnostic::new(Code::RuleUnproven, message);
                diagnostic.rule = Some(rule.name.clone());
                diagnostic
            })
            .collect()
    }
}

/// The example's files are evaluated. The cycle search covers them and the repository files that
/// import a replaced file, directly or through other files: a new file has no importers, because
/// the repository's imports were resolved before it existed.
fn example_scope<'a>(graph: &'a ImportGraph, overlay: &'a OverlayFiles) -> Scope<'a> {
    let evaluated: Vec<Entry<'a>> = overlay
        .files
        .iter()
        .zip(&overlay.resolutions)
        .map(|(file, resolutions)| Entry { file, resolutions })
        .collect();
    let own: BTreeSet<&str> = overlay.files.iter().map(|f| f.path.as_str()).collect();
    let reaching: BTreeSet<&str> = if search_whole_graph() {
        graph.files().iter().map(|f| f.path.as_str()).collect()
    } else {
        let replaced: Vec<&str> = own
            .iter()
            .copied()
            .filter(|path| graph.file(path).is_some())
            .collect();
        graph.reaching(&replaced).into_iter().collect()
    };
    let mut searched = evaluated.clone();
    searched.extend(
        reaching
            .into_iter()
            .filter(|path| !own.contains(path))
            .filter_map(|path| {
                Some(Entry {
                    file: graph.file(path)?,
                    resolutions: graph.resolutions(path)?,
                })
            }),
    );
    Scope {
        evaluated,
        searched,
    }
}

/// Decides an example from the diagnostics its files gave.
struct Judge<'j> {
    architecture: &'j Architecture,
    rule: usize,
    paths: &'j BTreeSet<&'j str>,
}

impl Judge<'_> {
    /// Whether a diagnostic is about a file of the example, or about a folder or instance that
    /// holds one.
    fn involves(&self, diagnostic: &Diagnostic) -> bool {
        let in_example = |path: &str| self.paths.contains(path);
        let holds_one = |folder: &str| {
            let prefix = format!("{folder}/");
            self.paths.iter().any(|path| path.starts_with(&prefix))
        };
        diagnostic
            .file
            .as_deref()
            .is_some_and(|file| in_example(file) || holds_one(file))
            || diagnostic.group.iter().any(|path| in_example(path))
    }

    fn failure(
        &self,
        diagnostics: &[Diagnostic],
        kind: ExampleKind,
        considered: bool,
    ) -> Option<Diagnostic> {
        let name = &self.architecture.config.rules[self.rule].name;
        let problems: Vec<&str> = diagnostics
            .iter()
            .filter(|d| !d.code.is_rule())
            .map(|d| d.message.as_str())
            .collect();
        if !problems.is_empty() {
            return Some(self.failed(format!(
                "the example cannot be checked, so it proves nothing about \"{name}\": {}",
                problems.join("; ")
            )));
        }
        let rejected_by: BTreeSet<&str> = diagnostics
            .iter()
            .filter(|d| self.involves(d))
            .filter_map(|d| d.rule.as_deref())
            .collect();
        match kind {
            ExampleKind::Rejected if rejected_by.contains(name.as_str()) => None,
            ExampleKind::Rejected => Some(self.failed(rejection_missing(name, &rejected_by))),
            ExampleKind::Accepted if rejected_by.contains(name.as_str()) => {
                Some(self.failed(format!(
                    "\"{name}\" rejected an example it should accept; fix the rule or the example"
                )))
            }
            ExampleKind::Accepted if !considered => {
                let message = format!(
                    "\"{name}\" never looked at an import of this example, so accepting it proves \
                     nothing; give the example an import between the elements the rule names"
                );
                let mut diagnostic = Diagnostic::new(Code::ExampleProvesNothing, message);
                diagnostic.rule = Some(name.clone());
                Some(diagnostic)
            }
            ExampleKind::Accepted => None,
        }
    }

    fn failed(&self, message: String) -> Diagnostic {
        let mut diagnostic = Diagnostic::new(Code::ExampleFailed, message);
        diagnostic.rule = Some(self.architecture.config.rules[self.rule].name.clone());
        diagnostic
    }
}

fn rejection_missing(name: &str, rejected_by: &BTreeSet<&str>) -> String {
    if rejected_by.is_empty() {
        return format!(
            "no rule rejected the example, and \"{name}\" should have; fix the rule or the example"
        );
    }
    let others: Vec<String> = rejected_by.iter().map(|r| format!("\"{r}\"")).collect();
    format!(
        "{} rejected the example, but \"{name}\" did not; make the example break \"{name}\" alone",
        others.join(", ")
    )
}

/// The `rules::search_whole_graph_per_example` fail point: searches every repository file for
/// cycles for each example, for the fault control.
fn search_whole_graph() -> bool {
    fail::fail_point!("rules::search_whole_graph_per_example", |_| true);
    false
}
