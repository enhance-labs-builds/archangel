//! Putting files and packages into elements, and matching a path against every pattern that
//! selectors and rules name, in one pass.

use std::collections::HashMap;

use regex::{Regex, RegexSet, RegexSetBuilder};

use crate::model::{Config, ElementId, ElementKind};
use crate::pattern::{Pattern, normalize_path};

/// Where a file belongs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Classification {
    /// The element.
    pub element: ElementId,
    /// The capture values, which together with the element name one instance.
    pub captures: Vec<String>,
    /// The name of each capture, in the order of the values.
    pub names: Vec<String>,
    /// The instance root.
    pub root: String,
    /// The file's place in each other set of elements, as in each preset's parts, or each other
    /// externals set of a package.
    pub also: Vec<Classification>,
}

impl Classification {
    /// The place in each set of elements, this one first.
    pub fn all(&self) -> impl Iterator<Item = &Self> {
        std::iter::once(self).chain(&self.also)
    }

    /// The place in one element, when the file or package is in it.
    #[must_use]
    pub fn of(&self, element: ElementId) -> Option<&Self> {
        self.all().find(|class| class.element == element)
    }

    /// Whether two places are one instance of one element.
    #[must_use]
    pub fn same_instance(&self, other: &Self) -> bool {
        self.element == other.element
            && self.captures == other.captures
            && self.names == other.names
            && self.root == other.root
    }
}

/// The memory the regex set of a config's glob patterns may use. A config generated from another
/// tool's rules can have thousands of patterns; past this, each glob is tried alone instead.
const GLOB_SET_LIMIT: usize = 256 * 1024 * 1024;

/// Every element pattern of a config, and every pattern its selectors and rules name, compiled so
/// one pass tests them all: a map for the patterns that name one exact path, and a regex set for
/// the globs. The element patterns come first, so the first element to match is the lowest index.
#[derive(Debug, Clone)]
pub struct ElementMatcher {
    /// Each exact path, with the indices of the patterns that name it, lowest first.
    exact: HashMap<String, Vec<usize>>,
    globs: Globs,
    file_patterns: Vec<(ElementId, Pattern)>,
    /// How many patterns the config's selectors and rules name; theirs follow the element patterns.
    selected: usize,
    packages: Vec<(ElementId, Pattern)>,
    /// For each element, its set, and whether a second match in the set is an overlap.
    sets: Vec<(u32, bool)>,
    /// For each element, whether it is an externals set.
    externals: Vec<bool>,
}

/// What one pass of every pattern found in a path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathMatch {
    /// The element instance, from the first element of each set that matches.
    pub classification: Option<Classification>,
    /// The parts that the file is in, when it is in two or more of the config's own.
    pub overlap: Vec<ElementId>,
    /// For each of the config's selector and rule patterns, whether it matches.
    pub patterns: Vec<bool>,
    /// The passes over the path: one, or one per glob when the set was too large to compile.
    pub passes: u64,
}

/// The glob patterns, with the index of each in `file_patterns`.
#[derive(Debug, Clone)]
enum Globs {
    /// One set that tests every glob in one pass.
    Set(RegexSet, Vec<usize>),
    /// Each glob alone, in declaration order, for a set too large to compile. Slower, never wrong.
    Each(Vec<(usize, Regex)>),
}

impl Globs {
    fn new(file_patterns: &[(ElementId, Pattern)]) -> Self {
        let globs: Vec<usize> = (0..file_patterns.len())
            .filter(|&i| !file_patterns[i].1.is_exact())
            .collect();
        let set = RegexSetBuilder::new(globs.iter().map(|&i| file_patterns[i].1.regex().as_str()))
            .size_limit(GLOB_SET_LIMIT)
            .dfa_size_limit(GLOB_SET_LIMIT)
            .build();
        match set {
            Ok(set) => Self::Set(set, globs),
            Err(_) => Self::Each(
                globs
                    .into_iter()
                    .map(|i| (i, file_patterns[i].1.regex().clone()))
                    .collect(),
            ),
        }
    }

    /// The indices of every glob that matches the path, lowest first, and the passes it took.
    fn all(&self, path: &str) -> (Vec<usize>, u64) {
        match self {
            Self::Set(set, indices) => (
                set.matches(path)
                    .into_iter()
                    .map(|at| indices[at])
                    .collect(),
                1,
            ),
            Self::Each(globs) => (
                globs
                    .iter()
                    .filter(|(_, regex)| regex.is_match(path))
                    .map(|(i, _)| *i)
                    .collect(),
                globs.len() as u64,
            ),
        }
    }
}

impl ElementMatcher {
    /// Compiles the element patterns of a validated config.
    #[must_use]
    pub fn new(config: &Config) -> Self {
        let mut file_patterns = Vec::new();
        let mut packages = Vec::new();
        for (index, element) in config.elements.iter().enumerate() {
            let id = ElementId(u32::try_from(index).unwrap_or(u32::MAX));
            match &element.kind {
                ElementKind::Files(patterns) => {
                    file_patterns.extend(patterns.iter().map(|p| (id, p.clone())));
                }
                ElementKind::Packages(patterns) => {
                    packages.extend(patterns.iter().map(|p| (id, p.clone())));
                }
            }
        }
        let elements = file_patterns.len();
        let none = ElementId(u32::MAX);
        file_patterns.extend(config.patterns.iter().map(|p| (none, p.clone())));
        let mut exact: HashMap<String, Vec<usize>> = HashMap::new();
        for (index, (_, pattern)) in file_patterns.iter().enumerate() {
            if pattern.is_exact() {
                exact
                    .entry(normalize_path(pattern.source()))
                    .or_default()
                    .push(index);
            }
        }
        Self {
            exact,
            globs: Globs::new(&file_patterns),
            selected: file_patterns.len() - elements,
            file_patterns,
            packages,
            sets: config.elements.iter().map(|e| (e.set, e.part)).collect(),
            externals: config.elements.iter().map(|e| e.externals).collect(),
        }
    }

    /// The element of a repository file, from the first declared element that matches.
    #[must_use]
    pub fn classify_file(&self, path: &str) -> Option<Classification> {
        self.path_match(path).classification
    }

    /// The element of a repository file and every selector and rule pattern it matches, in one
    /// pass.
    #[must_use]
    pub fn path_match(&self, path: &str) -> PathMatch {
        let path = normalize_path(path);
        let (mut found, passes) = self.globs.all(&path);
        if let Some(exact) = self.exact.get(&path) {
            found.extend(exact);
            found.sort_unstable();
        }
        let elements = self.file_patterns.len() - self.selected;
        let (classification, overlap) = self.classification(&found, elements, &path);
        let mut patterns = vec![false; self.selected];
        for index in found.into_iter().filter(|&i| i >= elements) {
            patterns[index - elements] = true;
        }
        PathMatch {
            classification,
            overlap,
            patterns,
            passes,
        }
    }

    /// The first element of each set among the matching patterns, the lowest set first, and the
    /// parts of a file that two of the config's own parts match.
    fn classification(
        &self,
        found: &[usize],
        elements: usize,
        path: &str,
    ) -> (Option<Classification>, Vec<ElementId>) {
        let mut places: Vec<Classification> = Vec::new();
        let mut overlap: Vec<ElementId> = Vec::new();
        for &index in found.iter().take_while(|&&index| index < elements) {
            let (element, pattern) = &self.file_patterns[index];
            let (set, part) = self.sets[element.index()];
            let first = places
                .iter()
                .find(|place| self.sets[place.element.index()].0 == set);
            if let Some(first) = first {
                if part && first.element != *element {
                    if overlap.is_empty() {
                        overlap.push(first.element);
                    }
                    if !overlap.contains(element) {
                        overlap.push(*element);
                    }
                }
                continue;
            }
            if let Some(matched) = pattern.matches(path) {
                places.push(Classification {
                    element: *element,
                    names: pattern.captures().to_vec(),
                    captures: matched.captures,
                    root: matched.root,
                    also: Vec::new(),
                });
            }
        }
        places.sort_by_key(|place| self.sets[place.element.index()].0);
        (joined(places), overlap)
    }

    /// The first element of a package.
    #[must_use]
    pub fn classify_package(&self, name: &str) -> Option<ElementId> {
        self.package_elements(name).first().copied()
    }

    /// The elements of a package: the first declared package element that lists it, as in 0.1, and
    /// every externals set that lists it.
    #[must_use]
    pub fn package_elements(&self, name: &str) -> Vec<ElementId> {
        let mut found: Vec<ElementId> = Vec::new();
        let mut element_found = false;
        for (element, pattern) in &self.packages {
            let externals = self.externals[element.index()];
            if (externals || !element_found)
                && !found.contains(element)
                && package_matches(pattern, name)
            {
                element_found |= !externals;
                found.push(*element);
            }
        }
        found
    }

    /// The place of a package in its elements, with the package's name as the root.
    #[must_use]
    pub fn package_place(&self, name: &str) -> Option<Classification> {
        joined(
            self.package_elements(name)
                .into_iter()
                .map(|element| Classification {
                    element,
                    captures: Vec::new(),
                    names: Vec::new(),
                    root: name.to_owned(),
                    also: Vec::new(),
                })
                .collect(),
        )
    }
}

/// The first place, holding the others.
fn joined(mut places: Vec<Classification>) -> Option<Classification> {
    if places.is_empty() {
        return None;
    }
    let mut first = places.remove(0);
    first.also = places;
    Some(first)
}

/// Whether a package name pattern names a package: `*` names every package, a scope ending in
/// `/*` every package in it, and `node:*` every built-in module.
#[must_use]
pub fn package_matches(pattern: &Pattern, name: &str) -> bool {
    pattern.source() == "*" || pattern.matches(name).is_some()
}
