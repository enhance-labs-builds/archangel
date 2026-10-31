//! The file watcher for watch mode and for the language server when the editor watches nothing:
//! notify with notify-debouncer-full, a full check after lost events, and polling, with a note
//! that says why, when the system has no watches left.

use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use notify::event::{EventKind, ModifyKind, RenameMode};
use notify::{ErrorKind, PollWatcher, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::file_id::FileId;
use notify_debouncer_full::{DebounceEventResult, DebouncedEvent, FileIdCache, new_debouncer_opt};

use crate::events::PathEvent;

/// How long the debouncer waits for an editor's save to settle.
const SETTLE: Duration = Duration::from_millis(50);

/// How long after the debouncer hears of a removal its own batch can still report it: the settle
/// time, and as long again for its timer, which ticks at a quarter of it.
const REMOVAL_SETTLES: Duration = Duration::from_millis(100);

/// How often the fallback checks for changes.
pub const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// What the watcher saw since the last batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Batch {
    /// File events, in the order they came. A path's event is a hint: the engine judges each path
    /// by what is on disk.
    Events(Vec<PathEvent>),
    /// The system lost events, so nothing short of a full check is safe.
    Lost,
}

/// What reaches the watcher from the debouncer.
enum Message {
    /// A batch of settled events, or the errors the system reported.
    Settled(DebounceEventResult),
    /// A path the debouncer was told is gone.
    Removed(PathBuf),
}

/// A watcher on the config folder and everything below it.
pub struct Watcher {
    messages: Receiver<Message>,
    /// The debouncer, held so that it keeps watching or polling.
    _debouncer: Box<dyn Send>,
    polling: Option<String>,
    lost_at_start: bool,
}

impl std::fmt::Debug for Watcher {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Watcher")
            .field("polling", &self.polling)
            .field("lost_at_start", &self.lost_at_start)
            .finish_non_exhaustive()
    }
}

impl Watcher {
    /// Starts watching `root`, with the system's watcher, or by polling when the system has no
    /// watches left.
    ///
    /// # Errors
    ///
    /// Returns the reason when neither way can watch the folder.
    pub fn start(root: &Path) -> Result<Self, String> {
        let native = if no_watches_left() {
            Err(notify::Error::new(ErrorKind::MaxFilesWatch))
        } else {
            Self::debounced::<RecommendedWatcher>(root, notify::Config::default())
        };
        match native {
            Ok(watcher) => Ok(watcher),
            Err(error) if matches!(error.kind, ErrorKind::MaxFilesWatch) => {
                // The poll watcher compares modification times in whole seconds, so a save in the
                // same second as the last poll would never be seen; comparing contents sees it.
                let config = notify::Config::default()
                    .with_poll_interval(POLL_INTERVAL)
                    .with_compare_contents(true);
                let mut watcher = Self::debounced::<PollWatcher>(root, config)
                    .map_err(|e| format!("archangel cannot poll {}: {e}", root.display()))?;
                watcher.polling = Some(polling_note());
                Ok(watcher)
            }
            Err(error) => Err(format!(
                "archangel cannot watch {}: {error}",
                root.display()
            )),
        }
    }

    /// Watches `root` recursively with the system watcher `W` behind the debouncer.
    fn debounced<W>(root: &Path, config: notify::Config) -> notify::Result<Self>
    where
        W: notify::Watcher + Send + 'static,
    {
        let (sender, messages) = channel();
        let removals = Removals {
            sender: sender.clone(),
        };
        let settled = move |result| {
            sender.send(Message::Settled(result)).ok();
        };
        let mut debouncer = new_debouncer_opt::<_, W, _>(SETTLE, None, settled, removals, config)?;
        debouncer.watch(root, RecursiveMode::Recursive)?;
        Ok(Self {
            messages,
            _debouncer: Box::new(debouncer),
            polling: None,
            lost_at_start: reports_lost_events(),
        })
    }

    /// Why the watcher polls, when it does.
    #[must_use]
    pub fn polling(&self) -> Option<&str> {
        self.polling.as_deref()
    }

    /// Waits for the next change, then takes every change already waiting, so that saves made
    /// during a check become one batch. Returns `None` when the watcher stopped.
    pub fn wait(&mut self) -> Option<Batch> {
        if std::mem::take(&mut self.lost_at_start) {
            return Some(Batch::Lost);
        }
        let mut gathered = Gathered::default();
        gathered.take(self.messages.recv().ok()?);
        gathered.take_all(self.messages.try_iter());
        // @justified intent
        // A removal the debouncer reports itself comes within its settle time. Waiting that long
        // after the last one keeps a file deleted and written again one change of text, as the
        // debouncer reports it, instead of a deletion now and a new file later.
        while let Some(left) = gathered.settling()
            && let Ok(message) = self.messages.recv_timeout(left)
        {
            gathered.take(message);
            gathered.take_all(self.messages.try_iter());
        }
        Some(gathered.batch())
    }
}

/// The messages of one batch.
#[derive(Default)]
struct Gathered {
    events: Vec<PathEvent>,
    removed: Vec<PathBuf>,
    removed_at: Option<Instant>,
    lost: bool,
}

impl Gathered {
    fn take_all(&mut self, messages: impl Iterator<Item = Message>) {
        for message in messages {
            self.take(message);
        }
    }

    /// How long a removal may still be on its way in the debouncer's own batch.
    fn settling(&self) -> Option<Duration> {
        self.removed_at
            .map(|at| (at + REMOVAL_SETTLES).saturating_duration_since(Instant::now()))
            .filter(|left| !left.is_zero())
    }

    fn take(&mut self, message: Message) {
        match message {
            Message::Settled(Ok(batch)) => {
                for event in &batch {
                    self.lost |= event.need_rescan();
                    self.events.extend(path_events(event));
                }
            }
            Message::Settled(Err(_)) => self.lost = true,
            Message::Removed(path) => {
                self.removed.push(path);
                self.removed_at = Some(Instant::now());
            }
        }
    }

    /// The batch, with each removed path the events do not name already.
    fn batch(mut self) -> Batch {
        if self.lost {
            return Batch::Lost;
        }
        let named: HashSet<PathBuf> = self
            .events
            .iter()
            .flat_map(|event| match event {
                PathEvent::Created(path) | PathEvent::Changed(path) | PathEvent::Deleted(path) => {
                    vec![path.clone()]
                }
                PathEvent::Renamed { from, to } => vec![from.clone(), to.clone()],
            })
            .collect();
        let unnamed: BTreeSet<PathBuf> = self
            .removed
            .into_iter()
            .filter(|path| !named.contains(path))
            .collect();
        self.events
            .extend(unnamed.into_iter().map(PathEvent::Deleted));
        Batch::Events(self.events)
    }
}

// @justified warning
// The debouncer drops the events of a path created and removed before they settle, and the events
// below a removed folder. FSEvents reports a deleted file with the creation flag it once had, so
// the debouncer drops the deletion of a file the graph knows. So every removal reaches the engine,
// which checks the path against the disk.
//
// The debouncer's own cache of file IDs, which notify recommends on macOS and Windows, walks every
// file below the root before the watch starts, node_modules included: 24 seconds and 1.7 GB on a
// monorepo of 9,800 files. It only pairs a rename's halves, and the engine plans a rename as the
// deletion and creation it is made of, so the watcher keeps no IDs.
/// The debouncer's file ID cache, which keeps no IDs and reports each path the debouncer is told is
/// gone.
struct Removals {
    sender: Sender<Message>,
}

impl FileIdCache for Removals {
    fn cached_file_id(&self, _path: &Path) -> Option<impl AsRef<FileId>> {
        None::<FileId>
    }

    fn add_path(&mut self, _path: &Path, _recursive_mode: RecursiveMode) {}

    fn remove_path(&mut self, path: &Path) {
        self.sender.send(Message::Removed(path.to_path_buf())).ok();
    }

    fn rescan(&mut self, _root_paths: &[(PathBuf, RecursiveMode)]) {}
}

/// The path events of one debounced event.
fn path_events(event: &DebouncedEvent) -> Vec<PathEvent> {
    let paths = &event.event.paths;
    match event.event.kind {
        EventKind::Create(_) => paths.iter().cloned().map(PathEvent::Created).collect(),
        EventKind::Remove(_) => paths.iter().cloned().map(PathEvent::Deleted).collect(),
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => match paths.as_slice() {
            [from, to] => vec![PathEvent::Renamed {
                from: from.clone(),
                to: to.clone(),
            }],
            _ => paths.iter().cloned().map(PathEvent::Changed).collect(),
        },
        EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
            paths.iter().cloned().map(PathEvent::Deleted).collect()
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
            paths.iter().cloned().map(PathEvent::Created).collect()
        }
        EventKind::Access(_) => Vec::new(),
        _ => paths.iter().cloned().map(PathEvent::Changed).collect(),
    }
}

fn polling_note() -> String {
    let raise = if cfg!(target_os = "linux") {
        "raise the inotify limit with `sudo sysctl fs.inotify.max_user_watches=524288`"
    } else {
        "raise the system's limit on file watches, or close programs that use many"
    };
    format!(
        "archangel polls for changes every second, because the system has no file watches left \
         (the watch limit); {raise}"
    )
}

/// The `engine::watcher_has_no_watches` fail point: acts as a system out of watches.
fn no_watches_left() -> bool {
    fail::fail_point!("engine::watcher_has_no_watches", |_| true);
    false
}

/// The `engine::watcher_reports_lost_events` fail point: reports lost events after the first
/// check.
fn reports_lost_events() -> bool {
    fail::fail_point!("engine::watcher_reports_lost_events", |_| true);
    false
}

#[cfg(test)]
mod tests;
