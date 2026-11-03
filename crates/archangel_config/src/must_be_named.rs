//! The value of a `mustBeNamed` condition: one style or name pattern, or a list of them.

use serde_json::Value;

use crate::model::{Condition, NameRule};
use crate::names::{NamePattern, Owner, Style};
use crate::problem::{ConfigProblem, ProblemCode};

/// The condition that a value of `mustBeNamed` at this field reads into, or nothing and the problems.
pub(crate) fn must_be_named(
    value: Option<&Value>,
    field: &str,
    folder_rule: bool,
    problems: &mut Vec<ConfigProblem>,
) -> Option<Condition> {
    let before = problems.len();
    let mut rules = Vec::new();
    match value {
        Some(Value::String(text)) => rules.extend(name_rule(text, field, folder_rule, problems)),
        Some(Value::Array(entries)) if entries.is_empty() => problems.push(invalid(
            field,
            "lists no names; list at least one style or name pattern, such as `kebab-case` or \
             `*.view`",
        )),
        Some(Value::Array(entries)) => {
            for (index, entry) in entries.iter().enumerate() {
                let at = format!("{field}[{index}]");
                match entry.as_str() {
                    Some(text) => rules.extend(name_rule(text, &at, folder_rule, problems)),
                    None => problems.push(invalid(
                        &at,
                        "must be a style or a name pattern, such as `kebab-case` or `*.view`",
                    )),
                }
            }
        }
        _ => problems.push(invalid(
            field,
            "must be a style, a name pattern or a list of them, such as `kebab-case` or \
             [`*.view`, `*.styles`]",
        )),
    }
    (problems.len() == before).then_some(Condition::MustBeNamed(rules))
}

/// One style or name pattern at this field. A rule about folders cannot use the file's name.
fn name_rule(
    text: &str,
    field: &str,
    folder_rule: bool,
    problems: &mut Vec<ConfigProblem>,
) -> Option<NameRule> {
    if let Some(style) = Style::from_name(text) {
        return Some(NameRule::Style(style));
    }
    let pattern = NamePattern::parse(text)
        .map_err(|message| {
            problems.push(ConfigProblem::new(ProblemCode::PatternInvalid, message).at(field));
        })
        .ok()?;
    if folder_rule && pattern.uses(Owner::File) {
        problems.push(invalid(
            field,
            "a folder has no file name; a rule about folders can use the spellings of \
             `{FolderName}`, the name of the folder that holds it",
        ));
        return None;
    }
    Some(NameRule::Pattern(pattern))
}

fn invalid(field: &str, message: &str) -> ConfigProblem {
    ConfigProblem::new(ProblemCode::ConfigInvalid, message).at(field)
}
