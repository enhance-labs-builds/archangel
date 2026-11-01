//! The config after validation: every name refers to a declared element, every pattern compiles,
//! and every rule has a unique name.

use std::collections::BTreeMap;

use crate::names::{NamePattern, Style};
use crate::pattern::Pattern;
use crate::problem::{ConfigProblem, ProblemCode};

/// The index of an element in [`Config::elements`], in declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ElementId(pub u32);

impl ElementId {
    /// The position of the element in the config.
    #[must_use]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// A validated config.
#[derive(Debug, Clone)]
pub struct Config {
    /// The elements, in declaration order, which is also match order.
    pub elements: Vec<Element>,
    /// The selectors, in declaration order.
    pub selectors: Vec<Selector>,
    /// The selectors in an order where each comes after the selectors it refers to.
    pub selector_order: Vec<SelectorId>,
    /// Every path pattern that a selector or a rule side names, each once; a [`FileSet`] refers
    /// to them by index, so one pass over a path tests them all.
    pub patterns: Vec<Pattern>,
    /// Lines in `.gitignore` syntax that remove files from the source files.
    pub ignore: Vec<String>,
    /// Specifiers that may stay unresolved, each with its reason.
    pub allow_unresolved: Vec<AllowUnresolved>,
    /// The rules, in declaration order.
    pub rules: Vec<Rule>,
    /// The plugins the config loads, by the name its rules use them by, in declaration order.
    pub plugins: Vec<Plugin>,
    /// The presets the config extends, in order.
    pub presets: Vec<Preset>,
}

impl Config {
    /// A problem for each name of an externals set that is a package of the workspace, which only
    /// a check can know, because an import of it resolves to a repository file.
    #[must_use]
    pub fn externals_in_workspace(
        &self,
        is_workspace_package: impl Fn(&str) -> bool,
    ) -> Vec<ConfigProblem> {
        let mut problems = Vec::new();
        for element in self.elements.iter().filter(|element| element.externals) {
            let ElementKind::Packages(patterns) = &element.kind else {
                continue;
            };
            for name in patterns.iter().map(Pattern::source) {
                if is_workspace_package(name) {
                    problems.push(
                        ConfigProblem::new(
                            ProblemCode::ConfigInvalid,
                            format!(
                                "`{name}` is a package of the workspace, so an import of it \
                                 resolves to a repository file and its part judges it; name that \
                                 part in the rule"
                            ),
                        )
                        .at(format!("externals.{}", element.name)),
                    );
                }
            }
        }
        problems
    }
}

/// A preset the config extends, after the merge: its name, its root, and what it declared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preset {
    /// The name its elements, selectors and rules carry as a prefix, such as `fsd`.
    pub name: String,
    /// The folder the preset applies to, relative to the config folder; empty for all of it.
    pub root: String,
    /// The elements it declared.
    pub elements: Vec<ElementId>,
    /// The selectors it declared.
    pub selectors: Vec<SelectorId>,
}

/// A plugin the config loads, as the loader describes it: its conditions, without their code,
/// which stays in the plugin process.
#[derive(Debug, Clone, PartialEq)]
pub struct Plugin {
    /// The name the config gives the plugin, which its rules write before the condition's name.
    pub name: String,
    /// The schema version of the view the plugin was written for.
    pub schema_version: u64,
    /// The conditions, in the plugin's order.
    pub conditions: Vec<PluginCondition>,
}

/// One condition of a plugin.
#[derive(Debug, Clone, PartialEq)]
pub struct PluginCondition {
    /// The condition's name within its plugin.
    pub name: String,
    /// What the condition requires, in one sentence.
    pub description: String,
    /// Where its documentation is.
    pub docs: Option<String>,
    /// The facts of each file the condition reads.
    pub reads: Reads,
}

/// The facts of a file that a plugin condition reads, beyond its path, element, captures and
/// selectors, which every condition sees.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Reads {
    /// The file's imports.
    pub imports: bool,
    /// The paths of the files that import it.
    pub importers: bool,
    /// What it exports.
    pub exports: bool,
}

impl Reads {
    /// The facts either of two conditions reads.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self {
            imports: self.imports || other.imports,
            importers: self.importers || other.importers,
            exports: self.exports || other.exports,
        }
    }
}

/// A rule's use of a plugin condition.
#[derive(Debug, Clone, PartialEq)]
pub struct PluginUse {
    /// The plugin's name in the config.
    pub plugin: String,
    /// The condition's name within the plugin.
    pub condition: String,
    /// The rule's options, valid against the condition's schema.
    pub options: serde_json::Value,
    /// The facts the condition reads.
    pub reads: Reads,
}

impl PluginUse {
    /// The condition as a rule names it, such as `team/one-store-per-entity`.
    #[must_use]
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.plugin, self.condition)
    }
}

/// The index of a selector in [`Config::selectors`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SelectorId(pub u32);

/// The index of a pattern in [`Config::patterns`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PatternId(pub u32);

/// A named set of files, which may overlap with any other.
#[derive(Debug, Clone)]
pub struct Selector {
    /// The name the rules use.
    pub name: String,
    /// Which files it selects.
    pub definition: FileSet,
}

/// A set of files, by path, element, instance, directive or other selectors, as its terms in
/// postfix order: each combination follows the terms it combines, so the set is read with a loop
/// and a stack of answers, however deeply the config nests it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileSet {
    /// The terms, each combination after the terms it combines.
    pub terms: Vec<Term>,
}

/// One term of a [`FileSet`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    /// The files whose path matches the pattern.
    Pattern(PatternId),
    /// The files in the element.
    Element(ElementId),
    /// The files in one instance of the element: each capture with its value.
    Instance {
        /// The element, which has the captures.
        element: ElementId,
        /// The captures and their values.
        captures: Vec<(String, String)>,
    },
    /// The files in any element.
    AnyElement,
    /// The files whose first statement is this directive, such as `use client`.
    Directive(String),
    /// The files of a declared selector.
    Selector(SelectorId),
    /// The files in every one of the last this many sets.
    All(usize),
    /// The files in any one of the last this many sets.
    Any(usize),
    /// The files not in the last set.
    Not,
}

impl FileSet {
    /// The set of one term.
    #[must_use]
    pub fn of(term: Term) -> Self {
        Self { terms: vec![term] }
    }

    /// The element, when the set is exactly one element's files.
    #[must_use]
    pub fn element(&self) -> Option<ElementId> {
        match self.terms.as_slice() {
            [Term::Element(element)] => Some(*element),
            _ => None,
        }
    }

    /// The selectors the set refers to.
    pub fn selectors(&self) -> impl Iterator<Item = SelectorId> + '_ {
        self.terms.iter().filter_map(|term| match term {
            Term::Selector(id) => Some(*id),
            _ => None,
        })
    }

    /// The patterns the set names itself, not through a selector, outside every `not`: those
    /// whose files the set takes in rather than leaves out.
    #[must_use]
    pub fn included_patterns(&self) -> Vec<PatternId> {
        // The index of the first term of each answer on the stack, so that a `not` knows which
        // terms it negates.
        let mut starts: Vec<usize> = Vec::new();
        let mut negated = vec![false; self.terms.len()];
        for (index, term) in self.terms.iter().enumerate() {
            let start = match term {
                Term::All(count) | Term::Any(count) => {
                    let first = starts.len().saturating_sub(*count);
                    let start = starts.get(first).copied().unwrap_or(index);
                    starts.truncate(first);
                    start
                }
                Term::Not => {
                    let start = starts.pop().unwrap_or(index);
                    for flag in negated.iter_mut().take(index).skip(start) {
                        *flag = true;
                    }
                    start
                }
                Term::Pattern(_)
                | Term::Element(_)
                | Term::Instance { .. }
                | Term::AnyElement
                | Term::Directive(_)
                | Term::Selector(_) => index,
            };
            starts.push(start);
        }
        self.terms
            .iter()
            .zip(negated)
            .filter(|(_, negated)| !negated)
            .filter_map(|(term, _)| term.pattern())
            .collect()
    }
}

impl Term {
    /// The pattern, when the term is one.
    #[must_use]
    pub fn pattern(&self) -> Option<PatternId> {
        match self {
            Self::Pattern(id) => Some(*id),
            _ => None,
        }
    }
}

/// What the other side of an import rule names: files, package elements and inline packages.
#[derive(Debug, Clone, Default)]
pub struct Side {
    /// Sets of files.
    pub files: Vec<FileSet>,
    /// Package elements.
    pub elements: Vec<ElementId>,
    /// Inline package names, such as `react`, `@acme/*`, `node:*` or `*`.
    pub packages: Vec<Pattern>,
    /// The side as the config wrote it, for messages and default names.
    pub text: String,
}

/// How an import's specifier is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecifierForm {
    /// Starting with `./` or `../`.
    Relative,
    /// Starting with `../`.
    Parent,
    /// Through a tsconfig path alias or a package `imports` field.
    Alias,
    /// A package name.
    Package,
}

/// How the instances at the two ends of an import compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceMatch {
    /// One instance.
    Same,
    /// Two instances.
    Different,
}

/// How the nearest package manifest of the importing file lists the imported package.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyKind {
    /// Only as a development dependency.
    Dev,
    /// Not at all.
    Undeclared,
}

/// The options that limit an import condition to some imports.
#[derive(Debug, Clone, Default)]
pub struct ImportOptions {
    /// Only imports that bring in one of these names.
    pub names: Option<Vec<String>>,
    /// Only imports whose specifier is written this way.
    pub specifier: Option<SpecifierForm>,
    /// Only imports whose ends compare this way, in the element that `files` names.
    pub instance: Option<InstanceMatch>,
    /// Only imports of packages that the nearest manifest lists this way.
    pub dependency: Option<DependencyKind>,
    /// A chain of imports of any length, not only a direct import.
    pub transitively: bool,
    /// The files where a chain ends without following their imports.
    pub stop_at: Option<FileSet>,
    /// The area: only imports whose other end is in it, for `mayOnlyImport` the target and for
    /// `mayOnlyBeImportedBy` the importer.
    pub within: Option<Side>,
}

impl ImportOptions {
    /// Whether the rule needs the import's own facts, beyond the classes of its two ends.
    #[must_use]
    pub fn needs_import(&self) -> bool {
        self.names.is_some()
            || self.specifier.is_some()
            || self.instance.is_some()
            || self.dependency.is_some()
    }
}

/// What a sentence requires of its files.
#[derive(Debug, Clone)]
pub enum Condition {
    /// No import from these files into the side.
    MayNotImport(Side),
    /// No import from these files into anything that is neither the side nor these files.
    MayOnlyImport(Side),
    /// No import into these files from anything that is neither the side nor these files.
    MayOnlyBeImportedBy(Side),
    /// No file in the selector.
    MustNotExist,
    /// Each file's or folder's name matches.
    MustBeNamed(NameRule),
    /// Each instance of the element, or each folder, has a file that matches one of the
    /// patterns, relative to the instance root or the folder.
    MustContain(Vec<Pattern>),
    /// Each file exports a name that matches.
    MustExport(NamePattern),
    /// No file has an export of this kind.
    MayNotExport(ExportKind),
    /// Each file has no top-level statement of its own.
    MustOnlyReExport,
    /// No group of these files that all import each other.
    NoCycles,
    /// A plugin's condition, which the plugin process evaluates over the view.
    Plugin(PluginUse),
    /// Imports into an instance of the element from outside it go through its entry files: one
    /// sentence of `mayOnlyBeImportedBy` per instance, which no single sentence can say.
    Entry {
        /// The element.
        element: ElementId,
        /// Entry file patterns, relative to the instance root.
        files: Vec<Pattern>,
    },
}

impl Condition {
    /// The condition's name, as the config writes it.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::MayNotImport(_) => "mayNotImport",
            Self::MayOnlyImport(_) => "mayOnlyImport",
            Self::MayOnlyBeImportedBy(_) => "mayOnlyBeImportedBy",
            Self::MustNotExist => "mustNotExist",
            Self::MustBeNamed(_) => "mustBeNamed",
            Self::MustContain(_) => "mustContain",
            Self::MustExport(_) => "mustExport",
            Self::MayNotExport(_) => "mayNotExport",
            Self::MustOnlyReExport => "mustOnlyReExport",
            Self::NoCycles => "noCycles",
            Self::Entry { .. } => "entry",
            Self::Plugin(_) => "plugin",
        }
    }

    /// The other side, for an import condition.
    #[must_use]
    pub fn side(&self) -> Option<&Side> {
        match self {
            Self::MayNotImport(side)
            | Self::MayOnlyImport(side)
            | Self::MayOnlyBeImportedBy(side) => Some(side),
            Self::MustNotExist
            | Self::NoCycles
            | Self::Entry { .. }
            | Self::Plugin(_)
            | Self::MustBeNamed(_)
            | Self::MustContain(_)
            | Self::MustExport(_)
            | Self::MayNotExport(_)
            | Self::MustOnlyReExport => None,
        }
    }

    /// Whether the condition is about what a file exports.
    #[must_use]
    pub fn is_export(&self) -> bool {
        matches!(
            self,
            Self::MustExport(_) | Self::MayNotExport(_) | Self::MustOnlyReExport
        )
    }

    /// Whether the condition is about where files are and what they and their folders are
    /// called: naming, `mustContain` and `mustNotExist`.
    #[must_use]
    pub fn is_structure(&self) -> bool {
        matches!(
            self,
            Self::MustBeNamed(_) | Self::MustContain(_) | Self::MustNotExist
        )
    }
}

/// What a name must be: in a style, or matching a pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameRule {
    /// The name, up to the first dot, is in the style.
    Style(Style),
    /// The full name, without the last extension, matches the pattern.
    Pattern(NamePattern),
}

impl NameRule {
    /// The rule as the config wrote it.
    #[must_use]
    pub fn source(&self) -> &str {
        match self {
            Self::Style(style) => style.as_str(),
            Self::Pattern(pattern) => pattern.source(),
        }
    }
}

/// A kind of export that a `mayNotExport` rule refuses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportKind {
    /// A default export, `export { default } from` included.
    Default,
    /// An `export *` or `export * as`.
    Star,
    /// A named export whose name matches.
    Names(NamePattern),
}

impl ExportKind {
    /// The kind as the config wrote it.
    #[must_use]
    pub fn source(&self) -> &str {
        match self {
            Self::Default => "default",
            Self::Star => "star",
            Self::Names(pattern) => pattern.source(),
        }
    }
}

/// One selector and one condition: the form every rule takes once it is validated.
#[derive(Debug, Clone)]
pub struct Sentence {
    /// The files the condition is about; empty for a rule about folders.
    pub files: FileSet,
    /// The folders a rule about folders is about.
    pub folders: Option<Folders>,
    /// The files as the config wrote them, for messages and default names.
    pub files_text: String,
    /// What the files must do.
    pub condition: Condition,
    /// The options that limit an import condition.
    pub options: ImportOptions,
}

/// The folders a rule about folders is about: those that one of `patterns` matches and none of
/// `except` does.
#[derive(Debug, Clone)]
pub struct Folders {
    /// The folders the rule covers.
    pub patterns: Vec<Pattern>,
    /// The folders it leaves out, written with a leading `!`.
    pub except: Vec<Pattern>,
}

impl Folders {
    /// Whether the rule covers the folder.
    #[must_use]
    pub fn covers(&self, folder: &str) -> bool {
        self.patterns.iter().any(|p| p.matches(folder).is_some())
            && !self.except.iter().any(|p| p.matches(folder).is_some())
    }
}

/// A named kind of file or package.
#[derive(Debug, Clone)]
pub struct Element {
    /// The name the rules use.
    pub name: String,
    /// What the element matches.
    pub kind: ElementKind,
    /// The set of elements it belongs to: a file is in one element of each set. Set 0 is the
    /// config's own; with `parts`, each preset has a set of its own.
    pub set: u32,
    /// Whether the config declares it under `parts`, so that a second match is `part-overlap`.
    pub part: bool,
    /// Whether the config declares it under `externals`, so that a package is in every such set
    /// that lists it.
    pub externals: bool,
}

/// What an element matches.
#[derive(Debug, Clone)]
pub enum ElementKind {
    /// Repository files, by path patterns.
    Files(Vec<Pattern>),
    /// Packages, by name patterns such as `react`, `@acme/*` or `node:*`.
    Packages(Vec<Pattern>),
}

impl Element {
    /// Whether any of the element's patterns has a capture.
    #[must_use]
    pub fn has_capture(&self) -> bool {
        match &self.kind {
            ElementKind::Files(patterns) => patterns.iter().any(|p| !p.captures().is_empty()),
            ElementKind::Packages(_) => false,
        }
    }
}

/// A specifier that may stay unresolved.
#[derive(Debug, Clone)]
pub struct AllowUnresolved {
    /// The specifier, which may end in `*`.
    pub specifier: String,
    /// Why it cannot be resolved.
    pub reason: String,
}

impl AllowUnresolved {
    /// Whether this entry permits the specifier.
    #[must_use]
    pub fn permits(&self, specifier: &str) -> bool {
        self.specifier
            .strip_suffix('*')
            .map_or(self.specifier == specifier, |prefix| {
                specifier.starts_with(prefix)
            })
    }
}

/// Which imports a rule checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportKinds {
    /// Runtime imports and type imports.
    All,
    /// Imports that stay in the compiled JavaScript.
    Runtime,
    /// Type imports only.
    Types,
}

/// One rule, validated.
#[derive(Debug, Clone)]
pub struct Rule {
    /// The rule's unique name, given or made from its kind or condition and its sides.
    pub name: String,
    /// The shorthand the rule was written as, or a sentence.
    pub kind: RuleKind,
    /// What the rule means, as sentences: one for a sentence, one or more for a shorthand.
    pub sentences: Vec<Sentence>,
    /// Why the team has the rule.
    pub reason: Option<String>,
    /// Where to read more.
    pub link: Option<String>,
    /// Which imports the rule checks.
    pub imports: ImportKinds,
    /// Files the rule must accept and must reject.
    pub examples: Examples,
}

impl Rule {
    /// The plugin condition the rule uses, if it is a plugin rule.
    #[must_use]
    pub fn plugin_use(&self) -> Option<&PluginUse> {
        self.sentences
            .iter()
            .find_map(|sentence| match &sentence.condition {
                Condition::Plugin(found) => Some(found),
                _ => None,
            })
    }
}

/// How a rule was written: as one of the three shorthands, or as a sentence.
#[derive(Debug, Clone)]
pub enum RuleKind {
    /// Each listed element may import only itself and later listed elements.
    Layers {
        /// The elements, first to last.
        order: Vec<ElementId>,
    },
    /// Instances of one element may not import each other.
    Isolate {
        /// The element, which has a capture.
        element: ElementId,
    },
    /// Code outside an instance imports it only through its entry files.
    Entry {
        /// The element.
        element: ElementId,
        /// Entry file patterns, relative to the instance root.
        files: Vec<Pattern>,
    },
    /// One selector and one condition, written with `rule()`.
    Sentence,
}

/// A set of example files: path relative to the config folder, to text.
pub type ExampleFiles = BTreeMap<String, String>;

/// A rule's examples.
#[derive(Debug, Clone, Default)]
pub struct Examples {
    /// File sets that the rule must accept.
    pub accepted: Vec<ExampleFiles>,
    /// File sets that the rule must reject.
    pub rejected: Vec<ExampleFiles>,
}

impl Config {
    /// The plugin with this name.
    #[must_use]
    pub fn plugin(&self, name: &str) -> Option<&Plugin> {
        self.plugins.iter().find(|plugin| plugin.name == name)
    }

    /// Whether a rule uses a plugin condition, so the plugin process must stay.
    #[must_use]
    pub fn uses_plugins(&self) -> bool {
        self.rules.iter().any(|rule| rule.plugin_use().is_some())
    }

    /// The selector with this name.
    #[must_use]
    pub fn selector(&self, name: &str) -> Option<SelectorId> {
        index_of(self.selectors.iter().map(|s| s.name.as_str()), name).map(SelectorId)
    }

    /// The element with this name.
    #[must_use]
    pub fn element(&self, name: &str) -> Option<ElementId> {
        index_of(self.elements.iter().map(|e| e.name.as_str()), name).map(ElementId)
    }
}

/// The position of a name among names, as an id.
fn index_of<'n>(mut names: impl Iterator<Item = &'n str>, name: &str) -> Option<u32> {
    names
        .position(|candidate| candidate == name)
        .and_then(|index| u32::try_from(index).ok())
}
