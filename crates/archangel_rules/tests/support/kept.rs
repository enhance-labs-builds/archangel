//! Shared by the tests of what a session keeps: a repository checked once, then changed on disk
//! and brought up to date the way a session does it, through the classes and the structure it
//! keeps rather than a fresh check.
#![cfg(test)]

use std::collections::BTreeMap;
use std::path::Path;

use archangel_graph::{GraphOptions, ImportGraph, Work, is_source};
use archangel_rules::{
    Architecture, Classes, FileCheck, Kept, Lookup, Report, Results, Structure, StructureChange,
};
use archangel_testkit::FixtureRepo;

/// A checked repository, with the classes, import checks and structure that a session keeps.
pub(crate) struct KeptCheck {
    repo: FixtureRepo,
    architecture: Architecture,
    graph: ImportGraph,
    classes: Classes,
    checks: BTreeMap<String, FileCheck>,
    structure: Structure,
}

/// Files written, new or edited, and files removed, on disk.
pub(crate) struct DiskChange<'a> {
    pub(crate) write: &'a [(&'a str, &'a str)],
    pub(crate) remove: &'a [&'a str],
}

/// What the graph took of a change: whether paths came or went, and the source files edited.
struct Applied {
    paths: bool,
    edited: Vec<String>,
}

impl KeptCheck {
    /// Checks the repository with the config, keeping the classes and the structure.
    pub(crate) fn open(config: &serde_json::Value, repo: FixtureRepo) -> Self {
        let config = archangel_config::parse_json(&config.to_string()).expect("a valid config");
        let architecture = Architecture::compile(config);
        let mut work = Work::default();
        let graph = ImportGraph::build(repo.path(), &GraphOptions::default(), &mut work);
        let mut classes = Classes::new();
        classes.sync(&architecture, &graph, &mut work);
        let checks = architecture.check_files(&graph, &mut classes, None, &mut work);
        let structure = architecture.structure(&graph, &mut classes, &mut work);
        Self {
            repo,
            architecture,
            graph,
            classes,
            checks,
            structure,
        }
    }

    /// Makes the change on disk, and brings the graph, the classes and the structure up to date
    /// with it. Returns the work that took.
    pub(crate) fn change(&mut self, change: &DiskChange<'_>) -> Work {
        let mut work = Work::default();
        let applied = self.apply(change, &mut work);
        let mut texts = Vec::new();
        let mut reclassified = Vec::new();
        for path in applied.edited {
            if self.graph.replace_file(&path, None, &mut work) {
                let lookup = Lookup::new(&self.architecture, &self.graph);
                if self.classes.reclassify(lookup, &path, &mut work) {
                    reclassified.push(path.clone());
                }
                texts.push(path);
            }
        }
        if applied.paths {
            self.classes
                .sync(&self.architecture, &self.graph, &mut work);
        }
        let structure_change = StructureChange {
            texts: &texts,
            reclassified: &reclassified,
            paths: applied.paths,
            every_export: false,
            every_folder: false,
        };
        self.update(&structure_change, &mut work);
        work
    }

    /// Writes and removes the files, and gives the graph the paths that came and went.
    fn apply(&mut self, change: &DiskChange<'_>, work: &mut Work) -> Applied {
        let (mut created, mut edited, mut deleted) = (Vec::new(), Vec::new(), Vec::new());
        let mut others = false;
        for (path, text) in change.write {
            let known = self.graph.file(path).is_some();
            self.repo.write(path, text);
            if !is_source(Path::new(path)) {
                others |= self.graph.set_other(path, true);
            } else if known {
                edited.push((*path).to_owned());
            } else {
                created.push((*path).to_owned());
            }
        }
        for path in change.remove {
            self.repo.remove(path);
            if is_source(Path::new(path)) {
                deleted.push((*path).to_owned());
            } else {
                others |= self.graph.set_other(path, false);
            }
        }
        let files = !created.is_empty() || !deleted.is_empty();
        if files {
            self.graph.apply_file_events(&created, &deleted, work);
        }
        Applied {
            paths: others || files,
            edited,
        }
    }

    /// Checks every file's imports again, and evaluates again what the change touched of the
    /// rules about files, folders and instances.
    fn update(&mut self, change: &StructureChange<'_>, work: &mut Work) {
        self.checks = self.architecture.check_files(
            &self.graph,
            &mut self.classes,
            None,
            &mut Work::default(),
        );
        let kept = Kept {
            graph: &self.graph,
            classes: &mut self.classes,
            structure: &mut self.structure,
        };
        self.architecture.update_structure(kept, change, work);
    }

    /// The report from what is kept.
    pub(crate) fn report(&mut self) -> Report {
        let results = Results {
            checks: &self.checks,
            structure: &self.structure,
            plugin: &[],
        };
        self.architecture.assemble(
            &self.graph,
            &mut self.classes,
            results,
            None,
            &mut Work::default(),
        )
    }

    /// The report of a fresh check of the repository as it is on disk now.
    pub(crate) fn fresh(&self) -> Report {
        let mut work = Work::default();
        let graph = ImportGraph::build(self.repo.path(), &GraphOptions::default(), &mut work);
        self.architecture.check(&graph, None, &mut work)
    }
}
