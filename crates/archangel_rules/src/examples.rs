//! Rule examples: the files of an example, judged by their own rule against the repository,
//! without a change to the repository's graph.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use archangel_config::{Condition, Rule, RuleKind, TableId};
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
    /// For an example of a table, the row that rejected it, which is the first in config order.
    pub rejected_by: Option<String>,
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

    /// Checks one written example of a table against all its rules: `overlay` is the example's
    /// files, laid over `graph`.
    #[must_use]
    pub fn check_table_example(
        &self,
        graph: &ImportGraph,
        overlay: &OverlayFiles,
        table: TableId,
        kind: ExampleKind,
        work: &mut Work,
    ) -> ExampleOutcome {
        let of_table = |row: bool| -> Vec<usize> {
            self.config
                .rules
                .iter()
                .enumerate()
                .filter(|(_, rule)| match &rule.kind {
                    RuleKind::TableRow { table: id, row: r } => {
                        *id == table && (!row || r.is_some())
                    }
                    _ => false,
                })
                .map(|(index, _)| index)
                .collect()
        };
        let proof = Proof {
            subject: Subject::Table(&self.config.tables[table.index()].name),
            judges: of_table(true),
            rejects: of_table(true),
            kind,
        };
        self.prove((graph, overlay), (&proof, &PluginFindings::default()), work)
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
        let proof = Proof {
            subject: Subject::Rule(rule),
            judges: vec![rule],
            rejects: vec![rule],
            kind,
        };
        self.prove((graph, overlay), (&proof, plugin), work)
    }

    fn prove(
        &self,
        (graph, overlay): (&ImportGraph, &OverlayFiles),
        (proof, plugin): (&Proof<'_>, &PluginFindings),
        work: &mut Work,
    ) -> ExampleOutcome {
        let mut classes = Classes::new();
        let mut run = Run::new(self, graph, &mut classes, example_scope(graph, overlay));
        run.manifests = graph.manifests_with(Some(overlay));
        run.watched = Some((proof.judges.clone(), false));
        run.imports(work);
        let checks = std::mem::take(&mut run.checks);
        run.absorb(checks.values());
        run.cycles(work);
        run.transitive(work);
        run.problems(&overlay.problems);
        let example = Example {
            graph,
            overlay,
            rules: &proof.judges,
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
            looked || plugin.considered || run.watched.as_ref().is_some_and(|(_, seen)| *seen);
        let report = run.finish();
        let paths: BTreeSet<&str> = overlay
            .files
            .iter()
            .map(|f| f.path.as_str())
            .chain(overlay.others.iter().map(String::as_str))
            .collect();
        let judge = Judge {
            architecture: self,
            proof,
            paths: &paths,
        };
        let failure = judge.failure(&report.diagnostics, considered);
        let rejected_by = judge.rejecting_row(&report.diagnostics);
        ExampleOutcome {
            passed: failure.is_none(),
            failure,
            diagnostics: report.diagnostics,
            rejected_by,
        }
    }

    /// A `rule-unproven` diagnostic for each rule without a rejected example. A row of a table needs
    /// none: the rows with no file, `table-empty` and the reach of `archangel rules` show that it
    /// can fire.
    #[must_use]
    pub fn unproven_rules(&self) -> Vec<Diagnostic> {
        self.config
            .rules
            .iter()
            .filter(|rule| !matches!(rule.kind, RuleKind::TableRow { .. }))
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

/// What one example must show.
struct Proof<'p> {
    subject: Subject<'p>,
    /// The rules that can judge an import of the example, for "proves nothing".
    judges: Vec<usize>,
    /// The rules that can reject it.
    rejects: Vec<usize>,
    kind: ExampleKind,
}

/// Whose example it is.
enum Subject<'p> {
    /// A rule, by its index.
    Rule(usize),
    /// A table, by its name.
    Table(&'p str),
}

/// Decides an example from the diagnostics its files gave.
struct Judge<'j> {
    architecture: &'j Architecture,
    proof: &'j Proof<'j>,
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

    fn name(&self) -> &str {
        match self.proof.subject {
            Subject::Rule(rule) => &self.architecture.config.rules[rule].name,
            Subject::Table(name) => name,
        }
    }

    /// The rules of the proof that the diagnostics name, in config order.
    fn rejecting(&self, diagnostics: &[Diagnostic]) -> Vec<&str> {
        let by = self.rejected_by(diagnostics);
        self.proof
            .rejects
            .iter()
            .map(|rule| self.architecture.config.rules[*rule].name.as_str())
            .filter(|name| by.contains(name))
            .collect()
    }

    /// For a table's example, the row that rejected it.
    fn rejecting_row(&self, diagnostics: &[Diagnostic]) -> Option<String> {
        matches!(self.proof.subject, Subject::Table(_))
            .then(|| self.rejecting(diagnostics).first().map(|n| (*n).to_owned()))
            .flatten()
    }

    fn rejected_by<'d>(&self, diagnostics: &'d [Diagnostic]) -> BTreeSet<&'d str> {
        diagnostics
            .iter()
            .filter(|d| self.involves(d))
            .filter_map(|d| d.rule.as_deref())
            .collect()
    }

    fn failure(&self, diagnostics: &[Diagnostic], considered: bool) -> Option<Diagnostic> {
        let name = self.name().to_owned();
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
        let rejecting = self.rejecting(diagnostics);
        match self.proof.kind {
            ExampleKind::Rejected if !rejecting.is_empty() => None,
            ExampleKind::Rejected => {
                let mut message = self.not_rejected(&name, &self.rejected_by(diagnostics));
                for file in self.files_with_no_row(diagnostics) {
                    let _ = write!(
                        message,
                        "; `{file}` has no row, so check that its path has the right kind"
                    );
                }
                Some(self.failed(message))
            }
            ExampleKind::Accepted if !rejecting.is_empty() => Some(self.failed(format!(
                "\"{}\" rejected an example it should accept; fix the rule or the example",
                rejecting[0]
            ))),
            ExampleKind::Accepted if !considered => {
                let mut diagnostic =
                    Diagnostic::new(Code::ExampleProvesNothing, self.proves_nothing(&name));
                diagnostic.rule = Some(name);
                Some(diagnostic)
            }
            ExampleKind::Accepted => None,
        }
    }

    /// For a table's example, the files of the example that are in no row.
    fn files_with_no_row<'d>(&self, diagnostics: &'d [Diagnostic]) -> Vec<&'d str> {
        if !matches!(self.proof.subject, Subject::Table(_)) {
            return Vec::new();
        }
        diagnostics
            .iter()
            .filter(|d| d.code == Code::EveryFileHasAKind && self.involves(d))
            .filter_map(|d| d.file.as_deref())
            .collect()
    }

    fn not_rejected(&self, name: &str, rejected_by: &BTreeSet<&str>) -> String {
        match self.proof.subject {
            Subject::Rule(_) => rejection_missing(name, rejected_by),
            Subject::Table(_) => {
                format!("no row of \"{name}\" rejected the example; fix the table or the example")
            }
        }
    }

    fn proves_nothing(&self, name: &str) -> String {
        let Subject::Rule(rule) = self.proof.subject else {
            return format!(
                "\"{name}\" never looked at an import of this example, so accepting it proves \
                 nothing; give the example an import from a file of a row into the repository"
            );
        };
        let rule = &self.architecture.config.rules[rule];
        match area_unchecked(rule) {
            Some(text) => format!("\"{name}\" {text}"),
            None => format!("\"{name}\" {}", unchecked(rule)),
        }
    }

    fn failed(&self, message: String) -> Diagnostic {
        let mut diagnostic = Diagnostic::new(Code::ExampleFailed, message);
        diagnostic.rule = Some(self.name().to_owned());
        diagnostic
    }
}

/// Why an accepted example proves nothing about a rule with an area, which names the area.
fn area_unchecked(rule: &Rule) -> Option<String> {
    let sentence = rule.sentences.first()?;
    let area = &sentence.options.within.as_ref()?.text;
    let wanted = if matches!(sentence.condition, Condition::MayOnlyBeImportedBy(_)) {
        format!("from \"{area}\" into the rule's files")
    } else {
        format!("from the rule's files into \"{area}\"")
    };
    Some(format!(
        "never looked at an import of this example, so accepting it proves nothing; give the \
         example an import {wanted}"
    ))
}

/// Why an accepted example that its rule never looked at proves nothing, in the terms of what the
/// rule looks at, and what to give the example.
fn unchecked(rule: &Rule) -> &'static str {
    let Some(sentence) = rule.sentences.first() else {
        return "never looked at this example, so accepting it proves nothing";
    };
    let folders = sentence.folders.is_some();
    match &sentence.condition {
        Condition::MustNotExist if folders => {
            "never looked at a folder of this example, and every folder it looks at breaks it, so \
             no accepted example can prove it; keep only rejected examples"
        }
        Condition::MustNotExist => {
            "never looked at a file of this example, and every file it looks at breaks it, so no \
             accepted example can prove it; keep only rejected examples"
        }
        Condition::MustBeNamed(_) | Condition::MustContain(_) if folders => {
            "never looked at a folder of this example, so accepting it proves nothing; give the \
             example a file in a folder the rule names"
        }
        Condition::MustContain(_) => {
            "never looked at an instance of this example, so accepting it proves nothing; give the \
             example a file in an instance of the element the rule names"
        }
        Condition::MustBeNamed(_)
        | Condition::MustExport(_)
        | Condition::MayNotExport(_)
        | Condition::MustOnlyReExport => {
            "never looked at a file of this example, so accepting it proves nothing; give the \
             example a file in the rule's files"
        }
        Condition::Plugin(_) => {
            "never saw a file of this example, so accepting it proves nothing; give the example a \
             file in the rule's files"
        }
        Condition::NoCycles => {
            "never looked at an import of this example, so accepting it proves nothing; give the \
             example an import between files in the rule's files"
        }
        Condition::MayNotImport(_)
        | Condition::MayOnlyImport(_)
        | Condition::MayOnlyBeImportedBy(_)
        | Condition::Entry { .. } => {
            "never looked at an import of this example, so accepting it proves nothing; give the \
             example an import between the elements the rule names"
        }
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
