//! `Session::apply`: a change from an editor or a watcher, at the cost of the work it needs.

use std::collections::{BTreeMap, BTreeSet};

use archangel_graph::{GraphOptions, ImportGraph, Resolution, Work};
use archangel_rules::{
    Architecture, Classes, Diagnostic, Kept, Lookup, Report, StructureChange, Summary,
};

use crate::baseline_file;
use crate::events::{self, Known, PathEvent, Plan, Step};
use crate::plugins::{Files, PluginAnswer};
use crate::session::{Session, config_problems};
use crate::view::FileRecord;

/// A change to the session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// An editor's text for a document, which opens it; `None` closes it, and the text on disk
    /// counts again.
    Text {
        /// The path relative to the config folder.
        path: String,
        /// The text, or `None` to close the document.
        text: Option<String>,
    },
    /// A batch of events from a file watcher or the editor.
    Paths(Vec<PathEvent>),
}

/// What a change did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Applied {
    /// The files whose diagnostics changed; diagnostics in no file are under the empty path.
    pub changed: BTreeSet<String>,
    /// The work the change took.
    pub work: Work,
}

impl Session {
    /// Applies changes that arrived together, as one batch: the events of every `Paths` change
    /// are netted with each other before any work starts.
    pub fn apply(&mut self, changes: Vec<Change>) -> Applied {
        let before = by_file(&self.report.diagnostics);
        let mut work = Work::default();
        if resolve_per_event() {
            for change in changes {
                let events = match change {
                    Change::Paths(events) => events,
                    text @ Change::Text { .. } => {
                        self.apply_batch(vec![text], &mut work);
                        continue;
                    }
                };
                for event in events {
                    self.apply_batch(vec![Change::Paths(vec![event])], &mut work);
                }
            }
        } else {
            self.apply_batch(changes, &mut work);
        }
        self.work.add(&work);
        let after = by_file(&self.report.diagnostics);
        Applied {
            changed: differing(&before, &after),
            work,
        }
    }

    /// Loads the config and the baseline again and checks everything from the start, as after
    /// the system lost file events. The open documents keep their texts.
    pub fn reload(&mut self) -> Applied {
        let before = by_file(&self.report.diagnostics);
        let mut work = Work::default();
        let plan = Plan {
            steps: [Step::Config, Step::Baseline].into(),
            ..Plan::default()
        };
        if self.load(&plan) {
            self.rebuild(&mut work);
        }
        self.work.add(&work);
        let after = by_file(&self.report.diagnostics);
        Applied {
            changed: differing(&before, &after),
            work,
        }
    }

    /// The plugins whose rules the plugin process is running, or `None` when it runs none. The
    /// report holds the plugin rules' earlier results until the process answers.
    #[must_use]
    pub fn plugins_running(&self) -> Option<Vec<String>> {
        self.plugins.running(&self.architecture)
    }

    /// Where the plugin process's answers arrive, while one runs; each goes to
    /// [`Session::receive_plugins`].
    #[must_use]
    pub fn plugin_answers(&self) -> Option<crossbeam_channel::Receiver<PluginAnswer>> {
        self.plugins.answers()
    }

    /// Takes an answer of the plugin process, and returns the files whose diagnostics changed.
    pub fn receive_plugins(&mut self, answer: PluginAnswer) -> Applied {
        let before = by_file(&self.report.diagnostics);
        let mut work = Work::default();
        let files = Files {
            architecture: &self.architecture,
            graph: &self.graph,
            classes: &self.classes,
        };
        if self.plugins.take(answer, files) {
            self.finish(&mut work);
        }
        self.work.add(&work);
        let after = by_file(&self.report.diagnostics);
        Applied {
            changed: differing(&before, &after),
            work,
        }
    }

    /// Waits until the plugin process answers the check it runs, and returns the files whose
    /// diagnostics changed.
    pub fn wait_for_plugins(&mut self) -> Applied {
        let before = by_file(&self.report.diagnostics);
        let mut work = Work::default();
        if self.plugins.running(&self.architecture).is_some() {
            self.wait_plugins();
            self.finish(&mut work);
        }
        self.work.add(&work);
        let after = by_file(&self.report.diagnostics);
        Applied {
            changed: differing(&before, &after),
            work,
        }
    }

    /// The records the plugin process holds, as last sent, in path order.
    #[must_use]
    pub fn plugin_records(&self) -> Vec<&FileRecord> {
        self.plugins.records()
    }

    /// What the config and the plugins printed since the last call, each line starting with the
    /// name of the plugin, or `config`, in brackets.
    pub fn take_plugin_output(&mut self) -> Vec<String> {
        self.plugins.take_printed()
    }

    /// Ends the plugin process once a command needs it no more, and waits until it has exited, so
    /// that `take_plugin_output` then has every line it printed.
    pub fn close_plugins(&mut self) {
        self.plugins.close();
    }

    fn apply_batch(&mut self, changes: Vec<Change>, work: &mut Work) {
        let mut edits: BTreeMap<String, Option<String>> = BTreeMap::new();
        let mut plan = Plan::default();
        let mut events = false;
        for change in changes {
            match change {
                Change::Text { path, text } => {
                    if let Some(text) = &text {
                        if self.open.insert(path.clone(), text.clone()).is_none() {
                            events::opened(&self.graph, &path, &mut plan);
                        }
                    } else {
                        self.open.remove(&path);
                        events::closed(&self.graph, &path, &mut plan);
                    }
                    edits.insert(path, text);
                }
                Change::Paths(batch) => {
                    events = true;
                    let known = Known {
                        graph: &self.graph,
                        config_path: &self.config_path,
                        config_inputs: &self.config_inputs,
                        open: &self.open,
                    };
                    plan.merge(if skip_netting() {
                        events::unnetted(&known, &batch)
                    } else {
                        events::plan(&known, &batch)
                    });
                }
            }
        }
        for path in std::mem::take(&mut plan.texts) {
            edits.entry(path).or_insert(None);
        }
        if events && resolve_per_event() {
            plan.steps.insert(Step::Resolve);
        }
        if full_check_fault(&plan, events, !edits.is_empty()) {
            self.rebuild(work);
            return;
        }
        self.execute(plan, &edits, work);
    }

    fn execute(
        &mut self,
        mut plan: Plan,
        edits: &BTreeMap<String, Option<String>>,
        work: &mut Work,
    ) {
        let ignore_before = self.architecture.config().ignore.clone();
        if !self.load(&plan) {
            return;
        }
        let config = plan.needs(Step::Config);
        let mut every_file = config;
        if config && self.architecture.config().ignore != ignore_before {
            plan.steps.insert(Step::Walk);
        }
        let mut again = BTreeSet::new();
        let mut resolved = BTreeSet::new();
        let mut paths = false;
        if plan.needs(Step::Walk) {
            let before = self.resolutions();
            self.walk_again(work);
            resolved.extend(self.moved_since(&before));
            every_file = true;
            paths = true;
        } else if plan.needs(Step::Resolve) {
            let before = self.resolutions();
            self.graph.resolve_all_again(work);
            let moved = self.moved_since(&before);
            resolved.extend(moved.iter().cloned());
            again.extend(moved);
        }
        for (path, present) in &plan.others {
            paths |= self.graph.set_other(path, *present);
        }
        if plan.needs(Step::Manifests) {
            let changed = self.graph.reread_manifests();
            again.extend(self.files_under(&changed));
        }
        self.edit_and_check(
            edits,
            Recheck {
                files: again,
                every_file,
                config,
                paths,
                resolved,
            },
            work,
        );
    }

    /// Where every file's imports resolve now.
    fn resolutions(&self) -> BTreeMap<String, Vec<Resolution>> {
        self.graph
            .files()
            .iter()
            .map(|file| {
                let resolutions = self.graph.resolutions(&file.path).unwrap_or(&[]).to_vec();
                (file.path.clone(), resolutions)
            })
            .collect()
    }

    /// The files whose imports resolve elsewhere than `before`, new files included, which are the
    /// only files whose checks a new resolution can change.
    fn moved_since(&self, before: &BTreeMap<String, Vec<Resolution>>) -> Vec<String> {
        self.graph
            .files()
            .iter()
            .filter(|file| {
                before.get(&file.path).map(Vec::as_slice) != self.graph.resolutions(&file.path)
            })
            .map(|file| file.path.clone())
            .collect()
    }

    /// The source files whose nearest manifest is one of `manifests`.
    fn files_under(&self, manifests: &[String]) -> Vec<String> {
        self.graph
            .files()
            .iter()
            .filter(|file| {
                self.graph
                    .nearest_manifest(&file.path)
                    .is_some_and(|manifest| manifests.contains(&manifest.path))
            })
            .map(|file| file.path.clone())
            .collect()
    }

    /// Parses the edited files and classifies them again, then checks them, the files in `again`
    /// and the importers of each file whose class changed, or every file when the change needs it.
    fn edit_and_check(
        &mut self,
        edits: &BTreeMap<String, Option<String>>,
        recheck: Recheck,
        work: &mut Work,
    ) {
        let Recheck {
            files: mut again,
            every_file,
            config,
            paths,
            resolved,
        } = recheck;
        // @justified intent
        // Only a check of every file, which a config change makes, or a new resolution changes
        // records beyond the edited files'; a file that is not source appearing changes none.
        let every_record = every_file || !resolved.is_empty();
        let mut records: BTreeSet<String> = edits.keys().cloned().collect();
        records.extend(edits.keys().flat_map(|path| self.targets(path)));
        let mut texts: Vec<String> = resolved.into_iter().collect();
        let mut reclassified = Vec::new();
        for (path, text) in edits {
            if self.graph.replace_file(path, text.as_deref(), work) {
                again.insert(path.clone());
                texts.push(path.clone());
                let lookup = Lookup::new(&self.architecture, &self.graph);
                let changed = self.classes.reclassify(lookup, path, work);
                if changed {
                    reclassified.push(path.clone());
                    if !every_file {
                        again.extend(self.graph.importers(path).into_iter().map(str::to_owned));
                    }
                }
            }
        }
        self.classify_new_paths(paths, work);
        if every_file || evaluate_every_file_per_edit() {
            self.classes.sync(&self.architecture, &self.graph, work);
            self.checks = self
                .architecture
                .check_files(&self.graph, &mut self.classes, None, work);
        } else {
            let paths: Vec<&str> = again.iter().map(String::as_str).collect();
            let checked =
                self.architecture
                    .check_files(&self.graph, &mut self.classes, Some(&paths), work);
            self.checks.extend(checked);
        }
        let change = StructureChange {
            texts: &texts,
            reclassified: &reclassified,
            paths,
            every_export: !edits.is_empty() && evaluate_exports_of_every_file(),
            every_folder: paths && evaluate_every_folder_per_event(),
        };
        self.check_structure(config, &change, work);
        records.extend(edits.keys().flat_map(|path| self.targets(path)));
        records.extend(again);
        self.check_plugins((!every_record).then_some(&records), work);
        self.finish(work);
    }

    /// Classifies the files that appeared, when `paths` says files may have, or every file under
    /// the `engine::classify_every_file_per_edit` fault.
    fn classify_new_paths(&mut self, paths: bool, work: &mut Work) {
        if classify_every_file_per_edit() {
            self.classes
                .reclassify_all(&self.architecture, &self.graph, work);
        }
        if paths {
            self.classes.sync(&self.architecture, &self.graph, work);
        }
    }

    /// The repository files that a file's imports resolve to, whose importers it is one of.
    fn targets(&self, path: &str) -> Vec<String> {
        self.graph
            .resolutions(path)
            .unwrap_or(&[])
            .iter()
            .filter_map(|resolution| match resolution {
                Resolution::File(target) => Some(target.clone()),
                _ => None,
            })
            .collect()
    }

    /// Evaluates the rules about files, folders and instances again: what the change touched, or
    /// everything after a config change.
    fn check_structure(&mut self, config: bool, change: &StructureChange<'_>, work: &mut Work) {
        if config {
            self.structure = self
                .architecture
                .structure(&self.graph, &mut self.classes, work);
        } else {
            let kept = Kept {
                graph: &self.graph,
                classes: &mut self.classes,
                structure: &mut self.structure,
            };
            self.architecture.update_structure(kept, change, work);
        }
    }

    /// Loads the config and the baseline again when the plan needs them. Returns false, with the
    /// problem as the report, when either cannot be used.
    fn load(&mut self, plan: &Plan) -> bool {
        if plan.needs(Step::Config) {
            match crate::plugins::load(&self.config_path) {
                Ok(loaded) => {
                    self.architecture = Architecture::compile(loaded.config);
                    self.classes = Classes::new();
                    self.plugins.replace(loaded.process, &self.architecture);
                    self.config_inputs = loaded.inputs;
                }
                Err(refused) => {
                    if !refused.inputs.is_empty() {
                        self.config_inputs = refused.inputs;
                    }
                    self.stop(config_problems(&self.config_path, &refused.problems));
                    return false;
                }
            }
        }
        if plan.needs(Step::Baseline) {
            match baseline_file::read_baseline(&self.config_dir) {
                Ok(baseline) => self.baseline = baseline,
                Err(diagnostic) => {
                    self.stop(vec![*diagnostic]);
                    return false;
                }
            }
        }
        true
    }

    /// Walks again, and gives each open document that the walk brought back its editor's text,
    /// because the walk reads the text on disk.
    fn walk_again(&mut self, work: &mut Work) {
        let before: BTreeSet<String> = self
            .open
            .keys()
            .filter(|path| self.graph.file(path).is_some())
            .cloned()
            .collect();
        let ignore = self.architecture.config().ignore.clone();
        self.graph.walk_again(&ignore, work);
        for (path, text) in &self.open {
            if !before.contains(path) {
                self.graph.replace_file(path, Some(text), work);
            }
        }
    }

    /// Reads, parses, resolves and checks everything again: after lost events, and for the fault
    /// controls.
    fn rebuild(&mut self, work: &mut Work) {
        let options = GraphOptions {
            ignore: self.architecture.config().ignore.clone(),
        };
        self.graph = ImportGraph::build(&self.config_dir, &options, work);
        for (path, text) in &self.open {
            self.graph.replace_file(path, Some(text), work);
        }
        self.classes = Classes::new();
        self.classes.sync(&self.architecture, &self.graph, work);
        self.checks = self
            .architecture
            .check_files(&self.graph, &mut self.classes, None, work);
        self.structure = self
            .architecture
            .structure(&self.graph, &mut self.classes, work);
        self.check_plugins(None, work);
        self.finish(work);
    }

    /// Shows what stops the check, such as a config that no longer loads, until a change fixes it.
    pub(crate) fn stop(&mut self, diagnostics: Vec<Diagnostic>) {
        self.report = Report {
            diagnostics,
            summary: Summary::default(),
        };
    }
}

/// The files to check again besides the edited ones, or every file, and what the structure
/// rules need to know of the change.
struct Recheck {
    files: BTreeSet<String>,
    every_file: bool,
    /// The config changed, so every rule about files, folders and instances runs again.
    config: bool,
    /// Files may have appeared or disappeared.
    paths: bool,
    /// The files whose imports resolve elsewhere now, whose export rules can change.
    resolved: BTreeSet<String>,
}

/// The files whose diagnostics differ between two reports.
fn differing(
    before: &BTreeMap<String, Vec<Diagnostic>>,
    after: &BTreeMap<String, Vec<Diagnostic>>,
) -> BTreeSet<String> {
    before
        .keys()
        .chain(after.keys())
        .filter(|file| before.get(*file) != after.get(*file))
        .cloned()
        .collect()
}

/// Whether a fail point turns this change into a full check, for its fault control.
fn full_check_fault(plan: &Plan, events: bool, edited: bool) -> bool {
    (plan.needs(Step::Config) && full_check_per_config())
        || (plan.needs(Step::Baseline) && full_check_per_baseline())
        || (events && full_check_per_event())
        || (edited && full_check_per_edit())
}

fn by_file(diagnostics: &[Diagnostic]) -> BTreeMap<String, Vec<Diagnostic>> {
    let mut found: BTreeMap<String, Vec<Diagnostic>> = BTreeMap::new();
    for diagnostic in diagnostics {
        found
            .entry(diagnostic.file.clone().unwrap_or_default())
            .or_default()
            .push(diagnostic.clone());
    }
    found
}

/// The `engine::resolve_per_event` fail point: applies each event of a batch on its own, and
/// resolves every import again for each file or manifest event.
fn resolve_per_event() -> bool {
    fail::fail_point!("engine::resolve_per_event", |_| true);
    false
}

/// The `engine::skip_netting` fail point: judges each event alone.
fn skip_netting() -> bool {
    fail::fail_point!("engine::skip_netting", |_| true);
    false
}

/// The `engine::full_check_per_edit` fail point.
fn full_check_per_edit() -> bool {
    fail::fail_point!("engine::full_check_per_edit", |_| true);
    false
}

/// The `engine::evaluate_every_file_per_edit` fail point.
fn evaluate_every_file_per_edit() -> bool {
    fail::fail_point!("engine::evaluate_every_file_per_edit", |_| true);
    false
}

/// The `engine::classify_every_file_per_edit` fail point.
fn classify_every_file_per_edit() -> bool {
    fail::fail_point!("engine::classify_every_file_per_edit", |_| true);
    false
}

/// The `engine::evaluate_every_folder_per_event` fail point.
fn evaluate_every_folder_per_event() -> bool {
    fail::fail_point!("engine::evaluate_every_folder_per_event", |_| true);
    false
}

/// The `engine::evaluate_exports_of_every_file` fail point.
fn evaluate_exports_of_every_file() -> bool {
    fail::fail_point!("engine::evaluate_exports_of_every_file", |_| true);
    false
}

/// The `engine::full_check_per_event` fail point.
fn full_check_per_event() -> bool {
    fail::fail_point!("engine::full_check_per_event", |_| true);
    false
}

/// The `engine::full_check_per_config` fail point.
fn full_check_per_config() -> bool {
    fail::fail_point!("engine::full_check_per_config", |_| true);
    false
}

/// The `engine::full_check_per_baseline` fail point.
fn full_check_per_baseline() -> bool {
    fail::fail_point!("engine::full_check_per_baseline", |_| true);
    false
}
