//! Archangel's language server: only the mapping between protocol messages and `Session`
//! changes, and from diagnostics to the protocol's diagnostics and quick fixes. The engine decides
//! everything else.
//!
//! One loop on one thread, as rust-analyzer's: each turn takes every message already waiting,
//! keeps the newest text of each document, merges the file events, and applies them as one change.
//!
//! This crate is part of [archangel](https://crates.io/crates/archangel) and is published so the
//! `archangel` command can be installed with Cargo. Its API follows the command's needs and may
//! change in any release; depend on the command, its JSON output or its language server instead.

mod convert;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use archangel_engine::{Applied, Batch, Change, PathEvent, PluginAnswer, Session, Watcher};
use crossbeam_channel::{Receiver, never, select, unbounded};
use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::notification::{self as note, Notification as _};
use lsp_types::request::{self as req, Request as _};
use lsp_types::{
    CodeActionProviderCapability, InitializeParams, InitializeResult, MessageType, ProgressParams,
    ProgressParamsValue, ProgressToken, PublishDiagnosticsParams, ServerCapabilities, ServerInfo,
    ShowMessageParams, TextDocumentSyncCapability, TextDocumentSyncKind, WorkDoneProgress,
    WorkDoneProgressBegin, WorkDoneProgressCreateParams, WorkDoneProgressEnd,
};
use serde_json::{Value, json};

/// The custom request that returns the work counts so far, for editors and tests.
pub const STATS: &str = "archangel/stats";

const WATCH_ID: &str = "archangel-files";

/// Serves one editor over `connection` until it shuts the server down.
///
/// # Errors
///
/// Returns the reason when the connection fails.
pub fn serve(connection: &Connection) -> Result<(), String> {
    let (id, raw) = connection.initialize_start().map_err(|e| e.to_string())?;
    let params: InitializeParams =
        serde_json::from_value(raw.clone()).map_err(|e| e.to_string())?;
    let result = InitializeResult {
        capabilities: ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
            ..ServerCapabilities::default()
        },
        server_info: Some(ServerInfo {
            name: "archangel".to_owned(),
            version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        }),
    };
    let result = serde_json::to_value(result).map_err(|e| e.to_string())?;
    connection
        .initialize_finish(id, result)
        .map_err(|e| e.to_string())?;
    let root = convert::root(&raw).unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    Server::new(connection, &root, &params).serve()
}

/// The server's state between messages.
struct Server<'c> {
    connection: &'c Connection,
    /// The workspace folder the editor opened.
    root: PathBuf,
    session: Option<Session>,
    /// The files whose diagnostics the editor shows now, so a change can clear them.
    published: BTreeSet<String>,
    /// The version of each open document.
    versions: BTreeMap<String, i32>,
    /// Batches from the server's own watcher, when the editor watches nothing.
    watched: Receiver<Batch>,
    /// Whether the editor shows progress that the server starts.
    progress: bool,
    /// The token of the progress message shown while the plugin process works.
    working: Option<ProgressToken>,
    /// How many progress messages the server has started, which names the next token.
    progresses: u32,
}

impl<'c> Server<'c> {
    fn new(connection: &'c Connection, root: &Path, params: &InitializeParams) -> Self {
        let editor_watches = params
            .capabilities
            .workspace
            .as_ref()
            .and_then(|w| w.did_change_watched_files.as_ref())
            .and_then(|c| c.dynamic_registration)
            .unwrap_or(false);
        let progress = params
            .capabilities
            .window
            .as_ref()
            .and_then(|w| w.work_done_progress)
            .unwrap_or(false);
        let session = Session::open(root, None);
        let mut server = Self {
            connection,
            root: root.to_path_buf(),
            session: None,
            published: BTreeSet::new(),
            versions: BTreeMap::new(),
            watched: never(),
            progress,
            working: None,
            progresses: 0,
        };
        if editor_watches {
            server.register_watches();
        } else {
            let watch_root = session
                .as_ref()
                .map_or_else(|_| root.to_path_buf(), |s| s.config_dir().to_path_buf());
            server.watch_myself(&watch_root);
        }
        match session {
            Ok(session) => {
                server.session = Some(session);
                server.publish_all();
                server.show_progress();
            }
            Err(stop) => server.show(MessageType::ERROR, &stop_message(&stop.diagnostics)),
        }
        server
    }

    fn serve(mut self) -> Result<(), String> {
        while let Some(first) = self.next_incoming() {
            let mut turn = Turn::default();
            let rest = if is_shutdown(&first) {
                Vec::new()
            } else {
                self.waiting()
            };
            for incoming in std::iter::once(first).chain(rest) {
                if self.take(incoming, &mut turn)? {
                    return Ok(());
                }
            }
            self.apply(turn);
        }
        Ok(())
    }

    /// The next message from the editor, batch from the watcher, or answer of the plugin process;
    /// `None` when the editor is gone.
    fn next_incoming(&self) -> Option<Incoming> {
        let answers = self
            .session
            .as_ref()
            .and_then(Session::plugin_answers)
            .unwrap_or_else(never);
        select! {
            recv(self.connection.receiver) -> message => message.ok().map(Incoming::Editor),
            recv(self.watched) -> batch => Some(batch.map_or(Incoming::WatcherStopped, Incoming::Watcher)),
            recv(answers) -> answer => Some(Incoming::Plugins(answer.unwrap_or_else(|_| PluginAnswer::stopped()))),
        }
    }

    /// Every message and batch already waiting, so that one turn handles them together. The turn
    /// ends at a shutdown request, because `handle_shutdown` reads the exit notification after it
    /// from the connection itself.
    fn waiting(&self) -> Vec<Incoming> {
        let mut found = Vec::new();
        for message in self.connection.receiver.try_iter() {
            let shutdown =
                matches!(&message, Message::Request(request) if request.method == "shutdown");
            found.push(Incoming::Editor(message));
            if shutdown {
                return found;
            }
        }
        found.extend(self.watched.try_iter().map(Incoming::Watcher));
        found
    }

    /// Adds one message to the turn, or answers it. Returns true when the editor shut down.
    fn take(&mut self, incoming: Incoming, turn: &mut Turn) -> Result<bool, String> {
        match incoming {
            Incoming::Watcher(Batch::Events(events)) => turn.events.extend(events),
            Incoming::Watcher(Batch::Lost) => turn.reload = true,
            Incoming::WatcherStopped => {
                self.watched = never();
                self.show(
                    MessageType::ERROR,
                    "archangel's file watcher stopped, so changes on disk no longer show; restart the server",
                );
            }
            Incoming::Editor(Message::Request(request)) => {
                if self
                    .connection
                    .handle_shutdown(&request)
                    .map_err(|e| e.to_string())?
                {
                    return Ok(true);
                }
                // A request sees the effect of every change before it.
                self.apply(std::mem::take(turn));
                self.answer(&request);
            }
            Incoming::Editor(Message::Notification(notification)) => {
                self.notification(notification, turn);
            }
            Incoming::Editor(Message::Response(_)) => {}
            Incoming::Plugins(answer) => {
                // The answer is for changes the server already applied, so the turn's own wait.
                self.apply(std::mem::take(turn));
                if let Some(session) = self.session.as_mut() {
                    let applied = session.receive_plugins(answer);
                    self.publish_changed(&applied);
                }
                self.show_progress();
            }
        }
        Ok(false)
    }

    fn notification(&mut self, notification: Notification, turn: &mut Turn) {
        let method = notification.method.as_str();
        match method {
            note::DidOpenTextDocument::METHOD => {
                if let Ok(p) = serde_json::from_value::<lsp_types::DidOpenTextDocumentParams>(
                    notification.params,
                ) {
                    let document = Document {
                        uri: p.text_document.uri.as_str(),
                        version: p.text_document.version,
                        text: Some(p.text_document.text),
                    };
                    self.edit(turn, document);
                }
            }
            note::DidChangeTextDocument::METHOD => {
                if let Ok(p) = serde_json::from_value::<lsp_types::DidChangeTextDocumentParams>(
                    notification.params,
                ) && let Some(change) = p.content_changes.into_iter().last()
                {
                    let document = Document {
                        uri: p.text_document.uri.as_str(),
                        version: p.text_document.version,
                        text: Some(change.text),
                    };
                    self.edit(turn, document);
                }
            }
            note::DidCloseTextDocument::METHOD => {
                if let Ok(p) = serde_json::from_value::<lsp_types::DidCloseTextDocumentParams>(
                    notification.params,
                ) {
                    self.edit(
                        turn,
                        Document {
                            uri: p.text_document.uri.as_str(),
                            version: 0,
                            text: None,
                        },
                    );
                }
            }
            note::DidChangeWatchedFiles::METHOD => {
                if let Ok(p) = serde_json::from_value::<lsp_types::DidChangeWatchedFilesParams>(
                    notification.params,
                ) {
                    turn.events
                        .extend(p.changes.iter().filter_map(convert::file_event));
                }
            }
            _ => {}
        }
    }

    /// Records the newest text of a document for this turn; an earlier text in the turn is dropped.
    fn edit(&mut self, turn: &mut Turn, document: Document<'_>) {
        let Some(path) = self.relative(document.uri) else {
            return;
        };
        if document.text.is_some() {
            self.versions.insert(path.clone(), document.version);
        } else {
            self.versions.remove(&path);
        }
        turn.texts.insert(path, document.text);
    }

    fn relative(&self, uri: &str) -> Option<String> {
        let root = self.session.as_ref()?.graph().root();
        convert::workspace_relative(root, &convert::uri_path(uri)?)
    }

    fn apply(&mut self, turn: Turn) {
        if turn.is_empty() {
            return;
        }
        let Some(session) = self.session.as_mut() else {
            self.retry_open();
            return;
        };
        let mut changes: Vec<Change> = turn
            .texts
            .into_iter()
            .map(|(path, text)| Change::Text { path, text })
            .collect();
        if !turn.events.is_empty() {
            changes.push(Change::Paths(turn.events));
        }
        let mut applied = session.apply(changes);
        if turn.reload {
            let reloaded = session.reload();
            applied.changed.extend(reloaded.changed);
        }
        self.publish_changed(&applied);
        self.show_progress();
    }

    /// Starts a progress message that names the plugins while the plugin process works, and ends
    /// it when the process has answered. What the config and the plugins printed goes to the
    /// editor's log first.
    fn show_progress(&mut self) {
        let printed = self
            .session
            .as_mut()
            .map(Session::take_plugin_output)
            .unwrap_or_default();
        if !printed.is_empty() {
            let params = lsp_types::LogMessageParams {
                typ: MessageType::LOG,
                message: printed.join("\n"),
            };
            self.send(Notification::new(note::LogMessage::METHOD.to_owned(), params).into());
        }
        let running = self.session.as_ref().and_then(Session::plugins_running);
        match (&self.working, running) {
            (None, Some(plugins)) if self.progress => {
                self.progresses += 1;
                let token = ProgressToken::String(format!("archangel-plugins-{}", self.progresses));
                let create = Request::new(
                    format!("archangel-progress-{}", self.progresses).into(),
                    req::WorkDoneProgressCreate::METHOD.to_owned(),
                    WorkDoneProgressCreateParams {
                        token: token.clone(),
                    },
                );
                self.send(create.into());
                let names: Vec<String> = plugins.iter().map(|p| format!("\"{p}\"")).collect();
                let begin = WorkDoneProgress::Begin(WorkDoneProgressBegin {
                    title: format!("archangel is running the plugin {}", names.join(", ")),
                    cancellable: Some(false),
                    message: None,
                    percentage: None,
                });
                self.progress(&token, begin);
                self.working = Some(token);
            }
            (Some(_), None) => {
                if let Some(token) = self.working.take() {
                    self.progress(
                        &token,
                        WorkDoneProgress::End(WorkDoneProgressEnd { message: None }),
                    );
                }
            }
            _ => {}
        }
    }

    fn progress(&self, token: &ProgressToken, value: WorkDoneProgress) {
        let params = ProgressParams {
            token: token.clone(),
            value: ProgressParamsValue::WorkDone(value),
        };
        self.send(Notification::new(note::Progress::METHOD.to_owned(), params).into());
    }

    /// Opens the session again after a change, when the first open failed.
    fn retry_open(&mut self) {
        if let Ok(session) = Session::open(&self.root, None) {
            self.session = Some(session);
            self.publish_all();
        }
    }

    fn answer(&mut self, request: &Request) {
        let result = match request.method.as_str() {
            STATS => self
                .session
                .as_ref()
                .map_or(Value::Null, |s| s.work().counts_json()),
            req::CodeActionRequest::METHOD => {
                serde_json::from_value::<lsp_types::CodeActionParams>(request.params.clone())
                    .ok()
                    .and_then(|params| {
                        let session = self.session.as_ref()?;
                        let path = self.relative(params.text_document.uri.as_str())?;
                        Some(convert::code_actions(session, &path, &params))
                    })
                    .unwrap_or(Value::Array(Vec::new()))
            }
            _ => Value::Null,
        };
        let response = Response::new_ok(request.id.clone(), result);
        self.send(response.into());
    }

    /// Publishes the diagnostics of every file that has some.
    fn publish_all(&mut self) {
        let files: BTreeSet<String> = self
            .session
            .as_ref()
            .map(|s| {
                s.report()
                    .diagnostics
                    .iter()
                    .map(|d| convert::file_of(s, d))
                    .collect()
            })
            .unwrap_or_default();
        for file in files {
            self.publish(&file);
        }
    }

    /// Publishes the files whose diagnostics changed, and clears files that have none now.
    fn publish_changed(&mut self, applied: &Applied) {
        let Some(session) = self.session.as_ref() else {
            return;
        };
        let config = convert::config_name(session);
        let mut files: BTreeSet<String> = applied
            .changed
            .iter()
            .map(|file| {
                if file.is_empty() {
                    config.clone()
                } else {
                    file.clone()
                }
            })
            .collect();
        let now: BTreeSet<String> = session
            .report()
            .diagnostics
            .iter()
            .map(|d| convert::file_of(session, d))
            .collect();
        files.extend(self.published.difference(&now).cloned());
        for file in files {
            self.publish(&file);
        }
    }

    fn publish(&mut self, file: &str) {
        let Some(session) = self.session.as_ref() else {
            return;
        };
        let diagnostics = convert::diagnostics_of(session, file);
        if diagnostics.is_empty() {
            self.published.remove(file);
        } else {
            self.published.insert(file.to_owned());
        }
        let Some(uri) = convert::uri(&session.graph().root().join(file)) else {
            return;
        };
        let params = PublishDiagnosticsParams {
            uri,
            diagnostics,
            version: self.versions.get(file).copied(),
        };
        let notification = Notification::new(note::PublishDiagnostics::METHOD.to_owned(), params);
        self.send(notification.into());
    }

    /// Asks the editor to report every file. The request takes the registration's id, which no
    /// other request of the server has.
    fn register_watches(&self) {
        let registration = json!({
            "registrations": [{
                "id": WATCH_ID,
                "method": note::DidChangeWatchedFiles::METHOD,
                "registerOptions": { "watchers": [{ "globPattern": "**/*" }] },
            }],
        });
        let request = Request::new(
            WATCH_ID.to_owned().into(),
            req::RegisterCapability::METHOD.to_owned(),
            registration,
        );
        self.send(request.into());
    }

    /// Starts the engine's watcher, for an editor that cannot register file watches.
    fn watch_myself(&mut self, root: &Path) {
        let mut watcher = match Watcher::start(root) {
            Ok(watcher) => watcher,
            Err(reason) => {
                self.show(MessageType::ERROR, &reason);
                return;
            }
        };
        if let Some(note) = watcher.polling() {
            self.show(MessageType::WARNING, note);
        }
        let (sender, receiver) = unbounded();
        self.watched = receiver;
        // The watcher lives on its thread until the server stops listening.
        std::thread::spawn(move || {
            while let Some(batch) = watcher.wait() {
                if sender.send(batch).is_err() {
                    break;
                }
            }
        });
    }

    fn show(&self, kind: MessageType, message: &str) {
        let params = ShowMessageParams {
            typ: kind,
            message: message.to_owned(),
        };
        self.send(Notification::new(note::ShowMessage::METHOD.to_owned(), params).into());
    }

    fn send(&self, message: Message) {
        // When the editor closed the connection, the loop ends at its next receive.
        self.connection.sender.send(message).ok();
    }
}

/// A message from the editor, a batch from the server's own watcher, or an answer of the
/// plugin process.
enum Incoming {
    Editor(Message),
    Watcher(Batch),
    WatcherStopped,
    Plugins(PluginAnswer),
}

/// An editor's text for a document, or `None` when it closed the document.
struct Document<'d> {
    uri: &'d str,
    version: i32,
    text: Option<String>,
}

/// The changes of one turn of the loop.
#[derive(Default)]
struct Turn {
    /// The newest text of each document, or `None` for a closed one.
    texts: BTreeMap<String, Option<String>>,
    events: Vec<PathEvent>,
    reload: bool,
}

impl Turn {
    fn is_empty(&self) -> bool {
        self.texts.is_empty() && self.events.is_empty() && !self.reload
    }
}

fn stop_message(diagnostics: &[archangel_rules::Diagnostic]) -> String {
    let lines: Vec<String> = diagnostics
        .iter()
        .map(|d| format!("[{}] {}", d.code.as_str(), d.message))
        .collect();
    format!(
        "archangel cannot check this workspace: {}",
        lines.join("; ")
    )
}

/// Whether the editor asks the server to shut down, after which it sends only the exit notification.
fn is_shutdown(incoming: &Incoming) -> bool {
    matches!(incoming, Incoming::Editor(Message::Request(request)) if request.method == "shutdown")
}
