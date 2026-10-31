//! The walk of a transitive rule: one breadth-first search backwards from the forbidden targets,
//! which gives every file that reaches one its shortest chain.

use std::collections::{HashMap, VecDeque};

use archangel_graph::{Resolution, Work};

use crate::architecture::{Architecture, Compiled, covers};
use crate::check::{Entry, Run};
use crate::classes::Lookup;
use crate::diagnostic::{Code, Diagnostic};
use crate::messages::{Site, import_diagnostic};

/// The imports of a transitive rule's walk, between nodes numbered in path order. A target ends a
/// chain as a violation; a stop ends it without one, and its imports are not followed.
#[derive(Debug, Clone, Default)]
pub struct ReachGraph {
    imports: Vec<Vec<usize>>,
    importers: Vec<Vec<usize>>,
    target: Vec<bool>,
    stop: Vec<bool>,
}

/// No distance yet.
const FAR: usize = usize::MAX;

impl ReachGraph {
    /// A graph of `count` nodes and no imports.
    #[must_use]
    pub fn with_nodes(count: usize) -> Self {
        Self {
            imports: vec![Vec::new(); count],
            importers: vec![Vec::new(); count],
            target: vec![false; count],
            stop: vec![false; count],
        }
    }

    /// Marks a node as a forbidden target.
    pub fn set_target(&mut self, node: usize) {
        self.target[node] = true;
    }

    /// Marks a node as a place where a chain stops.
    pub fn set_stop(&mut self, node: usize) {
        self.stop[node] = true;
    }

    /// Adds an import, in the importing node's order of imports.
    pub fn add_import(&mut self, from: usize, to: usize) {
        self.imports[from].push(to);
        self.importers[to].push(from);
    }

    /// The shortest chain from each source to a target, first in path order among chains of one
    /// length, or `None` for a source that reaches none; and the count of nodes the walk visited
    /// is added to `visited`. A chain passes through no target and no stop.
    pub fn chains(&self, sources: &[usize], visited: &mut u64) -> Vec<Option<Vec<usize>>> {
        let distance = if walk_per_file() {
            self.distances_per_source(sources, visited)
        } else {
            self.distances(visited)
        };
        sources
            .iter()
            .map(|&source| self.chain(source, &distance))
            .collect()
    }

    /// Each node's number of steps to the nearest target, through nodes that are neither targets
    /// nor stops: one search backwards from every target at once.
    fn distances(&self, visited: &mut u64) -> Vec<usize> {
        let mut distance = vec![FAR; self.target.len()];
        let mut queue: VecDeque<usize> = VecDeque::new();
        for (node, &target) in self.target.iter().enumerate() {
            if target {
                distance[node] = 0;
                queue.push_back(node);
            }
        }
        while let Some(node) = queue.pop_front() {
            *visited += 1;
            for &importer in &self.importers[node] {
                if distance[importer] == FAR && !self.target[importer] && !self.stop[importer] {
                    distance[importer] = distance[node] + 1;
                    queue.push_back(importer);
                }
            }
        }
        distance
    }

    /// The same distances, after a search forwards from each source alone: the cost the backward
    /// walk avoids, for the fault control.
    fn distances_per_source(&self, sources: &[usize], visited: &mut u64) -> Vec<usize> {
        for &source in sources {
            let mut seen = vec![false; self.target.len()];
            let mut queue = VecDeque::from([source]);
            seen[source] = true;
            while let Some(node) = queue.pop_front() {
                *visited += 1;
                for &next in &self.imports[node] {
                    if !seen[next] {
                        seen[next] = true;
                        queue.push_back(next);
                    }
                }
            }
        }
        self.distances(&mut 0)
    }

    /// A source's chain: each step to the import, among those one step nearer, whose node comes
    /// first in path order.
    fn chain(&self, source: usize, distance: &[usize]) -> Option<Vec<usize>> {
        if self.stop[source] {
            return None;
        }
        let first = self.imports[source]
            .iter()
            .copied()
            .filter(|&next| distance[next] != FAR)
            .min_by_key(|&next| (distance[next], next))?;
        let mut chain = vec![source, first];
        let mut at = first;
        while !self.target[at] {
            at = self.imports[at]
                .iter()
                .copied()
                .filter(|&next| distance[next] != FAR && distance[next] + 1 == distance[at])
                .min()?;
            chain.push(at);
        }
        Some(chain)
    }
}

/// The `rules::walk_per_file` fail point: walks forwards from each file in the selector, for the
/// fault control.
fn walk_per_file() -> bool {
    fail::fail_point!("rules::walk_per_file", |_| true);
    false
}

/// The nodes of a walk: every file, and every file and package an import points to, in path order,
/// with whether each is a package.
struct Nodes<'e> {
    names: Vec<(&'e str, bool)>,
    index: HashMap<&'e str, usize>,
}

impl<'e> Nodes<'e> {
    fn of(entries: &[Entry<'e>]) -> Self {
        let mut names: Vec<(&'e str, bool)> = entries
            .iter()
            .map(|entry| (entry.file.path.as_str(), false))
            .collect();
        for entry in entries {
            names.extend(
                entry
                    .resolutions
                    .iter()
                    .filter_map(|resolution| match resolution {
                        Resolution::File(path) => Some((path.as_str(), false)),
                        Resolution::Package(name) => Some((name.as_str(), true)),
                        Resolution::Unresolved(_) => None,
                    }),
            );
        }
        names.sort_unstable();
        names.dedup_by(|a, b| a.0 == b.0);
        let index = names
            .iter()
            .enumerate()
            .map(|(i, (name, _))| (*name, i))
            .collect();
        Self { names, index }
    }

    fn of_resolution(&self, resolution: &Resolution) -> Option<usize> {
        match resolution {
            Resolution::File(path) | Resolution::Package(path) => {
                self.index.get(path.as_str()).copied()
            }
            Resolution::Unresolved(_) => None,
        }
    }
}

/// Every file a walk covers, and its nodes.
struct Files<'e> {
    entries: Vec<Entry<'e>>,
    nodes: Nodes<'e>,
}

impl<'a> Run<'a> {
    /// Each transitive rule: one walk backwards from its targets over every file, the evaluated
    /// files in place of the repository's at the same path.
    pub(crate) fn transitive(&mut self, work: &mut Work) {
        let transitive: Vec<usize> = (0..self.architecture.sentences.len())
            .filter(|&i| {
                let compiled = &self.architecture.sentences[i];
                self.architecture.config.rules[compiled.rule].sentences[compiled.sentence]
                    .options
                    .transitively
            })
            .collect();
        if transitive.is_empty() {
            return;
        }
        let own: HashMap<&str, Entry<'a>> = self
            .scope
            .evaluated
            .iter()
            .map(|entry| (entry.file.path.as_str(), *entry))
            .collect();
        let mut entries: Vec<Entry<'a>> = self
            .graph
            .files()
            .iter()
            .filter(|file| !own.contains_key(file.path.as_str()))
            .map(|file| Entry {
                file,
                resolutions: self.graph.resolutions(&file.path).unwrap_or(&[]),
            })
            .chain(own.values().copied())
            .collect();
        entries.sort_by(|a, b| a.file.path.cmp(&b.file.path));
        let files = Files {
            nodes: Nodes::of(&entries),
            entries,
        };
        for sentence in transitive {
            let compiled = self.architecture.sentences[sentence].clone();
            self.walk(&compiled, &files, work);
        }
    }

    fn walk(&mut self, compiled: &Compiled, files: &Files<'a>, work: &mut Work) {
        let (entries, nodes) = (&files.entries, &files.nodes);
        let reach = self.reach_graph(compiled, entries, nodes);
        let sources: Vec<(usize, Entry<'a>)> = entries
            .iter()
            .filter(|entry| self.classes.file_in(&entry.file.path, compiled.files))
            .filter_map(|entry| Some((*nodes.index.get(entry.file.path.as_str())?, *entry)))
            .collect();
        let starts: Vec<usize> = sources.iter().map(|(node, _)| *node).collect();
        let chains = reach.chains(&starts, &mut work.transitive_files_visited);
        for ((_, entry), chain) in sources.into_iter().zip(chains) {
            let Some(chain) = chain else { continue };
            let names: Vec<String> = chain.iter().map(|&n| nodes.names[n].0.to_owned()).collect();
            if let Some(diagnostic) = self.chain_diagnostic(compiled.rule, &entry, names) {
                self.violations.push(diagnostic);
            }
        }
    }

    /// The imports a transitive rule counts, between the walk's nodes, with its targets and stops.
    fn reach_graph(
        &mut self,
        compiled: &Compiled,
        entries: &[Entry<'_>],
        nodes: &Nodes<'_>,
    ) -> ReachGraph {
        let imports = self.architecture.config.rules[compiled.rule].imports;
        let mut reach = ReachGraph::with_nodes(nodes.names.len());
        for (node, &(name, package)) in nodes.names.iter().enumerate() {
            let (class, _) = if package {
                self.classes.package(self.architecture, name)
            } else {
                let lookup = Lookup::new(self.architecture, self.graph);
                self.classes.file(lookup, name, &mut Work::default())
            };
            if Architecture::in_side_class(compiled, self.classes.key(class)) {
                reach.set_target(node);
            }
            if compiled
                .stop_at
                .is_some_and(|stop| self.classes.file_in(name, stop))
            {
                reach.set_stop(node);
            }
        }
        for entry in entries {
            let (Some(facts), Some(&from)) =
                (&entry.file.facts, nodes.index.get(entry.file.path.as_str()))
            else {
                continue;
            };
            for (import, resolution) in facts.imports.iter().zip(entry.resolutions) {
                if let Some(to) = nodes.of_resolution(resolution)
                    && covers(imports, import.kind)
                {
                    reach.add_import(from, to);
                }
            }
        }
        reach
    }

    /// A transitive violation at the first import of its chain.
    fn chain_diagnostic(
        &self,
        rule: usize,
        entry: &Entry<'_>,
        chain: Vec<String>,
    ) -> Option<Diagnostic> {
        let rule = &self.architecture.config.rules[rule];
        let facts = entry.file.facts.as_ref()?;
        let mut occurrences: HashMap<&str, u32> = HashMap::new();
        let (import, occurrence) =
            facts
                .imports
                .iter()
                .zip(entry.resolutions)
                .find_map(|(import, resolution)| {
                    let seen = occurrences.entry(import.specifier.as_str()).or_default();
                    let occurrence = *seen;
                    *seen += 1;
                    let target = match resolution {
                        Resolution::File(path) | Resolution::Package(path) => path.as_str(),
                        Resolution::Unresolved(_) => return None,
                    };
                    (target == chain[1] && covers(rule.imports, import.kind))
                        .then_some((import, occurrence))
                })?;
        let site = Site {
            file: &entry.file.path,
            import,
            occurrence,
        };
        let target = chain.last().cloned().unwrap_or_default();
        let message = format!(
            "{} reaches {target} through {}",
            entry.file.path,
            chain.join(" → ")
        );
        let help = "remove an import of the chain, or move what the file needs into code that \
                    does not reach it"
            .to_owned();
        let mut diagnostic = import_diagnostic(rule, &site, (Code::MayNotImport, message, help));
        diagnostic.chain = chain;
        Some(diagnostic)
    }
}
