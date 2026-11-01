//! One-line examples: `'src/a.ts imports src/b.ts'` becomes the file set it stands for, and is
//! refused on a rule whose verdict depends on what a file says beyond its imports.

use std::fmt::Write as _;

use crate::model::{Condition, Config, ExampleFiles, FileSet, ImportKinds, Rule, RuleKind, Term};
use crate::problem::{ConfigProblem, ProblemCode};

const FORM: &str = "write `<file> imports <file or package>`, with more `imports <target>` for a \
                    chain, or one file path";

/// The extensions of a source file, which tell a file from a package in a one-line example.
const SOURCE_EXTENSIONS: [&str; 8] = [".ts", ".tsx", ".mts", ".cts", ".js", ".jsx", ".mjs", ".cjs"];

/// Whether a path is a path inside the repository, relative to the config folder.
pub(crate) fn path_is_inside(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.split('/').any(|part| part == ".." || part.is_empty())
}

/// Whether a path in a one-line example starts at the config folder, inside the repository.
fn line_path(path: &str) -> bool {
    path_is_inside(path) && !path.starts_with('.')
}

fn is_source(path: &str) -> bool {
    SOURCE_EXTENSIONS.iter().any(|end| path.ends_with(end))
}

fn is_file_target(target: &str) -> bool {
    is_source(target) && !target.starts_with('@') && !target.starts_with("node:")
}

/// The file set that one line stands for, or why the line has no such set.
pub(crate) fn one_line(line: &str) -> Result<ExampleFiles, String> {
    let refuse = |why: String| format!("`{line}` is not a one-line example: {why}; {FORM}");
    let steps: Vec<&str> = line.split(" imports ").collect();
    if let Some(step) = steps
        .iter()
        .find(|step| step.is_empty() || step.contains(char::is_whitespace))
    {
        return Err(refuse(format!("\"{step}\" is not one path or package")));
    }
    let Some((first, _)) = steps.split_first() else {
        return Err(refuse("it has no path".to_owned()));
    };
    if !line_path(first) {
        return Err(refuse(format!(
            "\"{first}\" must be a path inside the repository, relative to the config folder"
        )));
    }
    let mut files = ExampleFiles::new();
    files.entry((*first).to_owned()).or_default();
    let last = steps.len() - 1;
    for (index, pair) in steps.windows(2).enumerate() {
        let [from, to] = pair else { continue };
        if !is_source(from) {
            return Err(refuse(format!(
                "only a source file imports, and \"{from}\" is not one"
            )));
        }
        let specifier = if is_file_target(to) {
            if !line_path(to) {
                return Err(refuse(format!(
                    "\"{to}\" must be a path inside the repository, relative to the config folder"
                )));
            }
            files.entry((*to).to_owned()).or_default();
            relative(from, to)
        } else if index != last - 1 {
            return Err(refuse(format!(
                "only the last target can be a package, and \"{to}\" is not a source file"
            )));
        } else if to.starts_with('.') || to.starts_with('/') {
            return Err(refuse(format!(
                "\"{to}\" is a relative path, and a target starts at the config folder"
            )));
        } else {
            (*to).to_owned()
        };
        let text = files.entry((*from).to_owned()).or_default();
        let _ = writeln!(text, "import '{specifier}';");
    }
    Ok(files)
}

/// The relative specifier from one path to another, with the target's own extension.
fn relative(from: &str, to: &str) -> String {
    let folder: Vec<&str> = from.split('/').collect();
    let folder = &folder[..folder.len() - 1];
    let target: Vec<&str> = to.split('/').collect();
    let shared = folder
        .iter()
        .zip(&target)
        .take_while(|(a, b)| a == b)
        .count()
        .min(target.len() - 1);
    let up = folder.len() - shared;
    let rest = target[shared..].join("/");
    if up == 0 {
        format!("./{rest}")
    } else {
        format!("{}{rest}", "../".repeat(up))
    }
}

/// A problem for each rule and table with a one-line example whose verdict depends on what a file
/// says beyond its imports: such an example would fail with no clear reason, or pass and prove
/// nothing.
pub(crate) fn one_liners_belong(config: &Config, problems: &mut Vec<ConfigProblem>) {
    for rule in config.rules.iter().chain(&config.off) {
        if rule.examples.one_line
            && let Some(reason) = needs_file_text(config, rule)
        {
            problems.push(refused(&rule.name, &reason));
        }
    }
    for (index, table) in config.tables.iter().enumerate() {
        if !table.examples.one_line {
            continue;
        }
        let reason = config.rules.iter().chain(&config.off).find_map(|rule| {
            let ours = matches!(
                &rule.kind,
                RuleKind::TableRow { table, row: Some(_) } if table.index() == index
            );
            needs_file_text(config, rule).filter(|_| ours)
        });
        if let Some(reason) = reason {
            problems.push(refused(&table.name, &reason));
        }
    }
}

fn refused(name: &str, reason: &str) -> ConfigProblem {
    ConfigProblem::new(
        ProblemCode::ConfigInvalid,
        format!(
            "\"{name}\" has a one-line example, whose files hold only runtime imports, and {reason}; \
             write this example as a file set, with the text of each file"
        ),
    )
}

/// Why the text of a file can decide the rule's verdict, when it can.
fn needs_file_text(config: &Config, rule: &Rule) -> Option<String> {
    if rule.imports == ImportKinds::Types {
        return Some("the rule counts only type imports".to_owned());
    }
    rule.sentences.iter().find_map(|sentence| {
        let options = &sentence.options;
        let limit = [
            (options.names.is_some(), "names"),
            (options.specifier.is_some(), "specifier"),
            (options.dependency.is_some(), "dependency"),
        ]
        .into_iter()
        .find_map(|(set, option)| set.then_some(option));
        if let Some(option) = limit {
            return Some(format!("the rule limits its imports with `{option}`"));
        }
        if sentence.condition.is_export() || matches!(sentence.condition, Condition::Plugin(_)) {
            return Some(format!(
                "the verdict of `{}` depends on what a file says",
                sentence.condition.name()
            ));
        }
        let side = sentence.condition.side().into_iter();
        let area = options.within.iter();
        let mut sets = side.chain(area).flat_map(|side| &side.files);
        let directive = uses_directive(config, &sentence.files)
            || sets.any(|set| uses_directive(config, set))
            || options
                .stop_at
                .as_ref()
                .is_some_and(|set| uses_directive(config, set));
        directive.then(|| "the rule selects files by directive".to_owned())
    })
}

/// Whether a set, or a group that it names at any depth, selects by directive.
fn uses_directive(config: &Config, set: &FileSet) -> bool {
    let mut pending = vec![set];
    while let Some(next) = pending.pop() {
        for term in &next.terms {
            match term {
                Term::Directive(_) => return true,
                Term::Selector(id) => pending.extend(
                    config
                        .selectors
                        .get(id.0 as usize)
                        .map(|selector| &selector.definition),
                ),
                _ => {}
            }
        }
    }
    false
}
