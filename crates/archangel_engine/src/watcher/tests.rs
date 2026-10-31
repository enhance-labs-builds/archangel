//! Scenarios from the language-server and command-line specs for the engine's watcher, which only
//! a unit test can reach: a simulated system watcher sends raw events, as notify's backends do, to
//! the real debouncer, and each test waits for the batch that answers them. The wait is bounded and
//! fails with the batches seen; no test sleeps.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::{Duration, Instant};

use archangel_testkit::FixtureRepo;
use notify::event::{
    AccessKind, CreateKind, DataChange, EventKind, Flag, ModifyKind, RemoveKind, RenameMode,
};
use notify::{Event, EventHandler, RecursiveMode};
use notify_debouncer_full::DebouncedEvent;
use serde_json::json;

use super::{Batch, Message, Watcher};
use crate::{Change, PathEvent, Session};

/// How long a test waits for a batch before it fails as hung.
const HUNG: Duration = Duration::from_secs(60);

/// How many batches a test reads, at most, for the one it waits for.
const MOST_BATCHES: usize = 100;

thread_local! {
    /// The debouncer's handler of raw events, from the simulated watcher made last on this thread.
    static RAW: RefCell<Option<Box<dyn EventHandler>>> = const { RefCell::new(None) };
}

/// A system watcher that sends only the raw events a test gives it.
struct Simulated;

impl notify::Watcher for Simulated {
    fn new<F: EventHandler>(handler: F, _config: notify::Config) -> notify::Result<Self> {
        RAW.with(|raw| *raw.borrow_mut() = Some(Box::new(handler)));
        Ok(Self)
    }

    fn watch(&mut self, _path: &Path, _recursive_mode: RecursiveMode) -> notify::Result<()> {
        Ok(())
    }

    fn unwatch(&mut self, _path: &Path) -> notify::Result<()> {
        Ok(())
    }

    fn kind() -> notify::WatcherKind {
        notify::WatcherKind::NullWatcher
    }
}

/// A watcher over the simulated system watcher, waiting on its own thread.
struct Watching {
    raw: Box<dyn EventHandler>,
    batches: Receiver<Option<Batch>>,
}

impl Watching {
    fn start(root: &Path) -> Self {
        let mut watcher = Watcher::debounced::<Simulated>(root, notify::Config::default())
            .expect("the simulated watcher starts");
        let raw = RAW
            .with(|raw| raw.borrow_mut().take())
            .expect("the debouncer made the simulated watcher");
        let (sender, batches) = channel();
        std::thread::spawn(move || {
            while let Some(batch) = watcher.wait() {
                if sender.send(Some(batch)).is_err() {
                    return;
                }
            }
            sender.send(None).ok();
        });
        Self { raw, batches }
    }

    /// Sends raw events, in order, as one system event's translation.
    fn send(&mut self, events: impl IntoIterator<Item = Event>) {
        for event in events {
            self.raw.handle_event(Ok(event));
        }
    }

    /// The next batch for which `wanted` is true, after the batches before it.
    fn until(&self, what: &str, mut wanted: impl FnMut(&Batch) -> bool) -> Batch {
        let deadline = Instant::now() + HUNG;
        let mut seen = Vec::new();
        for _ in 0..MOST_BATCHES {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.batches.recv_timeout(left) {
                Ok(Some(batch)) if wanted(&batch) => return batch,
                Ok(Some(batch)) => seen.push(batch),
                Ok(None) => panic!("the watcher stopped before a batch {what}; it gave {seen:?}"),
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => break,
            }
        }
        panic!("no batch {what} came; the batches were {seen:?}")
    }

    /// The next batch.
    fn next(&self) -> Batch {
        self.until("at all", |_| true)
    }
}

fn event(kind: EventKind, paths: &[&PathBuf]) -> Event {
    paths.iter().fold(Event::new(kind), |event, path| {
        event.add_path((*path).clone())
    })
}

fn events_of(batch: &Batch) -> &[PathEvent] {
    match batch {
        Batch::Events(events) => events,
        Batch::Lost => &[],
    }
}

/// Holds the fail points for one test, so that no other test's fault reaches it.
#[cfg(feature = "failpoints")]
fn faults() -> fail::FailScenario<'static> {
    fail::FailScenario::setup()
}

/// Without fail points there is nothing to hold.
#[cfg(not(feature = "failpoints"))]
struct NoFaults;

#[cfg(not(feature = "failpoints"))]
fn faults() -> NoFaults {
    NoFaults
}

/// macOS reports a deletion with the creation flag the file once had, which notify turns into a
/// creation and a removal that the debouncer drops as a pair; the deletion still reaches the session.
#[test]
fn a_deletion_reported_with_a_stale_creation_flag_reaches_the_session() {
    let _faults = faults();
    let repo = FixtureRepo::new()
        .file("src/a.ts", "import './b';\n")
        .file("src/b.ts", "")
        .json_config(&json!({ "elements": {}, "rules": [{ "name": "no-cycles", "files": "**", "noCycles": true }] }));
    let mut session = Session::open(repo.path(), None).expect("the session opens");
    let mut watching = Watching::start(repo.path());
    repo.remove("src/b.ts");
    let gone = repo.join("src/b.ts");
    watching.send([
        event(EventKind::Create(CreateKind::File), &[&gone]),
        event(EventKind::Remove(RemoveKind::File), &[&gone]),
    ]);
    let batch = watching.until("that names src/b.ts", |batch| {
        events_of(batch)
            .iter()
            .any(|e| *e == PathEvent::Deleted(gone.clone()))
    });
    session.apply(vec![Change::Paths(events_of(&batch).to_vec())]);
    assert_eq!(
        session.report().summary.files,
        1,
        "src/b.ts is gone from the graph"
    );
    assert!(
        session.report().diagnostics.iter().any(|d| {
            d.code == archangel_rules::Code::ImportUnresolved
                && d.file.as_deref() == Some("src/a.ts")
        }),
        "a.ts's import of b.ts is unresolved: {:?}",
        session.report().diagnostics
    );
}

/// Each kind of raw event becomes the path event of its kind, and a read of a file becomes none.
#[test]
fn each_raw_event_is_the_path_event_of_its_kind() {
    let _faults = faults();
    let repo = FixtureRepo::new();
    let mut watching = Watching::start(repo.path());
    let [new, saved, gone, out, moved_in, read] = [
        "new.ts", "saved.ts", "gone.ts", "out.ts", "in.ts", "read.ts",
    ]
    .map(|f| repo.join(f));
    watching.send([
        event(EventKind::Access(AccessKind::Read), &[&read]),
        event(EventKind::Create(CreateKind::File), &[&new]),
        event(
            EventKind::Modify(ModifyKind::Data(DataChange::Content)),
            &[&saved],
        ),
        event(EventKind::Remove(RemoveKind::File), &[&gone]),
        event(
            EventKind::Modify(ModifyKind::Name(RenameMode::From)),
            &[&out],
        ),
        event(
            EventKind::Modify(ModifyKind::Name(RenameMode::To)),
            &[&moved_in],
        ),
    ]);
    let mut found = Vec::new();
    watching.until("with every kind", |batch| {
        found.extend(events_of(batch).iter().cloned());
        found.len() >= 5
    });
    found.sort_by_key(|e| format!("{e:?}"));
    let mut expected = vec![
        PathEvent::Created(new),
        PathEvent::Changed(saved),
        PathEvent::Deleted(gone),
        PathEvent::Deleted(out),
        PathEvent::Created(moved_in),
    ];
    expected.sort_by_key(|e| format!("{e:?}"));
    assert_eq!(
        found, expected,
        "a creation, a save, a deletion, a move out and a move in; reading the file is no change"
    );
}

#[test]
fn a_rename_the_system_pairs_is_one_rename() {
    let _faults = faults();
    let repo = FixtureRepo::new();
    let mut watching = Watching::start(repo.path());
    let (old, new) = (repo.join("old.ts"), repo.join("new.ts"));
    watching.send([
        event(
            EventKind::Modify(ModifyKind::Name(RenameMode::From)),
            &[&old],
        )
        .set_tracker(7),
        event(EventKind::Modify(ModifyKind::Name(RenameMode::To)), &[&new]).set_tracker(7),
    ]);
    assert_eq!(
        watching.next(),
        Batch::Events(vec![PathEvent::Renamed { from: old, to: new }]),
        "the two halves of the move are one rename"
    );
}

/// A file deleted and written again, as some editors save, costs one parse of its new text.
#[test]
fn a_file_deleted_and_written_again_is_one_change_of_text() {
    let _faults = faults();
    let repo = FixtureRepo::new()
        .file("src/a.ts", "import './b';\n")
        .file("src/b.ts", "")
        .json_config(&json!({ "elements": {}, "rules": [{ "name": "no-cycles", "files": "**", "noCycles": true }] }));
    let mut session = Session::open(repo.path(), None).expect("the session opens");
    let mut watching = Watching::start(repo.path());
    repo.write("src/a.ts", "import './b';\nexport const a = 1;\n");
    let a = repo.join("src/a.ts");
    watching.send([
        event(EventKind::Remove(RemoveKind::File), &[&a]),
        event(EventKind::Create(CreateKind::File), &[&a]),
    ]);
    let mut work = archangel_graph::Work::default();
    watching.until("that names src/a.ts as new", |batch| {
        work.add(
            &session
                .apply(vec![Change::Paths(events_of(batch).to_vec())])
                .work,
        );
        events_of(batch).contains(&PathEvent::Created(a.clone()))
    });
    assert_eq!(
        (work.files_parsed, work.resolutions),
        (1, 1),
        "(parsed, resolutions) for a change to a.ts's text"
    );
}

#[test]
fn events_the_system_lost_are_a_lost_batch() {
    let _faults = faults();
    let repo = FixtureRepo::new();
    let mut watching = Watching::start(repo.path());
    watching.send([Event::new(EventKind::Other).set_flag(Flag::Rescan)]);
    assert_eq!(watching.next(), Batch::Lost, "a rescan is lost events");
    watching
        .raw
        .handle_event(Err(notify::Error::generic("the queue overflowed")));
    assert_eq!(watching.next(), Batch::Lost, "an error is lost events");
}

#[test]
fn the_system_watcher_watches_without_polling() {
    let _faults = faults();
    let repo = FixtureRepo::new();
    let watcher = Watcher::start(repo.path()).expect("the system watcher starts");
    assert_eq!(watcher.polling(), None, "nothing to say about polling");
}

#[test]
fn a_folder_that_does_not_exist_cannot_be_watched() {
    let _faults = faults();
    let repo = FixtureRepo::new();
    let missing = repo.join("missing");
    let error = Watcher::start(&missing).expect_err("there is nothing to watch");
    assert!(
        error.starts_with(&format!("archangel cannot watch {}", missing.display())),
        "the error names the folder: {error}"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn a_system_without_watches_left_polls_and_says_why() {
    let _faults = faults();
    fail::cfg("engine::watcher_has_no_watches", "return").unwrap();
    let repo = FixtureRepo::new();
    let watcher = Watcher::start(repo.path()).expect("the poll watcher starts");
    let note = watcher.polling().expect("a note that it polls").to_owned();
    for words in ["polls for changes every second", "the watch limit", "raise"] {
        assert!(note.contains(words), "the note says {words:?}: {note}");
    }
}

#[cfg(feature = "failpoints")]
#[test]
fn lost_events_at_the_start_come_before_any_change() {
    let _faults = faults();
    fail::cfg("engine::watcher_reports_lost_events", "return").unwrap();
    let repo = FixtureRepo::new();
    let mut watching = Watching::start(repo.path());
    fail::remove("engine::watcher_reports_lost_events");
    assert_eq!(
        watching.next(),
        Batch::Lost,
        "the first batch is lost events"
    );
    let new = repo.join("new.ts");
    watching.send([event(EventKind::Create(CreateKind::File), &[&new])]);
    assert_eq!(
        watching.next(),
        Batch::Events(vec![PathEvent::Created(new)]),
        "the next batch is the change"
    );
}

fn rename_halves(from: &PathBuf, to: &PathBuf) -> [Event; 2] {
    [
        event(
            EventKind::Modify(ModifyKind::Name(RenameMode::From)),
            &[from],
        ),
        event(EventKind::Modify(ModifyKind::Name(RenameMode::To)), &[to]),
    ]
}

#[test]
fn a_rename_the_system_does_not_pair_is_a_deletion_and_a_creation() {
    let _faults = faults();
    let repo = FixtureRepo::new().file("old.ts", "");
    let mut watching = Watching::start(repo.path());
    repo.rename("old.ts", "new.ts");
    let (old, new) = (repo.join("old.ts"), repo.join("new.ts"));
    watching.send(rename_halves(&old, &new));
    let batch = watching.until("both halves of the rename", |batch| {
        matches!(batch, Batch::Events(events) if events.contains(&PathEvent::Created(new.clone())))
    });
    assert!(
        events_of(&batch).contains(&PathEvent::Deleted(old)),
        "the watcher keeps no file IDs, so the old path is a deletion the engine checks on disk"
    );
}

/// A watcher whose debouncer has already sent `messages`, for the batching of what waits.
fn waiting(messages: Vec<Message>) -> Watcher {
    let (sender, received) = channel();
    for message in messages {
        sender.send(message).expect("the watcher's channel is open");
    }
    Watcher {
        messages: received,
        _debouncer: Box::new(sender),
        polling: None,
        lost_at_start: false,
    }
}

fn settled(events: Vec<Event>) -> Message {
    Message::Settled(Ok(events
        .into_iter()
        .map(|event| DebouncedEvent::new(event, Instant::now()))
        .collect()))
}

/// Every change already waiting when the watcher wakes joins one batch, as saves made during a
/// check do.
#[test]
fn changes_already_waiting_are_one_batch() {
    let _faults = faults();
    let (a, b) = (PathBuf::from("/r/a.ts"), PathBuf::from("/r/b.ts"));
    let mut watcher = waiting(vec![
        settled(vec![event(EventKind::Create(CreateKind::File), &[&a])]),
        settled(vec![event(EventKind::Create(CreateKind::File), &[&b])]),
    ]);
    assert_eq!(
        watcher.wait(),
        Some(Batch::Events(vec![
            PathEvent::Created(a),
            PathEvent::Created(b)
        ])),
        "both saves in one batch"
    );
}

/// A removal the debouncer reports itself is named once, and one it dropped is named too.
#[test]
fn each_removal_is_named_once() {
    let _faults = faults();
    let (reported, dropped) = (PathBuf::from("/r/a.ts"), PathBuf::from("/r/b.ts"));
    let mut watcher = waiting(vec![
        Message::Removed(reported.clone()),
        Message::Removed(dropped.clone()),
        Message::Removed(dropped.clone()),
        settled(vec![event(
            EventKind::Remove(RemoveKind::File),
            &[&reported],
        )]),
    ]);
    assert_eq!(
        watcher.wait(),
        Some(Batch::Events(vec![
            PathEvent::Deleted(reported),
            PathEvent::Deleted(dropped)
        ])),
        "the debouncer's removal, then the one it dropped"
    );
}
