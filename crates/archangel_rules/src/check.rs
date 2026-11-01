//! A check: every import against every rule, cycles, exceptions, the baseline, and empty elements.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use archangel_config::{
    Classification, Condition, ElementId, ElementKind, FileSet, ImportKinds, Pattern, RuleKind,
    Table, TableId, Term,
};
use archangel_graph::{
    GraphProblem, Import, ImportGraph, Manifests, Resolution, SourceFile, UnresolvedReason, Work,
};

use crate::architecture::{Architecture, Edge, End, Origin, covers};
use crate::baseline::{Baseline, Key};
use crate::classes::{ClassKey, Classes, Lookup};
use crate::cycles::CycleGraph;
use crate::diagnostic::{Code, Diagnostic};
use crate::exceptions::{self, Exception};
use crate::messages::{
    Parties, Site, Texts, cycle_diagnostic, exception_problem, import_diagnostic, parse_failed,
    pattern_list,
};
use crate::structure::Structure;

/// What a check found.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// Every diagnostic, sorted by file, line, column and rule.
    pub diagnostics: Vec<Diagnostic>,
    /// The counts after the diagnostics.
    pub summary: Summary,
}

/// The part of a check's output after the diagnostics.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Summary {
    /// Source files checked.
    pub files: u64,
    /// Imports checked.
    pub imports: u64,
    /// Exceptions that permitted a violation, per rule.
    pub exceptions: BTreeMap<String, u64>,
    /// Violations that the baseline matched, per rule.
    pub baselined: BTreeMap<String, u64>,
    /// Unresolved imports that the config permits.
    pub permitted_unresolved: u64,
    /// Dynamic imports and require calls whose specifier is computed.
    pub unfollowed: u64,
    /// Symbolic links that the walk did not follow.
    pub links_not_followed: Vec<String>,
    /// Folders below the config folder that are repositories of their own.
    pub nested_repositories: Vec<String>,
    /// The elements and selectors that presets declared and that match nothing, which is no
    /// error, because a project need not use every part of a preset.
    pub preset_names_matching_nothing: Vec<String>,
    /// The kind rows of tables that have no file in this repository, in config order, or `None` for
    /// a config with no table.
    pub rows_with_no_file: Option<Vec<String>>,
    /// Why each table of a preset in `preset_names_matching_nothing` matches no file.
    pub preset_tables_matching_nothing: BTreeMap<String, String>,
}

/// What the rules found in one file's imports. A session keeps one per file, so that an edit to a
/// file checks that file's imports again and no other.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileCheck {
    violations: Vec<Diagnostic>,
    unresolved: Vec<Diagnostic>,
    parsed: bool,
    imports: u64,
    unfollowed: u64,
    permitted_unresolved: u64,
    elements: BTreeSet<ElementId>,
    /// The inline package names of rule sides that the file's imports reach, by index.
    package_names: BTreeSet<usize>,
}

impl Architecture {
    /// Checks a graph against the rules, with an optional baseline.
    #[must_use]
    pub fn check(
        &self,
        graph: &ImportGraph,
        baseline: Option<&Baseline>,
        work: &mut Work,
    ) -> Report {
        let mut classes = Classes::new();
        classes.sync(self, graph, work);
        let checks = self.check_files(graph, &mut classes, None, work);
        let structure = self.structure(graph, &mut classes, work);
        let results = Results {
            checks: &checks,
            structure: &structure,
            plugin: &[],
        };
        self.assemble(graph, &mut classes, results, baseline, work)
    }

    /// Checks the imports of the files at `paths`, or of every file, against the rules, with the
    /// classes the files already have.
    #[must_use]
    pub fn check_files(
        &self,
        graph: &ImportGraph,
        classes: &mut Classes,
        paths: Option<&[&str]>,
        work: &mut Work,
    ) -> BTreeMap<String, FileCheck> {
        let scope = match paths {
            None => Scope::whole(graph),
            Some(paths) => Scope {
                evaluated: paths
                    .iter()
                    .filter_map(|path| {
                        Some(Entry {
                            file: graph.file(path)?,
                            resolutions: graph.resolutions(path)?,
                        })
                    })
                    .collect(),
                searched: Vec::new(),
            },
        };
        let mut run = Run::new(self, graph, classes, scope);
        run.imports(work);
        run.checks
    }

    /// The report from the checks of every file's imports, the results of the rules about
    /// files, folders and instances, and the plugin rules' diagnostics: with the cycles, the
    /// transitive rules, the exceptions, the empty elements and selectors, the graph's problems
    /// and the baseline, none of which evaluates one import against a rule cell.
    #[must_use]
    pub fn assemble(
        &self,
        graph: &ImportGraph,
        classes: &mut Classes,
        results: Results<'_>,
        baseline: Option<&Baseline>,
        work: &mut Work,
    ) -> Report {
        let mut run = Run::new(self, graph, classes, Scope::whole(graph));
        run.absorb(results.checks.values());
        run.cycles(work);
        run.transitive(work);
        let found = self
            .structure_diagnostics(results.structure)
            .into_iter()
            .chain(results.plugin.iter().cloned());
        for diagnostic in found {
            if diagnostic.code.is_rule() {
                run.violations.push(diagnostic);
            } else {
                run.other.push(diagnostic);
            }
        }
        run.exceptions();
        run.empty_elements();
        run.empty_selectors();
        run.graph_problems();
        run.part_overlaps();
        run.tables();
        run.externals_in_workspace();
        if let Some(baseline) = baseline {
            run.apply_baseline(baseline);
        }
        run.finish()
    }
}

/// What the kept results of a session hold, which a report is assembled from.
#[derive(Debug, Clone, Copy)]
pub struct Results<'a> {
    /// The check of each file's imports.
    pub checks: &'a BTreeMap<String, FileCheck>,
    /// The results of the rules about files, folders and instances.
    pub structure: &'a Structure,
    /// The plugin rules' violations and failures.
    pub plugin: &'a [Diagnostic],
}

/// One file in a run, with where its imports resolve.
#[derive(Clone, Copy)]
pub(crate) struct Entry<'a> {
    pub(crate) file: &'a SourceFile,
    pub(crate) resolutions: &'a [Resolution],
}

/// The files a run evaluates, and the files its cycle search covers.
pub(crate) struct Scope<'a> {
    /// The files whose imports the rules evaluate.
    pub(crate) evaluated: Vec<Entry<'a>>,
    /// The files of the cycle search, the evaluated files first.
    pub(crate) searched: Vec<Entry<'a>>,
}

impl<'a> Scope<'a> {
    fn whole(graph: &'a ImportGraph) -> Self {
        let entries: Vec<Entry<'a>> = graph
            .files()
            .iter()
            .map(|file| Entry {
                file,
                resolutions: graph.resolutions(&file.path).unwrap_or(&[]),
            })
            .collect();
        Self {
            evaluated: entries.clone(),
            searched: entries,
        }
    }
}

/// One check in progress.
pub(crate) struct Run<'a> {
    pub(crate) architecture: &'a Architecture,
    pub(crate) graph: &'a ImportGraph,
    /// The package manifests the evaluated files see.
    pub(crate) manifests: Manifests<'a>,
    pub(crate) classes: &'a mut Classes,
    pub(crate) scope: Scope<'a>,
    /// The rules whose example this run checks, and whether one considered an import of it.
    pub(crate) watched: Option<(Vec<usize>, bool)>,
    pub(crate) violations: Vec<Diagnostic>,
    pub(crate) other: Vec<Diagnostic>,
    used_elements: BTreeSet<ElementId>,
    /// The elements the file being checked touched.
    touched: BTreeSet<ElementId>,
    /// The inline package names that imports reached.
    used_names: BTreeSet<usize>,
    /// The inline package names that the file being checked reached.
    touched_names: BTreeSet<usize>,
    /// The check of each file whose imports this run evaluated.
    pub(crate) checks: BTreeMap<String, FileCheck>,
    summary: Summary,
}

impl<'a> Run<'a> {
    pub(crate) fn new(
        architecture: &'a Architecture,
        graph: &'a ImportGraph,
        classes: &'a mut Classes,
        scope: Scope<'a>,
    ) -> Self {
        let summary = Summary {
            links_not_followed: graph.links_not_followed().to_vec(),
            nested_repositories: graph.nested_repositories().to_vec(),
            ..Summary::default()
        };
        Self {
            architecture,
            graph,
            manifests: graph.manifests_with(None),
            classes,
            scope,
            watched: None,
            violations: Vec::new(),
            other: Vec::new(),
            used_elements: BTreeSet::new(),
            touched: BTreeSet::new(),
            used_names: BTreeSet::new(),
            touched_names: BTreeSet::new(),
            checks: BTreeMap::new(),
            summary,
        }
    }

    /// The class of a file, and its element instance, which marks the element as used.
    fn classify(&mut self, path: &str, work: &mut Work) -> (u32, Option<Classification>) {
        let lookup = Lookup::new(self.architecture, self.graph);
        let found = self.classes.file(lookup, path, work);
        if let Some(class) = &found.1 {
            for place in class.all() {
                self.used_elements.insert(place.element);
                self.touched.insert(place.element);
            }
        }
        found
    }

    /// Evaluates the imports of every file in the scope, one file check each.
    pub(crate) fn imports(&mut self, work: &mut Work) {
        for index in 0..self.scope.evaluated.len() {
            let Entry { file, resolutions } = self.scope.evaluated[index];
            let check = self.file_imports(file, resolutions, work);
            self.checks.insert(file.path.clone(), check);
        }
    }

    fn file_imports(
        &mut self,
        file: &SourceFile,
        resolutions: &[Resolution],
        work: &mut Work,
    ) -> FileCheck {
        let (violations, other) = (self.violations.len(), self.other.len());
        self.touched.clear();
        let from = if self
            .graph
            .file(&file.path)
            .is_some_and(|known| std::ptr::eq(known, file))
        {
            self.classify(&file.path, work)
        } else {
            let found = self.classes.example_file(self.architecture, file, work);
            if let Some(class) = &found.1 {
                self.touched.extend(class.all().map(|place| place.element));
            }
            found
        };
        let mut check = FileCheck::default();
        if let Some(facts) = &file.facts {
            check.parsed = true;
            check.unfollowed = facts.unfollowed.len() as u64;
            check.imports = facts.imports.len() as u64;
            let mut occurrences: HashMap<&str, u32> = HashMap::new();
            for (import, resolution) in facts.imports.iter().zip(resolutions) {
                let occurrence = occurrences.entry(import.specifier.as_str()).or_default();
                let site = Site {
                    file: &file.path,
                    import,
                    occurrence: *occurrence,
                };
                *occurrence += 1;
                self.import((from.0, from.1.as_ref()), &site, resolution, work);
            }
        }
        check.violations = self.violations.split_off(violations);
        check.unresolved = self.other.split_off(other);
        check.permitted_unresolved = std::mem::take(&mut self.summary.permitted_unresolved);
        check.elements = std::mem::take(&mut self.touched);
        check.package_names = std::mem::take(&mut self.touched_names);
        check
    }

    /// Adds file checks to the run: their diagnostics, their counts and the elements they touched.
    pub(crate) fn absorb<'c>(&mut self, checks: impl Iterator<Item = &'c FileCheck>) {
        for check in checks {
            self.violations.extend(check.violations.iter().cloned());
            self.other.extend(check.unresolved.iter().cloned());
            self.summary.files += u64::from(check.parsed);
            self.summary.imports += check.imports;
            self.summary.unfollowed += check.unfollowed;
            self.summary.permitted_unresolved += check.permitted_unresolved;
            self.used_elements.extend(check.elements.iter().copied());
            self.used_names.extend(check.package_names.iter().copied());
        }
    }

    fn import(
        &mut self,
        (from_class, from): (u32, Option<&Classification>),
        site: &Site<'_>,
        resolution: &Resolution,
        work: &mut Work,
    ) {
        let (to_class, to, path, package) = match resolution {
            Resolution::File(path) => {
                let (class, classification) = self.classify(path, work);
                (class, classification, Some(path.as_str()), None)
            }
            Resolution::Package(name) => {
                let (class, classification) = self.package_target(name);
                (class, classification, None, Some(name.as_str()))
            }
            Resolution::Unresolved(reason) => {
                self.unresolved(site, reason);
                return;
            }
        };
        let edge = Edge {
            from: End {
                class: from_class,
                classification: from,
                path: Some(site.file),
                package: None,
            },
            to: End {
                class: to_class,
                classification: to.as_ref(),
                path,
                package,
            },
            file: site.file,
            import: site.import,
            resolution,
            manifests: Some(self.manifests),
        };
        let (broken, evaluations) = self.architecture.evaluate(self.classes, &edge);
        work.rule_evaluations += evaluations;
        self.watch(&edge);
        for sentence in broken {
            let violation = self.violation(sentence, from, to.as_ref(), site, (path, package));
            self.violations.push(violation);
        }
    }

    /// The class of a package that an import names, which notes the sets and names it reaches.
    fn package_target(&mut self, name: &str) -> (u32, Option<Classification>) {
        let (class, classification) = self.classes.package(self.architecture, name);
        if let Some(class) = &classification {
            for place in class.all() {
                self.used_elements.insert(place.element);
                self.touched.insert(place.element);
            }
        }
        if let ClassKey::Package { names, .. } = self.classes.key(class) {
            let reached = names
                .iter()
                .enumerate()
                .filter(|(_, n)| **n)
                .map(|(i, _)| i);
            self.touched_names.extend(reached);
        }
        (class, classification)
    }

    /// Notes whether the rule whose example this run checks looked at an import.
    fn watch(&mut self, edge: &Edge<'_>) {
        if let Some((rules, considered)) = &mut self.watched {
            *considered |= rules.iter().any(|rule| {
                covers(
                    self.architecture.config.rules[*rule].imports,
                    edge.import.kind,
                ) && self.architecture.looks_at(*rule, edge, self.classes)
            });
        }
    }

    fn unresolved(&mut self, site: &Site<'_>, reason: &UnresolvedReason) {
        let specifier = &site.import.specifier;
        if self
            .architecture
            .config
            .allow_unresolved
            .iter()
            .any(|allowed| allowed.permits(specifier))
        {
            self.summary.permitted_unresolved += 1;
            return;
        }
        if *reason == UnresolvedReason::ResolutionFile {
            return;
        }
        let mut diagnostic = Diagnostic::new(Code::ImportUnresolved, reason.message(specifier)).at(
            site.file,
            site.import.line,
            site.import.column,
        );
        diagnostic.span = Some(site.import.span);
        diagnostic.utf16 = Some(site.import.utf16);
        diagnostic.specifier = Some(specifier.clone());
        diagnostic.help = Some(format!(
            "fix the specifier, or add \"{specifier}\" to the config's allowUnresolved with the \
             reason it cannot be resolved"
        ));
        self.other.push(diagnostic);
    }

    fn violation(
        &self,
        sentence: usize,
        from: Option<&Classification>,
        to: Option<&Classification>,
        site: &Site<'_>,
        ends: (Option<&str>, Option<&str>),
    ) -> Diagnostic {
        let compiled = &self.architecture.sentences[sentence];
        let rule = &self.architecture.config.rules[compiled.rule];
        if let RuleKind::TableRow {
            table,
            row: Some(row),
        } = &rule.kind
        {
            let described = crate::messages::RowViolation {
                architecture: self.architecture,
                classes: self.classes,
                table: *table,
                row,
                compiled,
                to,
                path: ends.0,
                package: ends.1,
            }
            .describe(site.import.specifier.as_str());
            return import_diagnostic(rule, site, described);
        }
        let texts = Texts {
            architecture: self.architecture,
            graph: self.graph,
            manifests: self.manifests,
        };
        let parties = Parties {
            from,
            to,
            specifier: &site.import.specifier,
            file: site.file,
        };
        let described = texts.describe(rule, &rule.sentences[compiled.sentence], &parties);
        import_diagnostic(rule, site, described)
    }

    pub(crate) fn cycles(&mut self, work: &mut Work) {
        let searched = &self.scope.searched;
        let index: HashMap<&str, usize> = searched
            .iter()
            .enumerate()
            .map(|(i, entry)| (entry.file.path.as_str(), i))
            .collect();
        for compiled in &self.architecture.sentences {
            let rule = &self.architecture.config.rules[compiled.rule];
            if !matches!(
                rule.sentences[compiled.sentence].condition,
                Condition::NoCycles
            ) {
                continue;
            }
            let selected: Vec<bool> = searched
                .iter()
                .map(|entry| {
                    let lookup = Lookup::new(self.architecture, self.graph);
                    self.classes.file(lookup, &entry.file.path, work);
                    self.classes.file_in(&entry.file.path, compiled.files)
                })
                .collect();
            let (graph, first_step) = self.cycle_graph(rule.imports, &index, &selected);
            work.cycle_files_visited += graph.file_count() as u64;
            if let Some((watched, considered)) = &mut self.watched
                && watched.contains(&compiled.rule)
            {
                let evaluated = self.scope.evaluated.len();
                *considered |= first_step.keys().any(|&(from, _)| from < evaluated);
            }
            for mut group in graph.groups() {
                group.sort_unstable();
                let cycle = graph.shortest_cycle(group[0], &group);
                let step = first_step.get(&(cycle[0], cycle[1])).copied();
                let names: Vec<String> = group
                    .iter()
                    .map(|&i| searched[i].file.path.clone())
                    .collect();
                let path: Vec<String> = cycle
                    .iter()
                    .map(|&i| searched[i].file.path.clone())
                    .collect();
                self.violations
                    .push(cycle_diagnostic(rule, names, path, step));
            }
        }
    }

    /// The graph of the imports between the selected files that a no-cycles rule counts, and the
    /// first import of each edge.
    fn cycle_graph(
        &self,
        imports: ImportKinds,
        index: &HashMap<&str, usize>,
        selected: &[bool],
    ) -> (CycleGraph, HashMap<(usize, usize), &'a Import>) {
        let searched = &self.scope.searched;
        let mut graph = CycleGraph::with_files(searched.len());
        let mut first_step = HashMap::new();
        for (from, &Entry { file, resolutions }) in searched.iter().enumerate() {
            let Some(facts) = file.facts.as_ref().filter(|_| selected[from]) else {
                continue;
            };
            for (import, resolution) in facts.imports.iter().zip(resolutions) {
                if let (true, Resolution::File(path)) = (covers(imports, import.kind), resolution)
                    && let Some(&to) = index.get(path.as_str())
                    && selected[to]
                {
                    graph.add_import(from, to);
                    first_step.entry((from, to)).or_insert(import);
                }
            }
        }
        (graph, first_step)
    }

    /// A declared selector, or an inline pattern of a rule, that matches no file the walk visited.
    /// What a preset declared is listed in the summary instead.
    fn empty_selectors(&mut self) {
        for (set, origin) in &self.architecture.reported {
            if self.classes.files_of_set(*set) > 0 {
                continue;
            }
            let name = match origin {
                Origin::Selector(name) | Origin::Inline(name) => name,
            };
            if self.architecture.from_preset(name) {
                if let Origin::Selector(name) = origin {
                    self.summary
                        .preset_names_matching_nothing
                        .push(name.clone());
                }
                continue;
            }
            let definition = self.architecture.definition(*set);
            let message = match origin {
                Origin::Selector(name) => format!(
                    "the {} \"{name}\" matches no file ({definition}), so no rule about it can \
                     fire; fix its definition or remove it",
                    self.architecture.config.group_word()
                ),
                Origin::Inline(rule) => format!(
                    "the rule \"{rule}\" names the pattern {definition}, which matches no file, so \
                     the rule cannot fire there; fix the pattern"
                ),
            };
            let mut diagnostic = Diagnostic::new(Code::SelectorEmpty, message);
            if let Origin::Inline(rule) = origin {
                diagnostic.rule = Some(rule.clone());
            }
            self.other.push(diagnostic);
        }
    }

    /// A file in two of the config's own parts. The config holds the fix, so the baseline does not
    /// hold it.
    fn part_overlaps(&mut self) {
        let elements = &self.architecture.config.elements;
        for (path, parts) in self.classes.overlaps() {
            let names: Vec<String> = parts
                .iter()
                .map(|part| format!("\"{}\"", elements[part.index()].name))
                .collect();
            let message = format!(
                "{path} is in the parts {}, and a file is in one part at most; narrow one of the \
                 patterns with `not`, or make one of them a group. The baseline does not hold this \
                 problem, so fix it in the config",
                names.join(" and ")
            );
            self.other
                .push(Diagnostic::new(Code::PartOverlap, message).at(path, 0, 0));
        }
    }

    /// For each table: the kind rows with no file, a table that checks nothing, and a file in two
    /// of its rows.
    fn tables(&mut self) {
        let architecture = self.architecture;
        if architecture.config.tables.is_empty() {
            return;
        }
        let mut no_file = Vec::new();
        for (index, table) in architecture.config.tables.iter().enumerate() {
            let id = TableId(u32::try_from(index).unwrap_or(u32::MAX));
            let rows: Vec<(&str, usize)> = architecture
                .sentences
                .iter()
                .filter(|compiled| compiled.sentence == 0)
                .filter_map(
                    |compiled| match &architecture.config.rules[compiled.rule].kind {
                        RuleKind::TableRow {
                            table,
                            row: Some(row),
                        } if *table == id => Some((row.as_str(), compiled.files)),
                        _ => None,
                    },
                )
                .collect();
            let live =
                architecture.config.rules.iter().any(
                    |rule| matches!(&rule.kind, RuleKind::TableRow { table, .. } if *table == id),
                );
            if !live {
                continue;
            }
            let files = rows
                .iter()
                .map(|(_, set)| self.classes.table_files_of_set(*set))
                .sum::<u64>();
            if files == 0 && (table.rows.is_empty() || !rows.is_empty()) {
                self.table_checks_nothing(table);
                continue;
            }
            for (row, set) in &rows {
                let is_kind = table.rows.iter().any(|(key, files)| {
                    key == row && matches!(files.terms.as_slice(), [Term::Kind { .. }])
                });
                if is_kind && self.classes.table_files_of_set(*set) == 0 {
                    no_file.push(format!("{}.{row}", table.name));
                }
            }
            self.row_overlaps(table, &rows);
        }
        self.summary.rows_with_no_file = Some(no_file);
    }

    /// A table whose rows together have no file: an error, unless another diagnostic already says
    /// why, or a preset declares it.
    fn table_checks_nothing(&mut self, table: &Table) {
        if table.from_preset {
            let why = table.kinds.as_ref().map_or_else(
                || "none of its rows has a file".to_owned(),
                |kinds| {
                    format!(
                        "no file matches its kind pattern {}",
                        kinds.pattern.source()
                    )
                },
            );
            self.summary
                .preset_names_matching_nothing
                .push(table.name.clone());
            self.summary
                .preset_tables_matching_nothing
                .insert(table.name.clone(), why);
            return;
        }
        if !table.rows.is_empty() && self.already_reported(table) {
            return;
        }
        let name = &table.name;
        let message = if table.rows.is_empty() {
            format!(
                "the table \"{name}\" has no row, so it checks nothing; add a row or remove the table"
            )
        } else {
            let pattern = table.kinds.as_ref().map_or_else(String::new, |kinds| {
                format!(
                    ", and no file matches its kind pattern {}",
                    kinds.pattern.source()
                )
            });
            format!(
                "the rows of the table \"{name}\" have no file{pattern}, so it checks nothing; check \
                 the pattern and the row names, or remove the table"
            )
        };
        self.other.push(Diagnostic::new(Code::TableEmpty, message));
    }

    /// Whether a pattern, a part or a group that the table names matches no file, for which
    /// `selector-empty` or `element-empty` already reports it.
    fn already_reported(&self, table: &Table) -> bool {
        let architecture = self.architecture;
        let empty_pattern = architecture.reported.iter().any(|(set, origin)| {
            matches!(origin, Origin::Inline(name) if *name == table.name)
                && self.classes.files_of_set(*set) == 0
        });
        let areas = architecture.config.rules.iter().flat_map(|rule| {
            let of_table = matches!(
                &rule.kind,
                RuleKind::TableRow { table: id, .. }
                    if architecture.config.tables[id.index()].name == table.name
            );
            rule.sentences
                .iter()
                .filter(move |_| of_table)
                .filter_map(|sentence| sentence.options.within.as_ref())
                .flat_map(|area| area.files.iter())
        });
        let mut sets: Vec<&FileSet> = table.rows.iter().map(|(_, files)| files).collect();
        sets.push(&table.files);
        sets.extend(areas);
        empty_pattern
            || sets
                .iter()
                .flat_map(|set| &set.terms)
                .any(|term| self.term_is_empty(term))
    }

    fn term_is_empty(&self, term: &Term) -> bool {
        match term {
            Term::Element(element) => self.classes.files_of(*element) == 0,
            Term::Selector(id) => self
                .architecture
                .sets
                .iter()
                .position(|set| set.terms == [Term::Selector(*id)])
                .is_some_and(|set| self.classes.files_of_set(set) == 0),
            _ => false,
        }
    }

    /// A file in two rows of one table. The config holds the fix, so the baseline does not hold it.
    /// Only a class with two rows needs a look at its files, so the search grows with classes.
    fn row_overlaps(&mut self, table: &Table, rows: &[(&str, usize)]) {
        let in_two = |key: &ClassKey| {
            matches!(key, ClassKey::File { bits, .. }
                if rows.iter().filter(|(_, set)| bits[*set]).count() > 1)
        };
        if !self.classes.counted_classes().any(in_two) {
            return;
        }
        let mut found = Vec::new();
        for path in self.classes.counted_files_in(in_two) {
            let names: Vec<String> = rows
                .iter()
                .filter(|(_, set)| self.classes.file_in(path, *set))
                .map(|(row, _)| format!("`{row}`"))
                .collect();
            let message = format!(
                "{path} is in the rows {} of the table \"{}\", so each of them judges it; narrow \
                 one of the rows with `not`, or leave the file out of the table's files. The \
                 baseline does not hold this problem, so fix it in the config",
                names.join(" and "),
                table.name
            );
            found.push(Diagnostic::new(Code::TableRowOverlap, message).at(path, 0, 0));
        }
        self.other.extend(found);
    }

    /// A name of an externals set that is a package of the workspace, which only the graph knows.
    fn externals_in_workspace(&mut self) {
        let graph = self.graph;
        let problems = self
            .architecture
            .config
            .externals_in_workspace(|name| graph.is_workspace_package(name));
        for problem in problems {
            let field = problem.field.unwrap_or_default();
            self.other.push(Diagnostic::new(
                Code::ConfigInvalid,
                format!("{field}: {}", problem.message),
            ));
        }
    }

    fn exceptions(&mut self) {
        let rule_names: Vec<&str> = self
            .architecture
            .config
            .rules
            .iter()
            .map(|r| r.name.as_str())
            .collect();
        for file in self.graph.files() {
            let Some(facts) = &file.facts else { continue };
            for exception in exceptions::read(&facts.allow_comments) {
                if let Some(problem) = exception_problem(&exception, &rule_names) {
                    self.other.push(problem.at(&file.path, exception.line, 1));
                } else {
                    self.permit(&file.path, &exception);
                }
            }
        }
    }

    /// Removes the violations an exception permits, or reports that it permits none.
    fn permit(&mut self, file: &str, exception: &Exception) {
        let before = self.violations.len();
        self.violations.retain(|v| {
            !(v.file.as_deref() == Some(file)
                && v.line == exception.target_line
                && v.rule.as_deref() == Some(exception.rule.as_str())
                && v.code != Code::NoCycles
                && !v.is_about_a_path())
        });
        let permitted = (before - self.violations.len()) as u64;
        if permitted == 0 {
            let about_the_file = self.violations.iter().any(|v| {
                v.file.as_deref() == Some(file)
                    && v.rule.as_deref() == Some(exception.rule.as_str())
                    && v.is_about_a_path()
            });
            let message = if about_the_file {
                format!(
                    "the exception for \"{}\" permits nothing, because that rule's violation is \
                     about the file, not an import, and only the baseline can permit it; run \
                     `archangel baseline add` and delete the comment",
                    exception.rule
                )
            } else {
                format!(
                    "the exception for \"{}\" permits nothing, because the next line does not break that rule; delete it",
                    exception.rule
                )
            };
            self.other
                .push(Diagnostic::new(Code::ExceptionUnused, message).at(file, exception.line, 1));
        } else {
            *self
                .summary
                .exceptions
                .entry(exception.rule.clone())
                .or_default() += permitted;
        }
    }

    fn empty_elements(&mut self) {
        let dependencies = self.graph.manifest_dependencies();
        for (index, element) in self.architecture.config.elements.iter().enumerate() {
            let id = ElementId(u32::try_from(index).unwrap_or(u32::MAX));
            if self.used_elements.contains(&id) {
                continue;
            }
            let (listed, what) = match &element.kind {
                ElementKind::Files(patterns) => (
                    self.classes.files_of(id) > 0,
                    pattern_list(patterns.iter().map(Pattern::source)),
                ),
                ElementKind::Packages(patterns) => (
                    // Node's built-in modules are always installed, though no manifest lists them.
                    patterns.iter().any(|p| p.source().starts_with("node:"))
                        || dependencies.iter().any(|name| {
                            self.architecture
                                .matcher
                                .package_elements(name)
                                .contains(&id)
                        }),
                    pattern_list(patterns.iter().map(Pattern::source)),
                ),
            };
            if !listed && self.architecture.from_preset(&element.name) {
                self.summary
                    .preset_names_matching_nothing
                    .push(element.name.clone());
            } else if !listed {
                let word = self.architecture.config.part_word();
                let message = format!(
                    "the {word} \"{}\" matches nothing ({what}), so no rule about it can fire; fix the \
                     pattern or remove the {word}",
                    element.name
                );
                self.other
                    .push(Diagnostic::new(Code::ElementEmpty, message));
            }
        }
        self.empty_package_names();
        self.empty_presets();
    }

    /// The sets of the rows of each table that a preset declares.
    fn tables_of_preset(&self, preset: &str) -> Vec<usize> {
        let architecture = self.architecture;
        let prefix = format!("{preset}/");
        architecture
            .sentences
            .iter()
            .filter(|compiled| compiled.sentence == 0)
            .filter_map(
                |compiled| match &architecture.config.rules[compiled.rule].kind {
                    RuleKind::TableRow {
                        table,
                        row: Some(_),
                    } if architecture.config.tables[table.index()]
                        .name
                        .starts_with(&prefix) =>
                    {
                        Some(compiled.files)
                    }
                    _ => None,
                },
            )
            .collect()
    }

    /// A preset whose file elements and tables together match no file: most likely a wrong root.
    fn empty_presets(&mut self) {
        for preset in &self.architecture.config.presets {
            let files: Vec<ElementId> = preset
                .elements
                .iter()
                .copied()
                .filter(|id| {
                    matches!(
                        self.architecture.config.elements[id.index()].kind,
                        ElementKind::Files(_)
                    )
                })
                .collect();
            let tables = self.tables_of_preset(&preset.name);
            let tables_have_files = tables
                .iter()
                .any(|set| self.classes.table_files_of_set(*set) > 0);
            if (files.is_empty() && tables.is_empty())
                || tables_have_files
                || files.iter().any(|&id| self.classes.files_of(id) > 0)
            {
                continue;
            }
            let root = if preset.root.is_empty() {
                "the config folder".to_owned()
            } else {
                format!("`{}`", preset.root)
            };
            let message = format!(
                "the preset \"{}\" matches no file under {root}, so none of its rules can fire; give \
                 it the folder it applies to as its root",
                preset.name
            );
            self.other.push(Diagnostic::new(Code::PresetEmpty, message));
        }
    }

    /// An inline package name on the side of a rule that permits imports, which no import reaches
    /// and no manifest lists: a misspelt package there means the rule refuses what it should
    /// permit. A `mayNotImport` side is exempt, because forbidding a package before anyone
    /// installs it is that rule's passing state.
    fn empty_package_names(&mut self) {
        let dependencies = self.graph.manifest_dependencies();
        let mut reported: BTreeSet<(usize, usize)> = BTreeSet::new();
        for compiled in &self.architecture.sentences {
            let rule = &self.architecture.config.rules[compiled.rule];
            if !matches!(
                rule.sentences[compiled.sentence].condition,
                Condition::MayOnlyImport(_) | Condition::MayOnlyBeImportedBy(_)
            ) {
                continue;
            }
            let area = compiled.area.iter().flat_map(|area| &area.packages);
            for &name in compiled.side.packages.iter().chain(area) {
                let pattern = &self.architecture.package_names[name];
                let always = pattern.source() == "*" || pattern.source().starts_with("node:");
                let listed = always
                    || self.used_names.contains(&name)
                    || dependencies
                        .iter()
                        .any(|dependency| self.architecture.package_named(name, dependency));
                if listed || !reported.insert((compiled.rule, name)) {
                    continue;
                }
                let message = format!(
                    "the rule \"{}\" names the package `{}`, which no import uses and no package \
                     manifest lists, so the rule never permits it; fix the name",
                    rule.name,
                    pattern.source()
                );
                let mut diagnostic = Diagnostic::new(Code::ElementEmpty, message);
                diagnostic.rule = Some(rule.name.clone());
                self.other.push(diagnostic);
            }
        }
    }

    fn graph_problems(&mut self) {
        self.problems(self.graph.problems());
    }

    pub(crate) fn problems(&mut self, problems: &[GraphProblem]) {
        for problem in problems {
            self.other.push(match problem {
                GraphProblem::FileUnreadable { path, reason } => {
                    Diagnostic::new(Code::FileUnreadable, reason.clone()).at(path, 0, 0)
                }
                GraphProblem::ParseFailed { path, failure } => parse_failed(path, failure),
                GraphProblem::ResolutionFileInvalid { path, reason } => {
                    Diagnostic::new(Code::ResolutionFileInvalid, reason.clone()).at(path, 0, 0)
                }
                GraphProblem::IgnoreLineInvalid { line, reason } => Diagnostic::new(
                    Code::IgnoreLineInvalid,
                    format!("the ignore line \"{line}\" in the config: {reason}"),
                ),
            });
        }
    }

    fn apply_baseline(&mut self, baseline: &Baseline) {
        let rule_names: BTreeSet<&str> = self
            .architecture
            .config
            .rules
            .iter()
            .map(|r| r.name.as_str())
            .collect();
        let mut remaining: BTreeMap<Key, u32> = BTreeMap::new();
        for entry in &baseline.entries {
            *remaining.entry(entry.key()).or_default() += entry.count;
        }
        let summary = &mut self.summary;
        self.violations.retain(|violation| {
            let Some(id) = &violation.id else { return true };
            match remaining.get_mut(&id.baseline_key()) {
                Some(count) if *count > 0 => {
                    *count -= 1;
                    *summary.baselined.entry(id.rule().to_owned()).or_default() += 1;
                    false
                }
                _ => true,
            }
        });
        for entry in &baseline.entries {
            let left = remaining.get(&entry.key()).copied().unwrap_or(0);
            if !rule_names.contains(entry.rule.as_str()) {
                let message = format!(
                    "the baseline has an entry for the rule \"{}\", which does not exist; run `archangel baseline prune`",
                    entry.rule
                );
                self.other
                    .push(Diagnostic::new(Code::BaselineStale, message));
            } else if left > 0 {
                let place = entry
                    .file
                    .clone()
                    .or_else(|| entry.path.clone())
                    .unwrap_or_else(|| entry.files.join(", "));
                let message = format!(
                    "the baseline permits {left} more violation(s) of \"{}\" in {place} than the code has, \
                     because they were fixed; run `archangel baseline prune`",
                    entry.rule
                );
                self.other
                    .push(Diagnostic::new(Code::BaselineStale, message));
            }
            remaining.remove(&entry.key());
        }
    }

    pub(crate) fn finish(self) -> Report {
        let mut diagnostics = self.violations;
        diagnostics.extend(self.other);
        diagnostics.sort_by(|a, b| {
            a.sort_key()
                .cmp(&b.sort_key())
                .then_with(|| a.message.cmp(&b.message))
        });
        Report {
            diagnostics,
            summary: self.summary,
        }
    }
}
