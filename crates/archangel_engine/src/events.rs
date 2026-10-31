//! Raw path events, from a watcher or an editor, netted into the work they need.
//!
//! A batch can hold a save by rename, a branch switch of a thousand files, or a tsconfig edit. Each
//! path is judged once, by what the graph knew before the batch and what is on disk after it:
//!
//! 1. A source file that existed before and after is a text change, and costs one parse.
//! 2. A source file that appeared or disappeared changes the set of files: walk again, parse the
//!    new files, and resolve every import once for the whole batch.
//! 3. The config, the baseline and a `.gitignore` file each have a change of their own.
//! 4. A file whose contents a resolution read, such as a tsconfig or a manifest, resolves every
//!    import again. A file whose existence a resolution found, such as an imported CSS file,
//!    matters only when it appeared or disappeared.
//! 5. A manifest's dependency names are read again; for a manifest no resolution read, that is
//!    all its change costs.
//! 6. A watcher reports a folder moved or deleted whole as the folder alone, so any path no rule
//!    above claims may be a folder. A folder that is gone changes the set of files when the graph
//!    had source files below it, and otherwise takes its other files out of the graph; a folder
//!    that is there with no source file below it changes the set of files when the walk would
//!    find one in it, and otherwise brings in the other files the walk would visit there. Either
//!    changes resolutions when a file they depend on is below it. A folder that is there with
//!    source files below it is not new, whatever its event says, because macOS can report a
//!    folder's creation long after it. Telling costs a search of the graph's files and a look at
//!    the disk per such path, a read of a new folder, and, when the batch resolves nothing yet, a
//!    binary search per resolution input.
//! 7. A source file the graph has costs nothing on disk while the editor has it open, because the
//!    editor's text wins.
//! 8. A file that is not source, such as an image, that appeared or disappeared where the walk
//!    would visit it is added to or removed from the graph's other files, for the structure
//!    rules, without a walk. A change to its contents costs nothing.
//!
//! An editor's document is judged too, when it opens or closes: a document the graph does not
//! have yet is a new file once the walk would find it on disk, and closing a document whose file
//! is gone deletes it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use archangel_graph::{ImportGraph, is_source};

use crate::baseline_file::BASELINE_NAME;

/// One event from a file watcher or an editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathEvent {
    /// A file appeared.
    Created(PathBuf),
    /// A file's contents changed.
    Changed(PathBuf),
    /// A file disappeared.
    Deleted(PathBuf),
    /// A file moved, over another file or to a new path.
    Renamed {
        /// Where it was.
        from: PathBuf,
        /// Where it is now.
        to: PathBuf,
    },
}

/// A step of work that a batch can need, besides parsing changed files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Step {
    /// The config changed.
    Config,
    /// The baseline changed.
    Baseline,
    /// The set of source files may have changed, or a `.gitignore` did: walk again.
    Walk,
    /// A file a resolution depends on changed: resolve every import again.
    Resolve,
    /// A manifest no resolution read changed: read the dependency names again.
    Manifests,
}

/// The work a batch needs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Plan {
    /// Source files whose text changed on disk.
    pub(crate) texts: BTreeSet<String>,
    /// Files that are not source that appeared, `true`, or disappeared, `false`.
    pub(crate) others: BTreeMap<String, bool>,
    /// The other steps.
    pub(crate) steps: BTreeSet<Step>,
}

impl Plan {
    /// Adds another plan's work to this one.
    pub(crate) fn merge(&mut self, other: Self) {
        self.texts.extend(other.texts);
        self.others.extend(other.others);
        self.steps.extend(other.steps);
    }

    /// Whether the batch needs the step.
    pub(crate) fn needs(&self, step: Step) -> bool {
        self.steps.contains(&step)
    }

    fn add(&mut self, step: Step) {
        self.steps.insert(step);
    }
}

/// What the session knows, to judge each path by.
pub(crate) struct Known<'k> {
    pub(crate) graph: &'k ImportGraph,
    pub(crate) config_path: &'k Path,
    /// The files the config imported, such as a plugin, whose change loads it again.
    pub(crate) config_inputs: &'k [PathBuf],
    pub(crate) open: &'k BTreeMap<String, String>,
}

/// Whether a path's events show that it appeared or disappeared, as opposed to a change of text.
#[derive(Default)]
struct Moves {
    appeared: bool,
    disappeared: bool,
}

/// The plan for a batch of events.
pub(crate) fn plan(known: &Known<'_>, events: &[PathEvent]) -> Plan {
    let mut moves: BTreeMap<PathBuf, Moves> = BTreeMap::new();
    for event in events {
        match event {
            PathEvent::Created(path) => moves.entry(path.clone()).or_default().appeared = true,
            PathEvent::Changed(path) => {
                moves.entry(path.clone()).or_default();
            }
            PathEvent::Deleted(path) => moves.entry(path.clone()).or_default().disappeared = true,
            PathEvent::Renamed { from, to } => {
                moves.entry(from.clone()).or_default().disappeared = true;
                moves.entry(to.clone()).or_default().appeared = true;
            }
        }
    }
    let mut plan = Plan::default();
    let mut folders = BTreeSet::new();
    for (path, moved) in &moves {
        if let Some(folder) = judge(known, path, moved, &mut plan) {
            folders.insert(folder);
        }
    }
    moved_folders(known.graph, &folders, &mut plan);
    plan
}

/// Each event as its own plan, as if no event had been netted with another, for the fault control.
pub(crate) fn unnetted(known: &Known<'_>, events: &[PathEvent]) -> Plan {
    let mut plan = Plan::default();
    for event in events {
        let mut one = self::plan(known, std::slice::from_ref(event));
        if !one.texts.is_empty() {
            one.texts.clear();
            one.add(Step::Walk);
        }
        plan.merge(one);
    }
    plan
}

/// One path, as the graph names it.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Place {
    full: PathBuf,
    relative: String,
    name: String,
}

impl Place {
    fn new(root: &Path, path: &Path) -> Self {
        let joined = if path.is_absolute() {
            path.to_path_buf()
        } else {
            root.join(path)
        };
        let full = joined
            .parent()
            .and_then(|folder| dunce::canonicalize(folder).ok())
            .and_then(|folder| joined.file_name().map(|name| folder.join(name)))
            .unwrap_or(joined);
        let relative = full
            .strip_prefix(root)
            .map(|inside| inside.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let name = full
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self {
            full,
            relative,
            name,
        }
    }
}

/// Adds a path's work to the plan. Returns the path when no rule claims it, because it may be a
/// folder.
fn judge(known: &Known<'_>, path: &Path, moved: &Moves, plan: &mut Plan) -> Option<Place> {
    let graph = known.graph;
    let place = Place::new(graph.root(), path);
    other_file(graph, &place, plan);
    let had = graph.file(&place.relative).is_some();
    // A file the config imported may also be a source file, so its own change still counts.
    if known.config_inputs.contains(&place.full) {
        plan.add(Step::Config);
    }
    if let Some(step) = own_step(known, &place) {
        plan.add(step);
    } else if had && known.open.contains_key(&place.relative) {
        // The editor's text wins while the document is open.
    } else if had || graph.would_walk(&place.full) {
        source(graph, place, plan);
    } else if !input(graph, &place, moved, plan) {
        return Some(place);
    }
    None
}

/// Records a file that is not source that appeared or disappeared where the walk visits.
fn other_file(graph: &ImportGraph, place: &Place, plan: &mut Plan) {
    if place.relative.is_empty() || is_source(&place.full) || place.full.is_dir() {
        return;
    }
    let present = place.full.is_file() && graph.would_visit(&place.full);
    if graph.is_other(&place.relative) != present {
        plan.others.insert(place.relative.clone(), present);
    }
}

/// The work for a document the editor opened, given as a path relative to the config folder: a
/// source file the graph does not have is judged as if a watcher had reported it.
pub(crate) fn opened(graph: &ImportGraph, path: &str, plan: &mut Plan) {
    if graph.file(path).is_none() {
        let place = Place::new(graph.root(), Path::new(path));
        if graph.would_walk(&place.full) {
            source(graph, place, plan);
        }
    }
}

/// The work for a document the editor closed: the text on disk counts again, and a file that is
/// no longer there is gone.
pub(crate) fn closed(graph: &ImportGraph, path: &str, plan: &mut Plan) {
    if graph.file(path).is_some() {
        source(graph, Place::new(graph.root(), Path::new(path)), plan);
    }
}

/// The work for the paths that no rule above claimed, any of which may be a folder, as the
/// module's rule 6 says. The config folder itself holds every file, and its own events say nothing
/// about them.
fn moved_folders(graph: &ImportGraph, places: &BTreeSet<Place>, plan: &mut Plan) {
    if plan.needs(Step::Walk) {
        return;
    }
    let mut changed = Vec::new();
    for place in places.iter().filter(|place| !place.relative.is_empty()) {
        match folder(graph, place, plan) {
            Folder::Walk => {
                plan.add(Step::Walk);
                return;
            }
            Folder::Changed => changed.push(place),
            Folder::Known => {}
        }
    }
    if plan.needs(Step::Resolve) || changed.is_empty() {
        return;
    }
    let folders = Folders::new(&changed);
    let inputs = graph.resolution_inputs();
    if inputs
        .contents
        .iter()
        .chain(&inputs.existence)
        .any(|input| folders.hold(input))
    {
        plan.add(Step::Resolve);
    }
}

/// What a path that may be a folder means for the batch.
enum Folder {
    /// The set of source files changed: walk again.
    Walk,
    /// Files below it may have gone or come, which resolutions can depend on.
    Changed,
    /// It is a folder the graph already knows with files below it.
    Known,
}

/// Judges a path that may be a folder, and adds the other files that went or came with it to
/// the plan.
fn folder(graph: &ImportGraph, place: &Place, plan: &mut Plan) -> Folder {
    let below = format!("{}/", place.relative);
    let files = graph.files();
    let first = files.partition_point(|file| file.path.as_str() < below.as_str());
    let holds_sources = files
        .get(first)
        .is_some_and(|file| file.path.starts_with(&below));
    let others = graph.others();
    let first = others.partition_point(|path| path.as_str() < below.as_str());
    let others_below: Vec<String> = others[first..]
        .iter()
        .take_while(|path| path.starts_with(&below))
        .cloned()
        .collect();
    match (place.full.is_dir(), holds_sources, others_below.is_empty()) {
        (false, true, _) => Folder::Walk,
        (false, false, false) => {
            plan.others
                .extend(others_below.into_iter().map(|path| (path, false)));
            Folder::Changed
        }
        (true, false, _) => match inside(graph, place) {
            Inside::Source => Folder::Walk,
            Inside::Others(found) => {
                plan.others
                    .extend(found.into_iter().map(|path| (path, true)));
                Folder::Changed
            }
        },
        (true, true, _) => Folder::Known,
        (false, false, true) => Folder::Changed,
    }
}

/// Folders, as the bytes of their paths with a separator after them, sorted, and none inside
/// another. The greatest one at or before a path is then the only one that can hold it, so a
/// path costs a binary search of byte comparisons.
struct Folders(Vec<Vec<u8>>);

impl Folders {
    fn new(places: &[&Place]) -> Self {
        let mut all: Vec<Vec<u8>> = places
            .iter()
            .map(|place| {
                let mut key = key(&place.full).into_owned();
                key.push(SEPARATOR);
                key
            })
            .collect();
        all.sort();
        let mut outermost: Vec<Vec<u8>> = Vec::with_capacity(all.len());
        for folder in all {
            if !outermost
                .last()
                .is_some_and(|last| folder.starts_with(last))
            {
                outermost.push(folder);
            }
        }
        Self(outermost)
    }

    fn hold(&self, path: &Path) -> bool {
        let path = key(path);
        let after = self
            .0
            .partition_point(|folder| folder.as_slice() <= path.as_ref());
        after
            .checked_sub(1)
            .and_then(|nearest| self.0.get(nearest))
            .is_some_and(|folder| path.starts_with(folder))
    }
}

/// The separator that [`key`] gives every path.
const SEPARATOR: u8 = b'/';

/// A path's bytes, with `/` for each separator.
fn key(path: &Path) -> std::borrow::Cow<'_, [u8]> {
    let bytes = path.as_os_str().as_encoded_bytes();
    if cfg!(windows) {
        bytes
            .iter()
            .map(|&byte| if byte == b'\\' { SEPARATOR } else { byte })
            .collect::<Vec<u8>>()
            .into()
    } else {
        bytes.into()
    }
}

/// What the walk would find in a folder that appeared.
enum Inside {
    /// A source file, so the set of source files changed.
    Source,
    /// Only these other files, relative to the config folder, which may be none.
    Others(Vec<String>),
}

/// What the walk would find in a folder that appeared. The walk never enters a `node_modules` or
/// `.git` folder, so those are not read.
fn inside(graph: &ImportGraph, place: &Place) -> Inside {
    let mut found = Vec::new();
    if place
        .relative
        .split('/')
        .any(|part| matches!(part, "node_modules" | ".git"))
    {
        return Inside::Others(found);
    }
    let mut folders = vec![place.full.clone()];
    while let Some(next) = folders.pop() {
        let Ok(entries) = std::fs::read_dir(&next) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => folders.push(path),
                Ok(kind) if kind.is_file() && graph.would_walk(&path) => return Inside::Source,
                Ok(kind) if kind.is_file() && graph.would_visit(&path) => {
                    if let Ok(relative) = path.strip_prefix(graph.root()) {
                        found.push(relative.to_string_lossy().replace('\\', "/"));
                    }
                }
                _ => {}
            }
        }
    }
    Inside::Others(found)
}

/// The config, the baseline and `.gitignore` files each have a step of their own.
fn own_step(known: &Known<'_>, place: &Place) -> Option<Step> {
    if place.full == known.config_path {
        Some(Step::Config)
    } else if place.full == known.graph.root().join(BASELINE_NAME) {
        Some(Step::Baseline)
    } else {
        (place.name == ".gitignore").then_some(Step::Walk)
    }
}

/// A source file that existed before and after changed its text; one that appeared or disappeared
/// changed the set of files; one that did neither, such as an editor's temporary file, is nothing.
fn source(graph: &ImportGraph, place: Place, plan: &mut Plan) {
    let before = graph.file(&place.relative).is_some();
    match (before, place.full.is_file()) {
        (true, true) => {
            plan.texts.insert(place.relative);
        }
        (false, false) => {}
        _ => plan.add(Step::Walk),
    }
}

/// A file that resolutions or the element of a package depend on. Returns false when the path is
/// none of those.
fn input(graph: &ImportGraph, place: &Place, moved: &Moves, plan: &mut Plan) -> bool {
    let inputs = graph.resolution_inputs();
    let manifest = place.name == "package.json";
    let moved = moved.appeared || moved.disappeared;
    if inputs.contents.contains(&place.full) {
        plan.add(Step::Resolve);
        if manifest {
            plan.add(Step::Manifests);
        }
    } else if inputs.existence.contains(&place.full) {
        if moved || resolve_per_found_file() {
            plan.add(Step::Resolve);
        }
    } else if manifest {
        plan.add(if moved { Step::Walk } else { Step::Manifests });
    } else {
        return false;
    }
    true
}

/// The `engine::resolve_per_found_file` fail point: resolves every import again for a change to
/// the text of any file a resolution found, for the fault control.
fn resolve_per_found_file() -> bool {
    fail::fail_point!("engine::resolve_per_found_file", |_| true);
    false
}
