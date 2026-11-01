//! `archangel test`: every example of every rule, each in memory against the repository.

use archangel_config::{ExampleFiles, RuleKind, TableId};
use archangel_graph::{GraphOptions, ImportGraph, Work};
use archangel_rules::{Diagnostic, ExampleKind, ExampleOutcome, PluginFindings};

use crate::plugins::Files;
use crate::session::Session;

/// One example, and how it fared.
#[derive(Debug, Clone)]
pub struct ExampleRun {
    /// The rule's name.
    pub rule: String,
    /// Whether the rule must accept or reject it.
    pub kind: ExampleKind,
    /// Its place among the rule's examples of that kind, from 1.
    pub number: usize,
    /// The paths of its files.
    pub files: Vec<String>,
    /// Its files, with their text, by path.
    pub texts: ExampleFiles,
    /// The judgement.
    pub outcome: ExampleOutcome,
}

/// Every example's result, the rules that have no rejected example, and the work it took.
#[derive(Debug, Clone)]
pub struct ExampleReport {
    /// The examples, in config order, each rule's accepted examples before its rejected ones.
    pub runs: Vec<ExampleRun>,
    /// A `rule-unproven` diagnostic for each rule without a rejected example.
    pub unproven: Vec<Diagnostic>,
    /// The work of the examples alone.
    pub work: Work,
}

impl ExampleReport {
    /// Whether every example passed and every rule has a rejected example.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.unproven.is_empty() && self.runs.iter().all(|run| run.outcome.passed)
    }
}

impl Session {
    /// Checks every example of every rule. Nothing is written, and the session's results do not
    /// change: the plugin process gets each example's records and then the ones they replaced.
    #[must_use]
    pub fn run_examples(&mut self) -> ExampleReport {
        let mut work = Work::default();
        let mut runs = Vec::new();
        let rules = self.architecture.config().rules.clone();
        for (index, rule) in rules.iter().enumerate() {
            let sets = [
                (ExampleKind::Accepted, &rule.examples.accepted),
                (ExampleKind::Rejected, &rule.examples.rejected),
            ];
            for (kind, examples) in sets {
                for (number, files) in examples.iter().enumerate() {
                    let example = Example {
                        of: Of::Rule(index),
                        kind,
                        files,
                    };
                    let outcome = self.outcome(&example, &mut work);
                    work.examples_checked += 1;
                    runs.push(ExampleRun {
                        rule: rule.name.clone(),
                        kind,
                        number: number + 1,
                        files: files.keys().cloned().collect(),
                        texts: files.clone(),
                        outcome,
                    });
                }
            }
        }
        let tables = self.architecture.config().tables.clone();
        for (index, table) in tables.iter().enumerate() {
            let id = TableId(u32::try_from(index).unwrap_or(u32::MAX));
            let live = rules
                .iter()
                .any(|rule| matches!(&rule.kind, RuleKind::TableRow { table, .. } if *table == id));
            let sets = [
                (ExampleKind::Accepted, &table.examples.accepted),
                (ExampleKind::Rejected, &table.examples.rejected),
            ];
            for (kind, examples) in sets.into_iter().filter(|_| live) {
                for (number, files) in examples.iter().enumerate() {
                    let example = Example {
                        of: Of::Table(id),
                        kind,
                        files,
                    };
                    let outcome = self.outcome(&example, &mut work);
                    work.examples_checked += 1;
                    runs.push(ExampleRun {
                        rule: table.name.clone(),
                        kind,
                        number: number + 1,
                        files: files.keys().cloned().collect(),
                        texts: files.clone(),
                        outcome,
                    });
                }
            }
        }
        ExampleReport {
            runs,
            unproven: self.architecture.unproven_rules(),
            work,
        }
    }

    fn outcome(&mut self, example: &Example<'_>, work: &mut Work) -> ExampleOutcome {
        if write_examples() {
            for (path, text) in example.files {
                let full = self.config_dir().join(path);
                if let Some(folder) = full.parent()
                    && std::fs::create_dir_all(folder).is_ok()
                    && std::fs::write(&full, text).is_ok()
                {
                    work.files_written += 1;
                }
            }
        }
        let reread = reread_per_example().then(|| {
            let options = GraphOptions {
                ignore: self.architecture().config().ignore.clone(),
            };
            ImportGraph::build(self.config_dir(), &options, work)
        });
        let graph = reread.as_ref().unwrap_or(&self.graph);
        if evaluate_graph_per_example() {
            drop(self.architecture.check(graph, None, work));
        }
        let overlay = graph.overlay(example.files, work);
        let rule = match example.of {
            Of::Rule(rule) => rule,
            Of::Table(table) => {
                return self.architecture.check_table_example(
                    graph,
                    &overlay,
                    table,
                    example.kind,
                    work,
                );
            }
        };
        let is_plugin = self.architecture.config().rules[rule]
            .plugin_use()
            .is_some();
        let findings = if is_plugin {
            let files = Files {
                architecture: &self.architecture,
                graph,
                classes: &self.classes,
            };
            let of = crate::plugins::PluginExample {
                overlay: &overlay,
                rule,
            };
            self.plugins.example(files, of, work)
        } else {
            PluginFindings::default()
        };
        self.architecture
            .check_example_with(graph, &overlay, (rule, example.kind), &findings, work)
    }
}

/// Whose example it is.
#[derive(Clone, Copy)]
enum Of {
    /// A rule, by its index.
    Rule(usize),
    /// A table, whose rules all judge it.
    Table(TableId),
}

/// One example of one rule or table.
struct Example<'e> {
    of: Of,
    kind: ExampleKind,
    files: &'e ExampleFiles,
}

/// The `engine::write_examples` fail point: writes each example's files to disk, for the fault
/// control of the files-written count.
fn write_examples() -> bool {
    fail::fail_point!("engine::write_examples", |_| true);
    false
}

/// The `engine::reread_per_example` fail point: reads the repository again for each example.
fn reread_per_example() -> bool {
    fail::fail_point!("engine::reread_per_example", |_| true);
    false
}

/// The `engine::evaluate_graph_per_example` fail point: checks the whole graph for each example.
fn evaluate_graph_per_example() -> bool {
    fail::fail_point!("engine::evaluate_graph_per_example", |_| true);
    false
}
