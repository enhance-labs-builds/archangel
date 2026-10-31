//! Finding the source files below the config folder.
//!
//! The source files are the same on every machine for the same commit: only the repository's own
//! `.gitignore` files and the config's ignore list decide what is left out. Git's global ignore
//! file, its local exclude file, `.ignore` files and `.gitignore` files above the repository are
//! never read, because they differ between machines.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use ignore::{WalkBuilder, WalkState};

use crate::graph::GraphProblem;

/// The extensions of the files archangel reads for imports.
pub const SOURCE_EXTENSIONS: [&str; 8] = ["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"];

/// Files with a source extension that are not source: Yarn's generated Plug'n'Play loader, which
/// resolution reads as an input.
const GENERATED: [&str; 2] = [".pnp.cjs", ".pnp.loader.mjs"];

/// What the walk found below the config folder.
#[derive(Debug, Default)]
pub(crate) struct SourceSet {
    /// Source files, as paths relative to the config folder with `/`, sorted.
    pub files: Vec<String>,
    /// Package manifests, relative to the config folder, sorted.
    pub manifests: Vec<String>,
    /// Every other file the walk visits, which archangel does not read, such as images, styles
    /// and manifests, relative to the config folder, sorted.
    pub others: Vec<String>,
    /// Symbolic links that the walk did not follow.
    pub links_not_followed: Vec<String>,
    /// Folders below the config folder that are repositories of their own.
    pub nested_repositories: Vec<String>,
    /// Folders and files the walk could not open.
    pub problems: Vec<GraphProblem>,
}

/// The nearest folder at or above `config_dir` that contains `.git`, as a folder or a file.
pub(crate) fn repository_folder(config_dir: &Path) -> PathBuf {
    config_dir
        .ancestors()
        .find(|folder| folder.join(".git").exists())
        .unwrap_or(config_dir)
        .to_path_buf()
}

pub(crate) fn walk(config_dir: &Path, ignore_lines: &[String]) -> SourceSet {
    let (rules, line_problems) = IgnoreRules::new(config_dir, ignore_lines);
    let found = Mutex::new(SourceSet {
        problems: line_problems,
        ..SourceSet::default()
    });
    let walker = WalkBuilder::new(config_dir)
        .hidden(false)
        .ignore(false)
        .git_global(false)
        .git_exclude(false)
        .parents(false)
        .git_ignore(true)
        .require_git(false)
        .follow_links(false)
        .filter_entry(move |entry| {
            let name = entry.file_name();
            !(name == ".git"
                || name == "node_modules"
                || rules.ignores(entry.path(), entry.file_type().is_some_and(|t| t.is_dir())))
        })
        .build_parallel();
    walker.run(|| {
        Box::new(|entry| {
            let Ok(mut set) = found.lock() else {
                return WalkState::Quit;
            };
            record(config_dir, entry, &mut set);
            WalkState::Continue
        })
    });
    let mut set = found.into_inner().unwrap_or_default();
    set.files.sort();
    set.manifests.sort();
    set.others.sort();
    set.links_not_followed.sort();
    set.nested_repositories.sort();
    set
}

fn record(config_dir: &Path, entry: Result<ignore::DirEntry, ignore::Error>, set: &mut SourceSet) {
    let entry = match entry {
        Ok(entry) => entry,
        Err(error) => {
            set.problems.push(walk_problem(config_dir, &error));
            return;
        }
    };
    let Some(kind) = entry.file_type() else {
        return;
    };
    let path = relative(config_dir, entry.path());
    if kind.is_symlink() {
        set.links_not_followed.push(path);
    } else if kind.is_dir() {
        if entry.depth() > 0 && entry.path().join(".git").exists() {
            set.nested_repositories.push(path);
        }
    } else if is_source(entry.path()) {
        set.files.push(path);
    } else {
        if entry.file_name() == "package.json" {
            set.manifests.push(path.clone());
        }
        set.others.push(path);
    }
}

fn walk_problem(config_dir: &Path, error: &ignore::Error) -> GraphProblem {
    let path = walk_error_path(error).map_or_else(String::new, |p| relative(config_dir, p));
    let reason = match error.io_error().map(std::io::Error::kind) {
        Some(std::io::ErrorKind::PermissionDenied) => {
            "archangel cannot open it: permission was refused".to_owned()
        }
        _ => format!("archangel cannot open it: {error}"),
    };
    GraphProblem::FileUnreadable { path, reason }
}

fn walk_error_path(error: &ignore::Error) -> Option<&Path> {
    std::iter::successors(Some(error), |error| match error {
        ignore::Error::WithDepth { err, .. } | ignore::Error::WithLineNumber { err, .. } => {
            Some(err)
        }
        _ => None,
    })
    .find_map(|error| match error {
        ignore::Error::WithPath { path, .. } => Some(path.as_path()),
        _ => None,
    })
}

/// Whether the walk would find `path` as a source file: below the config folder, with a source
/// extension, outside `.git` and `node_modules`, and ignored by no `.gitignore` file on its way
/// and no line of the config's ignore list. It reads the `.gitignore` files above the path only.
pub(crate) fn would_walk(config_dir: &Path, ignore_lines: &[String], path: &Path) -> bool {
    is_source(path) && would_visit(config_dir, ignore_lines, path)
}

/// Whether the walk would visit the file at `path`, as a source file or as another file: below
/// the config folder, outside `.git` and `node_modules`, and ignored by no `.gitignore` file on
/// its way and no line of the config's ignore list.
pub(crate) fn would_visit(config_dir: &Path, ignore_lines: &[String], path: &Path) -> bool {
    let Ok(inside) = path.strip_prefix(config_dir) else {
        return false;
    };
    let skipped = inside
        .components()
        .any(|part| matches!(part.as_os_str().to_str(), Some(".git" | "node_modules")));
    if skipped || inside.as_os_str().is_empty() {
        return false;
    }
    let (rules, _) = IgnoreRules::new(config_dir, ignore_lines);
    if rules.ignores(path, false) {
        return false;
    }
    !path
        .ancestors()
        .skip(1)
        .take_while(|folder| folder.starts_with(config_dir))
        .any(|folder| {
            let mut builder = GitignoreBuilder::new(folder);
            builder.add(folder.join(".gitignore")).is_none()
                && builder.build().is_ok_and(|matcher| {
                    matcher.matched_path_or_any_parents(path, false).is_ignore()
                })
        })
}

/// Whether the path is a source file: one of the source extensions, and not generated.
#[must_use]
pub fn is_source(path: &Path) -> bool {
    let generated = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| GENERATED.contains(&name));
    !generated
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| SOURCE_EXTENSIONS.contains(&extension))
}

/// A path relative to the config folder, with `/` between folders.
pub(crate) fn relative(config_dir: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(config_dir).unwrap_or(path);
    relative.to_string_lossy().replace('\\', "/")
}

/// The `.gitignore` files between the repository folder and the config folder, which the walk
/// does not read itself, and the config's ignore list.
struct IgnoreRules {
    matchers: Vec<Gitignore>,
}

impl IgnoreRules {
    fn new(config_dir: &Path, ignore_lines: &[String]) -> (Self, Vec<GraphProblem>) {
        let repository = repository_folder(config_dir);
        let mut matchers = Vec::new();
        for folder in config_dir.ancestors().skip(1) {
            if !folder.starts_with(&repository) {
                break;
            }
            let mut builder = GitignoreBuilder::new(folder);
            if builder.add(folder.join(".gitignore")).is_none()
                && let Ok(matcher) = builder.build()
            {
                matchers.push(matcher);
            }
        }
        let mut builder = GitignoreBuilder::new(config_dir);
        let mut problems = Vec::new();
        for line in ignore_lines {
            if let Err(error) = builder.add_line(None, line) {
                problems.push(GraphProblem::IgnoreLineInvalid {
                    line: line.clone(),
                    reason: format!("it is not valid .gitignore syntax: {error}"),
                });
            }
        }
        if let Ok(matcher) = builder.build() {
            matchers.push(matcher);
        }
        (Self { matchers }, problems)
    }

    fn ignores(&self, path: &Path, is_dir: bool) -> bool {
        self.matchers.iter().any(|matcher| {
            path.starts_with(matcher.path())
                && matcher
                    .matched_path_or_any_parents(path, is_dir)
                    .is_ignore()
        })
    }
}
