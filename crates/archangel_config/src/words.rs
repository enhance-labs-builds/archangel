//! The words that a rule may use: its fields, its conditions and the values of its options.

use serde_json::{Map, Value};

use crate::model::{DependencyKind, InstanceMatch, SpecifierForm};
use crate::problem::{ConfigProblem, ProblemCode};

pub(crate) const COMMON_FIELDS: [&str; 7] = [
    "name", "reason", "because", "link", "severity", "imports", "examples",
];

pub(crate) const SHORTHANDS: [&str; 3] = ["layers", "isolate", "entry"];

/// The fields of a rule that uses a plugin condition.
pub(crate) const PLUGIN_FIELDS: [&str; 9] = [
    "files", "plugin", "options", "name", "reason", "because", "link", "severity", "examples",
];

/// The conditions about imports, which the import options limit.
pub(crate) const IMPORT_CONDITIONS: [&str; 3] =
    ["mayNotImport", "mayOnlyImport", "mayOnlyBeImportedBy"];

/// The conditions a rule about folders takes.
pub(crate) const FOLDER_CONDITIONS: [&str; 3] = ["mustBeNamed", "mustContain", "mustNotExist"];

/// The fields of a rule written as several sentences, which belong to the rule, not to one of
/// its sentences.
pub(crate) const SENTENCES_FIELDS: [&str; 8] = [
    "name",
    "reason",
    "because",
    "link",
    "severity",
    "imports",
    "examples",
    "sentences",
];

/// Every condition the rule language names, so that a rule with two of them is refused as one
/// mistake, whichever two they are.
pub(crate) const CONDITIONS: [&str; 10] = [
    "mayNotImport",
    "mayOnlyImport",
    "mayOnlyBeImportedBy",
    "mustBeNamed",
    "mustContain",
    "mustNotExist",
    "mustExport",
    "mayNotExport",
    "mustOnlyReExport",
    "noCycles",
];

pub(crate) const SPECIFIERS: [(&str, SpecifierForm); 4] = [
    ("relative", SpecifierForm::Relative),
    ("parent", SpecifierForm::Parent),
    ("alias", SpecifierForm::Alias),
    ("package", SpecifierForm::Package),
];

pub(crate) const INSTANCES: [(&str, InstanceMatch); 2] = [
    ("same", InstanceMatch::Same),
    ("different", InstanceMatch::Different),
];

pub(crate) const DEPENDENCIES: [(&str, DependencyKind); 2] = [
    ("dev", DependencyKind::Dev),
    ("undeclared", DependencyKind::Undeclared),
];

pub(crate) const OPTIONS: [&str; 7] = [
    "names",
    "specifier",
    "instance",
    "dependency",
    "transitively",
    "stopAt",
    "within",
];

/// Whether a rule's `severity` is `'warn'`. The default, `'error'`, fails the check.
pub(crate) fn severity(
    object: &Map<String, Value>,
    field: &str,
    problems: &mut Vec<ConfigProblem>,
) -> bool {
    match object.get("severity") {
        None => false,
        Some(Value::String(value)) if value == "error" => false,
        Some(Value::String(value)) if value == "warn" => true,
        Some(_) => {
            problems.push(invalid(
                &format!("{field}.severity"),
                "must be 'error', which fails the check, or 'warn', which reports the violation \
                 and does not",
            ));
            false
        }
    }
}

fn invalid(field: &str, message: &str) -> ConfigProblem {
    ConfigProblem::new(ProblemCode::ConfigInvalid, message).at(field)
}
