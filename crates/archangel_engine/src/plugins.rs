//! The Node process that loads a TypeScript or JavaScript config and hosts its plugins.
//!
//! The engine starts Node with the loader, and the loader's first answer is the config. When no
//! rule uses a plugin condition, the engine closes Node's standard input and waits for it to
//! exit; otherwise the process stays, for one command or for an editor session, and the engine
//! speaks one protocol with it: one message per file record, then the rules to run, answered with
//! their violations. Standard error is read on its own thread, so a plugin that prints a lot
//! cannot fill the pipe and stop the protocol.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::thread::JoinHandle;

use archangel_config::{Config, ConfigProblem, Reads};
use archangel_graph::{ImportGraph, OverlayFiles, SourceFile, Work};
use archangel_rules::{Architecture, Classes, Code, Diagnostic, PluginFindings, ViolationId};
use crossbeam_channel::{Receiver, unbounded};
use serde_json::{Value, json};

use crate::view::{self, FileRecord};

/// The lines of standard error that a plugin failure quotes, the last ones the process printed.
const QUOTED_LINES: usize = 20;

/// A message from the plugin process, which [`crate::Session::receive_plugins`] takes.
#[derive(Debug, Clone)]
pub struct PluginAnswer(Line);

impl PluginAnswer {
    /// The answer for a channel that closed without saying so: the process stopped.
    #[must_use]
    pub fn stopped() -> Self {
        Self(Line::Closed)
    }
}

#[derive(Debug, Clone)]
enum Line {
    Message(Value),
    /// The process closed its standard output: it stopped.
    Closed,
}

/// A config, loaded.
#[derive(Debug)]
pub(crate) struct Loaded {
    pub(crate) config: Config,
    /// The plugin process, when a rule uses a plugin condition.
    pub(crate) process: Option<Process>,
    /// The files the config imported, whose change loads it again.
    pub(crate) inputs: Vec<PathBuf>,
}

/// A config that does not load, and the files it imported before it failed.
#[derive(Debug)]
pub(crate) struct Refused {
    pub(crate) problems: Vec<ConfigProblem>,
    pub(crate) inputs: Vec<PathBuf>,
}

/// Loads the config at `path`: JSON directly, and TypeScript or JavaScript through Node, whose
/// process comes back with the config when a rule uses a plugin condition.
pub(crate) fn load(path: &Path) -> Result<Loaded, Refused> {
    let refused = |problems| Refused {
        problems,
        inputs: Vec::new(),
    };
    if archangel_config::is_json(path) {
        return archangel_config::load_json(path)
            .map(|config| Loaded {
                config,
                process: None,
                inputs: Vec::new(),
            })
            .map_err(refused);
    }
    let (mut process, answer) = Process::start(path).map_err(|problem| refused(vec![problem]))?;
    let inputs: Vec<PathBuf> = answer["files"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(|file| dunce::canonicalize(file).unwrap_or_else(|_| PathBuf::from(file)))
        .collect();
    let config = match archangel_config::from_loader(&answer) {
        Ok(config) => config,
        Err(problems) => {
            process.close();
            return Err(Refused { problems, inputs });
        }
    };
    let process = if config.uses_plugins() {
        Some(process)
    } else {
        process.close();
        None
    };
    Ok(Loaded {
        config,
        process,
        inputs,
    })
}

/// The running Node process.
#[derive(Debug)]
pub(crate) struct Process {
    child: Child,
    /// The queue of messages to standard input; dropping it closes standard input.
    writer: Option<mpsc::Sender<String>>,
    lines: Receiver<PluginAnswer>,
    printed: Arc<Mutex<Vec<String>>>,
    stderr: Option<JoinHandle<()>>,
}

impl Process {
    /// Starts Node with the loader, and returns its first answer.
    fn start(config: &Path) -> Result<(Self, Value), ConfigProblem> {
        let version = Command::new("node")
            .arg("--version")
            .output()
            .map_err(|_| archangel_config::node_missing())?;
        if let Some(problem) =
            archangel_config::node_version_problem(&String::from_utf8_lossy(&version.stdout))
        {
            return Err(problem);
        }
        let mut child = Command::new("node")
            .args([
                "--no-warnings",
                "--input-type=module",
                "-e",
                archangel_config::LOADER,
            ])
            .arg(config)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| archangel_config::node_missing())?;
        let (Some(mut stdin), Some(stdout), Some(stderr)) =
            (child.stdin.take(), child.stdout.take(), child.stderr.take())
        else {
            child.kill().ok();
            child.wait().ok();
            return Err(archangel_config::load_stopped(
                "archangel could not connect to Node's standard input and output",
            ));
        };
        let (writer, queue) = mpsc::channel::<String>();
        // Each thread ends when its pipe closes.
        std::thread::spawn(move || {
            for message in queue {
                if stdin.write_all(message.as_bytes()).is_err() {
                    break;
                }
            }
        });
        let (sender, lines) = unbounded();
        std::thread::spawn(move || read_answers(stdout, &sender));
        let printed = Arc::new(Mutex::new(Vec::new()));
        let shared = Arc::clone(&printed);
        let stderr = std::thread::spawn(move || read_lines(stderr, &shared));
        let mut process = Self {
            child,
            writer: Some(writer),
            lines,
            printed,
            stderr: Some(stderr),
        };
        if let Ok(PluginAnswer(Line::Message(answer))) = process.lines.recv() {
            Ok((process, answer))
        } else {
            process.close();
            Err(archangel_config::load_stopped(&process.printed_text()))
        }
    }

    fn send(&self, message: &Value) {
        if let Some(writer) = &self.writer {
            // A process that stopped says so on its standard output, which the protocol reads.
            writer.send(format!("{message}\n")).ok();
        }
    }

    /// Closes standard input and waits for the process to exit, and for what it printed.
    fn close(&mut self) {
        self.writer = None;
        self.child.wait().ok();
        if let Some(thread) = self.stderr.take() {
            thread.join().ok();
        }
    }

    /// Stops the process at once, as the `engine::plugin_dies` fault does.
    fn kill(&mut self) {
        self.child.kill().ok();
    }

    /// The last lines the process printed on standard error.
    fn printed_text(&self) -> String {
        let printed = self.printed.lock().unwrap_or_else(PoisonError::into_inner);
        let from = printed.len().saturating_sub(QUOTED_LINES);
        printed[from..].join("\n")
    }

    /// Takes the lines printed since the last call.
    fn take_printed(&self) -> Vec<String> {
        std::mem::take(&mut *self.printed.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if self.writer.take().is_some() {
            self.child.kill().ok();
            self.child.wait().ok();
        }
    }
}

/// Reads the process's answers, one per line, and then says that it closed its output.
fn read_answers(stdout: impl Read, sender: &crossbeam_channel::Sender<PluginAnswer>) {
    for line in BufReader::new(stdout).lines() {
        let Ok(text) = line else { break };
        let message = serde_json::from_str(&text)
            .unwrap_or_else(|_| json!({ "type": "printed", "text": text }));
        if sender.send(PluginAnswer(Line::Message(message))).is_err() {
            return;
        }
    }
    sender.send(PluginAnswer(Line::Closed)).ok();
}

fn read_lines(stream: impl Read, printed: &Mutex<Vec<String>>) {
    for line in BufReader::new(stream).lines() {
        let Ok(line) = line else { break };
        printed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(line);
    }
}

/// What the engine knows of a session's files, which the records come from.
#[derive(Clone, Copy)]
pub(crate) struct Files<'a> {
    pub(crate) architecture: &'a Architecture,
    pub(crate) graph: &'a ImportGraph,
    pub(crate) classes: &'a Classes,
}

/// The plugin process of a session, the records it was sent, and what each plugin rule found.
#[derive(Debug, Default)]
pub(crate) struct Plugins {
    process: Option<Process>,
    /// The record of each file in some plugin rule's `files`, as last sent, with those rules.
    sent: BTreeMap<String, (Vec<usize>, FileRecord)>,
    /// What each plugin rule found in its last run, by its index in the config.
    results: BTreeMap<usize, Vec<Diagnostic>>,
    /// The id of the check the process is running, and its rules.
    pending: Option<(u64, BTreeSet<usize>)>,
    checks: u64,
    /// The failures of a process that stopped, until one starts again and answers.
    stopped: Vec<Diagnostic>,
    /// What the process printed, kept from processes that stopped.
    printed: Vec<String>,
}

impl Plugins {
    /// The plugins of a freshly loaded config, with its process.
    pub(crate) fn new(process: Option<Process>, architecture: &Architecture) -> Self {
        let mut plugins = Self::default();
        plugins.replace(process, architecture);
        plugins
    }

    /// Uses a new process, after the config loaded again: nothing is sent to it yet.
    pub(crate) fn replace(&mut self, process: Option<Process>, architecture: &Architecture) {
        if let Some(old) = self.process.take() {
            self.printed.extend(old.take_printed());
        }
        self.process = process;
        self.sent.clear();
        self.results.clear();
        self.pending = None;
        self.stopped.clear();
        if let Some(process) = &self.process {
            let rules: Vec<Value> = architecture
                .plugin_rules()
                .map(|(rule, used)| {
                    json!({ "rule": rule, "plugin": used.plugin, "condition": used.condition,
                            "options": used.options })
                })
                .collect();
            process.send(&json!({ "type": "rules", "rules": rules }));
        }
    }

    /// Whether the config has plugin rules but no process runs them, because it stopped.
    pub(crate) fn needs_start(&self, architecture: &Architecture) -> bool {
        self.process.is_none() && architecture.plugin_rules().next().is_some()
    }

    /// Sends every record and runs every plugin rule, after a full check, a new config, or a new
    /// process.
    pub(crate) fn check_all(&mut self, files: Files<'_>, work: &mut Work) {
        if self.process.is_none() {
            return;
        }
        self.sync(files, None, work);
        let all = files
            .architecture
            .plugin_rules()
            .map(|(rule, _)| rule)
            .collect();
        self.dispatch(all, work);
    }

    /// Sends the records of `paths` that changed, and runs the plugin rules whose records did.
    pub(crate) fn check_changed(
        &mut self,
        files: Files<'_>,
        paths: &BTreeSet<String>,
        work: &mut Work,
    ) {
        if self.process.is_none() {
            return;
        }
        let affected = self.sync(files, Some(paths), work);
        self.dispatch(affected, work);
    }

    /// Sends the records of `paths`, or of every file, that differ from those last sent, and
    /// returns the plugin rules whose records changed.
    fn sync(
        &mut self,
        files: Files<'_>,
        paths: Option<&BTreeSet<String>>,
        work: &mut Work,
    ) -> BTreeSet<usize> {
        let every = send_every_file();
        let all_reads = files
            .architecture
            .plugin_rules()
            .fold(Reads::default(), |reads, (_, used)| reads.union(used.reads));
        let scope: BTreeSet<String> = match paths {
            Some(paths) => paths.clone(),
            None => files
                .graph
                .files()
                .iter()
                .map(|file| file.path.clone())
                .chain(self.sent.keys().cloned())
                .collect(),
        };
        let mut affected = BTreeSet::new();
        for path in scope {
            let place = files.architecture.plugin_place(files.classes, &path);
            let rules = place.rules.clone();
            let reads = if every {
                all_reads
            } else {
                reads_of(files.architecture, &rules)
            };
            let now = (every || !rules.is_empty())
                .then(|| view::record(files.graph, &path, place, reads))
                .flatten();
            let before = self.sent.get(&path);
            match (before, now) {
                (Some((_, old)), Some(new)) if *old == new => {}
                (before, Some(new)) => {
                    affected.extend(before.into_iter().flat_map(|(old, _)| old.iter().copied()));
                    affected.extend(rules.iter().copied());
                    self.send_record(&rules, &new, work);
                    self.sent.insert(path, (rules, new));
                }
                (Some((old, _)), None) => {
                    affected.extend(old.iter().copied());
                    self.send_removal(&path, work);
                    self.sent.remove(&path);
                }
                (None, None) => {}
            }
        }
        affected
    }

    fn send_record(&self, rules: &[usize], record: &FileRecord, work: &mut Work) {
        let Some(process) = &self.process else {
            return;
        };
        let mut record = serde_json::to_value(record).unwrap_or(Value::Null);
        if view_gains_a_field() {
            record["fieldFromALaterArchangel"] = json!({ "unknown": true });
        }
        process.send(&json!({ "type": "file", "rules": rules, "record": record }));
        work.plugin_records_sent += 1;
    }

    fn send_removal(&self, path: &str, work: &mut Work) {
        if let Some(process) = &self.process {
            process.send(&json!({ "type": "remove", "path": path }));
            work.plugin_records_sent += 1;
        }
    }

    /// Asks the process to run `rules`, with the rules of a check it has not answered yet, whose
    /// answer is then no longer wanted.
    fn dispatch(&mut self, mut rules: BTreeSet<usize>, work: &mut Work) {
        if rules.is_empty() {
            return;
        }
        if let Some((_, waiting)) = self.pending.take() {
            rules.extend(waiting);
        }
        let Some(process) = self.process.as_mut() else {
            return;
        };
        if plugin_dies() {
            process.kill();
        }
        self.checks += 1;
        process.send(&json!({ "type": "check", "id": self.checks, "rules": rules }));
        work.plugin_checks += rules.len() as u64;
        self.pending = Some((self.checks, rules));
    }

    /// The plugins of the rules the process is running now, or `None` when it runs none.
    pub(crate) fn running(&self, architecture: &Architecture) -> Option<Vec<String>> {
        let (_, rules) = self.pending.as_ref()?;
        let mut names: Vec<String> = architecture
            .plugin_rules()
            .filter(|(rule, _)| rules.contains(rule))
            .map(|(_, used)| used.plugin.clone())
            .collect();
        names.dedup();
        Some(names)
    }

    /// Where the process's answers arrive, while it runs.
    pub(crate) fn answers(&self) -> Option<Receiver<PluginAnswer>> {
        self.process.as_ref().map(|process| process.lines.clone())
    }

    /// Waits for the answer to the check the process is running.
    pub(crate) fn wait(&mut self, files: Files<'_>) {
        while self.pending.is_some() {
            let Some(answer) = self.process.as_ref().map(|p| p.lines.recv()) else {
                self.pending = None;
                return;
            };
            let answer = answer.unwrap_or(PluginAnswer(Line::Closed));
            self.take(answer, files);
        }
    }

    /// Whether a folder holds a file that was sent for the rule.
    fn holds(&self, rule: usize, folder: &str) -> bool {
        self.sent
            .iter()
            .any(|(path, (rules, _))| rules.contains(&rule) && inside(path, folder))
    }

    /// Takes one answer from the process. Returns whether the results changed.
    pub(crate) fn take(&mut self, answer: PluginAnswer, files: Files<'_>) -> bool {
        match answer.0 {
            Line::Message(message) if message["type"] == "printed" => {
                let text = message["text"].as_str().unwrap_or_default();
                self.printed.push(format!("[standard output] {text}"));
                false
            }
            Line::Message(message) => {
                let wanted = self.pending.as_ref().map(|(id, _)| *id);
                if message["type"] != "result" || message["id"].as_u64() != wanted {
                    return false;
                }
                self.pending = None;
                self.stopped.clear();
                let in_view = |rule: usize, path: &str| {
                    self.sent
                        .get(path)
                        .is_some_and(|(rules, _)| rules.contains(&rule))
                };
                let holds = |rule: usize, folder: &str| self.holds(rule, folder);
                let file_of = |path: &str| files.graph.file(path);
                let seen = Seen {
                    architecture: files.architecture,
                    in_view: &in_view,
                    holds: &holds,
                    file_of: &file_of,
                };
                let found = results(&seen, &message);
                self.results.extend(found);
                true
            }
            Line::Closed => {
                self.stopped(files.architecture);
                true
            }
        }
    }

    /// Records that the process stopped: one failure for each plugin it was running, or for
    /// each plugin when it ran none, and no results, until a process starts again.
    fn stopped(&mut self, architecture: &Architecture) {
        let pending = self.pending.take().map(|(_, rules)| rules);
        let printed = self
            .process
            .take()
            .map(|mut process| {
                process.close();
                let text = process.printed_text();
                self.printed.extend(process.take_printed());
                text
            })
            .unwrap_or_default();
        let mut seen = BTreeSet::new();
        self.stopped = architecture
            .plugin_rules()
            .filter(|(rule, _)| pending.as_ref().is_none_or(|rules| rules.contains(rule)))
            .filter(|(_, used)| seen.insert(used.plugin.clone()))
            .map(|(rule, _)| {
                let name = &architecture.config().rules[rule].name;
                let what = if printed.is_empty() {
                    format!("the plugin process stopped while it ran the rule \"{name}\"")
                } else {
                    format!(
                        "the plugin process stopped while it ran the rule \"{name}\"; it printed:\n{printed}"
                    )
                };
                failure(architecture, rule, &what)
            })
            .collect();
        self.results.clear();
        self.sent.clear();
    }

    /// The records last sent, in path order.
    pub(crate) fn records(&self) -> Vec<&FileRecord> {
        self.sent.values().map(|(_, record)| record).collect()
    }

    /// The plugin rules' violations and failures.
    pub(crate) fn diagnostics(&self) -> Vec<Diagnostic> {
        self.results
            .values()
            .flatten()
            .chain(&self.stopped)
            .cloned()
            .collect()
    }

    /// Closes the process and waits for it to exit, so that everything it printed is read.
    pub(crate) fn close(&mut self) {
        if let Some(mut process) = self.process.take() {
            process.close();
            self.printed.extend(process.take_printed());
        }
    }

    /// What the process printed since the last call, each line starting with the name of the
    /// plugin, or `config`, that printed it, or with `standard output` for a line written there.
    pub(crate) fn take_printed(&mut self) -> Vec<String> {
        let mut printed = std::mem::take(&mut self.printed);
        if let Some(process) = &self.process {
            printed.extend(process.take_printed());
        }
        printed
    }

    /// Runs a plugin rule over an example: sends the records of the example's files in the rule's
    /// `files`, runs the rule, and sends the records they replaced back.
    pub(crate) fn example(
        &mut self,
        files: Files<'_>,
        example: PluginExample<'_>,
        work: &mut Work,
    ) -> PluginFindings {
        self.wait(files);
        let shown = if self.process.is_some() {
            self.send_example(files, example, work)
        } else {
            Vec::new()
        };
        if self.process.is_some() && shown.is_empty() {
            return PluginFindings::default();
        }
        self.dispatch(BTreeSet::from([example.rule]), work);
        let Some(message) = self.answer() else {
            self.stopped(files.architecture);
            return PluginFindings {
                diagnostics: self.stopped.clone(),
                considered: true,
            };
        };
        for path in &shown {
            match self.sent.get(path) {
                Some((rules, record)) => self.send_record(rules, record, work),
                None => self.send_removal(path, work),
            }
        }
        let in_view = |found: usize, path: &str| {
            found == example.rule
                && (shown.iter().any(|p| p == path)
                    || self
                        .sent
                        .get(path)
                        .is_some_and(|(rules, _)| rules.contains(&found)))
        };
        let holds = |found: usize, folder: &str| {
            found == example.rule
                && (shown.iter().any(|p| inside(p, folder)) || self.holds(found, folder))
        };
        let file_of = |path: &str| {
            let overlay = example.overlay.files.iter();
            overlay
                .clone()
                .find(|file| file.path == path)
                .or_else(|| files.graph.file(path))
        };
        let seen = Seen {
            architecture: files.architecture,
            in_view: &in_view,
            holds: &holds,
            file_of: &file_of,
        };
        PluginFindings {
            diagnostics: results(&seen, &message).into_values().flatten().collect(),
            considered: true,
        }
    }

    /// Sends the records of an example's files that are in the rule's `files`, and returns their
    /// paths.
    fn send_example(
        &mut self,
        files: Files<'_>,
        example: PluginExample<'_>,
        work: &mut Work,
    ) -> Vec<String> {
        let reads = reads_of(files.architecture, &[example.rule]);
        let mut shown = Vec::new();
        let overlay = example.overlay;
        for (file, resolutions) in overlay.files.iter().zip(&overlay.resolutions) {
            let place = files.architecture.example_plugin_place(file, work);
            if !place.rules.contains(&example.rule) {
                continue;
            }
            let rules = place.rules.clone();
            let importers = || {
                let found = overlay.importers(files.graph, &file.path);
                found.into_iter().map(str::to_owned).collect()
            };
            let source = view::Source { file, resolutions };
            let record = view::record_of(source, importers, place, reads);
            self.send_record(&rules, &record, work);
            shown.push(file.path.clone());
        }
        shown
    }

    /// Waits for the answer to the check the process runs, or `None` when the process stopped.
    fn answer(&mut self) -> Option<Value> {
        while let Some((id, _)) = self.pending {
            let Some(Ok(PluginAnswer(line))) = self.process.as_ref().map(|p| p.lines.recv()) else {
                return None;
            };
            match line {
                Line::Message(found) if found["id"].as_u64() == Some(id) => {
                    self.pending = None;
                    return Some(found);
                }
                Line::Message(_) => {}
                Line::Closed => return None,
            }
        }
        None
    }
}

/// An example's files, laid over the repository, and the plugin rule it is an example of.
#[derive(Clone, Copy)]
pub(crate) struct PluginExample<'a> {
    pub(crate) overlay: &'a OverlayFiles,
    pub(crate) rule: usize,
}

/// What a plugin's answer is judged against: which files each rule's view holds, and the facts
/// of each file.
struct Seen<'s, 'g> {
    architecture: &'s Architecture,
    in_view: &'s dyn Fn(usize, &str) -> bool,
    /// Whether a folder holds a file of a rule's view, so a violation may name the folder.
    holds: &'s dyn Fn(usize, &str) -> bool,
    file_of: &'s dyn Fn(&str) -> Option<&'g SourceFile>,
}

/// Whether a path is inside a folder.
fn inside(path: &str, folder: &str) -> bool {
    path.strip_prefix(folder)
        .is_some_and(|rest| rest.starts_with('/'))
}

/// The facts the plugin rules read, together.
fn reads_of(architecture: &Architecture, rules: &[usize]) -> Reads {
    architecture
        .plugin_rules()
        .filter(|(rule, _)| rules.contains(rule))
        .fold(Reads::default(), |reads, (_, used)| reads.union(used.reads))
}

/// Each rule's diagnostics from a `result` message: its violations, or one failure when the
/// condition threw or reported what its view does not hold.
fn results(seen: &Seen<'_, '_>, message: &Value) -> BTreeMap<usize, Vec<Diagnostic>> {
    let architecture = seen.architecture;
    let mut found = BTreeMap::new();
    for result in message["rules"].as_array().into_iter().flatten() {
        let Some(rule) = result["rule"]
            .as_u64()
            .and_then(|rule| usize::try_from(rule).ok())
            .filter(|rule| architecture.config().rules.get(*rule).is_some())
        else {
            continue;
        };
        let diagnostics = if let Some(error) = result["error"].as_str() {
            vec![failure(
                architecture,
                rule,
                &format!("the condition threw `{error}`"),
            )]
        } else {
            let converted: Result<Vec<Diagnostic>, String> = result["violations"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|violation| plugin_violation(seen, rule, violation))
                .collect();
            converted.unwrap_or_else(|what| vec![failure(architecture, rule, &what)])
        };
        found.insert(rule, diagnostics);
    }
    found
}

/// One violation a condition reported, as a diagnostic, or what is wrong with it.
fn plugin_violation(
    seen: &Seen<'_, '_>,
    rule: usize,
    violation: &Value,
) -> Result<Diagnostic, String> {
    let architecture = seen.architecture;
    let Some(file) = violation["file"].as_str() else {
        return Err(format!(
            "the condition reported a violation without a file: {violation}"
        ));
    };
    let folder = !(seen.in_view)(rule, file);
    if folder && !(seen.holds)(rule, file) {
        return Err(format!(
            "the condition reported a violation for {file}, which is not in its view, nor a \
             folder that holds a file of it"
        ));
    }
    let config_rule = &architecture.config().rules[rule];
    let message = violation["message"].as_str().unwrap_or_default();
    let mut diagnostic = Diagnostic::new(Code::Plugin, message);
    diagnostic.rule = Some(config_rule.name.clone());
    diagnostic.reason.clone_from(&config_rule.reason);
    diagnostic.link = config_rule
        .link
        .clone()
        .or_else(|| docs_of(architecture, rule));
    let named = &violation["import"];
    let specifier = named.as_str().or_else(|| named["specifier"].as_str());
    if folder && specifier.is_some() {
        return Err(format!(
            "the condition reported a violation of the folder {file} with an import, and only a \
             file has imports"
        ));
    }
    let Some(specifier) = specifier else {
        diagnostic.file = Some(file.to_owned());
        diagnostic.id = Some(ViolationId::Path {
            rule: config_rule.name.clone(),
            path: file.to_owned(),
        });
        return Ok(diagnostic);
    };
    let occurrence = named["occurrence"].as_u64().unwrap_or(0);
    let id = ViolationId::Import {
        rule: config_rule.name.clone(),
        file: file.to_owned(),
        specifier: specifier.to_owned(),
        occurrence: u32::try_from(occurrence).unwrap_or(u32::MAX),
    };
    at_import(seen, diagnostic, id)
}

/// A violation of an import, at the import it names, or what is wrong with that.
fn at_import(
    seen: &Seen<'_, '_>,
    diagnostic: Diagnostic,
    id: ViolationId,
) -> Result<Diagnostic, String> {
    let ViolationId::Import {
        file,
        specifier,
        occurrence,
        ..
    } = &id
    else {
        return Ok(diagnostic);
    };
    let import = (seen.file_of)(file)
        .and_then(|source| source.facts.as_ref())
        .and_then(|facts| {
            facts
                .imports
                .iter()
                .filter(|import| import.specifier == *specifier)
                .nth(usize::try_from(*occurrence).unwrap_or(usize::MAX))
        })
        .ok_or_else(|| {
            format!(
                "the condition reported import {occurrence} of `{specifier}` in {file}, which has \
                 no such import"
            )
        })?;
    let mut diagnostic = diagnostic.at(file, import.line, import.column);
    diagnostic.span = Some(import.span);
    diagnostic.utf16 = Some(import.utf16);
    diagnostic.specifier = Some(specifier.clone());
    diagnostic.id = Some(id);
    Ok(diagnostic)
}

/// The documentation link of a plugin rule's condition.
fn docs_of(architecture: &Architecture, rule: usize) -> Option<String> {
    let used = architecture.config().rules[rule].plugin_use()?;
    architecture
        .config()
        .plugin(&used.plugin)?
        .conditions
        .iter()
        .find(|condition| condition.name == used.condition)?
        .docs
        .clone()
}

/// The `plugin-failed` diagnostic of a plugin rule, saying what happened.
fn failure(architecture: &Architecture, rule: usize, what: &str) -> Diagnostic {
    let config_rule = &architecture.config().rules[rule];
    let (plugin, condition) = config_rule
        .plugin_use()
        .map(|used| (used.plugin.clone(), used.full_name()))
        .unwrap_or_default();
    let message = format!(
        "the plugin \"{plugin}\" failed in its condition `{condition}` for the rule \"{}\": {what}. \
         The check fails, because a rule whose code did not run proves nothing; turn the rule off \
         with `off`, or report this to the plugin's author",
        config_rule.name
    );
    let mut diagnostic = Diagnostic::new(Code::PluginFailed, message);
    diagnostic.rule = Some(config_rule.name.clone());
    diagnostic
}

/// The `engine::plugin_dies` fail point: stops the plugin process before a check.
fn plugin_dies() -> bool {
    fail::fail_point!("engine::plugin_dies", |_| true);
    false
}

/// The `engine::send_every_file` fail point: sends a record of every file, whatever the filters.
fn send_every_file() -> bool {
    fail::fail_point!("engine::send_every_file", |_| true);
    false
}

/// The `engine::view_gains_a_field` fail point: adds a field to every record that no plugin
/// knows, as a later archangel would.
fn view_gains_a_field() -> bool {
    fail::fail_point!("engine::view_gains_a_field", |_| true);
    false
}
