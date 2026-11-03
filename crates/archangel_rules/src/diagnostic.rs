//! Diagnostics as data: every result archangel gives, before any output format.

use serde::{Deserialize, Serialize};

/// The kind of a diagnostic, from a fixed list. Each code has a page in this crate's `docs/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Code {
    /// A layers rule.
    Layers,
    /// An isolate rule.
    Isolate,
    /// An entry rule.
    Entry,
    /// A `mayNotImport` condition.
    MayNotImport,
    /// A `mayOnlyImport` condition.
    MayOnlyImport,
    /// A `mayOnlyBeImportedBy` condition.
    MayOnlyBeImportedBy,
    /// A row of an import table.
    MayImport,
    /// A file in the files of a table has no row.
    EveryFileHasAKind,
    /// A `mustNotExist` condition.
    MustNotExist,
    /// A `mustBeNamed` condition.
    MustBeNamed,
    /// A `mustContain` condition.
    MustContain,
    /// A `mustExport` condition.
    MustExport,
    /// A `mayNotExport` condition.
    MayNotExport,
    /// A `mustOnlyReExport` condition.
    MustOnlyReExport,
    /// A `noCycles` condition.
    NoCycles,
    /// A plugin's condition.
    Plugin,
    /// No config was found.
    ConfigMissing,
    /// A TypeScript config with no usable Node.
    ConfigNeedsNode,
    /// The config's code failed to load.
    ConfigLoadFailed,
    /// The config has fields or values archangel does not know.
    ConfigInvalid,
    /// A pattern that cannot mean what it says.
    PatternInvalid,
    /// A rule names an element the config does not declare.
    RuleUnknownElement,
    /// An isolate rule on an element without a capture.
    RuleNeedsCapture,
    /// Two rules share a name.
    RuleNameDuplicate,
    /// An element that matches nothing.
    ElementEmpty,
    /// A selector, or an inline pattern of a rule, that matches no file.
    SelectorEmpty,
    /// A preset whose elements together match no file.
    PresetEmpty,
    /// A file in two of the config's own parts.
    PartOverlap,
    /// A file in two rows of one table.
    TableRowOverlap,
    /// A table with no row, or whose rows together have no file.
    TableEmpty,
    /// A file whose exports an export rule needs, which an `export *` chain hides.
    ExportsUnknown,
    /// A file or folder that cannot be opened, or a file that is not text.
    FileUnreadable,
    /// A source file with syntax errors.
    ParseFailed,
    /// An import with no resolution.
    ImportUnresolved,
    /// A tsconfig or package manifest that resolution cannot use.
    ResolutionFileInvalid,
    /// A line of the config's ignore list that is not valid.
    IgnoreLineInvalid,
    /// An exception without a reason.
    ExceptionWithoutReason,
    /// An exception for a rule the config does not have.
    ExceptionUnknownRule,
    /// An exception that permits nothing.
    ExceptionUnused,
    /// A baseline file that does not parse.
    BaselineInvalid,
    /// A baseline entry that matches fewer violations than its count.
    BaselineStale,
    /// A rule with no rejected example.
    RuleUnproven,
    /// An accepted example that its rule never checked.
    ExampleProvesNothing,
    /// An example that its rule did not judge as the example says.
    ExampleFailed,
    /// A plugin that threw, reported a file outside its view, or whose process stopped.
    PluginFailed,
}

/// The names of the codes, in declaration order, so a code's discriminant indexes its name.
const NAMES: [&str; 45] = [
    "layers",
    "isolate",
    "entry",
    "may-not-import",
    "may-only-import",
    "may-only-be-imported-by",
    "may-import",
    "every-file-has-a-kind",
    "must-not-exist",
    "must-be-named",
    "must-contain",
    "must-export",
    "may-not-export",
    "must-only-re-export",
    "no-cycles",
    "plugin",
    "config-missing",
    "config-needs-node",
    "config-load-failed",
    "config-invalid",
    "pattern-invalid",
    "rule-unknown-element",
    "rule-needs-capture",
    "rule-name-duplicate",
    "element-empty",
    "selector-empty",
    "preset-empty",
    "part-overlap",
    "table-row-overlap",
    "table-empty",
    "exports-unknown",
    "file-unreadable",
    "parse-failed",
    "import-unresolved",
    "resolution-file-invalid",
    "ignore-line-invalid",
    "exception-without-reason",
    "exception-unknown-rule",
    "exception-unused",
    "baseline-invalid",
    "baseline-stale",
    "rule-unproven",
    "example-proves-nothing",
    "example-failed",
    "plugin-failed",
];

/// Each code's documentation page, in the order of [`NAMES`].
const PAGES: [&str; 45] = [
    include_str!("../docs/layers.md"),
    include_str!("../docs/isolate.md"),
    include_str!("../docs/entry.md"),
    include_str!("../docs/may-not-import.md"),
    include_str!("../docs/may-only-import.md"),
    include_str!("../docs/may-only-be-imported-by.md"),
    include_str!("../docs/may-import.md"),
    include_str!("../docs/every-file-has-a-kind.md"),
    include_str!("../docs/must-not-exist.md"),
    include_str!("../docs/must-be-named.md"),
    include_str!("../docs/must-contain.md"),
    include_str!("../docs/must-export.md"),
    include_str!("../docs/may-not-export.md"),
    include_str!("../docs/must-only-re-export.md"),
    include_str!("../docs/no-cycles.md"),
    include_str!("../docs/plugin.md"),
    include_str!("../docs/config-missing.md"),
    include_str!("../docs/config-needs-node.md"),
    include_str!("../docs/config-load-failed.md"),
    include_str!("../docs/config-invalid.md"),
    include_str!("../docs/pattern-invalid.md"),
    include_str!("../docs/rule-unknown-element.md"),
    include_str!("../docs/rule-needs-capture.md"),
    include_str!("../docs/rule-name-duplicate.md"),
    include_str!("../docs/element-empty.md"),
    include_str!("../docs/selector-empty.md"),
    include_str!("../docs/preset-empty.md"),
    include_str!("../docs/part-overlap.md"),
    include_str!("../docs/table-row-overlap.md"),
    include_str!("../docs/table-empty.md"),
    include_str!("../docs/exports-unknown.md"),
    include_str!("../docs/file-unreadable.md"),
    include_str!("../docs/parse-failed.md"),
    include_str!("../docs/import-unresolved.md"),
    include_str!("../docs/resolution-file-invalid.md"),
    include_str!("../docs/ignore-line-invalid.md"),
    include_str!("../docs/exception-without-reason.md"),
    include_str!("../docs/exception-unknown-rule.md"),
    include_str!("../docs/exception-unused.md"),
    include_str!("../docs/baseline-invalid.md"),
    include_str!("../docs/baseline-stale.md"),
    include_str!("../docs/rule-unproven.md"),
    include_str!("../docs/example-proves-nothing.md"),
    include_str!("../docs/example-failed.md"),
    include_str!("../docs/plugin-failed.md"),
];

impl Code {
    /// Every code, in the order the documentation lists them.
    pub const ALL: [Self; 45] = [
        Self::Layers,
        Self::Isolate,
        Self::Entry,
        Self::MayNotImport,
        Self::MayOnlyImport,
        Self::MayOnlyBeImportedBy,
        Self::MayImport,
        Self::EveryFileHasAKind,
        Self::MustNotExist,
        Self::MustBeNamed,
        Self::MustContain,
        Self::MustExport,
        Self::MayNotExport,
        Self::MustOnlyReExport,
        Self::NoCycles,
        Self::Plugin,
        Self::ConfigMissing,
        Self::ConfigNeedsNode,
        Self::ConfigLoadFailed,
        Self::ConfigInvalid,
        Self::PatternInvalid,
        Self::RuleUnknownElement,
        Self::RuleNeedsCapture,
        Self::RuleNameDuplicate,
        Self::ElementEmpty,
        Self::SelectorEmpty,
        Self::PresetEmpty,
        Self::PartOverlap,
        Self::TableRowOverlap,
        Self::TableEmpty,
        Self::ExportsUnknown,
        Self::FileUnreadable,
        Self::ParseFailed,
        Self::ImportUnresolved,
        Self::ResolutionFileInvalid,
        Self::IgnoreLineInvalid,
        Self::ExceptionWithoutReason,
        Self::ExceptionUnknownRule,
        Self::ExceptionUnused,
        Self::BaselineInvalid,
        Self::BaselineStale,
        Self::RuleUnproven,
        Self::ExampleProvesNothing,
        Self::ExampleFailed,
        Self::PluginFailed,
    ];

    /// The code as the developer sees it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        NAMES[self as usize]
    }

    /// The code's documentation page, in Markdown: what it means, an example, and what to do.
    #[must_use]
    pub fn docs(self) -> &'static str {
        PAGES[self as usize]
    }

    /// The page's first paragraph: what the code means, in one sentence.
    #[must_use]
    pub fn summary(self) -> &'static str {
        self.docs().split("\n\n").nth(1).map_or("", str::trim)
    }

    /// The code with this spelling.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        NAMES
            .iter()
            .position(|n| *n == name)
            .map(|index| Self::ALL[index])
    }

    /// Whether the code is a violation of a rule: of a shorthand kind or of a condition.
    #[must_use]
    pub fn is_rule(self) -> bool {
        matches!(
            self,
            Self::Layers
                | Self::Isolate
                | Self::Entry
                | Self::MayNotImport
                | Self::MayOnlyImport
                | Self::MayOnlyBeImportedBy
                | Self::MayImport
                | Self::EveryFileHasAKind
                | Self::MustNotExist
                | Self::MustBeNamed
                | Self::MustContain
                | Self::MustExport
                | Self::MayNotExport
                | Self::MustOnlyReExport
                | Self::NoCycles
                | Self::Plugin
        )
    }

    /// Whether the code is a violation about a file, a folder or an instance rather than an
    /// import, which only the baseline can permit.
    #[must_use]
    pub fn is_about_a_path(self) -> bool {
        matches!(
            self,
            Self::MustNotExist
                | Self::EveryFileHasAKind
                | Self::MustBeNamed
                | Self::MustContain
                | Self::MustExport
                | Self::MayNotExport
                | Self::MustOnlyReExport
        )
    }
}

/// Which violation this is, in a form that survives an edit that moves lines.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ViolationId {
    /// One import that breaks a rule.
    Import {
        /// The rule's name.
        rule: String,
        /// The importing file.
        file: String,
        /// The import's specifier.
        specifier: String,
        /// Which import of that specifier in the file, from 0.
        occurrence: u32,
    },
    /// One group of files that import each other in a cycle.
    Group {
        /// The rule's name.
        rule: String,
        /// Every file in the group, sorted.
        files: Vec<String>,
    },
    /// One file, folder or instance that breaks a rule about it, by its path, so that a change
    /// to what a folder holds does not change the identity. An export rule's violation at an
    /// export is the file's too.
    Path {
        /// The rule's name.
        rule: String,
        /// The file's or folder's path, or the instance's root.
        path: String,
    },
}

impl ViolationId {
    /// The rule this violation breaks.
    #[must_use]
    pub fn rule(&self) -> &str {
        match self {
            Self::Import { rule, .. } | Self::Group { rule, .. } | Self::Path { rule, .. } => rule,
        }
    }
}

/// One result.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct Diagnostic {
    /// The kind.
    pub code: Code,
    /// The rule a violation breaks.
    pub rule: Option<String>,
    /// The file, relative to the config folder.
    pub file: Option<String>,
    /// The line, from 1, or 0 when the diagnostic is about a whole file or the config.
    pub line: u32,
    /// The column, from 1, or 0.
    pub column: u32,
    /// The byte offsets of the specifier, when the diagnostic is about an import.
    pub span: Option<(u32, u32)>,
    /// The start and end of the specifier on its line, from 0, in UTF-16 code units, when the
    /// diagnostic is about an import.
    pub utf16: Option<(u32, u32)>,
    /// The specifier, when the diagnostic is about an import.
    pub specifier: Option<String>,
    /// What is wrong.
    pub message: String,
    /// What the code may do instead.
    pub help: Option<String>,
    /// The rule's reason, from the config.
    pub reason: Option<String>,
    /// Whether the rule wrote its reason with `because`, which the text output then says.
    pub because: bool,
    /// Where to read more, from the config.
    pub link: Option<String>,
    /// The violation's identity.
    pub id: Option<ViolationId>,
    /// The files of a cycle, first to last and back to the first.
    pub cycle: Vec<String>,
    /// Every file in a group of files that import each other.
    pub group: Vec<String>,
    /// The files of a transitive violation's chain, from the importing file to what it reaches.
    pub chain: Vec<String>,
    /// An edit that removes the violation, checked before it is offered.
    pub fix: Option<Fix>,
}

/// A checked edit to one specifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct Fix {
    /// The byte offsets of the text between the specifier's quotes.
    pub span: (u32, u32),
    /// The new specifier, without quotes.
    pub specifier: String,
}

impl Diagnostic {
    /// A diagnostic with a code and a message, and nothing else yet.
    #[must_use]
    pub fn new(code: Code, message: impl Into<String>) -> Self {
        Self {
            code,
            rule: None,
            file: None,
            line: 0,
            column: 0,
            span: None,
            utf16: None,
            specifier: None,
            message: message.into(),
            help: None,
            reason: None,
            because: false,
            link: None,
            id: None,
            cycle: Vec::new(),
            group: Vec::new(),
            chain: Vec::new(),
            fix: None,
        }
    }

    /// The same diagnostic, at a file and position.
    #[must_use]
    pub fn at(mut self, file: &str, line: u32, column: u32) -> Self {
        self.file = Some(file.to_owned());
        self.line = line;
        self.column = column;
        self
    }

    /// Whether the diagnostic is a violation about a file, a folder or an instance rather than an
    /// import, which only the baseline can permit: a plugin's violation is one when it names no
    /// import.
    #[must_use]
    pub fn is_about_a_path(&self) -> bool {
        self.code.is_about_a_path() || (self.code == Code::Plugin && self.specifier.is_none())
    }

    /// The key that sorts diagnostics by file, line, column and rule.
    #[must_use]
    pub fn sort_key(&self) -> (&str, u32, u32, &str) {
        (
            self.file.as_deref().unwrap_or_default(),
            self.line,
            self.column,
            self.rule.as_deref().unwrap_or(self.code.as_str()),
        )
    }
}
