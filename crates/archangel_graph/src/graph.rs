//! The import graph: every source file, its facts, and where each of its imports resolves.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use oxc_allocator::Allocator;
use rayon::prelude::*;

use crate::files::{self, SourceSet};
use crate::imports::{self, FileFacts, PARSER_LIMIT_BYTES, ParseFailure};
use crate::resolve::{
    ImportResolver, Importer, Resolution, ResolutionFileProblem, ResolutionInputs,
};
use crate::text;
use crate::work::Work;

/// The stack each parse thread gets. The parser descends with recursion, and code nested 10,000
/// levels deep needs between 32 and 64 MB in a debug build. The stack is reserved address space;
/// a thread uses only what it touches.
pub const PARSE_STACK_BYTES: usize = 64 * 1024 * 1024;

/// How to build the graph.
#[derive(Debug, Clone, Default)]
pub struct GraphOptions {
    /// Lines in `.gitignore` syntax that remove files from the source files.
    pub ignore: Vec<String>,
}

/// One source file.
#[derive(Debug, Clone)]
pub struct SourceFile {
    /// The path relative to the config folder, with `/` between folders.
    pub path: String,
    /// What the file imports and exports, or `None` when it could not be read or parsed.
    pub facts: Option<FileFacts>,
}

/// A problem that stops archangel from reading part of the repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphProblem {
    /// A folder or file that cannot be opened, or a file that is not text.
    FileUnreadable {
        /// The path relative to the config folder.
        path: String,
        /// Why, in plain words.
        reason: String,
    },
    /// A source file with syntax errors, whose imports are not used.
    ParseFailed {
        /// The path relative to the config folder.
        path: String,
        /// Every error, and whether the file is Flow or too large.
        failure: ParseFailure,
    },
    /// A line of the config's ignore list that is not valid `.gitignore` syntax.
    IgnoreLineInvalid {
        /// The line as the config gives it.
        line: String,
        /// What is wrong, in plain words.
        reason: String,
    },
    /// A tsconfig or package manifest that resolution needs and cannot use.
    ResolutionFileInvalid {
        /// The path relative to the config folder.
        path: String,
        /// What is wrong, in plain words.
        reason: String,
    },
}

/// Files that exist only in memory, such as the files of an example, parsed and resolved against a
/// repository without a change to its graph.
#[derive(Debug, Clone, Default)]
pub struct OverlayFiles {
    /// The files, in path order.
    pub files: Vec<SourceFile>,
    /// Where each file's imports resolve, in the order of its imports.
    pub resolutions: Vec<Vec<Resolution>>,
    /// The parse failures of the files, and the resolution files they cannot use.
    pub problems: Vec<GraphProblem>,
    /// The paths of the files that are not source, which are not parsed, in path order.
    pub others: Vec<String>,
    /// What each package manifest among the files lists, by the folder that holds it: those the
    /// walk would visit, so none in `node_modules`.
    pub manifests: HashMap<String, Manifest>,
}

impl OverlayFiles {
    /// The files whose imports resolve to `path`, directly, with these files laid over `graph`:
    /// the files of the overlay that import it, and the repository files that import it and that
    /// no file of the overlay replaces. Each once, in path order.
    #[must_use]
    pub fn importers<'a>(&'a self, graph: &'a ImportGraph, path: &str) -> Vec<&'a str> {
        let replaced = |from: &str| self.files.iter().any(|file| file.path == from);
        let imports_it = |resolutions: &[Resolution]| {
            resolutions
                .iter()
                .any(|r| matches!(r, Resolution::File(target) if target == path))
        };
        let mut found: Vec<&str> = graph
            .importers(path)
            .into_iter()
            .filter(|from| !replaced(from))
            .chain(
                self.files
                    .iter()
                    .zip(&self.resolutions)
                    .filter(|(_, resolutions)| imports_it(resolutions))
                    .map(|(file, _)| file.path.as_str()),
            )
            .collect();
        found.sort_unstable();
        found.dedup();
        found
    }
}

/// The graph of one repository.
#[derive(Debug, Clone)]
pub struct ImportGraph {
    root: PathBuf,
    ignore: Vec<String>,
    files: Vec<SourceFile>,
    index: HashMap<String, usize>,
    /// The read or parse problem of each file, aligned with `files`.
    parse_problems: Vec<Option<GraphProblem>>,
    /// A hash of each file's text, aligned with `files`.
    hashes: Vec<Option<u64>>,
    resolutions: Vec<Vec<Resolution>>,
    resolution_problems: BTreeSet<ResolutionFileProblem>,
    walk_problems: Vec<GraphProblem>,
    /// Every problem, kept in step with the three kinds above.
    problems: Vec<GraphProblem>,
    importers: Vec<Vec<usize>>,
    links_not_followed: Vec<String>,
    nested_repositories: Vec<String>,
    manifests: Vec<String>,
    /// Every other file the walk visited, sorted.
    others: Vec<String>,
    manifest_dependencies: BTreeSet<String>,
    /// What each manifest lists, by the folder that holds it.
    manifest_lists: HashMap<String, Manifest>,
    inputs: ResolutionInputs,
    resolver: Arc<ImportResolver>,
    /// The threads that parse, each with the parse stack, kept for the parses after the build.
    pool: Option<Arc<rayon::ThreadPool>>,
    timings: GraphTimings,
}

/// How long each step of a build took. Tests assert work counts, never these.
#[derive(Debug, Clone, Copy, Default)]
pub struct GraphTimings {
    /// Listing the source files.
    pub walk: std::time::Duration,
    /// Reading and parsing them, in parallel.
    pub read_and_parse: std::time::Duration,
    /// Resolving every import, in parallel.
    pub resolve: std::time::Duration,
}

impl ImportGraph {
    /// Reads, parses and resolves every source file below `config_dir`.
    #[must_use]
    pub fn build(config_dir: &Path, options: &GraphOptions, work: &mut Work) -> Self {
        // std's canonical path on Windows is `\\?\C:\…`, which no resolved path matches.
        let root = dunce::canonicalize(config_dir).unwrap_or_else(|_| config_dir.to_path_buf());
        let started = std::time::Instant::now();
        let set = files::walk(&root, &options.ignore);
        work.files_found += set.files.len() as u64;
        let walked = started.elapsed();
        let pool = rayon::ThreadPoolBuilder::new()
            .stack_size(PARSE_STACK_BYTES)
            .build()
            .ok()
            .map(Arc::new);
        let started = std::time::Instant::now();
        let parsed = run_in(pool.as_deref(), || parse_files(&root, &set.files));
        let read_and_parsed = started.elapsed();
        let started = std::time::Instant::now();
        let resolver = ImportResolver::new(&root, &files::repository_folder(&root), HashMap::new());
        let outcomes = run_in(pool.as_deref(), || {
            resolve_files(&resolver, &root, &set.files, &parsed)
        });
        let resolve_time = started.elapsed();
        let outcomes = parsed.into_iter().zip(outcomes).collect();
        let resolver = Arc::new(resolver);
        let mut graph = Self::assemble(
            Assembly {
                root,
                set,
                resolver,
                pool,
            },
            outcomes,
            work,
        );
        graph.ignore.clone_from(&options.ignore);
        graph.timings = GraphTimings {
            walk: walked,
            read_and_parse: read_and_parsed,
            resolve: resolve_time,
        };
        graph
    }

    /// How long each step of the build took.
    #[must_use]
    pub fn timings(&self) -> GraphTimings {
        self.timings
    }

    fn assemble(assembly: Assembly, outcomes: Vec<(Parsed, Resolved)>, work: &mut Work) -> Self {
        let Assembly {
            root,
            set,
            resolver,
            pool,
        } = assembly;
        let SourceSet {
            files: paths,
            manifests,
            others,
            links_not_followed,
            nested_repositories,
            problems: walk_problems,
        } = set;
        let mut collected = Collected::default();
        for (path, outcome) in paths.into_iter().zip(outcomes) {
            collected.add(path, outcome, work);
        }
        let manifest_lists = read_manifests(&root, &manifests);
        let manifest_dependencies = dependency_names_of(&manifest_lists);
        let mut inputs = collected.inputs;
        inputs.contents.extend(resolver.files_read());
        let mut graph = Self {
            root,
            ignore: Vec::new(),
            files: collected.files,
            index: HashMap::new(),
            parse_problems: collected.parse_problems,
            hashes: collected.hashes,
            resolutions: collected.resolutions,
            resolution_problems: collected.file_problems,
            walk_problems,
            problems: Vec::new(),
            importers: Vec::new(),
            links_not_followed,
            nested_repositories,
            manifests,
            others,
            manifest_dependencies,
            manifest_lists,
            inputs,
            resolver,
            pool,
            timings: GraphTimings::default(),
        };
        graph.refresh();
        graph
    }

    /// Rebuilds the index, the importers and the problem list after a change to the files.
    fn refresh(&mut self) {
        self.index = self
            .files
            .iter()
            .enumerate()
            .map(|(i, file)| (file.path.clone(), i))
            .collect();
        self.importers = importers(&self.index, &self.resolutions);
        self.refresh_problems();
    }

    /// Rebuilds the problem list from the problems of the walk, the parses and the resolutions.
    fn refresh_problems(&mut self) {
        self.problems = self
            .walk_problems
            .iter()
            .chain(self.parse_problems.iter().flatten())
            .cloned()
            .chain(self.resolution_problems.iter().map(|problem| {
                GraphProblem::ResolutionFileInvalid {
                    path: problem.path.clone(),
                    reason: problem.reason.clone(),
                }
            }))
            .collect();
    }

    /// Parses one source file again, from `text` or, without it, from disk, and resolves its
    /// imports with the resolver's cache. Returns false, having parsed nothing, when the path is
    /// not a source file of the graph or its text is the text the graph has.
    pub fn replace_file(&mut self, path: &str, text: Option<&str>, work: &mut Work) -> bool {
        let Some(&index) = self.index.get(path) else {
            return false;
        };
        let read = match text {
            Some(text) => Ok(text.to_owned()),
            None => read_text(&self.root, path, work),
        };
        if let Ok(text) = &read
            && self.hashes[index] == Some(text_hash(text))
        {
            return false;
        }
        let parsed = run_in(self.pool.as_deref(), || match &read {
            Ok(text) => parse_text(&mut Allocator::default(), path, text, Work::default()),
            Err(problem) => failed(problem.clone(), Work::default()),
        });
        let resolved = resolve_file(&self.resolver, &self.root.join(path), parsed.facts.as_ref());
        work.add(&parsed.work);
        work.resolutions += resolved.resolutions.len() as u64;
        self.files[index].facts = parsed.facts;
        self.mark_imports(index, resolved.verbatim_module_syntax);
        self.parse_problems[index] = parsed.problem;
        self.hashes[index] = parsed.hash;
        let old = std::mem::replace(&mut self.resolutions[index], resolved.resolutions);
        self.move_importer(index, &old);
        self.resolution_problems.extend(resolved.problems);
        self.inputs.contents.extend(resolved.inputs.contents);
        self.inputs.existence.extend(resolved.inputs.existence);
        self.inputs.contents.extend(self.resolver.files_read());
        self.refresh_problems();
        true
    }

    fn mark_imports(&mut self, index: usize, verbatim_module_syntax: bool) {
        if let Some(facts) = &mut self.files[index].facts {
            facts.mark_imports(verbatim_module_syntax);
        }
    }

    /// Takes the file at `index` off the importer lists of its old targets and puts it on those
    /// of its new ones, so an edit touches the lists of the files it imported and imports alone.
    fn move_importer(&mut self, index: usize, old: &[Resolution]) {
        let targets = |resolutions: &[Resolution]| -> BTreeSet<usize> {
            resolutions
                .iter()
                .filter_map(|resolution| match resolution {
                    Resolution::File(path) => self.index.get(path).copied(),
                    Resolution::Package(_) | Resolution::Unresolved(_) => None,
                })
                .collect()
        };
        let (before, after) = (targets(old), targets(&self.resolutions[index]));
        for &gone in before.difference(&after) {
            self.importers[gone].retain(|&from| from != index);
        }
        for &new in after.difference(&before) {
            let list = &mut self.importers[new];
            if let Err(at) = list.binary_search(&index) {
                list.insert(at, index);
            }
        }
    }

    /// Removes the `deleted` source files, reads and parses the `created` ones, and then resolves
    /// every import once, because a file that appears or disappears can change any resolution.
    pub fn apply_file_events(&mut self, created: &[String], deleted: &[String], work: &mut Work) {
        let deleted: BTreeSet<&str> = deleted.iter().map(String::as_str).collect();
        let new: Vec<&String> = created
            .iter()
            .filter(|path| {
                !self.index.contains_key(path.as_str()) && !deleted.contains(path.as_str())
            })
            .collect();
        let root = &self.root;
        let parsed: Vec<Parsed> = run_in(self.pool.as_deref(), || {
            new.par_iter()
                .map_init(Allocator::default, |allocator, path| {
                    read_and_parse(root, path, allocator)
                })
                .collect()
        });
        let mut rows: Vec<Row> = std::mem::take(&mut self.files)
            .into_iter()
            .zip(std::mem::take(&mut self.parse_problems))
            .zip(std::mem::take(&mut self.hashes))
            .map(|((file, problem), hash)| Row {
                file,
                problem,
                hash,
            })
            .filter(|row| !deleted.contains(row.file.path.as_str()))
            .collect();
        for (path, parsed) in new.into_iter().zip(parsed) {
            work.add(&parsed.work);
            rows.push(Row {
                file: SourceFile {
                    path: path.clone(),
                    facts: parsed.facts,
                },
                problem: parsed.problem,
                hash: parsed.hash,
            });
        }
        rows.sort_by(|a, b| a.file.path.cmp(&b.file.path));
        self.parse_problems = rows.iter_mut().map(|row| row.problem.take()).collect();
        self.hashes = rows.iter().map(|row| row.hash).collect();
        self.files = rows.into_iter().map(|row| row.file).collect();
        self.resolve_all_again(work);
    }

    /// Resolves every import again with an empty cache, as after a change to a tsconfig, a package
    /// manifest the resolver read, or the set of files.
    pub fn resolve_all_again(&mut self, work: &mut Work) {
        self.resolver = Arc::new(ImportResolver::new(
            &self.root,
            &files::repository_folder(&self.root),
            HashMap::new(),
        ));
        let resolved = run_in(self.pool.as_deref(), || {
            resolve_all(&self.resolver, &self.root, &self.files)
        });
        self.resolutions = Vec::with_capacity(resolved.len());
        self.resolution_problems = BTreeSet::new();
        self.inputs = ResolutionInputs::default();
        for (index, outcome) in resolved.into_iter().enumerate() {
            work.resolutions += outcome.resolutions.len() as u64;
            self.mark_imports(index, outcome.verbatim_module_syntax);
            self.resolution_problems.extend(outcome.problems);
            self.inputs.contents.extend(outcome.inputs.contents);
            self.inputs.existence.extend(outcome.inputs.existence);
            self.resolutions.push(outcome.resolutions);
        }
        self.inputs.contents.extend(self.resolver.files_read());
        self.refresh();
    }

    /// Walks the repository again with `ignore`, after a change to a `.gitignore` file or to the
    /// config's ignore list, and applies the files that appeared or disappeared.
    pub fn walk_again(&mut self, ignore: &[String], work: &mut Work) {
        let set = files::walk(&self.root, ignore);
        work.files_found += set.files.len() as u64;
        let found: BTreeSet<&String> = set.files.iter().collect();
        let deleted: Vec<String> = self
            .files
            .iter()
            .map(|f| f.path.clone())
            .filter(|path| !found.contains(path))
            .collect();
        let created: Vec<String> = set
            .files
            .iter()
            .filter(|path| !self.index.contains_key(path.as_str()))
            .cloned()
            .collect();
        self.ignore = ignore.to_vec();
        self.walk_problems = set.problems;
        self.links_not_followed = set.links_not_followed;
        self.nested_repositories = set.nested_repositories;
        self.manifests = set.manifests;
        self.others = set.others;
        self.reread_manifests();
        self.apply_file_events(&created, &deleted, work);
    }

    /// Reads every package manifest again, and returns the manifests whose lists changed.
    pub fn reread_manifests(&mut self) -> Vec<String> {
        let lists = read_manifests(&self.root, &self.manifests);
        let mut changed: Vec<String> = lists
            .iter()
            .filter(|(folder, manifest)| self.manifest_lists.get(*folder) != Some(manifest))
            .chain(
                self.manifest_lists
                    .iter()
                    .filter(|(folder, _)| !lists.contains_key(*folder)),
            )
            .map(|(_, manifest)| manifest.path.clone())
            .collect();
        changed.sort();
        changed.dedup();
        self.manifest_dependencies = dependency_names_of(&lists);
        self.manifest_lists = lists;
        changed
    }

    /// The nearest package manifest at or above the folder of a file, relative to the config
    /// folder, among the manifests below the config folder.
    #[must_use]
    pub fn nearest_manifest(&self, file: &str) -> Option<&Manifest> {
        self.manifests_with(None).nearest(file)
    }

    /// Whether a package of this name is a manifest of the repository, as in a workspace.
    #[must_use]
    pub fn is_workspace_package(&self, name: &str) -> bool {
        self.manifests_with(None).is_workspace_package(name)
    }

    /// The repository's package manifests, with the manifests of `overlay` laid over them.
    #[must_use]
    pub fn manifests_with<'a>(&'a self, overlay: Option<&'a OverlayFiles>) -> Manifests<'a> {
        Manifests {
            repository: &self.manifest_lists,
            over: overlay.map(|overlay| &overlay.manifests),
        }
    }

    /// Whether the walk would find the path, absolute or relative to the config folder, as a
    /// source file.
    #[must_use]
    pub fn would_walk(&self, path: &Path) -> bool {
        files::would_walk(&self.root, &self.ignore, &self.root.join(path))
    }

    /// Whether the walk would visit the file at the path, absolute or relative to the config
    /// folder, as a source file or as another file.
    #[must_use]
    pub fn would_visit(&self, path: &Path) -> bool {
        files::would_visit(&self.root, &self.ignore, &self.root.join(path))
    }

    /// Every file the walk visited that is not a source file, such as an image, a stylesheet or a
    /// manifest, relative to the config folder, sorted. Archangel does not read them.
    #[must_use]
    pub fn others(&self) -> &[String] {
        &self.others
    }

    /// Whether the walk visited a file at the path that is not a source file.
    #[must_use]
    pub fn is_other(&self, path: &str) -> bool {
        self.others
            .binary_search_by(|known| known.as_str().cmp(path))
            .is_ok()
    }

    /// Records that a file that is not source appeared or disappeared, without a walk. Returns
    /// whether the graph's list changed.
    pub fn set_other(&mut self, path: &str, present: bool) -> bool {
        match (
            self.others
                .binary_search_by(|known| known.as_str().cmp(path)),
            present,
        ) {
            (Err(at), true) => {
                self.others.insert(at, path.to_owned());
                true
            }
            (Ok(at), false) => {
                self.others.remove(at);
                true
            }
            _ => false,
        }
    }

    /// The ignore lines the graph was walked with.
    #[must_use]
    pub fn ignore(&self) -> &[String] {
        &self.ignore
    }

    /// The config folder, with symbolic links resolved.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Every source file, sorted by path.
    #[must_use]
    pub fn files(&self) -> &[SourceFile] {
        &self.files
    }

    /// The source file at a path relative to the config folder.
    #[must_use]
    pub fn file(&self, path: &str) -> Option<&SourceFile> {
        self.index.get(path).map(|&i| &self.files[i])
    }

    /// Where each import of a file resolves, in the order of its imports.
    #[must_use]
    pub fn resolutions(&self, path: &str) -> Option<&[Resolution]> {
        self.index
            .get(path)
            .map(|&i| self.resolutions[i].as_slice())
    }

    /// Problems that stopped archangel from reading part of the repository.
    #[must_use]
    pub fn problems(&self) -> &[GraphProblem] {
        &self.problems
    }

    /// Symbolic links that the walk did not follow.
    #[must_use]
    pub fn links_not_followed(&self) -> &[String] {
        &self.links_not_followed
    }

    /// Folders below the config folder that are repositories of their own.
    #[must_use]
    pub fn nested_repositories(&self) -> &[String] {
        &self.nested_repositories
    }

    /// Package manifests below the config folder.
    #[must_use]
    pub fn manifests(&self) -> &[String] {
        &self.manifests
    }

    /// Every package name that a package manifest below the config folder lists as a dependency.
    #[must_use]
    pub fn manifest_dependencies(&self) -> &BTreeSet<String> {
        &self.manifest_dependencies
    }

    /// The files that the resolutions depend on.
    #[must_use]
    pub fn resolution_inputs(&self) -> &ResolutionInputs {
        &self.inputs
    }

    /// Where `specifier` would resolve from the source file `file`, with the resolver and tsconfigs
    /// of the build. Fixes use it to try a new specifier without parsing a file.
    #[must_use]
    pub fn resolve(&self, file: &str, specifier: &str) -> Resolution {
        let full = self.root.join(file);
        let (tsconfig, _) = self.resolver.tsconfig(&full);
        let importer = Importer {
            file: &full,
            tsconfig: tsconfig.as_deref(),
        };
        let (resolution, _) =
            self.resolver
                .resolve(&importer, specifier, &mut ResolutionInputs::default());
        resolution
    }

    /// The repository files whose imports resolve to `path`, directly.
    #[must_use]
    pub fn importers(&self, path: &str) -> Vec<&str> {
        self.index.get(path).map_or_else(Vec::new, |&i| {
            self.importers[i]
                .iter()
                .map(|&from| self.files[from].path.as_str())
                .collect()
        })
    }

    /// The repository files that import any of `paths`, directly or through other files, in path
    /// order. A file of `paths` is in the list only when another one of them reaches it.
    #[must_use]
    pub fn reaching(&self, paths: &[&str]) -> Vec<&str> {
        let mut seen = vec![false; self.files.len()];
        let mut stack: Vec<usize> = paths
            .iter()
            .filter_map(|path| self.index.get(*path).copied())
            .collect();
        while let Some(file) = stack.pop() {
            for &from in &self.importers[file] {
                if !seen[from] {
                    seen[from] = true;
                    stack.push(from);
                }
            }
        }
        self.files
            .iter()
            .zip(seen)
            .filter_map(|(file, seen)| seen.then_some(file.path.as_str()))
            .collect()
    }

    /// Parses and resolves files held in memory, keyed by their path relative to the config
    /// folder, as if they were saved over the repository. A file at the path of a repository file
    /// replaces it. Nothing is written, and the graph does not change.
    #[must_use]
    pub fn overlay(&self, files: &BTreeMap<String, String>, work: &mut Work) -> OverlayFiles {
        let in_memory = files
            .iter()
            .map(|(path, text)| (self.root.join(path), text.clone().into_bytes()))
            .collect();
        let repository = files::repository_folder(&self.root);
        let resolver = ImportResolver::new(&self.root, &repository, in_memory);
        let mut allocator = Allocator::default();
        let mut collected = Collected::default();
        let mut others = Vec::new();
        let mut manifests = HashMap::new();
        for (path, text) in files {
            if !files::is_source(Path::new(path)) {
                if Path::new(path).file_name() == Some("package.json".as_ref())
                    && self.would_visit(Path::new(path))
                {
                    let (folder, manifest) = manifest_of(path, text);
                    manifests.insert(folder, manifest);
                }
                others.push(path.clone());
                continue;
            }
            let parsed = parse_text(&mut allocator, path, text, Work::default());
            let outcome = resolve_file(&resolver, &self.root.join(path), parsed.facts.as_ref());
            collected.add(path.clone(), (parsed, outcome), work);
        }
        let (files, resolutions, problems) = collected.finish();
        OverlayFiles {
            files,
            resolutions,
            problems,
            others,
            manifests,
        }
    }
}

/// The package manifests that the files of a check see: the repository's, and over them, those
/// of an example, which replace the repository's manifest at their path.
#[derive(Debug, Clone, Copy)]
pub struct Manifests<'a> {
    repository: &'a HashMap<String, Manifest>,
    over: Option<&'a HashMap<String, Manifest>>,
}

impl<'a> Manifests<'a> {
    /// The manifest in a folder, relative to the config folder.
    fn in_folder(&self, folder: &str) -> Option<&'a Manifest> {
        self.over
            .and_then(|over| over.get(folder))
            .or_else(|| self.repository.get(folder))
    }

    /// The nearest package manifest at or above the folder of a file, relative to the config
    /// folder.
    #[must_use]
    pub fn nearest(&self, file: &str) -> Option<&'a Manifest> {
        let folders = file.match_indices('/').map(|(end, _)| &file[..end]).rev();
        folders
            .chain(std::iter::once(""))
            .find_map(|folder| self.in_folder(folder))
    }

    /// Whether a package of this name is a manifest of the repository, as in a workspace. An
    /// example's manifests are not among them: a workspace package is reached through its link
    /// in `node_modules`, which only the repository on disk has.
    #[must_use]
    pub fn is_workspace_package(&self, name: &str) -> bool {
        self.repository
            .values()
            .any(|manifest| manifest.name.as_deref() == Some(name))
    }
}

/// One file with what the graph keeps beside it, while the files are sorted again.
struct Row {
    file: SourceFile,
    problem: Option<GraphProblem>,
    hash: Option<u64>,
}

/// What a build hands to `assemble`, besides the outcomes.
struct Assembly {
    root: PathBuf,
    set: SourceSet,
    resolver: Arc<ImportResolver>,
    pool: Option<Arc<rayon::ThreadPool>>,
}

/// The files of a build or an overlay, gathered one by one.
#[derive(Default)]
struct Collected {
    files: Vec<SourceFile>,
    resolutions: Vec<Vec<Resolution>>,
    parse_problems: Vec<Option<GraphProblem>>,
    hashes: Vec<Option<u64>>,
    file_problems: BTreeSet<ResolutionFileProblem>,
    inputs: ResolutionInputs,
}

impl Collected {
    fn add(&mut self, path: String, (mut parsed, resolved): (Parsed, Resolved), work: &mut Work) {
        if let Some(facts) = &mut parsed.facts {
            facts.mark_imports(resolved.verbatim_module_syntax);
        }
        work.add(&parsed.work);
        work.resolutions += resolved.resolutions.len() as u64;
        self.parse_problems.push(parsed.problem);
        self.hashes.push(parsed.hash);
        self.file_problems.extend(resolved.problems);
        self.inputs.contents.extend(resolved.inputs.contents);
        self.inputs.existence.extend(resolved.inputs.existence);
        self.files.push(SourceFile {
            path,
            facts: parsed.facts,
        });
        self.resolutions.push(resolved.resolutions);
    }

    /// The files, their resolutions, and their problems with one per resolution file at the end.
    fn finish(self) -> (Vec<SourceFile>, Vec<Vec<Resolution>>, Vec<GraphProblem>) {
        let problems = self
            .parse_problems
            .into_iter()
            .flatten()
            .chain(self.file_problems.into_iter().map(|problem| {
                GraphProblem::ResolutionFileInvalid {
                    path: problem.path,
                    reason: problem.reason,
                }
            }))
            .collect();
        (self.files, self.resolutions, problems)
    }
}

/// What reading and parsing one file gave.
struct Parsed {
    facts: Option<FileFacts>,
    problem: Option<GraphProblem>,
    /// A hash of the text parsed, so that a change that leaves the text as it was costs no parse.
    hash: Option<u64>,
    work: Work,
}

/// What resolving one file's imports gave.
struct Resolved {
    resolutions: Vec<Resolution>,
    problems: BTreeSet<ResolutionFileProblem>,
    inputs: ResolutionInputs,
    /// Whether the file's tsconfig sets `verbatimModuleSyntax`.
    verbatim_module_syntax: bool,
}

fn run_in<T: Send>(pool: Option<&rayon::ThreadPool>, job: impl FnOnce() -> T + Send) -> T {
    match pool {
        Some(pool) => pool.install(job),
        None => job(),
    }
}

fn parse_files(root: &Path, paths: &[String]) -> Vec<Parsed> {
    paths
        .par_iter()
        .map_init(Allocator::default, |allocator, path| {
            read_and_parse(root, path, allocator)
        })
        .collect()
}

fn resolve_files(
    resolver: &ImportResolver,
    root: &Path,
    paths: &[String],
    parsed: &[Parsed],
) -> Vec<Resolved> {
    paths
        .par_iter()
        .zip(parsed.par_iter())
        .map(|(path, parsed)| resolve_file(resolver, &root.join(path), parsed.facts.as_ref()))
        .collect()
}

fn resolve_all(resolver: &ImportResolver, root: &Path, files: &[SourceFile]) -> Vec<Resolved> {
    files
        .par_iter()
        .map(|file| resolve_file(resolver, &root.join(&file.path), file.facts.as_ref()))
        .collect()
}

fn read_and_parse(root: &Path, path: &str, allocator: &mut Allocator) -> Parsed {
    let mut work = Work::default();
    match read_text(root, path, &mut work) {
        Ok(text) => parse_text(allocator, path, &text, work),
        Err(problem) => failed(problem, work),
    }
}

/// The text of a source file, or the problem that stops archangel from reading it.
fn read_text(root: &Path, path: &str, work: &mut Work) -> Result<String, GraphProblem> {
    let full = root.join(path);
    if std::fs::metadata(&full).is_ok_and(|meta| too_large(meta.len())) {
        let failure = ParseFailure {
            too_large: true,
            ..ParseFailure::default()
        };
        return Err(GraphProblem::ParseFailed {
            path: path.to_owned(),
            failure,
        });
    }
    let bytes = std::fs::read(&full).map_err(|error| GraphProblem::FileUnreadable {
        path: path.to_owned(),
        reason: format!("archangel cannot read it: {error}"),
    })?;
    work.files_read += 1;
    work.bytes_read += bytes.len() as u64;
    text::decode(&bytes).map_err(|reason| GraphProblem::FileUnreadable {
        path: path.to_owned(),
        reason,
    })
}

/// Whether a file of `bytes` is larger than the parser accepts.
fn too_large(bytes: u64) -> bool {
    bytes > PARSER_LIMIT_BYTES
}

/// A hash of a file's text, the same in every run.
fn text_hash(text: &str) -> u64 {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

fn failed(problem: GraphProblem, work: Work) -> Parsed {
    Parsed {
        facts: None,
        problem: Some(problem),
        hash: None,
        work,
    }
}

fn parse_text(allocator: &mut Allocator, path: &str, text: &str, mut work: Work) -> Parsed {
    allocator.reset();
    let result = imports::extract(allocator, path, text);
    work.files_parsed += 1;
    if parse_twice() {
        allocator.reset();
        drop(imports::extract(allocator, path, text));
        work.files_parsed += 1;
    }
    match result {
        Ok(facts) => {
            work.imports_found += facts.imports.len() as u64;
            Parsed {
                facts: Some(facts),
                problem: None,
                hash: Some(text_hash(text)),
                work,
            }
        }
        Err(failure) => Parsed {
            hash: Some(text_hash(text)),
            ..failed(
                GraphProblem::ParseFailed {
                    path: path.to_owned(),
                    failure,
                },
                work,
            )
        },
    }
}

/// The `graph::parse_twice` fail point: parses each file a second time, for the fault control.
fn parse_twice() -> bool {
    fail::fail_point!("graph::parse_twice", |_| true);
    false
}

fn resolve_file(resolver: &ImportResolver, file: &Path, facts: Option<&FileFacts>) -> Resolved {
    let mut problems = BTreeSet::new();
    let mut inputs = ResolutionInputs::default();
    let Some(facts) = facts else {
        return Resolved {
            resolutions: Vec::new(),
            problems,
            inputs,
            verbatim_module_syntax: false,
        };
    };
    let (tsconfig, problem) = resolver.tsconfig(file);
    problems.extend(problem);
    if let Some(tsconfig) = &tsconfig {
        inputs.contents.insert(tsconfig.path.clone());
        inputs
            .contents
            .extend(tsconfig.references_resolved.iter().map(|r| r.path.clone()));
    }
    let verbatim_module_syntax = tsconfig
        .as_ref()
        .is_some_and(|found| found.compiler_options.verbatim_module_syntax == Some(true));
    let importer = Importer {
        file,
        tsconfig: tsconfig.as_deref(),
    };
    let resolutions = facts
        .imports
        .iter()
        .map(|import| {
            let (resolution, problem) = resolver.resolve(&importer, &import.specifier, &mut inputs);
            problems.extend(problem);
            resolution
        })
        .collect();
    Resolved {
        resolutions,
        problems,
        inputs,
        verbatim_module_syntax,
    }
}

/// For each file, the files whose imports resolve to it, each once, in path order.
fn importers(index: &HashMap<String, usize>, resolutions: &[Vec<Resolution>]) -> Vec<Vec<usize>> {
    let mut importers = vec![Vec::new(); resolutions.len()];
    for (from, resolved) in resolutions.iter().enumerate() {
        for resolution in resolved {
            if let Resolution::File(path) = resolution
                && let Some(&to) = index.get(path)
                && importers[to].last() != Some(&from)
            {
                importers[to].push(from);
            }
        }
    }
    importers
}

fn dependency_names_of(lists: &HashMap<String, Manifest>) -> BTreeSet<String> {
    lists
        .values()
        .flat_map(|manifest| manifest.dependencies.iter().chain(&manifest.development))
        .cloned()
        .collect()
}

/// What a package manifest lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// The manifest, relative to the config folder.
    pub path: String,
    /// The package's own name.
    pub name: Option<String>,
    /// The packages it lists for production: `dependencies`, `peerDependencies` and
    /// `optionalDependencies`.
    pub dependencies: BTreeSet<String>,
    /// The packages it lists under `devDependencies`.
    pub development: BTreeSet<String>,
}

/// Every manifest's lists, by the folder that holds it. A manifest that does not parse lists
/// none; the resolver reports it when an import needs it.
fn read_manifests(root: &Path, manifests: &[String]) -> HashMap<String, Manifest> {
    manifests
        .iter()
        .map(|path| {
            let text = std::fs::read_to_string(root.join(path)).unwrap_or_default();
            manifest_of(path, &text)
        })
        .collect()
}

/// What the manifest at `path`, relative to the config folder, lists, and the folder that holds
/// it. A manifest that does not parse lists none.
fn manifest_of(path: &str, text: &str) -> (String, Manifest) {
    let value = serde_json::from_str::<serde_json::Value>(text).unwrap_or_default();
    let names = |fields: &[&str]| -> BTreeSet<String> {
        fields
            .iter()
            .filter_map(|field| value.get(field).and_then(serde_json::Value::as_object))
            .flat_map(|names| names.keys().cloned())
            .collect()
    };
    let folder = path.rfind('/').map_or("", |end| &path[..end]).to_owned();
    let manifest = Manifest {
        path: path.to_owned(),
        name: value
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        dependencies: names(&["dependencies", "peerDependencies", "optionalDependencies"]),
        development: names(&["devDependencies"]),
    };
    (folder, manifest)
}

#[cfg(test)]
mod tests;
