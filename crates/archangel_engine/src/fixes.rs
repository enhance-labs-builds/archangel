//! Fixes for entry violations: a shorter specifier that reaches the instance's entry file, offered
//! only when that file exports every name the import uses and the new import breaks no rule. The
//! check uses the facts and resolutions the graph already has, and parses nothing.

use std::collections::HashMap;

use archangel_config::{Pattern, RuleKind};
use archangel_graph::{Import, ImportGraph, Resolution, Work};
use archangel_rules::{Architecture, Code, Diagnostic, Exports, Fix, Target, exports_name};

/// Adds a fix to each entry violation that has one, and says in its help why the others have none.
pub(crate) fn attach(
    diagnostics: &mut [Diagnostic],
    graph: &ImportGraph,
    architecture: &Architecture,
    work: &mut Work,
) {
    let mut fixer = Fixer {
        graph,
        architecture,
        entries: HashMap::new(),
    };
    for diagnostic in diagnostics.iter_mut().filter(|d| d.code == Code::Entry) {
        if reparse_per_fix()
            && let Some(file) = &diagnostic.file
        {
            drop(archangel_graph::extract_file(file, ""));
            work.files_parsed += 1;
        }
        match fixer.fix(diagnostic) {
            Ok(fix) => {
                diagnostic.help = Some(format!("import it through \"{}\"", fix.specifier));
                diagnostic.fix = Some(fix);
            }
            // With no reason of its own, the help that the check wrote stands.
            Err(why) => diagnostic.help = why.or_else(|| diagnostic.help.take()),
        }
    }
}

struct Fixer<'a> {
    graph: &'a ImportGraph,
    architecture: &'a Architecture,
    /// The entry files of each instance root, found once per root.
    entries: HashMap<String, Vec<String>>,
}

impl<'a> Fixer<'a> {
    /// The fix, or why there is none: `Err(None)` keeps the help the check wrote.
    fn fix(&mut self, diagnostic: &Diagnostic) -> Result<Fix, Option<String>> {
        let (Some(file), Some(span), Some(rule)) =
            (&diagnostic.file, diagnostic.span, &diagnostic.rule)
        else {
            return Err(None);
        };
        let graph = self.graph;
        let facts = graph
            .file(file)
            .and_then(|f| f.facts.as_ref())
            .ok_or(None)?;
        let resolutions = graph.resolutions(file).ok_or(None)?;
        let (import, resolution) = facts
            .imports
            .iter()
            .zip(resolutions)
            .find(|(import, _)| import.span == span)
            .ok_or(None)?;
        let Resolution::File(target) = resolution else {
            return Err(None);
        };
        let patterns = self.entry_patterns(rule).ok_or(None)?;
        let root = self
            .architecture
            .matcher()
            .classify_file(target)
            .ok_or(None)?
            .root;
        let entries = self.entries_of(&root, patterns);
        let (specifier, entry) = shorter(&import.specifier)
            .into_iter()
            .find_map(|candidate| match graph.resolve(file, &candidate) {
                Resolution::File(path) if entries.contains(&path) => Some((candidate, path)),
                _ => None,
            })
            .ok_or(None)?;
        missing_export(graph, &entry, &import.names, &specifier)?;
        let mut moved = import.clone();
        moved.specifier.clone_from(&specifier);
        let broken = self.breaks(file, &moved, &entry);
        if !broken.is_empty() {
            return Err(Some(format!(
                "importing {entry} instead would break {}; change the code so that it needs neither import",
                broken.join(", ")
            )));
        }
        Ok(Fix {
            span: (span.0 + 1, span.1.saturating_sub(1)),
            specifier,
        })
    }

    fn entry_patterns(&self, rule: &str) -> Option<&'a [Pattern]> {
        let architecture: &'a Architecture = self.architecture;
        architecture
            .config()
            .rules
            .iter()
            .find(|r| r.name == rule)
            .and_then(|r| match &r.kind {
                RuleKind::Entry { files, .. } => Some(files.as_slice()),
                _ => None,
            })
    }

    fn entries_of(&mut self, root: &str, patterns: &[Pattern]) -> Vec<String> {
        let graph = self.graph;
        self.entries
            .entry(root.to_owned())
            .or_insert_with(|| {
                graph
                    .files()
                    .iter()
                    .filter(|f| {
                        f.path
                            .strip_prefix(&format!("{root}/"))
                            .is_some_and(|inside| {
                                patterns.iter().any(|p| p.matches(inside).is_some())
                            })
                    })
                    .map(|f| f.path.clone())
                    .collect()
            })
            .clone()
    }

    /// The rules that `import`, from `file`, would break if it pointed at `entry`.
    fn breaks(&self, file: &str, import: &Import, entry: &str) -> Vec<String> {
        self.architecture
            .broken_rules_in(Some(self.graph), file, Target::File(entry), import)
            .into_iter()
            .map(|index| self.architecture.config().rules[index].name.clone())
            .collect()
    }
}

/// Why an entry file cannot take an import's names: one it does not export, or one an `export *`
/// from a package or an unresolved specifier may or may not pass on.
fn missing_export(
    graph: &ImportGraph,
    entry: &str,
    names: &[String],
    specifier: &str,
) -> Result<(), Option<String>> {
    for name in names {
        match exports_name(graph, entry, name) {
            Exports::Yes => {}
            Exports::No => {
                return Err(Some(format!(
                    "the entry file {entry} does not export `{name}`; export it there, then import it through \"{specifier}\""
                )));
            }
            Exports::Unknown(from) => {
                return Err(Some(format!(
                    "the entry file {entry} passes names on through `export *` from \"{}\", whose exports archangel cannot know, so it cannot tell whether it exports `{name}`; if it does, import it through \"{specifier}\"",
                    from.join("\", \"")
                )));
            }
        }
    }
    Ok(())
}

/// The specifier with one, two, and more trailing segments removed, longest first, leaving out
/// those made only of `.` and `/`. Those name a folder that holds the importing file, and importing
/// that folder's entry file from inside it is the usual way into a cycle, which the fix's check of
/// the rules does not see.
fn shorter(specifier: &str) -> Vec<String> {
    let segments: Vec<&str> = specifier.split('/').collect();
    (1..segments.len())
        .rev()
        .map(|keep| segments[..keep].join("/"))
        .filter(|candidate| !candidate.chars().all(|c| c == '.' || c == '/'))
        .collect()
}

/// The `engine::reparse_per_fix` fail point: parses a file again for each fix, for the fault
/// control.
fn reparse_per_fix() -> bool {
    fail::fail_point!("engine::reparse_per_fix", |_| true);
    false
}
