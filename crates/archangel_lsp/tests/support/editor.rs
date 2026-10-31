//! The editor that the server tests drive: it sends the protocol's messages over an in-memory
//! connection, answers the server's own requests, and waits for answers, never for a duration.
#![cfg(test)]

use std::collections::VecDeque;
use std::path::PathBuf;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use archangel_testkit::FixtureRepo;
use crossbeam_channel::RecvTimeoutError;
use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use serde_json::{Value, json};

// @justified intent
// Every answer takes milliseconds, so only a hung server reaches this bound. It is short because a
// hung server makes every test wait for it, and the run has to fail before cargo-mutants' 20-second
// timeout calls the mutant a timeout instead of caught.
pub(crate) const HUNG: Duration = Duration::from_secs(5);
pub(crate) const MOST_MESSAGES: usize = 100_000;

/// The `file:` URI an editor sends for a path: `file:///C:/…` on Windows, `file:///…` elsewhere.
pub(crate) fn file_uri(path: &std::path::Path) -> String {
    url::Url::from_file_path(path)
        .expect("an absolute path")
        .to_string()
}

/// An editor connected to the server.
pub(crate) struct Editor {
    pub(crate) connection: Connection,
    pub(crate) root: PathBuf,
    pub(crate) next_id: i32,
    /// The notifications that no wait has taken yet.
    pub(crate) waiting: VecDeque<Value>,
    /// The server's own requests, such as a watch registration, each answered with null.
    pub(crate) asked: Vec<Value>,
    /// The server's answer to `initialize`.
    pub(crate) initialized: Value,
    pub(crate) server: Option<JoinHandle<Result<(), String>>>,
}

impl Editor {
    pub(crate) fn start(repo: &FixtureRepo, watches: bool) -> Self {
        Self::start_after(repo, watches, |_| {})
    }

    /// Starts the server after `queue` sends its notifications, so that the server's first turn
    /// finds them all waiting.
    pub(crate) fn start_after(
        repo: &FixtureRepo,
        watches: bool,
        queue: impl FnOnce(&Self),
    ) -> Self {
        let capabilities = json!({
            "workspace": { "didChangeWatchedFiles": { "dynamicRegistration": watches } },
            "window": { "workDoneProgress": true },
        });
        Self::start_with(repo, &capabilities, queue)
    }

    /// Starts the server for an editor with these capabilities.
    pub(crate) fn start_with(
        repo: &FixtureRepo,
        capabilities: &Value,
        queue: impl FnOnce(&Self),
    ) -> Self {
        let (server, connection) = Connection::memory();
        let root = dunce::canonicalize(repo.path()).expect("the root");
        let mut editor = Self {
            connection,
            root,
            next_id: 0,
            waiting: VecDeque::new(),
            asked: Vec::new(),
            initialized: Value::Null,
            server: None,
        };
        let uri = file_uri(&editor.root);
        let id = editor.send(
            "initialize",
            &json!({ "processId": null, "rootUri": uri, "capabilities": capabilities }),
        );
        editor.notify("initialized", &json!({}));
        queue(&editor);
        editor.server = Some(std::thread::spawn(move || archangel_lsp::serve(&server)));
        editor.initialized = editor.answer(&id, "initialize");
        editor
    }

    /// The URI an editor gives a path, which encodes a space as `%20`.
    pub(crate) fn uri(&self, path: &str) -> String {
        file_uri(&self.root.join(path))
    }

    pub(crate) fn notify(&self, method: &str, params: &Value) {
        let message = Notification::new(method.to_owned(), params);
        self.connection
            .sender
            .send(message.into())
            .expect("the server listens");
    }

    pub(crate) fn send(&mut self, method: &str, params: &Value) -> RequestId {
        self.next_id += 1;
        let id = RequestId::from(self.next_id);
        let message = Request::new(id.clone(), method.to_owned(), params);
        self.connection
            .sender
            .send(message.into())
            .expect("the server listens");
        id
    }

    /// Sends a request and waits for its response, keeping the notifications that come first.
    pub(crate) fn request(&mut self, method: &str, params: &Value) -> Value {
        let id = self.send(method, params);
        self.answer(&id, method)
    }

    pub(crate) fn answer(&mut self, id: &RequestId, method: &str) -> Value {
        let deadline = Instant::now() + HUNG;
        let wanted = format!("the response to {method}");
        for _ in 0..MOST_MESSAGES {
            match self.receive(deadline, &wanted) {
                Message::Response(Response {
                    id: answered,
                    response_result,
                }) if answered == *id => {
                    return response_result.unwrap_or(Value::Null);
                }
                Message::Notification(note) => self
                    .waiting
                    .push_back(json!({ "method": note.method, "params": note.params })),
                _ => {}
            }
        }
        panic!("the server sent {MOST_MESSAGES} messages and no response to {method}")
    }

    /// The next message, answering the server's own requests. Fails the test when the server
    /// stops, or sends nothing more before `deadline`.
    pub(crate) fn receive(&mut self, deadline: Instant, wanted: &str) -> Message {
        for _ in 0..MOST_MESSAGES {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.connection.receiver.recv_timeout(left) {
                Ok(Message::Request(request)) => {
                    let reply = Response::new_ok(request.id, Value::Null);
                    self.asked
                        .push(json!({ "method": request.method, "params": request.params }));
                    self.connection
                        .sender
                        .send(reply.into())
                        .expect("the server listens");
                }
                Ok(message) => return message,
                Err(RecvTimeoutError::Timeout) => {
                    panic!("the server hangs: {wanted} did not come within {HUNG:?}")
                }
                Err(RecvTimeoutError::Disconnected) => {
                    // The connection closes only as the server's thread ends, so this join returns.
                    let outcome = self.server.take().map(JoinHandle::join);
                    panic!("the server stopped before {wanted} came: {outcome:?}")
                }
            }
        }
        panic!("the server sent {MOST_MESSAGES} requests before {wanted} came")
    }

    /// Takes the first waiting notification of `method` for which `wanted` is true, and drops the
    /// notifications before it.
    pub(crate) fn take(&mut self, method: &str, wanted: &impl Fn(&Value) -> bool) -> Option<Value> {
        let at = self
            .waiting
            .iter()
            .position(|n| n["method"] == method && wanted(&n["params"]))?;
        let found = self.waiting.remove(at)?;
        self.waiting.drain(..at);
        Some(found["params"].clone())
    }

    /// The first notification of `method` for which `wanted` is true, among those the server
    /// sends before it answers a request sent now.
    pub(crate) fn notified(&mut self, method: &str, wanted: impl Fn(&Value) -> bool) -> Value {
        if let Some(found) = self.take(method, &wanted) {
            return found;
        }
        self.work();
        self.take(method, &wanted).unwrap_or_else(|| {
            panic!(
                "the server answered a later request without the {method} the test wants; it sent {:?}",
                self.waiting
            )
        })
    }

    /// The next notification of `method` for which `wanted` is true, for a change that no request
    /// can follow, such as one that the server's own watcher reports.
    pub(crate) fn until(&mut self, method: &str, wanted: impl Fn(&Value) -> bool) -> Value {
        if let Some(found) = self.take(method, &wanted) {
            return found;
        }
        self.waiting.clear();
        let deadline = Instant::now() + HUNG;
        let looking = format!("the {method} the test wants");
        for _ in 0..MOST_MESSAGES {
            if let Message::Notification(note) = self.receive(deadline, &looking)
                && note.method == method
                && wanted(&note.params)
            {
                return note.params;
            }
        }
        panic!("the server sent {MOST_MESSAGES} messages before {looking}")
    }

    /// Whether published diagnostics are for `path` and `wanted` is true of them.
    pub(crate) fn about<W: Fn(&[Value]) -> bool>(
        &self,
        path: &str,
        wanted: W,
    ) -> impl Fn(&Value) -> bool + use<W> {
        let uri = self.uri(path);
        move |params| {
            params["uri"] == uri.as_str()
                && wanted(params["diagnostics"].as_array().map_or(&[], Vec::as_slice))
        }
    }

    /// The first diagnostics for `path` for which `wanted` is true, among those the server
    /// publishes before it answers a request sent now.
    pub(crate) fn published(&mut self, path: &str, wanted: impl Fn(&[Value]) -> bool) -> Value {
        let about = self.about(path, wanted);
        self.notified("textDocument/publishDiagnostics", about)
    }

    /// The waiting diagnostics for `path`, oldest first.
    pub(crate) fn waiting_for(&self, path: &str) -> Vec<&Value> {
        let uri = self.uri(path);
        self.waiting
            .iter()
            .filter(|n| {
                n["method"] == "textDocument/publishDiagnostics"
                    && n["params"]["uri"] == uri.as_str()
            })
            .map(|n| &n["params"])
            .collect()
    }

    /// The work counts so far.
    pub(crate) fn work(&mut self) -> Value {
        self.request("archangel/stats", &json!({}))
    }

    pub(crate) fn open(&self, path: &str, version: i32, text: &str) {
        self.notify(
            "textDocument/didOpen",
            &json!({ "textDocument": { "uri": self.uri(path), "languageId": "typescript", "version": version, "text": text } }),
        );
    }

    pub(crate) fn change(&self, path: &str, version: i32, text: &str) {
        self.notify(
            "textDocument/didChange",
            &json!({ "textDocument": { "uri": self.uri(path), "version": version }, "contentChanges": [{ "text": text }] }),
        );
    }

    /// Reports file events: 1 created, 2 changed, 3 deleted.
    pub(crate) fn files(&self, changes: &[(&str, u8)]) {
        let changes: Vec<Value> = changes
            .iter()
            .map(|(path, kind)| json!({ "uri": self.uri(path), "type": kind }))
            .collect();
        self.notify(
            "workspace/didChangeWatchedFiles",
            &json!({ "changes": changes }),
        );
    }
}
