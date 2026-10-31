//! The config compiled for checking. Every rule is a list of sentences; every file and package
//! gets a class, the files in one element and the same sets of files, and an import is checked
//! by the pair of classes at its ends, so it costs the same for five rules or five hundred.

use archangel_config::{
    Classification, Condition, Config, ElementId, ElementMatcher, FileSet, ImportKinds, Pattern,
    RuleKind, Sentence, Term, package_matches,
};
use archangel_graph::{Import, ImportGraph, ImportKind, Resolution, Work};

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
    pub(crate) side_files: Vec<usize>,
    pub(crate) side_elements: Vec<ElementId>,
    pub(crate) side_packages: Vec<usize>,
    pub(crate) stop_at: Option<usize>,
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
    pub(crate) graph: Option<&'a ImportGraph>,
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
                architecture.report_inline(&rule.name, sentence);
            }
        }
        architecture
    }

    fn sentence(&mut self, rule: usize, index: usize, sentence: &Sentence) -> Compiled {
        let side = sentence.condition.side().cloned().unwrap_or_default();
        Compiled {
            rule,
            sentence: index,
            files: self.set(&sentence.files),
            side_files: side.files.iter().map(|s| self.set(s)).collect(),
            side_elements: side.elements.clone(),
            side_packages: side.packages.iter().map(|p| self.package_name(p)).collect(),
            stop_at: sentence.options.stop_at.as_ref().map(|s| self.set(s)),
        }
    }

    /// Each pattern a sentence writes inline must match a file, except in the files of a
    /// `mustNotExist` rule, which pass by matching nothing.
    fn report_inline(&mut self, rule: &str, sentence: &Sentence) {
        let side = sentence.condition.side().cloned().unwrap_or_default();
        let own =
            (!matches!(sentence.condition, Condition::MustNotExist)).then_some(&sentence.files);
        let sets = own
            .into_iter()
            .chain(&side.files)
            .chain(sentence.options.stop_at.as_ref());
        let patterns: Vec<_> = sets.flat_map(FileSet::patterns).collect();
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
            .filter(|c| set(c.files) || c.side_files.iter().any(|&s| set(s)))
            .map(|c| (c.rule, c.sentence))
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
            graph,
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
        let cell = classes.cell(self, edge.from.class, edge.to.class);
        let evaluations = if search_every_rule() {
            self.sentences.len()
        } else {
            cell.len()
        } as u64;
        let broken = cell
            .iter()
            .filter(|check| {
                let rule = &self.config.rules[self.sentences[check.sentence].rule];
                covers(rule.imports, edge.import.kind)
            })
            .filter(|check| self.test(check, edge))
            .map(|check| check.sentence)
            .collect();
        (broken, evaluations)
    }

    /// The checks of the sentences that can apply to an import between two classes.
    pub(crate) fn fill(&self, from: &ClassKey, to: &ClassKey) -> Vec<Check> {
        let ClassKey::File { bits, .. } = from else {
            return Vec::new();
        };
        self.sentences
            .iter()
            .enumerate()
            .filter_map(|(index, compiled)| {
                let sentence = &self.config.rules[compiled.rule].sentences[compiled.sentence];
                let applies = !sentence.options.transitively
                    && Self::applies(compiled, &sentence.condition, bits, to);
                applies.then(|| Check {
                    sentence: index,
                    test: if matches!(sentence.condition, Condition::Entry { .. }) {
                        Test::Entry
                    } else if sentence.options.needs_import() {
                        Test::Facts
                    } else {
                        Test::Always
                    },
                })
            })
            .collect()
    }

    /// Whether a sentence's condition can refuse an import from a file with these bits into `to`,
    /// before the import's own facts.
    fn applies(compiled: &Compiled, condition: &Condition, from: &[bool], to: &ClassKey) -> bool {
        let from_in = from[compiled.files];
        let to_in_files = matches!(to, ClassKey::File { bits, .. } if bits[compiled.files]);
        match condition {
            Condition::MayNotImport(_) => from_in && Self::in_side_class(compiled, to),
            Condition::MayOnlyImport(_) => {
                from_in && !Self::in_side_class(compiled, to) && !to_in_files
            }
            Condition::MayOnlyBeImportedBy(_) => {
                let from_in_side = compiled.side_files.iter().any(|&s| from[s]);
                to_in_files && !from_in_side && !from_in
            }
            Condition::Entry { element, .. } => {
                matches!(to, ClassKey::File { element: Some(e), .. } if e == element)
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
        match to {
            ClassKey::File { bits, .. } => compiled.side_files.iter().any(|&s| bits[s]),
            ClassKey::Package { element, names } => {
                element.is_some_and(|e| compiled.side_elements.contains(&e))
                    || compiled.side_packages.iter().any(|&n| names[n])
            }
        }
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
        let element = |key: &ClassKey| match key {
            ClassKey::File { element, .. } | ClassKey::Package { element, .. } => *element,
        };
        match &self.config.rules[rule].kind {
            RuleKind::Layers { order } => {
                element(from).is_some_and(|f| order.contains(&f))
                    && element(to).is_some_and(|t| order.contains(&t))
            }
            RuleKind::Isolate { element: e } => {
                element(from) == Some(*e) && element(to) == Some(*e)
            }
            RuleKind::Entry { element: e, .. } => element(to) == Some(*e),
            RuleKind::Sentence => self
                .sentences
                .iter()
                .filter(|compiled| compiled.rule == rule)
                .any(|compiled| {
                    let sentence = &self.config.rules[rule].sentences[compiled.sentence];
                    let in_files = |key: &ClassKey| {
                        matches!(key, ClassKey::File { bits, .. } if bits[compiled.files])
                    };
                    match sentence.condition {
                        Condition::MayNotImport(_) | Condition::MayOnlyImport(_) => in_files(from),
                        Condition::MayOnlyBeImportedBy(_) | Condition::Entry { .. } => in_files(to),
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

    fn test(&self, check: &Check, edge: &Edge<'_>) -> bool {
        let compiled = &self.sentences[check.sentence];
        let sentence = &self.config.rules[compiled.rule].sentences[compiled.sentence];
        match check.test {
            Test::Always => true,
            Test::Entry => {
                edge.from.classification != edge.to.classification && !is_entry(sentence, &edge.to)
            }
            Test::Facts => crate::facts::hold(sentence, edge),
        }
    }
}

/// Whether the target of an import is one of an entry sentence's entry files.
fn is_entry(sentence: &Sentence, to: &End<'_>) -> bool {
    let (Condition::Entry { files, .. }, Some(class), Some(path)) =
        (&sentence.condition, to.classification, to.path)
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
