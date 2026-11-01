//! The config compiled for checking. Every rule is a list of sentences; every file and package
//! gets a class, the files in one element and the same sets of files, and an import is checked
//! by the pair of classes at its ends, so it costs the same for five rules or five hundred.

use archangel_config::{
    Classification, Condition, Config, ElementId, ElementMatcher, FileSet, ImportKinds,
    OwnInstance, Pattern, RuleKind, Sentence, Side, TableId, Term, package_matches,
};
use archangel_graph::{Import, ImportGraph, ImportKind, Manifests, Resolution, Work};

use crate::classes::{ClassKey, Classes, Lookup};

/// The config, compiled.
#[derive(Debug, Clone)]
pub struct Architecture {
    pub(crate) config: Config,
    pub(crate) matcher: ElementMatcher,
    /// Every set of files whose membership is part of a file's class.
    pub(crate) sets: Vec<FileSet>,
    /// Every inline package name on a rule side; a package's class holds which of them name it.
    pub(crate) package_names: Vec<Pattern>,
    /// Every sentence of every rule, in config order.
    pub(crate) sentences: Vec<Compiled>,
    /// The sets that must match a file, with what declared them.
    pub(crate) reported: Vec<(usize, Origin)>,
}

/// One sentence, with the indices of its sets.
#[derive(Debug, Clone)]
pub(crate) struct Compiled {
    pub(crate) rule: usize,
    pub(crate) sentence: usize,
    pub(crate) files: usize,
    pub(crate) side: CompiledSide,
    /// The area of the sentence, which holds the other end of each import that it judges.
    pub(crate) area: Option<CompiledSide>,
    /// The set of the sentence's own files: its files, or for a row of a table the files of the
    /// row's name.
    pub(crate) own_files: usize,
    /// What makes an instance of the sentence's own files, for a `mayOnlyImport` sentence that
    /// permits only the same instance of them.
    pub(crate) own_instance: Option<OwnInstance>,
    pub(crate) stop_at: Option<usize>,
}

/// A rule side or an area, with the indices of its sets.
#[derive(Debug, Clone, Default)]
pub(crate) struct CompiledSide {
    pub(crate) files: Vec<usize>,
    pub(crate) elements: Vec<ElementId>,
    pub(crate) packages: Vec<usize>,
}

impl CompiledSide {
    /// Whether a file or a package of this class is in the side.
    pub(crate) fn holds(&self, class: &ClassKey) -> bool {
        match class {
            ClassKey::File { bits, .. } => self.files.iter().any(|&s| bits[s]),
            ClassKey::Package { elements, names } => {
                elements.iter().any(|e| self.elements.contains(e))
                    || self.packages.iter().any(|&n| names[n])
            }
        }
    }
}

/// What declared a set that must match a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Origin {
    /// A declared selector, by name.
    Selector(String),
    /// An inline pattern in a rule, by the rule's name.
    Inline(String),
}

/// What an import points to.
#[derive(Debug, Clone, Copy)]
pub enum Target<'a> {
    /// A repository file, relative to the config folder.
    File(&'a str),
    /// A package, by name.
    Package(&'a str),
}

/// One sentence in one cell, and the test it still needs.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Check {
    pub(crate) sentence: usize,
    test: Test,
}

#[derive(Debug, Clone, Copy)]
enum Test {
    /// The pair of classes alone breaks the sentence.
    Always,
    /// The import's own facts decide: its names, specifier, instances or manifest.
    Facts,
    /// The files are in different instances and the target is not an entry file.
    Entry,
    /// The target is among the sentence's own files: it breaks the sentence only from another
    /// instance, and when the import's own facts hold.
    OtherInstance,
}

/// One end of an import, classified.
#[derive(Debug, Clone, Copy)]
pub(crate) struct End<'a> {
    pub(crate) class: u32,
    pub(crate) classification: Option<&'a Classification>,
    pub(crate) path: Option<&'a str>,
    pub(crate) package: Option<&'a str>,
}

/// One import, with its two ends and what the tests of its facts need.
#[derive(Clone, Copy)]
pub(crate) struct Edge<'a> {
    pub(crate) from: End<'a>,
    pub(crate) to: End<'a>,
    pub(crate) file: &'a str,
    pub(crate) import: &'a Import,
    pub(crate) resolution: &'a Resolution,
    /// The package manifests the importing file sees, or `None` without the facts of a graph.
    pub(crate) manifests: Option<Manifests<'a>>,
}

impl Architecture {
    /// Compiles a validated config.
    #[must_use]
    pub fn compile(config: Config) -> Self {
        let matcher = ElementMatcher::new(&config);
        let mut architecture = Self {
            config,
            matcher,
            sets: Vec::new(),
            package_names: Vec::new(),
            sentences: Vec::new(),
            reported: Vec::new(),
        };
        for index in 0..architecture.config.selectors.len() {
            let id = archangel_config::SelectorId(u32::try_from(index).unwrap_or(u32::MAX));
            let set = architecture.set(&FileSet::of(Term::Selector(id)));
            let name = architecture.config.selectors[index].name.clone();
            architecture.reported.push((set, Origin::Selector(name)));
        }
        let rules = architecture.config.rules.clone();
        for (rule_index, rule) in rules.iter().enumerate() {
            for (sentence_index, sentence) in rule.sentences.iter().enumerate() {
                let compiled = architecture.sentence(rule_index, sentence_index, sentence);
                architecture.sentences.push(compiled);
                let origin = match &rule.kind {
                    RuleKind::TableRow { table, .. } => {
                        architecture.config.tables[table.index()].name.clone()
                    }
                    _ => rule.name.clone(),
                };
                architecture.report_inline(&origin, sentence);
            }
        }
        architecture
    }

    fn sentence(&mut self, rule: usize, index: usize, sentence: &Sentence) -> Compiled {
        let side = sentence.condition.side().cloned().unwrap_or_default();
        let may_only_import = matches!(sentence.condition, Condition::MayOnlyImport(_));
        let own_instance = match &sentence.options.own {
            Some(own) => own.instance.filter(|_| may_only_import && own.permitted),
            None => sentence
                .files
                .element()
                .filter(|element| {
                    may_only_import && self.config.elements[element.index()].has_capture()
                })
                .map(OwnInstance::Element),
        };
        let files = self.set(&sentence.files);
        let own_files = sentence
            .options
            .own
            .as_ref()
            .map_or(files, |own| self.set(&own.files));
        Compiled {
            rule,
            sentence: index,
            files,
            own_files,
            side: self.side(&side),
            area: sentence.options.within.as_ref().map(|area| self.side(area)),
            own_instance,
            stop_at: sentence.options.stop_at.as_ref().map(|s| self.set(s)),
        }
    }

    fn side(&mut self, side: &Side) -> CompiledSide {
        CompiledSide {
            files: side.files.iter().map(|s| self.set(s)).collect(),
            elements: side.elements.clone(),
            packages: side.packages.iter().map(|p| self.package_name(p)).collect(),
        }
    }

    /// Each pattern a sentence writes inline must match a file, except in the files of a
    /// `mustNotExist` rule, which pass by matching nothing, and inside a `not`, where leaving out
    /// nothing changes nothing.
    fn report_inline(&mut self, rule: &str, sentence: &Sentence) {
        let side = sentence.condition.side().cloned().unwrap_or_default();
        let own =
            (!matches!(sentence.condition, Condition::MustNotExist)).then_some(&sentence.files);
        let sets = own
            .into_iter()
            .chain(&side.files)
            .chain(sentence.options.within.iter().flat_map(|area| &area.files))
            .chain(sentence.options.stop_at.as_ref());
        let patterns: Vec<_> = sets.flat_map(FileSet::included_patterns).collect();
        for pattern in patterns {
            let set = self.set(&FileSet::of(Term::Pattern(pattern)));
            let origin = Origin::Inline(rule.to_owned());
            if !self.reported.contains(&(set, origin.clone())) {
                self.reported.push((set, origin));
            }
        }
    }

    fn set(&mut self, set: &FileSet) -> usize {
        if let Some(index) = self.sets.iter().position(|known| known == set) {
            return index;
        }
        self.sets.push(set.clone());
        self.sets.len() - 1
    }

    fn package_name(&mut self, name: &Pattern) -> usize {
        if let Some(index) = self
            .package_names
            .iter()
            .position(|known| known.source() == name.source())
        {
            return index;
        }
        self.package_names.push(name.clone());
        self.package_names.len() - 1
    }

    /// The validated config.
    #[must_use]
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// The element matcher.
    #[must_use]
    pub fn matcher(&self) -> &ElementMatcher {
        &self.matcher
    }

    /// The bits of a repository file's class, classified alone.
    fn bits_of(&self, graph: &ImportGraph, path: &str) -> Vec<bool> {
        let mut classes = Classes::new();
        let (class, _) = classes.file(Lookup::new(self, graph), path, &mut Work::default());
        match classes.key(class) {
            ClassKey::File { bits, .. } => bits.clone(),
            ClassKey::Package { .. } => Vec::new(),
        }
    }

    /// Whether a name of an element, a selector or a rule is one a preset declared: its prefix
    /// is the preset's name.
    #[must_use]
    pub fn from_preset(&self, name: &str) -> bool {
        name.split_once('/').is_some_and(|(prefix, _)| {
            self.config
                .presets
                .iter()
                .any(|preset| preset.name == prefix)
        })
    }

    /// The declared selectors that a repository file is in, in declaration order.
    #[must_use]
    pub fn selectors_of(&self, graph: &ImportGraph, path: &str) -> Vec<String> {
        let bits = self.bits_of(graph, path);
        self.reported
            .iter()
            .filter_map(|(set, origin)| match origin {
                Origin::Selector(name) if bits.get(*set) == Some(&true) => Some(name.clone()),
                _ => None,
            })
            .collect()
    }

    /// The sentences that are about a repository file, as (rule, sentence) indices: those whose
    /// files or whose side it is in.
    #[must_use]
    pub fn sentences_about(&self, graph: &ImportGraph, path: &str) -> Vec<(usize, usize)> {
        let bits = self.bits_of(graph, path);
        let set = |index: usize| bits.get(index) == Some(&true);
        self.sentences
            .iter()
            .filter(|c| set(c.files) || c.side.files.iter().any(|&s| set(s)))
            .map(|c| (c.rule, c.sentence))
            .collect()
    }

    /// The kinds of a repository file: each table that gives it one, with the kind.
    #[must_use]
    pub fn kinds_of(&self, graph: &ImportGraph, path: &str) -> Vec<(String, String)> {
        let mut classes = Classes::new();
        classes.file(Lookup::new(self, graph), path, &mut Work::default());
        self.kinds_in(&classes, path)
    }

    /// The kinds of a file that `classes` has classified.
    pub(crate) fn kinds_in(&self, classes: &Classes, path: &str) -> Vec<(String, String)> {
        self.config
            .tables
            .iter()
            .enumerate()
            .filter_map(|(index, table)| {
                let id = TableId(u32::try_from(index).unwrap_or(u32::MAX));
                let place = classes.kind_place(path, id)?;
                Some((table.name.clone(), place.kind.clone()))
            })
            .collect()
    }

    /// The rules that an import from the file `from` into `target` breaks, by their index in the
    /// config, without the facts of a graph: a `dependency` option sees no manifest.
    #[must_use]
    pub fn broken_rules(&self, from: &str, target: Target<'_>, import: &Import) -> Vec<usize> {
        self.broken_rules_in(None, from, target, import)
    }

    /// The rules that an import from the file `from` into `target` breaks, by their index in the
    /// config, with the graph's directives and manifests.
    #[must_use]
    pub fn broken_rules_in(
        &self,
        graph: Option<&ImportGraph>,
        from: &str,
        target: Target<'_>,
        import: &Import,
    ) -> Vec<usize> {
        let mut classes = Classes::new();
        let mut work = Work::default();
        let lookup = graph.map_or_else(|| Lookup::paths(self), |graph| Lookup::new(self, graph));
        let (from_class, from_classification) = classes.file(lookup, from, &mut work);
        let (resolution, (to_class, to_classification), path, package) = match target {
            Target::File(path) => (
                Resolution::File(path.to_owned()),
                classes.file(lookup, path, &mut work),
                Some(path),
                None,
            ),
            Target::Package(name) => (
                Resolution::Package(name.to_owned()),
                classes.package(self, name),
                None,
                Some(name),
            ),
        };
        let edge = Edge {
            from: End {
                class: from_class,
                classification: from_classification.as_ref(),
                path: Some(from),
                package: None,
            },
            to: End {
                class: to_class,
                classification: to_classification.as_ref(),
                path,
                package,
            },
            file: from,
            import,
            resolution: &resolution,
            manifests: graph.map(|graph| graph.manifests_with(None)),
        };
        let (broken, _) = self.evaluate(&mut classes, &edge);
        let mut rules: Vec<usize> = broken
            .into_iter()
            .map(|sentence| self.sentences[sentence].rule)
            .collect();
        rules.dedup();
        rules
    }

    /// The sentences an import breaks, and how many sentence checks that took.
    pub(crate) fn evaluate(&self, classes: &mut Classes, edge: &Edge<'_>) -> (Vec<usize>, u64) {
        classes.fill_cell(self, edge.from.class, edge.to.class);
        let classes = &*classes;
        let cell = classes.cell(edge.from.class, edge.to.class);
        let mut evaluations = if search_every_rule() {
            self.sentences.len()
        } else {
            cell.len()
        } as u64;
        if area_per_import() {
            evaluations += self.sentences.iter().filter(|c| c.area.is_some()).count() as u64;
        }
        if every_row() {
            evaluations += self
                .sentences
                .iter()
                .filter(|c| matches!(self.config.rules[c.rule].kind, RuleKind::TableRow { .. }))
                .count() as u64;
        }
        let broken = cell
            .iter()
            .filter(|check| {
                let rule = &self.config.rules[self.sentences[check.sentence].rule];
                covers(rule.imports, edge.import.kind)
            })
            .filter(|check| self.test(check, edge, classes))
            .map(|check| check.sentence)
            .collect();
        (broken, evaluations)
    }

    /// The checks of the sentences that can apply to an import between two classes.
    pub(crate) fn fill(&self, from: &ClassKey, to: &ClassKey) -> Vec<Check> {
        if !matches!(from, ClassKey::File { .. }) {
            return Vec::new();
        }
        self.sentences
            .iter()
            .enumerate()
            .filter_map(|(index, compiled)| {
                let sentence = &self.config.rules[compiled.rule].sentences[compiled.sentence];
                let applies = !sentence.options.transitively
                    && Self::applies(compiled, &sentence.condition, from, to);
                applies.then(|| Check {
                    sentence: index,
                    test: if matches!(sentence.condition, Condition::Entry { .. }) {
                        Test::Entry
                    } else if compiled.own_instance.is_some() && in_own(compiled, to) {
                        Test::OtherInstance
                    } else if sentence.options.needs_import() {
                        Test::Facts
                    } else {
                        Test::Always
                    },
                })
            })
            .collect()
    }

    /// Whether a sentence's condition can refuse an import from a file of the class `from` into
    /// `to`, before the import's own facts.
    fn applies(compiled: &Compiled, condition: &Condition, from: &ClassKey, to: &ClassKey) -> bool {
        let in_area = |end: &ClassKey| compiled.area.as_ref().is_none_or(|area| area.holds(end));
        let from_in = in_files(compiled, from);
        match condition {
            Condition::MayNotImport(_) => from_in && in_area(to) && compiled.side.holds(to),
            Condition::MayOnlyImport(_) => {
                from_in
                    && in_area(to)
                    && !compiled.side.holds(to)
                    && (!in_own(compiled, to) || compiled.own_instance.is_some())
            }
            Condition::MayOnlyBeImportedBy(_) => {
                in_files(compiled, to) && in_area(from) && !compiled.side.holds(from) && !from_in
            }
            Condition::Entry { element, .. } => {
                matches!(to, ClassKey::File { elements, .. } if elements.contains(element))
            }
            Condition::MustNotExist
            | Condition::NoCycles
            | Condition::MustBeNamed(_)
            | Condition::MustContain(_)
            | Condition::MustExport(_)
            | Condition::MayNotExport(_)
            | Condition::MustOnlyReExport
            | Condition::Plugin(_) => false,
        }
    }

    /// Whether the other end of an import is on a sentence's side.
    pub(crate) fn in_side_class(compiled: &Compiled, to: &ClassKey) -> bool {
        compiled.side.holds(to)
    }

    /// Whether a package's name is named by an inline package name.
    pub(crate) fn package_named(&self, index: usize, name: &str) -> bool {
        package_matches(&self.package_names[index], name)
    }

    /// Whether a rule's definition looks at an import between two classes at all, whatever it
    /// decides, before its `imports` option. An accepted example proves something only when its
    /// rule looked at one of its imports. No-cycles looks at the graph, not at single imports, so
    /// it answers false here.
    pub(crate) fn considers(&self, rule: usize, from: &ClassKey, to: &ClassKey) -> bool {
        fn elements(key: &ClassKey) -> &[ElementId] {
            match key {
                ClassKey::File { elements, .. } | ClassKey::Package { elements, .. } => elements,
            }
        }
        match &self.config.rules[rule].kind {
            RuleKind::Layers { order } => {
                elements(from).iter().any(|f| order.contains(f))
                    && elements(to).iter().any(|t| order.contains(t))
            }
            RuleKind::Isolate { element: e } => {
                elements(from).contains(e) && elements(to).contains(e)
            }
            RuleKind::Entry { element: e, .. } => elements(to).contains(e),
            RuleKind::Sentence | RuleKind::TableRow { .. } => self
                .sentences
                .iter()
                .filter(|compiled| compiled.rule == rule)
                .any(|compiled| {
                    let sentence = &self.config.rules[rule].sentences[compiled.sentence];
                    let in_area =
                        |end: &ClassKey| compiled.area.as_ref().is_none_or(|area| area.holds(end));
                    match sentence.condition {
                        Condition::MayNotImport(_) | Condition::MayOnlyImport(_) => {
                            in_files(compiled, from) && in_area(to)
                        }
                        Condition::MayOnlyBeImportedBy(_) => {
                            in_files(compiled, to) && in_area(from)
                        }
                        Condition::Entry { .. } => in_files(compiled, to),
                        Condition::MustNotExist
                        | Condition::NoCycles
                        | Condition::MustBeNamed(_)
                        | Condition::MustContain(_)
                        | Condition::MustExport(_)
                        | Condition::MayNotExport(_)
                        | Condition::MustOnlyReExport
                        | Condition::Plugin(_) => false,
                    }
                }),
        }
    }

    pub(crate) fn test(&self, check: &Check, edge: &Edge<'_>, classes: &Classes) -> bool {
        let compiled = &self.sentences[check.sentence];
        let sentence = &self.config.rules[compiled.rule].sentences[compiled.sentence];
        match check.test {
            Test::Always => true,
            Test::Entry => different_instances(sentence, edge) && !is_entry(sentence, &edge.to),
            Test::Facts => crate::facts::hold(sentence, edge, classes),
            Test::OtherInstance => {
                compiled.own_instance.is_some_and(|own| match own {
                    OwnInstance::Element(element) => instances_differ(element, edge),
                    OwnInstance::Kind(table) => classes.kind_instances(edge, table) != Some(true),
                }) && crate::facts::hold(sentence, edge, classes)
            }
        }
    }
}

/// Whether a file of this class is in a set of files.
fn in_set(set: usize, class: &ClassKey) -> bool {
    matches!(class, ClassKey::File { bits, .. } if bits[set])
}

/// Whether a file of this class is among a sentence's files.
pub(crate) fn in_files(compiled: &Compiled, class: &ClassKey) -> bool {
    in_set(compiled.files, class)
}

/// Whether a file of this class is among a sentence's own files.
fn in_own(compiled: &Compiled, class: &ClassKey) -> bool {
    in_set(compiled.own_files, class)
}

/// Whether the two ends of an import are in different instances of an entry sentence's element.
fn different_instances(sentence: &Sentence, edge: &Edge<'_>) -> bool {
    let Condition::Entry { element, .. } = &sentence.condition else {
        return false;
    };
    instances_differ(*element, edge)
}

/// Whether the two ends of an import are in different instances of an element, or only one of them
/// is in it.
fn instances_differ(element: ElementId, edge: &Edge<'_>) -> bool {
    let from = edge.from.classification.and_then(|c| c.of(element));
    let to = edge.to.classification.and_then(|c| c.of(element));
    match (from, to) {
        (Some(from), Some(to)) => !from.same_instance(to),
        (None, None) => false,
        _ => true,
    }
}

/// Whether the target of an import is one of an entry sentence's entry files.
fn is_entry(sentence: &Sentence, to: &End<'_>) -> bool {
    let Condition::Entry { files, element } = &sentence.condition else {
        return false;
    };
    let (Some(class), Some(path)) = (to.classification.and_then(|c| c.of(*element)), to.path)
    else {
        return false;
    };
    let inside = path
        .strip_prefix(&format!("{}/", class.root))
        .unwrap_or(path);
    files
        .iter()
        .any(|pattern| pattern.matches(inside).is_some())
}

/// Whether a rule that counts `imports` counts an import of `kind`.
pub(crate) fn covers(imports: ImportKinds, kind: ImportKind) -> bool {
    match imports {
        ImportKinds::All => true,
        ImportKinds::Runtime => kind == ImportKind::Runtime,
        ImportKinds::Types => kind == ImportKind::Type,
    }
}

/// The `rules::search_every_rule` fail point: counts each import as a search of every sentence,
/// for the fault control.
fn search_every_rule() -> bool {
    fail::fail_point!("rules::search_every_rule", |_| true);
    false
}

/// The `rules::area_per_import` fail point: counts each import as a test of the area of every
/// sentence that has one, for the fault control.
fn area_per_import() -> bool {
    fail::fail_point!("rules::area_per_import", |_| true);
    false
}

/// The `rules::every_row` fail point: counts each import as a test of every row of every table, for
/// the fault control.
fn every_row() -> bool {
    fail::fail_point!("rules::every_row", |_| true);
    false
}
