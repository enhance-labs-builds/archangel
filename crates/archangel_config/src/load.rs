//! Finding the config, reading it from JSON, and reading the answer of the loader that Node runs
//! for a TypeScript or JavaScript config.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::model::Config;
use crate::parse::{from_value, parse_json};
use crate::problem::{ConfigProblem, ProblemCode};

/// The file names archangel looks for, in this order, in each folder.
pub const CONFIG_NAMES: [&str; 5] = [
    "archangel.config.ts",
    "archangel.config.mts",
    "archangel.config.js",
    "archangel.config.mjs",
    "archangel.config.json",
];

/// The oldest Node that runs a TypeScript file without a build step.
const NODE_MINIMUM: (u32, u32) = (22, 18);

/// The loader that Node runs for a TypeScript or JavaScript config: it answers with the config,
/// and then hosts the config's plugins until its standard input closes.
pub const LOADER: &str = include_str!("load.mjs");

/// The nearest config at or above `start`.
#[must_use]
pub fn discover(start: &Path) -> Option<PathBuf> {
    start.ancestors().find_map(|folder| {
        CONFIG_NAMES
            .iter()
            .map(|name| folder.join(name))
            .find(|candidate| candidate.is_file())
    })
}

/// The problem for a folder with no config in it or above it.
#[must_use]
pub fn missing(start: &Path) -> ConfigProblem {
    ConfigProblem::new(
        ProblemCode::ConfigMissing,
        format!(
            "no config in {} or any folder above it; archangel looks for {}",
            start.display(),
            CONFIG_NAMES.join(", ")
        ),
    )
}

/// Loads and validates a JSON config. A TypeScript or JavaScript config is loaded through Node,
/// which the engine starts with [`LOADER`], and validated with [`from_loader`].
///
/// # Errors
///
/// Returns every problem with the config.
pub fn load_json(path: &Path) -> Result<Config, Vec<ConfigProblem>> {
    let text = std::fs::read_to_string(path).map_err(|error| {
        vec![ConfigProblem::new(
            ProblemCode::ConfigInvalid,
            format!("archangel cannot read {}: {error}", path.display()),
        )]
    })?;
    parse_json(&text)
}

/// Whether the config at `path` is JSON, which archangel reads without Node.
#[must_use]
pub fn is_json(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "json")
}

/// Validates the loader's first answer: the config, or the error that stopped it.
///
/// # Errors
///
/// Returns every problem with the config, or the reason it did not load.
pub fn from_loader(answer: &Value) -> Result<Config, Vec<ConfigProblem>> {
    match answer.get("type").and_then(Value::as_str) {
        Some("config") => from_value(answer.get("config").unwrap_or(&Value::Null)),
        Some("error") => Err(vec![load_failure(
            answer.get("error").unwrap_or(&Value::Null),
        )]),
        _ => Err(vec![ConfigProblem::new(
            ProblemCode::ConfigLoadFailed,
            format!("Node answered with `{answer}` instead of the config"),
        )]),
    }
}

/// The problem for a Node that stopped before it gave the config, with what it printed.
#[must_use]
pub fn load_stopped(printed: &str) -> ConfigProblem {
    ConfigProblem::new(
        ProblemCode::ConfigLoadFailed,
        format!(
            "Node stopped before it loaded the config: {}",
            printed.trim()
        ),
    )
}

/// The problem for a Node that `node --version` shows is missing or too old, or `None` when it
/// can run a TypeScript config.
#[must_use]
pub fn node_version_problem(printed: &str) -> Option<ConfigProblem> {
    let version = printed.trim();
    let mut numbers = version
        .trim_start_matches('v')
        .split('.')
        .map(str::parse::<u32>);
    let (Some(Ok(major)), Some(Ok(minor))) = (numbers.next(), numbers.next()) else {
        return Some(needs_node(&format!(
            "`node --version` printed `{version}`, which is not a version"
        )));
    };
    ((major, minor) < NODE_MINIMUM).then(|| {
        needs_node(&format!(
            "the `node` on the search path is version {}, and archangel needs {}.{} or later to run a \
             TypeScript config",
            version.trim_start_matches('v'),
            NODE_MINIMUM.0,
            NODE_MINIMUM.1
        ))
    })
}

/// The problem for a search path with no `node` on it.
#[must_use]
pub fn node_missing() -> ConfigProblem {
    needs_node("archangel found no `node` program on the search path")
}

fn needs_node(cause: &str) -> ConfigProblem {
    ConfigProblem::new(
        ProblemCode::ConfigNeedsNode,
        format!("{cause}. A JSON config, archangel.config.json, needs no Node."),
    )
}

fn load_failure(error: &Value) -> ConfigProblem {
    let kind = error
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let code = error
        .get("code")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let text = if kind == "no-default-export" {
        "the config has no default export; write `export default defineConfig({ ... })`".to_owned()
    } else if code == "ERR_UNSUPPORTED_TYPESCRIPT_SYNTAX" && message.contains("enum") {
        format!(
            "the config declares an enum, which Node cannot run because it only strips types; use a \
             constant object with `as const` instead ({message})"
        )
    } else if code == "ERR_UNSUPPORTED_TYPESCRIPT_SYNTAX" {
        format!(
            "the config uses TypeScript that Node cannot strip, such as a namespace or a parameter \
             property; write it as plain JavaScript with types ({message})"
        )
    } else if code == "ERR_MODULE_NOT_FOUND" {
        format!(
            "the config imports a file Node cannot find; Node needs the `.ts` extension on relative \
             imports, and does not use tsconfig path aliases ({message})"
        )
    } else {
        format!("the config threw an error while it loaded: {message}")
    };
    ConfigProblem::new(ProblemCode::ConfigLoadFailed, text)
}
