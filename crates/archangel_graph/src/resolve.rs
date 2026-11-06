//! Resolving imports as the TypeScript compiler does, through the oxc resolver.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use oxc_resolver::{
    ResolveContext, ResolveError, ResolveOptions, ResolverGeneric, TsConfig, TsconfigDiscovery,
};

use crate::files::relative;
use crate::overlay::Overlay;

/// Where an import points.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Resolution {
    /// A repository file, relative to the config folder, which need not be a source file.
    File(String),
    /// A package, by name; a Node built-in has the `node:` prefix.
    Package(String),
    /// Nowhere, and why.
    Unresolved(UnresolvedReason),
}

/// Why an import has no resolution.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UnresolvedReason {
    /// No file at the path and no package of that name.
    NotFound,
    /// A `.js` specifier with no TypeScript or JavaScript file behind it.
    NoFileBehindJs,
    /// A package whose `exports` field does not offer the path.
    NotExported,
    /// A `#` specifier that the package's `imports` field does not define.
    ImportNotDefined,
    /// A package manifest whose `exports` or `imports` target is not valid.
    InvalidTarget,
    /// A specifier that is not valid.
    InvalidSpecifier,
    /// A chain of aliases or `exports` that loops or runs past the resolver's 64 steps.
    Loop,
    /// Yarn's Plug'n'Play map does not let the importing package depend on the package.
    NotDeclared,
    /// The resolver could not read a file or folder.
    Io(String),
    /// A problem in a tsconfig or package manifest, which has its own diagnostic.
    ResolutionFile,
    /// A template import whose pattern matches no file.
    TemplateMatchesNothing,
    /// A failure that this version of archangel cannot describe yet.
    Unknown,
}

impl UnresolvedReason {
    /// The reason in plain words, for the import's specifier.
    #[must_use]
    pub fn message(&self, specifier: &str) -> String {
        match self {
            Self::NotFound => {
                format!("there is no file and no installed package for `{specifier}`")
            }
            Self::NoFileBehindJs => format!(
                "there is no file `{specifier}`, and no TypeScript or declaration file beside it of the same name"
            ),
            Self::NotExported => format!(
                "the package's `exports` field does not offer the path that `{specifier}` names"
            ),
            Self::ImportNotDefined => {
                format!("the package's `imports` field does not define `{specifier}`")
            }
            Self::InvalidTarget => {
                format!("the package manifest maps `{specifier}` to a target that is not valid")
            }
            Self::InvalidSpecifier => format!("`{specifier}` is not a valid specifier"),
            Self::Loop => format!(
                "the alias or `exports` chain for `{specifier}` loops or is too long to follow"
            ),
            Self::NotDeclared => format!(
                "Yarn's Plug'n'Play map does not list `{specifier}` as a dependency of the importing \
                 package; add it to that package's dependencies and run `yarn install`"
            ),
            Self::Io(path) => format!("archangel could not read {path} to resolve `{specifier}`"),
            Self::ResolutionFile => format!(
                "a tsconfig or package manifest that `{specifier}` depends on is not valid; see its \
                 own diagnostic"
            ),
            Self::TemplateMatchesNothing => format!(
                "no file matches `{specifier}`, whose each `${{…}}` stands for one file or folder \
                 name, so the import can load nothing"
            ),
            Self::Unknown => format!(
                "the resolver could not resolve `{specifier}` for a reason archangel cannot describe \
                 yet; please report it with the specifier and the files involved"
            ),
        }
    }
}

/// Describes a resolver error in archangel's terms, without the resolver's own text.
#[must_use]
pub fn describe_resolve_error(error: &ResolveError) -> UnresolvedReason {
    match error {
        ResolveError::NotFound(_) | ResolveError::MatchedAliasNotFound(..) => {
            UnresolvedReason::NotFound
        }
        ResolveError::ExtensionAlias(..) => UnresolvedReason::NoFileBehindJs,
        ResolveError::PackagePathNotExported { .. } => UnresolvedReason::NotExported,
        ResolveError::PackageImportNotDefined(..) => UnresolvedReason::ImportNotDefined,
        ResolveError::InvalidPackageTarget(..) => UnresolvedReason::InvalidTarget,
        ResolveError::InvalidModuleSpecifier(..)
        | ResolveError::Specifier(_)
        | ResolveError::PathNotSupported(_) => UnresolvedReason::InvalidSpecifier,
        ResolveError::Recursion => UnresolvedReason::Loop,
        ResolveError::IOError(error) => UnresolvedReason::Io(error.to_string()),
        ResolveError::Json(_)
        | ResolveError::TsconfigNotFound(_)
        | ResolveError::TsconfigCircularExtend(_)
        | ResolveError::TsconfigSelfReference(_)
        | ResolveError::InvalidPackageConfig(_)
        | ResolveError::InvalidPackageConfigDefault(_)
        | ResolveError::InvalidPackageConfigDirectory(_)
        | ResolveError::FailedToFindYarnPnpManifest(_) => UnresolvedReason::ResolutionFile,
        ResolveError::YarnPnpError(error) => match error {
            pnp::Error::BadSpecifier(_) => UnresolvedReason::InvalidSpecifier,
            pnp::Error::MissingPeerDependency(_)
            | pnp::Error::UndeclaredDependency(_)
            | pnp::Error::MissingDependency(_) => UnresolvedReason::NotDeclared,
            pnp::Error::FailedManifestHydration(_) => UnresolvedReason::ResolutionFile,
        },
        // `ResolveError` is non-exhaustive. This arm covers `Ignored`, `Unimplemented`, `Builtin`
        // (which `resolve` handles before it gets here) and any variant a later resolver adds.
        _ => UnresolvedReason::Unknown,
    }
}

/// A problem in a file that resolution reads, found once however many imports it affects.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ResolutionFileProblem {
    pub path: String,
    pub reason: String,
    /// The package that a tsconfig's `extends` names, when it is not installed.
    pub missing_package: Option<String>,
}

/// The files that resolutions depend on. Hash sets, because every import adds the paths it tried and
/// most repeat: a check of VS Code adds about a million paths to keep eleven thousand.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolutionInputs {
    /// Files whose contents decide resolutions: tsconfigs, manifests and the Plug'n'Play file.
    pub contents: HashSet<PathBuf>,
    /// Files whose existence decides resolutions: files found and paths tried.
    pub existence: HashSet<PathBuf>,
}

/// An importing file and the tsconfig that governs it.
#[derive(Clone, Copy)]
pub(crate) struct Importer<'a> {
    pub file: &'a Path,
    pub tsconfig: Option<&'a TsConfig>,
}

/// The resolver for one repository, with options that do not depend on the machine.
pub(crate) struct ImportResolver {
    resolver: ResolverGeneric<Overlay>,
    reads: Arc<std::sync::Mutex<BTreeSet<PathBuf>>>,
    /// The reads `files_read` has already returned, so each edit looks only at what is new.
    reported: std::sync::Mutex<HashSet<PathBuf>>,
    /// The names of each package module whose names an import needed, with the files read for
    /// them, kept for the life of the resolver.
    modules: std::sync::Mutex<HashMap<PathBuf, ModuleNames>>,
    /// Every file the walk visited, source files and others, sorted, which a template import's
    /// pattern is matched against.
    listing: std::sync::RwLock<Arc<[String]>>,
    /// The Plug'n'Play file, which the resolver reads through its own loader.
    pnp: Option<PathBuf>,
    root: PathBuf,
}

impl std::fmt::Debug for ImportResolver {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ImportResolver")
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

impl ImportResolver {
    /// A resolver that sees `in_memory`, keyed by absolute path, over the files on disk.
    pub(crate) fn new(
        root: &Path,
        repository: &Path,
        in_memory: HashMap<PathBuf, Vec<u8>>,
    ) -> Self {
        let yarn_pnp = repository.join(".pnp.cjs").is_file();
        let options = ResolveOptions {
            cwd: Some(repository.to_path_buf()),
            tsconfig: Some(TsconfigDiscovery::Auto),
            node_path: false,
            yarn_pnp,
            extensions: [
                ".ts", ".tsx", ".mts", ".cts", ".js", ".jsx", ".mjs", ".cjs", ".json", ".d.ts",
            ]
            .map(String::from)
            .to_vec(),
            // @justified informative
            // TypeScript's own order for a specifier with a JavaScript extension, in every
            // `moduleResolution` mode: the TypeScript file, then the declaration file, then the
            // JavaScript file.
            extension_alias: [
                (".js", &[".ts", ".tsx", ".d.ts", ".js", ".jsx"][..]),
                (".jsx", &[".tsx", ".ts", ".d.ts", ".jsx", ".js"][..]),
                (".mjs", &[".mts", ".d.mts", ".mjs"][..]),
                (".cjs", &[".cts", ".d.cts", ".cjs"][..]),
            ]
            .map(|(from, to)| (from.to_owned(), to.iter().map(|&e| e.to_owned()).collect()))
            .to_vec(),
            condition_names: ["types", "import", "module", "node", "default", "require"]
                .map(String::from)
                .to_vec(),
            main_fields: ["types", "module", "main"].map(String::from).to_vec(),
            builtin_modules: true,
            ..ResolveOptions::default()
        };
        let overlay = Overlay::new(yarn_pnp, in_memory);
        let reads = overlay.reads();
        Self {
            resolver: ResolverGeneric::new_with_file_system(overlay, options),
            reads,
            reported: std::sync::Mutex::default(),
            modules: std::sync::Mutex::default(),
            listing: std::sync::RwLock::new(Arc::from(Vec::new())),
            pnp: yarn_pnp.then(|| repository.join(".pnp.cjs")),
            root: root.to_path_buf(),
        }
    }

    /// The files the resolver has read since the last call, at the path it read and, where that
    /// path passes through a link, at its real path: the files whose contents decide resolutions. A later call
    /// returns only new reads, so an edit costs nothing for the reads of earlier ones.
    pub(crate) fn files_read(&self) -> HashSet<PathBuf> {
        let seen = self
            .reads
            .lock()
            .map(|reads| reads.clone())
            .unwrap_or_default();
        let Ok(mut reported) = self.reported.lock() else {
            return seen.into_iter().chain(self.pnp.clone()).collect();
        };
        let new: Vec<PathBuf> = seen
            .into_iter()
            .filter(|path| reported.insert(path.clone()))
            .collect();
        // A link anywhere on the path, to the file or to a folder above it, gives it another real
        // path, and the watcher reports an edit there. Each read is reported once, so this costs
        // one lookup per file read.
        let real: Vec<PathBuf> = new
            .iter()
            .filter_map(|path| dunce::canonicalize(path).ok())
            .collect();
        new.into_iter()
            .chain(real)
            .chain(self.pnp.clone())
            .collect()
    }

    /// The tsconfig of an importing file, or the problem that stops archangel from using it.
    pub(crate) fn tsconfig(
        &self,
        file: &Path,
    ) -> (Option<Arc<TsConfig>>, Option<ResolutionFileProblem>) {
        if let Some(unreadable) = self.unreadable_tsconfig(file) {
            return (None, Some(unreadable));
        }
        match self.resolver.find_tsconfig(file) {
            Ok(found) => (found, None),
            Err(error) => (None, Some(self.file_problem(file, &error))),
        }
    }

    /// The nearest tsconfig when it cannot be read. It looks in the same folders as the resolver,
    /// which uses a tsconfig above the config folder too.
    fn unreadable_tsconfig(&self, file: &Path) -> Option<ResolutionFileProblem> {
        let candidate = file
            .ancestors()
            .skip(1)
            .map(|folder| folder.join("tsconfig.json"))
            .find(|candidate| candidate.is_file())?;
        std::fs::File::open(&candidate)
            .err()
            .map(|error| ResolutionFileProblem {
                path: relative(&self.root, &candidate),
                reason: if error.kind() == std::io::ErrorKind::PermissionDenied {
                    "archangel cannot read it: permission was refused".to_owned()
                } else {
                    format!("archangel cannot read it: {error}")
                },
                missing_package: None,
            })
    }

    /// Resolves one import from a file.
    pub(crate) fn resolve(
        &self,
        importer: &Importer<'_>,
        specifier: &str,
        inputs: &mut ResolutionInputs,
    ) -> (Resolution, Option<ResolutionFileProblem>) {
        let (resolution, problem, _) = self.resolve_to_file(importer, specifier, inputs);
        (resolution, problem)
    }

    /// Resolves one import, as a type import when the compiler removes it.
    pub(crate) fn resolve_import(
        &self,
        importer: &Importer<'_>,
        (specifier, removed): (&str, bool),
        inputs: &mut ResolutionInputs,
    ) -> (Resolution, Option<ResolutionFileProblem>, Option<PathBuf>) {
        if removed {
            self.resolve_type_import(importer, specifier, inputs)
        } else {
            self.resolve_to_file(importer, specifier, inputs)
        }
    }

    /// Resolves an import that the compiler removes. When its bare specifier names no installed
    /// package, it falls back to the `@types` package, as TypeScript does: `@types/<name>`, or
    /// `@types/<scope>__<name>` for a scoped package. The resolution keeps the name it imports.
    pub(crate) fn resolve_type_import(
        &self,
        importer: &Importer<'_>,
        specifier: &str,
        inputs: &mut ResolutionInputs,
    ) -> (Resolution, Option<ResolutionFileProblem>, Option<PathBuf>) {
        let found = self.resolve_to_file(importer, specifier, inputs);
        let not_found = found.0 == Resolution::Unresolved(UnresolvedReason::NotFound);
        let Some(package) = bare_package(specifier).filter(|_| not_found) else {
            return found;
        };
        let rest = &specifier[package.len()..];
        let types = match package.strip_prefix('@') {
            Some(scoped) => format!("@types/{}{rest}", scoped.replacen('/', "__", 1)),
            None => format!("@types/{package}{rest}"),
        };
        match self.resolve_to_file(importer, &types, inputs) {
            (Resolution::Package(_), problem, file) => {
                (Resolution::Package(package.to_owned()), problem, file)
            }
            _ => found,
        }
    }

    /// Resolves one import from a file, with the package file it resolves to, if any.
    pub(crate) fn resolve_to_file(
        &self,
        importer: &Importer<'_>,
        specifier: &str,
        inputs: &mut ResolutionInputs,
    ) -> (Resolution, Option<ResolutionFileProblem>, Option<PathBuf>) {
        let Importer { file, tsconfig } = *importer;
        let folder = file.parent().unwrap_or(&self.root);
        let mut context = ResolveContext::default();
        let result = self
            .resolver
            .resolve_with_context(folder, specifier, tsconfig, &mut context);
        // @justified intent
        // The files the resolver found and the paths it tried decide resolutions by existing. The
        // manifests and tsconfigs among them are also read, so `files_read` adds them to the
        // contents.
        inputs.existence.extend(context.file_dependencies);
        inputs.existence.extend(context.missing_dependencies);
        match result {
            Ok(resolution) => {
                let place = self.place(
                    resolution.path(),
                    resolution.package_json().and_then(|p| p.name()),
                );
                let package_file = matches!(place, Resolution::Package(_))
                    .then(|| resolution.path().to_path_buf());
                (place, None, package_file)
            }
            Err(ResolveError::Builtin { resolved, .. }) => {
                let name = if resolved.starts_with("node:") {
                    resolved
                } else {
                    format!("node:{resolved}")
                };
                (Resolution::Package(name), None, None)
            }
            Err(error) => {
                let mut reason = describe_resolve_error(&error);
                let javascript = [".js", ".mjs", ".cjs"]
                    .iter()
                    .any(|e| specifier.ends_with(e));
                if reason == UnresolvedReason::NotFound && javascript && is_relative(specifier) {
                    reason = UnresolvedReason::NoFileBehindJs;
                }
                let problem = (reason == UnresolvedReason::ResolutionFile)
                    .then(|| self.file_problem(file, &error));
                (Resolution::Unresolved(reason), problem, None)
            }
        }
    }

    /// Sets the files that template imports are matched against.
    pub(crate) fn set_listing(&self, mut paths: Vec<String>) {
        paths.sort();
        paths.dedup();
        if let Ok(mut listing) = self.listing.write() {
            *listing = Arc::from(paths);
        }
    }

    /// The files, relative to the config folder, that a template import's pattern reaches from
    /// `file`: each `*` stands for one file or folder name, or for part of one.
    pub(crate) fn template_files(&self, file: &Path, pattern: &str) -> Vec<String> {
        let folder = file
            .parent()
            .map_or_else(String::new, |folder| relative(&self.root, folder));
        let mut names: Vec<&str> = folder.split('/').filter(|name| !name.is_empty()).collect();
        for name in pattern.split('/') {
            match name {
                "." | "" => {}
                ".." => {
                    if names.pop().is_none() {
                        return Vec::new();
                    }
                }
                _ => names.push(name),
            }
        }
        let Ok(listing) = self.listing.read() else {
            return Vec::new();
        };
        listing
            .iter()
            .filter(|path| {
                let parts: Vec<&str> = path.split('/').collect();
                parts.len() == names.len()
                    && parts
                        .iter()
                        .zip(&names)
                        .all(|(part, name)| wildcard(name, part))
            })
            .cloned()
            .collect()
    }

    /// The names that a package module exports, and how many package files this call parsed to
    /// find them. Each file it reads decides the names, so it joins the resolution inputs.
    pub(crate) fn module_names(
        &self,
        module: &Path,
        inputs: &mut ResolutionInputs,
    ) -> (Option<Arc<[String]>>, u64) {
        let Ok(mut modules) = self.modules.lock() else {
            return (None, 0);
        };
        let mut parsed = 0;
        let found = modules.entry(module.to_path_buf()).or_insert_with(|| {
            let (names, read) = module_reach(self, module);
            parsed = read.len() as u64;
            ModuleNames {
                names: names.map(Arc::from),
                read,
            }
        });
        inputs.contents.extend(found.read.iter().cloned());
        (found.names.clone(), parsed)
    }

    /// A repository file, or a package when the path is inside `node_modules` or Yarn's cache.
    fn place(&self, path: &Path, package_name: Option<&str>) -> Resolution {
        let in_packages = path.components().any(|part| {
            matches!(part, Component::Normal(name) if name == "node_modules" || name == ".yarn")
        });
        if in_packages {
            let name = package_name.map_or_else(|| package_from_path(path), str::to_owned);
            return Resolution::Package(name);
        }
        Resolution::File(relative(&self.root, path))
    }

    fn file_problem(&self, file: &Path, error: &ResolveError) -> ResolutionFileProblem {
        let missing_package = match error {
            ResolveError::TsconfigNotFound(path) => extends_package(path),
            _ => None,
        };
        let (path, reason) = match error {
            ResolveError::Json(json) => (
                json.path.clone(),
                format!(
                    "it is not valid JSON at line {}, column {}: {}",
                    json.line, json.column, json.message
                ),
            ),
            ResolveError::TsconfigNotFound(path) => (
                self.tsconfig_of(file),
                format!(
                    "its `extends` names {}, which is not installed or does not exist",
                    extends_name(path)
                ),
            ),
            ResolveError::TsconfigCircularExtend(chain) => (
                self.tsconfig_of(file),
                // The chain as repository paths, so the message is the same on every machine.
                format!(
                    "its `extends` chain loops: {}",
                    chain
                        .paths()
                        .iter()
                        .map(|link| format!("`{}`", relative(&self.root, link)))
                        .collect::<Vec<_>>()
                        .join(" -> ")
                ),
            ),
            ResolveError::TsconfigSelfReference(path) => {
                (path.clone(), "it extends or references itself".to_owned())
            }
            ResolveError::InvalidPackageConfig(path)
            | ResolveError::InvalidPackageConfigDefault(path)
            | ResolveError::InvalidPackageConfigDirectory(path) => (
                path.clone(),
                "its `exports` or `imports` field is not valid".to_owned(),
            ),
            // The resolver names no path once the file is gone, so the diagnostic names the file
            // that this resolver was made to read.
            ResolveError::FailedToFindYarnPnpManifest(path) => (
                self.pnp.clone().unwrap_or_else(|| path.clone()),
                "the Plug'n'Play file is gone, so no package can be resolved through it".to_owned(),
            ),
            _ => (self.tsconfig_of(file), "it is not valid".to_owned()),
        };
        ResolutionFileProblem {
            path: relative(&self.root, &path),
            reason,
            missing_package,
        }
    }

    fn tsconfig_of(&self, file: &Path) -> PathBuf {
        file.ancestors()
            .skip(1)
            .map(|folder| folder.join("tsconfig.json"))
            .find(|candidate| candidate.is_file())
            .unwrap_or_else(|| self.root.join("tsconfig.json"))
    }
}

/// The names of a package module, with the files read to find them.
#[derive(Debug, Clone)]
struct ModuleNames {
    /// `None` when archangel cannot see every name.
    names: Option<Arc<[String]>>,
    read: Vec<PathBuf>,
}

/// The most package files that one module's names may take to read.
const MODULE_FILES_LIMIT: usize = 64;

/// The names that `module` exports, through its relative `export * from` statements, with the
/// files read to find them. The names are `None` when archangel cannot see them all: a file shows
/// no ESM export (a `CommonJS` module, `export =`, or no export at all) or does not parse, an
/// `export *` names a bare specifier or does not resolve, or the walk needs more than
/// `MODULE_FILES_LIMIT` files.
fn module_reach(resolver: &ImportResolver, module: &Path) -> (Option<Vec<String>>, Vec<PathBuf>) {
    let mut read: Vec<PathBuf> = Vec::new();
    let mut pending = vec![module.to_path_buf()];
    let mut names = Vec::new();
    for _ in 0..=MODULE_FILES_LIMIT {
        let Some(next) = pending.pop() else {
            names.sort();
            names.dedup();
            return (Some(names), read);
        };
        if read.len() == MODULE_FILES_LIMIT {
            break;
        }
        let exports = own_exports(resolver, &next);
        read.push(next);
        let Some((own, targets)) = exports else {
            return (None, read);
        };
        names.extend(own);
        for target in targets {
            if !read.contains(&target) && !pending.contains(&target) {
                pending.push(target);
            }
        }
    }
    (None, read)
}

/// The names that one package file exports itself, and the files that its `export * from`
/// statements name, or `None` when archangel cannot see them all.
fn own_exports(resolver: &ImportResolver, module: &Path) -> Option<(Vec<String>, Vec<PathBuf>)> {
    let text = std::fs::read_to_string(module).ok()?;
    let facts = crate::imports::extract_file(&module.to_string_lossy(), &text).ok()?;
    if facts.exports.entries.is_empty() {
        return None;
    }
    let folder = module.parent()?;
    let targets = facts
        .exports
        .star_from
        .iter()
        .map(|specifier| {
            is_relative(specifier).then_some(())?;
            resolver
                .resolver
                .resolve_with_context(folder, specifier, None, &mut ResolveContext::default())
                .ok()
                .map(|target| target.path().to_path_buf())
        })
        .collect::<Option<Vec<_>>>()?;
    Some((facts.exports.names, targets))
}

/// The package name of a bare specifier, `@scope/name` or `name`; `None` for a relative or
/// absolute path, a `#` import, a URL-like specifier or an `@types` package.
fn bare_package(specifier: &str) -> Option<&str> {
    if is_relative(specifier)
        || specifier.starts_with('#')
        || specifier.contains(':')
        || specifier.starts_with("@types/")
    {
        return None;
    }
    let end = if specifier.starts_with('@') {
        specifier.match_indices('/').nth(1).map(|(index, _)| index)
    } else {
        specifier.find('/')
    };
    let package = &specifier[..end.unwrap_or(specifier.len())];
    (!package.is_empty() && (!package.starts_with('@') || package.contains('/'))).then_some(package)
}

/// Whether a name matches a pattern of one name, in which each `*` stands for any text.
fn wildcard(pattern: &str, name: &str) -> bool {
    let mut pieces = pattern.split('*');
    let first = pieces.next().unwrap_or("");
    let Some(mut rest) = name.strip_prefix(first) else {
        return false;
    };
    let pieces: Vec<&str> = pieces.collect();
    let Some((last, middle)) = pieces.split_last() else {
        return rest.is_empty();
    };
    for piece in middle {
        match rest.find(piece) {
            Some(at) => rest = &rest[at + piece.len()..],
            None => return false,
        }
    }
    rest.len() >= last.len() && rest.ends_with(last)
}

fn is_relative(specifier: &str) -> bool {
    specifier.starts_with('.') || specifier.starts_with('/')
}

/// The package name from a path such as `.../node_modules/@scope/name/x.js`.
fn package_from_path(path: &Path) -> String {
    let parts: Vec<String> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    let after = parts
        .iter()
        .rposition(|part| part == "node_modules")
        .map_or(0, |i| i + 1);
    match parts.get(after) {
        Some(scope) if scope.starts_with('@') => {
            format!(
                "{scope}/{}",
                parts.get(after + 1).map_or("", String::as_str)
            )
        }
        Some(name) => name.clone(),
        None => path.display().to_string(),
    }
}

/// The package that a missing `extends` names, which the resolver reports as written: `@scope/name`
/// or `name` from a bare specifier, and nothing for a relative or absolute path.
fn extends_package(written: &Path) -> Option<String> {
    let text = written.to_string_lossy().replace('\\', "/");
    if is_relative(&text) || written.is_absolute() {
        return None;
    }
    let mut parts = text.split('/');
    let first = parts.next()?;
    if first.starts_with('@') {
        parts.next().map(|name| format!("{first}/{name}"))
    } else {
        Some(first.to_owned())
    }
}

/// The package or path an `extends` named, from the path the resolver looked for.
fn extends_name(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    if let Some(scope) = text.find('@') {
        let rest: Vec<&str> = text[scope..].split('/').collect();
        if rest.len() >= 2 {
            return format!("`{}/{}`", rest[0], rest[1]);
        }
    }
    format!("`{text}`")
}
