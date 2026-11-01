//! `archangel init`: a starter config with one part for each folder of source files, and a
//! no-cycles rule with the examples that prove it, so `check` and `test` pass from the start.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::{Answer, Options, Outcome};

pub(crate) fn run_init(options: &Options, typescript: bool) -> Answer {
    let dir = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(error) => {
            return Answer::stopped(format!(
                "error: archangel cannot read the current folder: {error}\n"
            ));
        }
    };
    if let Some(existing) = options
        .config
        .as_deref()
        .filter(|path| path.exists())
        .map(|path| path.display().to_string())
        .or_else(|| {
            archangel_config::CONFIG_NAMES
                .iter()
                .find(|name| dir.join(name).exists())
                .map(ToString::to_string)
        })
    {
        return Answer::stopped(format!(
            "error: {existing} already exists; archangel init writes a config only where there is none\n"
        ));
    }
    let (root, folders) = match source_folders(&dir) {
        Ok(found) => found,
        Err(error) => {
            return Answer::stopped(format!(
                "error: archangel cannot list the source files: {error}\n"
            ));
        }
    };
    if folders.is_empty() {
        return Answer::stopped(format!(
            "error: archangel found no folder of source files in {}; init names the parts after them\n",
            root.unwrap_or(".")
        ));
    }
    let parts: Vec<(String, String)> = folders
        .iter()
        .map(|folder| {
            let path = root.map_or_else(|| folder.clone(), |root| format!("{root}/{folder}"));
            (folder.clone(), format!("{path}/**"))
        })
        .collect();
    write_config(&dir, options, &parts, typescript)
}

/// Writes the config, and says what it wrote and what to do next.
fn write_config(
    dir: &Path,
    options: &Options,
    parts: &[(String, String)],
    typescript: bool,
) -> Answer {
    let (name, text) = if typescript {
        ("archangel.config.ts", typescript_config(parts))
    } else {
        ("archangel.config.json", json_config(parts))
    };
    let path = options
        .config
        .clone()
        .unwrap_or_else(|| PathBuf::from(name));
    if let Err(error) = std::fs::write(dir.join(&path), text) {
        return Answer::stopped(format!(
            "error: archangel cannot write {}: {error}\n",
            path.display()
        ));
    }
    let names: Vec<&str> = parts.iter().map(|(name, _)| name.as_str()).collect();
    let mut out = format!(
        "wrote {} with {} parts: {}\n",
        path.display(),
        names.len(),
        names.join(", ")
    );
    if typescript {
        out.push_str("It imports `@enhance-eng/archangel`, so install the package first: npm install --save-dev @enhance-eng/archangel\n");
    }
    out.push_str("Write what each part may import in a table, then run `archangel check`; `archangel docs may-import` shows how.\n");
    Answer::out(out, Outcome::Clean)
}

/// The folder the parts live in (`src` when it holds source files), and each folder directly
/// in it that holds a source file at any depth. The walk honours `.gitignore`, as `check` does.
fn source_folders(dir: &Path) -> Result<(Option<&'static str>, BTreeSet<String>), ignore::Error> {
    let mut in_src = BTreeSet::new();
    let mut at_top = BTreeSet::new();
    for entry in ignore::WalkBuilder::new(dir).require_git(false).build() {
        let entry = entry?;
        let Ok(relative) = entry.path().strip_prefix(dir) else {
            continue;
        };
        let is_source = entry.file_type().is_some_and(|kind| kind.is_file())
            && archangel_graph::is_source(relative);
        if !is_source {
            continue;
        }
        let parts: Vec<String> = relative
            .components()
            .map(|part| part.as_os_str().to_string_lossy().into_owned())
            .collect();
        if parts.len() >= 2 {
            at_top.insert(parts[0].clone());
        }
        if parts.len() >= 3 && parts[0] == "src" {
            in_src.insert(parts[1].clone());
        }
    }
    Ok(if in_src.is_empty() {
        (None, at_top)
    } else {
        (Some("src"), in_src)
    })
}

/// The reason for the starter rule.
const BECAUSE: &str = "Files that import each other cannot change or load one at a time.";

/// The accepted example: one file importing another that imports nothing.
const NO_CYCLE: &str = "cycle/a.ts imports cycle/b.ts";

/// The rejected example: two files that import each other.
const CYCLE: &str = "cycle/a.ts imports cycle/b.ts imports cycle/a.ts";

fn json_config(parts: &[(String, String)]) -> String {
    let config = serde_json::json!({
        "$schema": "./node_modules/@enhance-eng/archangel/archangel.schema.json",
        "parts": parts.iter().map(|(name, pattern)| (name.clone(), serde_json::json!(pattern))).collect::<serde_json::Map<_, _>>(),
        "rules": [{
            "name": "no-cycles",
            "files": "**",
            "noCycles": true,
            "because": BECAUSE,
            "examples": { "accepted": [NO_CYCLE], "rejected": [CYCLE] },
        }],
    });
    let mut text = serde_json::to_string_pretty(&config).unwrap_or_default();
    text.push('\n');
    text
}

/// A TypeScript string literal in single quotes, as most TypeScript is written.
fn quoted(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n");
    format!("'{escaped}'")
}

/// An object key: bare when it is an identifier, quoted otherwise.
fn key(name: &str) -> String {
    let identifier = name
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$');
    if identifier {
        name.to_owned()
    } else {
        quoted(name)
    }
}

fn typescript_config(parts: &[(String, String)]) -> String {
    let mut out = String::from(
        "import { defineConfig } from '@enhance-eng/archangel';\n\nexport default defineConfig({\n  parts: {\n",
    );
    for (name, pattern) in parts {
        let _ = writeln!(out, "    {}: {},", key(name), quoted(pattern));
    }
    let _ = write!(
        out,
        "  }},\n  rules: ({{ rule }}) => [\n    rule('no-cycles', {{\n      files: '**',\n      noCycles: true,\n      because: {},\n      examples: {{\n        accepted: [{}],\n        rejected: [{}],\n      }},\n    }}),\n  ],\n}});\n",
        quoted(BECAUSE),
        quoted(NO_CYCLE),
        quoted(CYCLE)
    );
    out
}
