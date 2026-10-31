//! `archangel why <from> <to>`: the shortest chain of imports from a source file to a repository
//! file or a package, found with a breadth-first search over the graph.

use std::collections::{HashMap, VecDeque};
use std::fmt::Write;
use std::path::Path;

use archangel_engine::Session;
use archangel_graph::Resolution;

use serde_json::json;

use crate::render::json::SCHEMA_VERSION;
use crate::{Answer, Format, Options, Outcome, paths};

/// One step of a chain: the file, the line of its import of the next step, and the specifier.
struct Step {
    from: String,
    line: u32,
    specifier: String,
}

pub(crate) fn run_why(options: &Options, from: &Path, to: &str) -> Answer {
    let session = match super::open_session(options) {
        Ok(session) => session,
        Err(answer) => return answer,
    };
    let start = match paths::source(&session, from) {
        Ok(path) => path,
        Err(reason) => return super::not_a_source_file(&reason),
    };
    let goal = if session
        .graph()
        .root()
        .join(paths::config_relative(&session, Path::new(to)))
        .exists()
    {
        paths::config_relative(&session, Path::new(to))
    } else {
        to.to_owned()
    };
    let steps = chain(&session, &start, &goal);
    if options.format == Format::Json {
        let outcome = if steps.is_some() {
            Outcome::Clean
        } else {
            Outcome::Found
        };
        let chain = steps.map(|steps| {
            steps
                .iter()
                .map(|step| json!({ "file": step.from, "line": step.line, "specifier": step.specifier }))
                .collect::<Vec<_>>()
        });
        let document =
            json!({ "schemaVersion": SCHEMA_VERSION, "from": start, "to": goal, "chain": chain });
        let mut text = serde_json::to_string_pretty(&document).unwrap_or_default();
        text.push('\n');
        return Answer::out(text, outcome);
    }
    match steps {
        Some(steps) => {
            let mut out = String::new();
            for step in &steps {
                let _ = writeln!(
                    out,
                    "{}:{}  imports '{}'",
                    step.from, step.line, step.specifier
                );
            }
            let _ = writeln!(out, "{goal}");
            Answer::out(out, Outcome::Clean)
        }
        None => Answer::out(
            format!("{start} does not reach {goal} through any chain of imports\n"),
            Outcome::Found,
        ),
    }
}

/// The name of where an import points: a file path or a package name.
fn place(resolution: &Resolution) -> Option<&str> {
    match resolution {
        Resolution::File(path) => Some(path),
        Resolution::Package(name) => Some(name),
        Resolution::Unresolved(_) => None,
    }
}

fn chain(session: &Session, start: &str, goal: &str) -> Option<Vec<Step>> {
    let graph = session.graph();
    let mut parent: HashMap<String, Step> = HashMap::new();
    let mut queue = VecDeque::from([start.to_owned()]);
    let mut seen = std::collections::HashSet::from([start.to_owned()]);
    while let Some(current) = queue.pop_front() {
        if current == goal {
            let mut steps = Vec::new();
            let mut at = current;
            while let Some(step) = parent.remove(&at) {
                at.clone_from(&step.from);
                steps.push(step);
            }
            steps.reverse();
            return Some(steps);
        }
        let (Some(facts), Some(resolutions)) = (
            graph.file(&current).and_then(|f| f.facts.as_ref()),
            graph.resolutions(&current),
        ) else {
            continue;
        };
        for (import, resolution) in facts.imports.iter().zip(resolutions) {
            if let Some(next) = place(resolution)
                && seen.insert(next.to_owned())
            {
                parent.insert(
                    next.to_owned(),
                    Step {
                        from: current.clone(),
                        line: import.line,
                        specifier: import.specifier.clone(),
                    },
                );
                queue.push_back(next.to_owned());
            }
        }
    }
    None
}
