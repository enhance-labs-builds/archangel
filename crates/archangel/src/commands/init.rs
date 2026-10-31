//! `archangel init`: a starter config with one element for each folder of source files, and a
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
            "error: archangel found no folder of source files in {}; init names the elements after them\n",
            root.unwrap_or(".")
        ));
    }
    let elements: Vec<(String, String)> = folders
        .iter()
        .map(|folder| {
            let path = root.map_or_else(|| folder.clone(), |root| format!("{root}/{folder}"));
            (folder.clone(), format!("{path}/**"))
        })
        .collect();
    write_config(&dir, options, &elements, typescript)
}

/// Writes the config, and says what it wrote and what to do next.
fn write_config(
    dir: &Path,
    options: &Options,
    elements: &[(String, String)],
    typescript: bool,
) -> Answer {
    let (name, text) = if typescript {
        ("archangel.config.ts", typescript_config(elements))
    } else {
        ("archangel.config.json", json_config(elements))
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
    let names: Vec<&str> = elements.iter().map(|(name, _)| name.as_str()).collect();
    let mut out = format!(
        "wrote {} with {} elements: {}\n",
        path.display(),
        names.len(),
        names.join(", ")
    );
    if typescript {
        out.push_str("It imports `@enhance-eng/archangel`, so install the package first: npm install --save-dev @enhance-eng/archangel\n");
    }
    out.push_str("Order the elements with a layers rule, then run `archangel check`; `archangel docs layers` shows how.\n");
    Answer::out(out, Outcome::Clean)
}

/// The folder the elements live in (`src` when it holds source files), and each folder directly
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

/// The rejected example: two files that import each other.
const CYCLE: [(&str, &str); 2] = [
    ("cycle/a.ts", "import \"./b\";\n"),
    ("cycle/b.ts", "import \"./a\";\n"),
];

/// The accepted example: one file importing another that imports nothing.
const NO_CYCLE: [(&str, &str); 2] = [
    ("cycle/a.ts", "import \"./b\";\n"),
    ("cycle/b.ts", "export {};\n"),
];

fn json_config(elements: &[(String, String)]) -> String {
    let files = |pairs: &[(&str, &str)]| -> serde_json::Value {
        pairs
            .iter()
            .map(|(path, text)| ((*path).to_owned(), serde_json::json!(text)))
            .collect()
    };
    let config = serde_json::json!({
        "$schema": "./node_modules/@enhance-eng/archangel/archangel.schema.json",
        "elements": elements.iter().map(|(name, pattern)| (name.clone(), serde_json::json!(pattern))).collect::<serde_json::Map<_, _>>(),
        "rules": [{
            "name": "no-cycles",
            "files": "**",
            "noCycles": true,
            "examples": { "accepted": [files(&NO_CYCLE)], "rejected": [files(&CYCLE)] },
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

fn typescript_config(elements: &[(String, String)]) -> String {
    let files = |pairs: &[(&str, &str)]| {
        let entries: Vec<String> = pairs
            .iter()
            .map(|(path, text)| format!("{}: {}", key(path), quoted(text)))
            .collect();
        format!("{{ {} }}", entries.join(", "))
    };
    let mut out = String::from(
        "import { defineConfig } from '@enhance-eng/archangel';\n\nexport default defineConfig({\n  elements: {\n",
    );
    for (name, pattern) in elements {
        let _ = writeln!(out, "    {}: {},", key(name), quoted(pattern));
    }
    let _ = write!(
        out,
        "  }},\n  rules: ({{ rule }}) => [\n    rule({{\n      name: 'no-cycles',\n      files: '**',\n      noCycles: true,\n      examples: {{\n        accepted: [{}],\n        rejected: [{}],\n      }},\n    }}),\n  ],\n}});\n",
        files(&NO_CYCLE),
        files(&CYCLE)
    );
    out
}
