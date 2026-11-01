//! The class of each file and package: its element and the sets of files it is in. Files of one
//! class are alike to every rule, so the rule table has one cell per pair of classes that an import
//! connects, and a session keeps each file's class so that an edit classifies one file.

use std::collections::{BTreeMap, HashMap};

use archangel_config::{Classification, ElementId, FileSet, Term};
use archangel_graph::{ImportGraph, SourceFile, Work};

use crate::architecture::{Architecture, Check};

/// What makes two files, or two packages, alike to every rule.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum ClassKey {
    /// A file: its elements and, for each set of files the rules name, whether it is in it.
    File {
        elements: Vec<ElementId>,
        bits: Vec<bool>,
    },
    /// A package: its elements and, for each inline package name, whether it names the package.
    Package {
        elements: Vec<ElementId>,
        names: Vec<bool>,
    },
}

/// One file's place.
#[derive(Debug, Clone)]
struct FileClass {
    classification: Option<Classification>,
    class: u32,
    /// Whether the walk visited the file, as a source file or another, which the counts of each
    /// class hold.
    counted: bool,
}

/// The compiled config and the graph whose files are classified.
#[derive(Debug, Clone, Copy)]
pub struct Lookup<'a> {
    architecture: &'a Architecture,
    graph: Option<&'a ImportGraph>,
}

impl<'a> Lookup<'a> {
    /// The config and the graph whose files' texts give their directives.
    #[must_use]
    pub fn new(architecture: &'a Architecture, graph: &'a ImportGraph) -> Self {
        Self {
            architecture,
            graph: Some(graph),
        }
    }

    /// The config alone, for paths whose texts are not known.
    pub(crate) fn paths(architecture: &'a Architecture) -> Self {
        Self {
            architecture,
            graph: None,
        }
    }
}

/// A file to put in its class: its path, its directive, and whether the counts hold it.
struct Placed<'p> {
    path: &'p str,
    directive: Option<&'p str>,
    counted: bool,
}

/// The classes of a check or a session, and the rule table cells of the pairs met so far.
#[derive(Debug, Clone, Default)]
pub struct Classes {
    keys: Vec<ClassKey>,
    index: HashMap<ClassKey, u32>,
    files: HashMap<String, FileClass>,
    packages: HashMap<String, (u32, Option<Classification>)>,
    /// The files the walk visited in each class. A file that only an import points to, such as
    /// one the ignore list leaves out, is in no count, because the walk never visits it.
    counts: Vec<u64>,
    cells: HashMap<(u32, u32), Vec<Check>>,
    /// The files that the walk visited in two or more of the config's own parts, with the parts.
    overlaps: BTreeMap<String, Vec<ElementId>>,
}

impl Classes {
    /// No class yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Classifies every file of the graph that has no class yet, source files and the other files
    /// the walk visited, and forgets the files the graph no longer has.
    pub fn sync(&mut self, architecture: &Architecture, graph: &ImportGraph, work: &mut Work) {
        let gone: Vec<String> = self
            .files
            .iter()
            .filter(|(path, file)| {
                file.counted && graph.file(path).is_none() && !graph.is_other(path)
            })
            .map(|(path, _)| path.clone())
            .collect();
        for path in gone {
            self.overlaps.remove(&path);
            if let Some(file) = self.files.remove(&path) {
                self.counts[file.class as usize] -= 1;
            }
        }
        let lookup = Lookup::new(architecture, graph);
        let paths = graph
            .files()
            .iter()
            .map(|file| file.path.as_str())
            .chain(graph.others().iter().map(String::as_str));
        for path in paths {
            if self.files.get(path).is_none_or(|known| !known.counted) {
                self.classify(lookup, path, work);
            }
        }
    }

    /// Classifies a source file again, after an edit to its text. Returns whether its class or
    /// its element instance changed.
    pub fn reclassify(&mut self, lookup: Lookup<'_>, path: &str, work: &mut Work) -> bool {
        let before = self
            .files
            .get(path)
            .map(|file| (file.class, file.classification.clone()));
        self.classify(lookup, path, work);
        let after = self
            .files
            .get(path)
            .map(|file| (file.class, file.classification.clone()));
        before != after
    }

    /// Classifies every source file again, as the fault control for a classification per edit.
    pub fn reclassify_all(
        &mut self,
        architecture: &Architecture,
        graph: &ImportGraph,
        work: &mut Work,
    ) {
        let lookup = Lookup::new(architecture, graph);
        for file in graph.files() {
            self.classify(lookup, &file.path, work);
        }
        for path in graph.others() {
            self.classify(lookup, path, work);
        }
    }

    /// The class of a file, classifying it the first time.
    pub(crate) fn file(
        &mut self,
        lookup: Lookup<'_>,
        path: &str,
        work: &mut Work,
    ) -> (u32, Option<Classification>) {
        if !self.files.contains_key(path) {
            self.classify(lookup, path, work);
        }
        self.known(path)
    }

    fn known(&self, path: &str) -> (u32, Option<Classification>) {
        self.files
            .get(path)
            .map_or((0, None), |file| (file.class, file.classification.clone()))
    }

    /// The class of an example's file, from its own text rather than the repository's. An
    /// example's classes are its own, so the file is not counted.
    pub(crate) fn example_file(
        &mut self,
        architecture: &Architecture,
        file: &SourceFile,
        work: &mut Work,
    ) -> (u32, Option<Classification>) {
        let placed = Placed {
            path: &file.path,
            directive: file.facts.as_ref().and_then(|f| f.directive.as_deref()),
            counted: false,
        };
        self.place(architecture, &placed, work);
        self.known(&file.path)
    }

    /// The class of a package.
    pub(crate) fn package(
        &mut self,
        architecture: &Architecture,
        name: &str,
    ) -> (u32, Option<Classification>) {
        if let Some((class, classification)) = self.packages.get(name) {
            return (*class, classification.clone());
        }
        let elements = architecture.matcher.package_elements(name);
        let names = (0..architecture.package_names.len())
            .map(|index| architecture.package_named(index, name))
            .collect();
        let class = self.intern(ClassKey::Package { elements, names });
        let classification = architecture.matcher.package_place(name);
        self.packages
            .insert(name.to_owned(), (class, classification.clone()));
        (class, classification)
    }

    fn classify(&mut self, lookup: Lookup<'_>, path: &str, work: &mut Work) {
        let source = lookup.graph.and_then(|g| g.file(path));
        let placed = Placed {
            path,
            directive: source
                .and_then(|file| file.facts.as_ref())
                .and_then(|facts| facts.directive.as_deref()),
            counted: source.is_some() || lookup.graph.is_some_and(|g| g.is_other(path)),
        };
        self.place(lookup.architecture, &placed, work);
    }

    fn place(&mut self, architecture: &Architecture, placed: &Placed<'_>, work: &mut Work) {
        let found = architecture.matcher.path_match(placed.path);
        work.files_classified += 1;
        let patterns = if match_per_selector() {
            work.pattern_passes += architecture.sets.len() as u64;
            architecture
                .config
                .patterns
                .iter()
                .map(|pattern| pattern.matches(placed.path).is_some())
                .collect()
        } else {
            work.pattern_passes += found.passes;
            found.patterns
        };
        let mut place = Place {
            patterns: &patterns,
            classification: found.classification.as_ref(),
            directive: placed.directive,
            selectors: vec![false; architecture.config.selectors.len()],
        };
        let mut answers = Vec::new();
        for id in &architecture.config.selector_order {
            let index = id.0 as usize;
            place.selectors[index] = place.contains(
                &architecture.config.selectors[index].definition,
                &mut answers,
            );
        }
        let bits = architecture
            .sets
            .iter()
            .map(|set| place.contains(set, &mut answers))
            .collect();
        let class = self.intern(ClassKey::File {
            elements: found
                .classification
                .iter()
                .flat_map(Classification::all)
                .map(|c| c.element)
                .collect(),
            bits,
        });
        if placed.counted {
            if found.overlap.is_empty() {
                self.overlaps.remove(placed.path);
            } else {
                self.overlaps.insert(placed.path.to_owned(), found.overlap);
            }
        }
        let file = FileClass {
            classification: found.classification,
            class,
            counted: placed.counted,
        };
        if let Some(old) = self.files.insert(placed.path.to_owned(), file)
            && old.counted
        {
            self.counts[old.class as usize] -= 1;
        }
        if placed.counted {
            self.counts[class as usize] += 1;
        }
    }

    fn intern(&mut self, key: ClassKey) -> u32 {
        if let Some(&class) = self.index.get(&key) {
            return class;
        }
        let class = u32::try_from(self.keys.len()).unwrap_or(u32::MAX);
        self.index.insert(key.clone(), class);
        self.keys.push(key);
        self.counts.push(0);
        class
    }

    pub(crate) fn key(&self, class: u32) -> &ClassKey {
        &self.keys[class as usize]
    }

    /// The checks of the cell for a pair of classes, filled the first time the pair is met.
    pub(crate) fn cell(&mut self, architecture: &Architecture, from: u32, to: u32) -> &[Check] {
        let keys = &self.keys;
        self.cells
            .entry((from, to))
            .or_insert_with(|| architecture.fill(&keys[from as usize], &keys[to as usize]))
    }

    /// The number of files the walk visited in a set of files, from the counts of each class.
    pub(crate) fn files_in(&self, set: usize) -> u64 {
        self.keys
            .iter()
            .zip(&self.counts)
            .filter(|(key, _)| matches!(key, ClassKey::File { bits, .. } if bits[set]))
            .map(|(_, count)| count)
            .sum()
    }

    /// The number of files the walk visited in an element, from the counts of each class.
    pub(crate) fn files_of(&self, element: ElementId) -> u64 {
        self.keys
            .iter()
            .zip(&self.counts)
            .filter(
                |(key, _)| matches!(key, ClassKey::File { elements, .. } if elements.contains(&element)),
            )
            .map(|(_, count)| count)
            .sum()
    }

    /// The files in two or more of the config's own parts, in path order, with the parts.
    pub(crate) fn overlaps(&self) -> &BTreeMap<String, Vec<ElementId>> {
        &self.overlaps
    }

    /// The element instance of a file that has a class.
    pub(crate) fn classification(&self, path: &str) -> Option<&Classification> {
        self.files
            .get(path)
            .and_then(|file| file.classification.as_ref())
    }

    /// For each set of files the rules name, whether a file with a class is in it.
    pub(crate) fn bits(&self, path: &str) -> Option<&[bool]> {
        let file = self.files.get(path)?;
        match &self.keys[file.class as usize] {
            ClassKey::File { bits, .. } => Some(bits),
            ClassKey::Package { .. } => None,
        }
    }

    /// Whether a file is in a set of files.
    pub(crate) fn file_in(&self, path: &str, set: usize) -> bool {
        self.files.get(path).is_some_and(|file| {
            matches!(&self.keys[file.class as usize], ClassKey::File { bits, .. } if bits[set])
        })
    }
}

/// What one pass over a path found, from which every set's membership follows.
struct Place<'a> {
    patterns: &'a [bool],
    classification: Option<&'a Classification>,
    directive: Option<&'a str>,
    /// Whether the file is in each declared selector, filled in the order that puts each selector
    /// after those it refers to.
    selectors: Vec<bool>,
}

impl Place<'_> {
    /// Whether the file is in a set: its terms in postfix order, read with a stack of answers.
    /// The caller lends the stack, so classifying a file against every set allocates it once.
    fn contains(&self, set: &FileSet, answers: &mut Vec<bool>) -> bool {
        answers.clear();
        for term in &set.terms {
            let answer = match term {
                Term::All(count) => {
                    let from = answers.len().saturating_sub(*count);
                    answers.drain(from..).all(|answer| answer)
                }
                Term::Any(count) => {
                    let from = answers.len().saturating_sub(*count);
                    answers.drain(from..).any(|answer| answer)
                }
                Term::Not => !answers.pop().unwrap_or(true),
                leaf => self.leaf(leaf),
            };
            answers.push(answer);
        }
        answers.pop().unwrap_or(false)
    }

    fn leaf(&self, term: &Term) -> bool {
        match term {
            Term::Pattern(id) => self.patterns[id.0 as usize],
            Term::Element(element) => self
                .classification
                .is_some_and(|c| c.of(*element).is_some()),
            Term::Instance { element, captures } => self
                .classification
                .and_then(|c| c.of(*element))
                .is_some_and(|c| {
                    captures
                        .iter()
                        .all(|(name, value)| capture(c, name).is_some_and(|found| found == value))
                }),
            Term::AnyElement => self.classification.is_some(),
            Term::Directive(text) => self.directive == Some(text.as_str()),
            Term::Selector(id) => self.selectors[id.0 as usize],
            Term::All(_) | Term::Any(_) | Term::Not => false,
        }
    }
}

/// The value of a named capture.
fn capture<'c>(class: &'c Classification, name: &str) -> Option<&'c str> {
    class
        .names
        .iter()
        .position(|c| c == name)
        .and_then(|at| class.captures.get(at))
        .map(String::as_str)
}

/// The `rules::match_per_selector` fail point: tests each pattern in a pass of its own, and counts
/// a pass per set of files, for the fault control.
fn match_per_selector() -> bool {
    fail::fail_point!("rules::match_per_selector", |_| true);
    false
}
