//! A file system for the resolver that shows files held in memory over the files on disk, so that
//! an example or an open document resolves as if it were saved, and nothing is written.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use oxc_resolver::{FileMetadata, FileSystem, FileSystemOs, ResolveError};

/// The files on disk, with some files replaced or added in memory. It records every file the
/// resolver reads, because those are the files whose contents decide resolutions: tsconfigs and
/// every config they extend, and package manifests.
#[derive(Clone)]
pub(crate) struct Overlay {
    disk: FileSystemOs,
    files: HashMap<PathBuf, Vec<u8>>,
    folders: HashSet<PathBuf>,
    reads: Arc<Mutex<BTreeSet<PathBuf>>>,
}

impl Overlay {
    /// The disk with `files`, keyed by absolute path, laid over it. Every folder above a file in
    /// memory exists, even when it does not exist on disk.
    #[must_use]
    pub(crate) fn new(yarn_pnp: bool, files: HashMap<PathBuf, Vec<u8>>) -> Self {
        let folders = files
            .keys()
            .flat_map(|path| path.ancestors().skip(1))
            .map(Path::to_path_buf)
            .collect();
        Self {
            disk: FileSystemOs::new(yarn_pnp),
            files,
            folders,
            reads: Arc::default(),
        }
    }

    /// The record of the files the resolver read, shared with every clone.
    pub(crate) fn reads(&self) -> Arc<Mutex<BTreeSet<PathBuf>>> {
        Arc::clone(&self.reads)
    }

    fn record(&self, path: &Path) {
        if let Ok(mut reads) = self.reads.lock() {
            reads.insert(path.to_path_buf());
        }
    }

    fn in_memory(&self, path: &Path) -> Option<FileMetadata> {
        if self.files.contains_key(path) {
            Some(FileMetadata::new(true, false, false))
        } else if self.folders.contains(path) && !path.is_dir() {
            Some(FileMetadata::new(false, true, false))
        } else {
            None
        }
    }
}

impl FileSystem for Overlay {
    fn new(yarn_pnp: bool) -> Self {
        Self::new(yarn_pnp, HashMap::new())
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.record(path);
        match self.files.get(path) {
            Some(bytes) => Ok(bytes.clone()),
            None => self.disk.read(path),
        }
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        self.record(path);
        match self.files.get(path) {
            Some(bytes) => String::from_utf8(bytes.clone())
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error)),
            None => self.disk.read_to_string(path),
        }
    }

    fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        self.in_memory(path)
            .map_or_else(|| self.disk.metadata(path), Ok)
    }

    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        self.in_memory(path)
            .map_or_else(|| self.disk.symlink_metadata(path), Ok)
    }

    fn read_link(&self, path: &Path) -> Result<PathBuf, ResolveError> {
        self.disk.read_link(path)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        if self.in_memory(path).is_some() {
            return Ok(path.to_path_buf());
        }
        self.disk.canonicalize(path)
    }
}
