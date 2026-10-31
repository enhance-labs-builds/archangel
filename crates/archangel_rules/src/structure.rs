//! The rules about files rather than imports: names of files and folders, files every instance or
//! folder must have, files that must not exist, and what files export. They cover every file the
//! walk visits, source or not, and a session keeps their results per file, folder and instance,
//! so that a change evaluates only what it touched.

use std::collections::{BTreeMap, BTreeSet};

use archangel_config::{Classification, Condition, NameRule, Rule};
use archangel_graph::{ImportGraph, OverlayFiles, Work};

use crate::architecture::{Architecture, Compiled};
use crate::classes::{Classes, Lookup};
use crate::diagnostic::{Code, Diagnostic};
use crate::exports::{self, About, Sources};
use crate::naming::{FileName, convert, expand, is_named, matches, spelled};

/// The results of the rules about files, folders and instances, kept between changes.
#[derive(Debug, Clone, Default)]
pub struct Structure {
    /// Every path the walk visited, source files and others, when the results were made.
    paths: BTreeSet<String>,
    /// Every folder that holds a path, with the number of paths below it.
    folders: BTreeMap<String, u64>,
    /// For each sentence, by its index, the number of folders it covers.
    covered: Vec<u64>,
    /// The naming and `mustNotExist` results of each file.
    files: BTreeMap<String, Vec<Diagnostic>>,
    /// The export results of each source file.
    exports: BTreeMap<String, Vec<Diagnostic>>,
    /// The results of each folder.
    folder_results: BTreeMap<String, Vec<Diagnostic>>,
    /// The results of each instance, by its element and root.
    instances: BTreeMap<(u32, String), Vec<Diagnostic>>,
}

/// What a session keeps for the rules about files, folders and instances: the graph, the class
/// of each file, and the results.
#[derive(Debug)]
pub struct Kept<'a> {
    /// The import graph, after the change.
    pub graph: &'a ImportGraph,
    /// The class of each file, after the change.
    pub classes: &'a mut Classes,
    /// The results, from before the change.
    pub structure: &'a mut Structure,
}

/// An example's files over the repository, and the rule whose example they are.
#[derive(Clone, Copy)]
pub(crate) struct Example<'a> {
    pub(crate) graph: &'a ImportGraph,
    pub(crate) overlay: &'a OverlayFiles,
    pub(crate) rule: usize,
}

/// What changed since the structure was last evaluated.
#[derive(Debug, Clone, Copy, Default)]
pub struct StructureChange<'a> {
    /// Files whose text changed, or whose imports resolve elsewhere: their export rules, and
    /// those of the files that pass their names on through `export *`, are evaluated again.
    pub texts: &'a [String],
    /// Files whose class changed: their naming and existence rules too.
    pub reclassified: &'a [String],
    /// Whether files may have appeared or disappeared, so that the paths are compared.
    pub paths: bool,
    /// Whether every file's export rules are evaluated again, as after new resolutions.
    pub every_export: bool,
    /// Whether every folder is evaluated again, for the fault control of folder evaluations.
    pub every_folder: bool,
}

impl Architecture {
    /// Evaluates the rules about files, folders and instances for the whole graph. The classes
    /// must hold every file the graph has.
    #[must_use]
    pub fn structure(
        &self,
        graph: &ImportGraph,
        classes: &mut Classes,
        work: &mut Work,
    ) -> Structure {
        let mut structure = Structure {
            covered: vec![0; self.sentences.len()],
            ..Structure::default()
        };
        let all = paths_of(graph);
        let added: Vec<&str> = all.iter().map(String::as_str).collect();
        structure.add_paths(self, &added);
        let mut evaluation = Evaluation {
            architecture: self,
            graph,
            overlay: None,
            classes,
            structure: &mut structure,
            work,
            watched: None,
        };
        evaluation.files(&added);
        evaluation.exports(&added);
        let folders: Vec<String> = evaluation.structure.folders.keys().cloned().collect();
        evaluation.folders(&folders);
        let instances = evaluation.instances_of(&added);
        evaluation.instances(&instances);
        structure
    }

    /// Evaluates again what a change touched. The classes must already hold the change.
    pub fn update_structure(&self, kept: Kept<'_>, change: &StructureChange<'_>, work: &mut Work) {
        let Kept {
            graph,
            classes,
            structure,
        } = kept;
        let (added, removed) = if change.paths {
            structure.difference(graph)
        } else {
            (Vec::new(), Vec::new())
        };
        let mut evaluation = Evaluation {
            architecture: self,
            graph,
            overlay: None,
            classes,
            structure,
            work,
            watched: None,
        };
        let gone_instances = evaluation.moved(&added, &removed);
        let added: Vec<&str> = added.iter().map(String::as_str).collect();
        let mut files = added.clone();
        files.extend(change.reclassified.iter().map(String::as_str));
        evaluation.files(&files);
        evaluation.changed_exports(change, &files);
        let folders = if change.every_folder {
            evaluation.structure.folders.keys().cloned().collect()
        } else {
            touched(&added, &removed)
        };
        evaluation.changed_folders(&folders);
        let mut instances = evaluation.instances_of(&added);
        instances.extend(gone_instances);
        evaluation.instances(&instances);
    }

    /// The results of the rules about files, folders and instances for an example's files, laid
    /// over the repository: each example file, the folders that hold one and the instances that
    /// have one. Returns them, and whether the example's rule looked at a file, folder or
    /// instance of the example.
    pub(crate) fn example_structure(
        &self,
        example: Example<'_>,
        classes: &mut Classes,
        work: &mut Work,
    ) -> (Vec<Diagnostic>, bool) {
        let Example {
            graph,
            overlay,
            rule,
        } = example;
        let own: Vec<String> = overlay
            .files
            .iter()
            .map(|f| f.path.clone())
            .chain(overlay.others.iter().cloned())
            .collect();
        let mut structure = Structure {
            paths: paths_of(graph),
            ..Structure::default()
        };
        structure.paths.extend(own.iter().cloned());
        let refs: Vec<&str> = own.iter().map(String::as_str).collect();
        let mut evaluation = Evaluation {
            architecture: self,
            graph,
            overlay: Some(overlay),
            classes,
            structure: &mut structure,
            work,
            watched: Some((rule, false)),
        };
        evaluation.files(&refs);
        let sources: Vec<&str> = overlay.files.iter().map(|f| f.path.as_str()).collect();
        evaluation.exports(&sources);
        evaluation.folders(&touched::<&str, &str>(&refs, &[]));
        let instances = evaluation.instances_of(&refs);
        evaluation.instances(&instances);
        let considered = evaluation.watched.is_some_and(|(_, considered)| considered);
        let found = structure
            .files
            .into_values()
            .chain(structure.exports.into_values())
            .chain(structure.folder_results.into_values())
            .chain(structure.instances.into_values())
            .flatten()
            .collect();
        (found, considered)
    }

    /// The diagnostics of the rules about files, folders and instances, and a `selector-empty`
    /// diagnostic for each folder pattern that matches no folder.
    pub(crate) fn structure_diagnostics(&self, structure: &Structure) -> Vec<Diagnostic> {
        let mut found: Vec<Diagnostic> = structure
            .files
            .values()
            .chain(structure.exports.values())
            .chain(structure.folder_results.values())
            .chain(structure.instances.values())
            .flatten()
            .cloned()
            .collect();
        for (index, compiled) in self.sentences.iter().enumerate() {
            let (rule, sentence) = self.sentence_of(compiled);
            let Some(patterns) = &sentence.folders else {
                continue;
            };
            if structure.covered.get(index).copied().unwrap_or(0) > 0
                || self.from_preset(&rule.name)
            {
                continue;
            }
            let listed: Vec<&str> = patterns
                .patterns
                .iter()
                .map(archangel_config::Pattern::source)
                .collect();
            let mut diagnostic = Diagnostic::new(
                Code::SelectorEmpty,
                format!(
                    "the rule \"{}\" names the folders {}, which match no folder, so the rule \
                     cannot fire; fix the pattern",
                    rule.name,
                    listed.join(", ")
                ),
            );
            diagnostic.rule = Some(rule.name.clone());
            found.push(diagnostic);
        }
        found
    }

    fn sentence_of(&self, compiled: &Compiled) -> (&Rule, &archangel_config::Sentence) {
        let rule = &self.config.rules[compiled.rule];
        (rule, &rule.sentences[compiled.sentence])
    }

    /// Whether a folder sentence covers a folder.
    fn covers_folder(&self, compiled: &Compiled, folder: &str) -> bool {
        let (_, sentence) = self.sentence_of(compiled);
        sentence
            .folders
            .as_ref()
            .is_some_and(|folders| folders.covers(folder))
    }
}

impl Structure {
    /// The paths that appeared and disappeared since the last evaluation.
    fn difference(&self, graph: &ImportGraph) -> (Vec<String>, Vec<String>) {
        let now = paths_of(graph);
        let added = now.difference(&self.paths).cloned().collect();
        let removed = self.paths.difference(&now).cloned().collect();
        (added, removed)
    }

    fn add_paths(&mut self, architecture: &Architecture, paths: &[&str]) {
        for path in paths {
            self.paths.insert((*path).to_owned());
            for folder in folders_of(path) {
                let count = self.folders.entry(folder.to_owned()).or_default();
                *count += 1;
                if *count == 1 {
                    self.count_cover(architecture, folder, true);
                }
            }
        }
    }

    fn remove_paths(&mut self, architecture: &Architecture, paths: &[&str]) {
        for path in paths {
            self.paths.remove(*path);
            for folder in folders_of(path) {
                let Some(count) = self.folders.get_mut(folder) else {
                    continue;
                };
                *count -= 1;
                if *count == 0 {
                    self.folders.remove(folder);
                    self.count_cover(architecture, folder, false);
                }
            }
        }
    }

    fn count_cover(&mut self, architecture: &Architecture, folder: &str, added: bool) {
        for (index, compiled) in architecture.sentences.iter().enumerate() {
            if architecture.covers_folder(compiled, folder) {
                if added {
                    self.covered[index] += 1;
                } else {
                    self.covered[index] = self.covered[index].saturating_sub(1);
                }
            }
        }
    }

    /// The paths below a folder, or an instance root, in path order.
    fn below<'s>(&'s self, folder: &str) -> impl Iterator<Item = &'s String> {
        let prefix = format!("{folder}/");
        self.paths
            .range(prefix.clone()..)
            .take_while(move |path| path.starts_with(&prefix))
    }
}

/// Every path of the graph: the source files and the other files the walk visited.
fn paths_of(graph: &ImportGraph) -> BTreeSet<String> {
    graph
        .files()
        .iter()
        .map(|file| file.path.clone())
        .chain(graph.others().iter().cloned())
        .collect()
}

/// The folders that hold a path, nearest last, the config folder itself left out.
fn folders_of(path: &str) -> impl Iterator<Item = &str> {
    path.match_indices('/').map(move |(end, _)| &path[..end])
}

/// The folders whose contents changed: those that hold a path that appeared or disappeared.
fn touched<A: AsRef<str>, R: AsRef<str>>(added: &[A], removed: &[R]) -> Vec<String> {
    let folders: BTreeSet<&str> = added
        .iter()
        .map(AsRef::as_ref)
        .chain(removed.iter().map(AsRef::as_ref))
        .flat_map(folders_of)
        .collect();
    folders.into_iter().map(str::to_owned).collect()
}

/// One evaluation of some files, folders and instances.
struct Evaluation<'e, 'w> {
    architecture: &'e Architecture,
    graph: &'e ImportGraph,
    /// An example's files, which count instead of the repository's at the same paths.
    overlay: Option<&'e OverlayFiles>,
    classes: &'e mut Classes,
    structure: &'e mut Structure,
    work: &'w mut Work,
    /// The rule whose example this evaluation checks, and whether it looked at anything.
    watched: Option<(usize, bool)>,
}

impl<'e> Evaluation<'e, '_> {
    /// Takes the paths that disappeared out of the results and the folders, and puts the paths
    /// that appeared in. Returns the instances that held a path that disappeared.
    fn moved(&mut self, added: &[String], removed: &[String]) -> BTreeSet<(u32, String)> {
        let gone = self.instances_of_known(removed);
        for path in removed {
            self.structure.files.remove(path);
            self.structure.exports.remove(path);
        }
        let added: Vec<&str> = added.iter().map(String::as_str).collect();
        let removed: Vec<&str> = removed.iter().map(String::as_str).collect();
        self.structure.remove_paths(self.architecture, &removed);
        self.structure.add_paths(self.architecture, &added);
        gone
    }

    /// The export rules of the files a change touched, and of the files that pass their names on
    /// through `export *`, or of every file.
    fn changed_exports(&mut self, change: &StructureChange<'_>, files: &[&str]) {
        if !self.any(|sentence| sentence.condition.is_export()) {
            return;
        }
        let graph = self.graph;
        if change.every_export {
            let every: Vec<&str> = graph.files().iter().map(|f| f.path.as_str()).collect();
            self.exports(&every);
            return;
        }
        let mut sources: BTreeSet<&str> = files.iter().copied().collect();
        sources.extend(change.texts.iter().map(String::as_str));
        let roots: Vec<&str> = sources.iter().copied().collect();
        let barrels = exports::star_importers(graph, &roots);
        sources.extend(barrels.iter().map(String::as_str));
        let sources: Vec<&str> = sources.into_iter().collect();
        self.exports(&sources);
    }

    /// The folder rules of the folders a change touched, forgetting the folders that are gone.
    fn changed_folders(&mut self, folders: &[String]) {
        let (existing, gone): (Vec<String>, Vec<String>) = folders
            .iter()
            .cloned()
            .partition(|folder| self.structure.folders.contains_key(folder));
        for folder in &gone {
            self.structure.folder_results.remove(folder);
        }
        self.folders(&existing);
    }

    /// Whether any sentence is of a kind, so that a config without such rules costs nothing.
    fn any(&self, kind: impl Fn(&archangel_config::Sentence) -> bool) -> bool {
        self.architecture
            .sentences
            .iter()
            .any(|compiled| kind(self.architecture.sentence_of(compiled).1))
    }

    /// Notes that a rule looked at a file, folder or instance.
    fn looked(&mut self, rule: usize) {
        if let Some((watched, considered)) = &mut self.watched
            && *watched == rule
        {
            *considered = true;
        }
    }

    fn sources(&self) -> Sources<'e> {
        Sources {
            graph: self.graph,
            overlay: self.overlay,
        }
    }

    /// The naming and existence rules of each file.
    fn files(&mut self, paths: &[&str]) {
        let architecture = self.architecture;
        let per_file = |sentence: &archangel_config::Sentence| {
            sentence.folders.is_none()
                && matches!(
                    sentence.condition,
                    Condition::MustBeNamed(_) | Condition::MustNotExist
                )
        };
        let candidates: Vec<_> = architecture
            .sentences
            .iter()
            .filter(|compiled| per_file(architecture.sentence_of(compiled).1))
            .collect();
        if candidates.is_empty() {
            return;
        }
        for path in paths {
            let lookup = Lookup::new(architecture, self.graph);
            self.classes.file(lookup, path, self.work);
            let bits = self
                .classes
                .bits(path)
                .map(<[bool]>::to_vec)
                .unwrap_or_default();
            let mut found = Vec::new();
            for compiled in &candidates {
                if !bits.get(compiled.files).copied().unwrap_or(false) {
                    continue;
                }
                let (rule, sentence) = architecture.sentence_of(compiled);
                self.work.structure_evaluations += 1;
                self.looked(compiled.rule);
                match &sentence.condition {
                    Condition::MustBeNamed(name) => found.extend(file_name(rule, name, path)),
                    _ => found.push(self.must_not_exist(rule, &sentence.files_text, path)),
                }
            }
            if found.is_empty() {
                self.structure.files.remove(*path);
            } else {
                self.structure.files.insert((*path).to_owned(), found);
            }
        }
    }

    fn must_not_exist(&self, rule: &Rule, files: &str, path: &str) -> Diagnostic {
        let elements = &self.architecture.config.elements;
        let unplaced = if self.classes.classification(path).is_none() && !elements.is_empty() {
            let names: Vec<String> = elements.iter().map(|e| format!("\"{}\"", e.name)).collect();
            format!(
                ", and it is in no element; the elements are {}",
                names.join(", ")
            )
        } else {
            String::new()
        };
        About { rule, path }.violation(
            Code::MustNotExist,
            format!("{path} must not exist, because it is in {files}{unplaced}"),
            "move the file to where the architecture puts it, or delete it",
        )
    }

    /// The export rules of each parsed source file.
    fn exports(&mut self, paths: &[&str]) {
        let architecture = self.architecture;
        if !self.any(|sentence| sentence.condition.is_export()) {
            return;
        }
        for path in paths {
            let Some((facts, _)) = self.sources().facts(path) else {
                self.structure.exports.remove(*path);
                continue;
            };
            let mut found = Vec::new();
            for compiled in &architecture.sentences {
                let (rule, sentence) = architecture.sentence_of(compiled);
                if !sentence.condition.is_export() || !self.classes.file_in(path, compiled.files) {
                    continue;
                }
                self.work.export_evaluations += 1;
                self.looked(compiled.rule);
                found.extend(exports::evaluate(
                    self.sources(),
                    About { rule, path },
                    &sentence.condition,
                    facts,
                ));
            }
            if found.is_empty() {
                self.structure.exports.remove(*path);
            } else {
                self.structure.exports.insert((*path).to_owned(), found);
            }
        }
    }

    /// The folder rules of each folder.
    fn folders(&mut self, folders: &[String]) {
        let architecture = self.architecture;
        if !self.any(|sentence| sentence.folders.is_some()) {
            return;
        }
        for folder in folders {
            let mut found = Vec::new();
            for compiled in &architecture.sentences {
                if !architecture.covers_folder(compiled, folder) {
                    continue;
                }
                let (rule, sentence) = architecture.sentence_of(compiled);
                self.work.folder_evaluations += 1;
                self.looked(compiled.rule);
                match &sentence.condition {
                    Condition::MustBeNamed(name) => found.extend(folder_name(rule, name, folder)),
                    Condition::MustContain(patterns) => {
                        let holds = self.structure.below(folder).any(|path| {
                            patterns
                                .iter()
                                .any(|p| p.matches(&path[folder.len() + 1..]).is_some())
                        });
                        if !holds {
                            let wanted = crate::words::either(patterns);
                            let first = patterns.first().map_or("", |p| p.source());
                            found.push(About { rule, path: folder }.violation(
                                Code::MustContain,
                                format!("the folder {folder} has no file that matches {wanted}"),
                                &format!("add {folder}/{first}"),
                            ));
                        }
                    }
                    Condition::MustNotExist => {
                        found.push(About { rule, path: folder }.violation(
                            Code::MustNotExist,
                            format!("the folder {folder} may not exist"),
                            &format!("move what {folder} holds elsewhere, and remove the folder"),
                        ));
                    }
                    _ => {}
                }
            }
            if found.is_empty() {
                self.structure.folder_results.remove(folder);
            } else {
                self.structure.folder_results.insert(folder.clone(), found);
            }
        }
    }

    /// The instances that hold the paths, of the elements that a `mustContain` rule is about.
    fn instances_of(&mut self, paths: &[&str]) -> BTreeSet<(u32, String)> {
        let elements = self.contained_elements();
        if elements.is_empty() {
            return BTreeSet::new();
        }
        let lookup = Lookup::new(self.architecture, self.graph);
        paths
            .iter()
            .filter_map(|path| {
                let (_, classification) = self.classes.file(lookup, path, self.work);
                classification
                    .filter(|c| elements.contains(&c.element.0))
                    .map(|c| (c.element.0, c.root))
            })
            .collect()
    }

    /// The instances that held paths before they disappeared. A path's element follows from the
    /// path alone, so the matcher gives it without a class.
    fn instances_of_known(&self, paths: &[String]) -> BTreeSet<(u32, String)> {
        let elements = self.contained_elements();
        if elements.is_empty() {
            return BTreeSet::new();
        }
        paths
            .iter()
            .filter_map(|path| self.architecture.matcher.classify_file(path))
            .filter(|c| elements.contains(&c.element.0))
            .map(|c| (c.element.0, c.root))
            .collect()
    }

    fn contained_elements(&self) -> BTreeSet<u32> {
        self.architecture
            .sentences
            .iter()
            .filter_map(|compiled| {
                let (_, sentence) = self.architecture.sentence_of(compiled);
                match (&sentence.condition, &sentence.folders) {
                    (Condition::MustContain(_), None) => sentence.files.element().map(|e| e.0),
                    _ => None,
                }
            })
            .collect()
    }

    /// The `mustContain` rules of each instance.
    fn instances(&mut self, instances: &BTreeSet<(u32, String)>) {
        let architecture = self.architecture;
        for (element, root) in instances {
            let members = self.members(*element, root);
            let mut found = Vec::new();
            for compiled in members
                .first()
                .map_or(&[][..], |_| &architecture.sentences[..])
            {
                let (rule, sentence) = architecture.sentence_of(compiled);
                let (Condition::MustContain(patterns), None) =
                    (&sentence.condition, &sentence.folders)
                else {
                    continue;
                };
                if sentence.files.element().map(|e| e.0) != Some(*element) {
                    continue;
                }
                self.work.structure_evaluations += 1;
                self.looked(compiled.rule);
                let holds = members.iter().any(|path| {
                    patterns
                        .iter()
                        .any(|p| p.matches(&path[root.len() + 1..]).is_some())
                });
                if !holds {
                    let instance = self
                        .classes
                        .classification(&members[0])
                        .map(|c| self.instance_name(c))
                        .unwrap_or_default();
                    let wanted = crate::words::either(patterns);
                    let first = patterns.first().map_or("", |p| p.source());
                    found.push(About { rule, path: root }.violation(
                        Code::MustContain,
                        format!(
                            "the instance {instance} has no file that matches {wanted}, such as \
                             {root}/{first}"
                        ),
                        &format!("add {root}/{first}"),
                    ));
                }
            }
            let key = (*element, root.clone());
            if found.is_empty() {
                self.structure.instances.remove(&key);
            } else {
                self.structure.instances.insert(key, found);
            }
        }
    }

    /// The paths of one instance: those below its root that are in it.
    fn members(&mut self, element: u32, root: &str) -> Vec<String> {
        let below: Vec<String> = self.structure.below(root).cloned().collect();
        let lookup = Lookup::new(self.architecture, self.graph);
        below
            .into_iter()
            .filter(|path| {
                self.classes
                    .file(lookup, path, self.work)
                    .1
                    .is_some_and(|c| c.element.0 == element && c.root == root)
            })
            .collect()
    }

    fn instance_name(&self, class: &Classification) -> String {
        let element = &self.architecture.config.elements[class.element.index()].name;
        format!("\"{}\" of \"{element}\"", class.captures.join("/"))
    }
}

/// The violation of a naming rule by a file, if it breaks it.
fn file_name(rule: &Rule, name: &NameRule, path: &str) -> Option<Diagnostic> {
    let names = FileName::of(path);
    match name {
        NameRule::Style(style) => (!is_named(names.name, *style)).then(|| {
            let renamed = names.renamed(*style);
            About { rule, path }.violation(
                Code::MustBeNamed,
                format!(
                    "the name of {} is {}, which is not {style}; in {style} the file is {renamed}",
                    names.base, names.name
                ),
                &format!("name it {renamed}"),
            )
        }),
        NameRule::Pattern(pattern) => {
            let pieces = expand(pattern, &names);
            (!matches(&pieces, names.full)).then(|| {
                let wanted = spelled(&pieces);
                About { rule, path }.violation(
                    Code::MustBeNamed,
                    format!(
                        "the full name of {} is {}, and it must be {wanted}",
                        names.base, names.full
                    ),
                    &format!("rename the file so that its name without the extension is {wanted}"),
                )
            })
        }
    }
}

/// The violation of a naming rule by a folder, if it breaks it. A folder's name is all of its
/// last part.
fn folder_name(rule: &Rule, name: &NameRule, folder: &str) -> Option<Diagnostic> {
    let names = FileName::of(folder);
    let parent = &folder[..folder.len() - names.base.len()];
    match name {
        NameRule::Style(style) => (!is_named(names.base, *style)).then(|| {
            let renamed = convert(names.base, *style);
            About { rule, path: folder }.violation(
                Code::MustBeNamed,
                format!("the folder {folder} is not {style}; in {style} it is {parent}{renamed}"),
                &format!("name it {parent}{renamed}"),
            )
        }),
        NameRule::Pattern(pattern) => {
            let pieces = expand(pattern, &names);
            (!matches(&pieces, names.base)).then(|| {
                let wanted = spelled(&pieces);
                About { rule, path: folder }.violation(
                    Code::MustBeNamed,
                    format!("the folder {folder} must be named {wanted}"),
                    &format!("name it {parent}{wanted}"),
                )
            })
        }
    }
}
