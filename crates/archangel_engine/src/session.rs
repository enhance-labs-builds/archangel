//! A session: the config, the graph, the rules, the baseline, and the last report.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use archangel_config::{Config, ConfigProblem, ProblemCode};
use archangel_graph::{GraphOptions, ImportGraph, Work};
use archangel_rules::{
    Architecture, Baseline, Classes, Code, Diagnostic, FileCheck, Report, Results, Structure,
};

use crate::plugins::{Files, Plugins};
use crate::{baseline_file, fixes, plugins};

/// What stops a session from checking: a config or baseline it cannot use. The command line
/// exits with status 2 for it.
#[derive(Debug, Clone)]
pub struct Stop {
    /// Every problem found, in the order found.
    pub diagnostics: Vec<Diagnostic>,
}

/// How long each stage of opening the session took.
#[derive(Debug, Clone, Copy, Default)]
pub struct Timings {
    /// Finding and loading the config.
    pub config: Duration,
    /// Walking, reading, parsing and resolving.
    pub graph: Duration,
    /// Checking the rules, the cycles, the exceptions and the baseline.
    pub rules: Duration,
    /// The steps of the graph stage.
    pub steps: archangel_graph::GraphTimings,
}

impl Timings {
    /// Each stage, and each step of the graph stage, with its name, in the order they run.
    #[must_use]
    pub fn stages(&self) -> [(&'static str, Duration); 6] {
        [
            ("config", self.config),
            ("graph", self.graph),
            ("  list files", self.steps.walk),
            ("  read and parse", self.steps.read_and_parse),
            ("  resolve", self.steps.resolve),
            ("rules", self.rules),
        ]
    }
}

/// One repository, checked.
#[derive(Debug)]
pub struct Session {
    pub(crate) config_path: PathBuf,
    /// The files the config imported, such as its presets and plugins, whose change loads it
    /// again.
    pub(crate) config_inputs: Vec<PathBuf>,
    pub(crate) config_dir: PathBuf,
    pub(crate) architecture: Architecture,
    pub(crate) graph: ImportGraph,
    pub(crate) baseline: Option<Baseline>,
    /// The class of each file, kept so that an edit classifies the edited file alone.
    pub(crate) classes: Classes,
    /// The rules' check of each file's imports, kept so that an edit checks one file again.
    pub(crate) checks: BTreeMap<String, FileCheck>,
    /// The results of the rules about files, folders and instances, kept so that a change
    /// evaluates only what it touched.
    pub(crate) structure: Structure,
    /// The documents an editor has open, with their texts, which count instead of the text on
    /// disk while they are open.
    pub(crate) open: BTreeMap<String, String>,
    /// The plugin process, the records it holds, and what the plugin rules found.
    pub(crate) plugins: Plugins,
    pub(crate) report: Report,
    pub(crate) work: Work,
    timings: Timings,
}

impl Session {
    /// Finds the config, from `config`, which is relative to `start`, or by searching `start` and
    /// its parents, and makes a full check.
    ///
    /// # Errors
    ///
    /// Returns the diagnostics that stop the check: no config, a config that does not load or is
    /// not valid, or a baseline that does not parse.
    pub fn open(start: &Path, config: Option<&Path>) -> Result<Self, Stop> {
        let started = Instant::now();
        let config_path = find_config(start, config)?;
        let plugins::Loaded {
            config: loaded,
            process,
            inputs,
        } = plugins::load(&config_path).map_err(|refused| Stop {
            diagnostics: config_problems(&config_path, &refused.problems),
        })?;
        let config_dir = config_path
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        let baseline = baseline_file::read_baseline(&config_dir).map_err(|diagnostic| Stop {
            diagnostics: vec![*diagnostic],
        })?;
        let config_time = started.elapsed();
        let mut work = Work::default();
        let options = GraphOptions {
            ignore: loaded.ignore.clone(),
        };
        let started = Instant::now();
        let graph = ImportGraph::build(&config_dir, &options, &mut work);
        let graph_time = started.elapsed();
        let architecture = compiled(loaded, &config_path);
        let started = Instant::now();
        let mut classes = Classes::new();
        classes.sync(&architecture, &graph, &mut work);
        let checks = architecture.check_files(&graph, &mut classes, None, &mut work);
        let structure = architecture.structure(&graph, &mut classes, &mut work);
        let plugins = Plugins::new(process, &architecture);
        let mut session = Self {
            config_path,
            config_inputs: inputs,
            config_dir,
            architecture,
            graph,
            baseline,
            classes,
            checks,
            structure,
            open: BTreeMap::new(),
            plugins,
            report: Report::default(),
            work: Work::default(),
            timings: Timings::default(),
        };
        session.check_plugins(None, &mut work);
        session.wait_plugins();
        session.finish(&mut work);
        session.work = work;
        session.timings = Timings {
            config: config_time,
            graph: graph_time,
            rules: started.elapsed(),
            steps: session.graph.timings(),
        };
        Ok(session)
    }

    /// Sends the plugin process the records that changed among `paths`, or every record, and runs
    /// the plugin rules they affect, or every plugin rule, starting the process again first if it
    /// stopped. It does not wait for the answer.
    pub(crate) fn check_plugins(
        &mut self,
        paths: Option<&std::collections::BTreeSet<String>>,
        work: &mut Work,
    ) {
        let mut every = paths.is_none();
        if self.plugins.needs_start(&self.architecture) {
            match plugins::load(&self.config_path) {
                Ok(loaded) => {
                    self.plugins.replace(loaded.process, &self.architecture);
                    every = true;
                }
                Err(refused) => {
                    self.stop(config_problems(&self.config_path, &refused.problems));
                    return;
                }
            }
        }
        let files = Files {
            architecture: &self.architecture,
            graph: &self.graph,
            classes: &self.classes,
        };
        match paths {
            Some(paths) if !every => self.plugins.check_changed(files, paths, work),
            _ => self.plugins.check_all(files, work),
        }
    }

    /// Waits for the plugin process to answer the check it runs.
    pub(crate) fn wait_plugins(&mut self) {
        let files = Files {
            architecture: &self.architecture,
            graph: &self.graph,
            classes: &self.classes,
        };
        self.plugins.wait(files);
    }

    /// The report from the kept results and the plugin rules' last results, with fixes.
    pub(crate) fn finish(&mut self, work: &mut Work) {
        let plugin = self.plugins.diagnostics();
        let results = Results {
            checks: &self.checks,
            structure: &self.structure,
            plugin: &plugin,
        };
        let mut report = self.architecture.assemble(
            &self.graph,
            &mut self.classes,
            results,
            self.baseline.as_ref(),
            work,
        );
        fixes::attach(
            &mut report.diagnostics,
            &self.graph,
            &self.architecture,
            work,
        );
        self.report = report;
    }

    /// The last report.
    #[must_use]
    pub fn report(&self) -> &Report {
        &self.report
    }

    /// The work the session has done.
    #[must_use]
    pub fn work(&self) -> &Work {
        &self.work
    }

    /// How long each stage took.
    #[must_use]
    pub fn timings(&self) -> Timings {
        self.timings
    }

    /// The import graph.
    #[must_use]
    pub fn graph(&self) -> &ImportGraph {
        &self.graph
    }

    /// The compiled config.
    #[must_use]
    pub fn architecture(&self) -> &Architecture {
        &self.architecture
    }

    /// The config file.
    #[must_use]
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    /// The config folder, which every path in a diagnostic is relative to.
    #[must_use]
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// The baseline in use, if there is one.
    #[must_use]
    pub fn baseline(&self) -> Option<&Baseline> {
        self.baseline.as_ref()
    }

    /// Every violation the code has now, plugin violations included, without the baseline.
    fn unbaselined(&self) -> Baseline {
        let plugin = self.plugins.diagnostics();
        let results = Results {
            checks: &self.checks,
            structure: &self.structure,
            plugin: &plugin,
        };
        let report = self.architecture.assemble(
            &self.graph,
            &mut self.classes.clone(),
            results,
            None,
            &mut Work::default(),
        );
        Baseline::from_report(&report)
    }

    /// Writes every current violation to the baseline file and returns the baseline written.
    ///
    /// # Errors
    ///
    /// Returns the error when the file cannot be written.
    pub fn baseline_add(&self) -> io::Result<Baseline> {
        let baseline = self.unbaselined();
        baseline_file::write_baseline(&self.config_dir, &baseline)?;
        Ok(baseline)
    }

    /// Lowers or removes the baseline entries that match fewer violations than their count, and
    /// returns the baseline written. It never adds an entry or raises a count.
    ///
    /// # Errors
    ///
    /// Returns the error when the file cannot be written.
    pub fn baseline_prune(&self) -> io::Result<Baseline> {
        let pruned = self
            .baseline
            .as_ref()
            .map(|baseline| baseline.pruned(&self.unbaselined()))
            .unwrap_or_default();
        baseline_file::write_baseline(&self.config_dir, &pruned)?;
        Ok(pruned)
    }
}

/// The config at `config`, relative to `start`, or the nearest one at or above `start`.
fn find_config(start: &Path, config: Option<&Path>) -> Result<PathBuf, Stop> {
    let config_path = match config {
        Some(path) => start.join(path),
        None => archangel_config::discover(start).ok_or_else(|| Stop {
            diagnostics: vec![problem(&archangel_config::missing(start), None)],
        })?,
    };
    Ok(dunce::canonicalize(&config_path).unwrap_or(config_path))
}

/// The config's problems as diagnostics, in the config file.
/// The architecture of a config, which holds its file's name: the config file is at the config
/// folder, and it is in no table.
pub(crate) fn compiled(config: Config, config_path: &Path) -> Architecture {
    let name = config_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    Architecture::compile(config).with_config_file(name)
}

pub(crate) fn config_problems(config_path: &Path, problems: &[ConfigProblem]) -> Vec<Diagnostic> {
    let name = config_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned());
    problems
        .iter()
        .map(|p| problem(p, name.as_deref()))
        .collect()
}

/// A config problem as a diagnostic, in the config file when the problem has a position.
fn problem(problem: &ConfigProblem, file: Option<&str>) -> Diagnostic {
    let code = match problem.code {
        ProblemCode::ConfigMissing => Code::ConfigMissing,
        ProblemCode::ConfigNeedsNode => Code::ConfigNeedsNode,
        ProblemCode::ConfigLoadFailed => Code::ConfigLoadFailed,
        ProblemCode::ConfigInvalid => Code::ConfigInvalid,
        ProblemCode::PatternInvalid => Code::PatternInvalid,
        ProblemCode::RuleUnknownElement => Code::RuleUnknownElement,
        ProblemCode::RuleNeedsCapture => Code::RuleNeedsCapture,
        ProblemCode::RuleNameDuplicate => Code::RuleNameDuplicate,
    };
    let message = match &problem.field {
        Some(field) => format!("{field}: {}", problem.message),
        None => problem.message.clone(),
    };
    let diagnostic = Diagnostic::new(code, message);
    match (file, problem.position) {
        (Some(file), Some((line, column))) => diagnostic.at(
            file,
            u32::try_from(line).unwrap_or(u32::MAX),
            u32::try_from(column).unwrap_or(u32::MAX),
        ),
        (Some(file), None) => diagnostic.at(file, 0, 0),
        (None, _) => diagnostic,
    }
}
