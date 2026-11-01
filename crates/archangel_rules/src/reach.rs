//! The reach of each rule: what it covered in this repository, for `archangel rules`. It reads the
//! classes and the report of a check, so what it says is what the check did.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use archangel_config::{Condition, Rule, RuleKind, TableId};
use archangel_graph::{Import, ImportGraph, Resolution, SourceFile, Work};

use crate::architecture::{Architecture, Edge, End, covers, in_files};
use crate::check::Report;
use crate::classes::{ClassKey, Classes, Lookup};

/// What one rule covered.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuleReach {
    /// The rule's index in the config.
    pub rule: usize,
    /// The files in its files; for a table's own rule, the files of the table.
    pub files: u64,
    /// For a rule about folders, the folders it covers.
    pub folders: Option<u64>,
    /// The imports it judged.
    pub imports_judged: u64,
    /// The violations that the check reports.
    pub problems: u64,
    /// The violations that an exception permits.
    pub exceptions: u64,
    /// The violations that the baseline matches.
    pub baseline_matches: u64,
}

/// An import between two repository files that no rule judged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnjudgedImport {
    /// The importing file.
    pub from: String,
    /// The line of the import.
    pub line: u32,
    /// The imported file.
    pub to: String,
}

/// What every rule covered, and what no rule did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reach {
    /// Each rule that applies under the path, in config order.
    pub rules: Vec<RuleReach>,
    /// The files under the path that no rule's files include.
    pub files_not_judged: Vec<String>,
    /// The imports from files under the path that no rule judged.
    pub imports_not_judged: Vec<UnjudgedImport>,
    /// The outside packages that no rule names, with how many imports under the path reach them.
    pub packages_not_named: Vec<(String, u64)>,
}

/// Whether a path is under a folder or file given on the command line, or no path was given.
fn is_under(path: &str, under: Option<&str>) -> bool {
    under.is_none_or(|under| {
        path == under
            || path
                .strip_prefix(under)
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

impl Architecture {
    /// The reach of every rule, from the report of a check of `graph`. With a path, only the rules
    /// whose files include a file under it, and only the files and imports under it. The config
    /// file is no part of the architecture, so it is in no list of what no rule judges.
    #[must_use]
    pub fn reach(
        &self,
        graph: &ImportGraph,
        report: &Report,
        under: Option<&str>,
        config_file: &str,
    ) -> Reach {
        let count = self.config.rules.len();
        let mut tally = Tally {
            architecture: self,
            graph,
            classes: Classes::new(),
            work: Work::default(),
            files: vec![0; count],
            imports: vec![0; count],
            applies_under: vec![false; count],
            of_class: HashMap::new(),
            judges: HashMap::new(),
            packages: BTreeMap::new(),
            found: Reach::default(),
        };
        tally.classes.sync(self, graph, &mut tally.work);
        for file in graph.files() {
            let here = is_under(&file.path, under) && file.path != config_file;
            tally.file(file, here);
        }
        tally.finish(report, under)
    }

    /// Whether a table's own rule belongs to a table with a row that applies under the path: the
    /// rule has no file of its own while every file has a row.
    fn owns_a_file_under(&self, rule: usize, applies_under: &[bool]) -> bool {
        let RuleKind::TableRow { table, row: None } = &self.config.rules[rule].kind else {
            return false;
        };
        self.rules_of_table(*table)
            .into_iter()
            .any(|row| applies_under[row])
    }

    fn rules_of_table(&self, id: TableId) -> Vec<usize> {
        self.config
            .rules
            .iter()
            .enumerate()
            .filter(
                |(_, rule)| matches!(&rule.kind, RuleKind::TableRow { table, .. } if *table == id),
            )
            .map(|(index, _)| index)
            .collect()
    }

    /// The rules whose files include a file of this class.
    fn rules_of_class(&self, class: &ClassKey) -> Vec<usize> {
        let mut rules: Vec<usize> = self
            .sentences
            .iter()
            .filter(|compiled| {
                let rule = &self.config.rules[compiled.rule];
                rule.sentences[compiled.sentence].folders.is_none() && in_files(compiled, class)
            })
            .map(|compiled| compiled.rule)
            .collect();
        rules.dedup();
        rules
    }

    /// Whether a rule's side or area names a package of this class.
    fn names_package(&self, class: &ClassKey) -> bool {
        self.sentences.iter().any(|compiled| {
            let sentence = &self.config.rules[compiled.rule].sentences[compiled.sentence];
            matches!(
                sentence.condition,
                Condition::MayNotImport(_)
                    | Condition::MayOnlyImport(_)
                    | Condition::MayOnlyBeImportedBy(_)
            ) && (compiled.side.holds(class)
                || compiled.area.as_ref().is_some_and(|area| area.holds(class)))
        })
    }
}

/// The counts of a reach in progress.
struct Tally<'t> {
    architecture: &'t Architecture,
    graph: &'t ImportGraph,
    classes: Classes,
    work: Work,
    files: Vec<u64>,
    imports: Vec<u64>,
    applies_under: Vec<bool>,
    /// The rules whose files include a file of each class.
    of_class: HashMap<u32, Vec<usize>>,
    /// The rules that look at an import between each pair of classes.
    judges: HashMap<(u32, u32), Vec<usize>>,
    packages: BTreeMap<String, u64>,
    found: Reach,
}

impl Tally<'_> {
    /// Counts one file, and each of its imports. `here` says whether it is under the path.
    fn file(&mut self, file: &SourceFile, here: bool) {
        let path = file.path.as_str();
        let lookup = Lookup::new(self.architecture, self.graph);
        let (from, _) = self.classes.file(lookup, path, &mut self.work);
        let architecture = self.architecture;
        let classes = &self.classes;
        let rules = self
            .of_class
            .entry(from)
            .or_insert_with(|| architecture.rules_of_class(classes.key(from)));
        for rule in rules.iter() {
            self.files[*rule] += 1;
            self.applies_under[*rule] |= here;
        }
        if here && rules.is_empty() {
            self.found.files_not_judged.push(path.to_owned());
        }
        let (Some(facts), Some(resolutions)) = (&file.facts, self.graph.resolutions(path)) else {
            return;
        };
        for (import, resolution) in facts.imports.iter().zip(resolutions) {
            self.import((path, from, here), import, resolution);
        }
    }

    fn import(
        &mut self,
        (path, from, here): (&str, u32, bool),
        import: &Import,
        resolution: &Resolution,
    ) {
        let architecture = self.architecture;
        let lookup = Lookup::new(architecture, self.graph);
        let to = match resolution {
            Resolution::File(to) => self.classes.file(lookup, to, &mut self.work).0,
            Resolution::Package(name) => {
                let (class, _) = self.classes.package(architecture, name);
                if here && !architecture.names_package(self.classes.key(class)) {
                    *self.packages.entry(name.clone()).or_default() += 1;
                }
                class
            }
            Resolution::Unresolved(_) => return,
        };
        let classes = &self.classes;
        let of_pair = self.judges.entry((from, to)).or_insert_with(|| {
            (0..architecture.config.rules.len())
                .filter(|rule| architecture.may_judge(*rule, classes.key(from), classes.key(to)))
                .collect()
        });
        let target = match resolution {
            Resolution::File(to) => classes.place_of(to),
            Resolution::Package(name) => classes.package_place_of(name),
            Resolution::Unresolved(_) => None,
        };
        let edge = Edge {
            from: End {
                class: from,
                classification: classes.place_of(path),
                path: Some(path),
                package: None,
            },
            to: End {
                class: to,
                classification: target,
                path: match resolution {
                    Resolution::File(to) => Some(to.as_str()),
                    _ => None,
                },
                package: match resolution {
                    Resolution::Package(name) => Some(name.as_str()),
                    _ => None,
                },
            },
            file: path,
            import,
            resolution,
            manifests: Some(self.graph.manifests_with(None)),
        };
        let mut in_a_rule = false;
        for rule in of_pair.iter() {
            if covers(architecture.config.rules[*rule].imports, import.kind)
                && architecture.judges(*rule, &edge, classes)
            {
                self.imports[*rule] += 1;
                in_a_rule = true;
            }
        }
        if let (false, true, Resolution::File(to)) = (in_a_rule, here, resolution) {
            self.found.imports_not_judged.push(UnjudgedImport {
                from: path.to_owned(),
                line: import.line,
                to: to.clone(),
            });
        }
    }

    /// The reach of each rule that applies under the path.
    fn finish(mut self, report: &Report, under: Option<&str>) -> Reach {
        self.found.packages_not_named = std::mem::take(&mut self.packages).into_iter().collect();
        let folders = folders_of(self.graph);
        let architecture = self.architecture;
        for (rule, definition) in architecture.config.rules.iter().enumerate() {
            let covered = covered_folders(definition, &folders);
            let about_folders = covered
                .as_ref()
                .is_some_and(|covered| covered.iter().any(|folder| is_under(folder, under)));
            let applies = under.is_none()
                || self.applies_under[rule]
                || about_folders
                || architecture.owns_a_file_under(rule, &self.applies_under);
            if !applies {
                continue;
            }
            let name = definition.name.as_str();
            let problems = report
                .diagnostics
                .iter()
                .filter(|d| d.code.is_rule() && d.rule.as_deref() == Some(name))
                .count() as u64;
            let reach = RuleReach {
                rule,
                files: self.files_of(rule),
                folders: covered.map(|covered| covered.len() as u64),
                imports_judged: self.imports[rule],
                problems,
                exceptions: report.summary.exceptions.get(name).copied().unwrap_or(0),
                baseline_matches: report.summary.baselined.get(name).copied().unwrap_or(0),
            };
            self.found.rules.push(reach);
        }
        self.found
    }

    /// The files of a rule, or for a table's own rule the files of all the table's rules.
    fn files_of(&self, rule: usize) -> u64 {
        let architecture = self.architecture;
        let RuleKind::TableRow { table, row: None } = &architecture.config.rules[rule].kind else {
            return self.files[rule];
        };
        let of_table = architecture.rules_of_table(*table);
        let sets: BTreeSet<usize> = architecture
            .sentences
            .iter()
            .filter(|compiled| of_table.contains(&compiled.rule))
            .map(|compiled| compiled.files)
            .collect();
        self.graph
            .files()
            .iter()
            .filter(|file| {
                sets.iter()
                    .any(|set| self.classes.file_in(&file.path, *set))
            })
            .count() as u64
    }
}

/// The folders that a rule about folders covers, or `None` for a rule about files.
fn covered_folders<'g>(rule: &Rule, folders: &BTreeSet<&'g str>) -> Option<BTreeSet<&'g str>> {
    rule.sentences
        .iter()
        .filter_map(|sentence| sentence.folders.as_ref())
        .map(|covers| {
            folders
                .iter()
                .copied()
                .filter(|folder| covers.covers(folder))
                .collect::<BTreeSet<&str>>()
        })
        .reduce(|a, b| a.union(&b).copied().collect())
}

/// Every folder that holds a file of the graph.
fn folders_of(graph: &ImportGraph) -> BTreeSet<&str> {
    let mut folders = BTreeSet::new();
    for file in graph.files() {
        let mut path = file.path.as_str();
        while let Some((folder, _)) = path.rsplit_once('/') {
            folders.insert(folder);
            path = folder;
        }
    }
    folders
}
