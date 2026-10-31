//! What can be wrong with a config, each with where it is and what to do.

use std::fmt;

/// The kind of problem, which is also the diagnostic code the developer sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProblemCode {
    /// No config in the folder or any parent folder.
    ConfigMissing,
    /// A TypeScript or JavaScript config with no usable Node.
    ConfigNeedsNode,
    /// The config code threw, has no default export, or uses TypeScript that Node cannot strip.
    ConfigLoadFailed,
    /// A field archangel does not know, a value of the wrong type, or text that does not parse.
    ConfigInvalid,
    /// A pattern that cannot mean what it says.
    PatternInvalid,
    /// A rule that names an element the config does not declare.
    RuleUnknownElement,
    /// An isolate rule on an element without a capture.
    RuleNeedsCapture,
    /// Two rules with one name.
    RuleNameDuplicate,
}

impl ProblemCode {
    /// The code as the developer sees it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConfigMissing => "config-missing",
            Self::ConfigNeedsNode => "config-needs-node",
            Self::ConfigLoadFailed => "config-load-failed",
            Self::ConfigInvalid => "config-invalid",
            Self::PatternInvalid => "pattern-invalid",
            Self::RuleUnknownElement => "rule-unknown-element",
            Self::RuleNeedsCapture => "rule-needs-capture",
            Self::RuleNameDuplicate => "rule-name-duplicate",
        }
    }
}

/// One problem with the config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigProblem {
    /// The diagnostic code.
    pub code: ProblemCode,
    /// What is wrong and what to do, in plain words.
    pub message: String,
    /// The field, such as `rules[0].reasn`, when the problem has one.
    pub field: Option<String>,
    /// The line and column in the config file, when the problem has them.
    pub position: Option<(usize, usize)>,
}

impl ConfigProblem {
    pub(crate) fn new(code: ProblemCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            field: None,
            position: None,
        }
    }

    pub(crate) fn at(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }
}

impl fmt::Display for ConfigProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.code.as_str())?;
        if let Some(field) = &self.field {
            write!(f, "{field}: ")?;
        }
        f.write_str(&self.message)
    }
}
