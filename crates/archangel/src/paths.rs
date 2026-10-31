//! Paths given on the command line, as the graph names them: relative to the config folder, with
//! `/` between folders.

use std::path::{Component, Path, PathBuf};

use archangel_engine::Session;

/// The path relative to the config folder, from a path relative to the current folder.
pub(crate) fn config_relative(session: &Session, path: &Path) -> String {
    let current = std::env::current_dir().unwrap_or_default();
    let full = lexical(&current.join(path));
    let full = dunce::canonicalize(&full).unwrap_or(full);
    let root = session.graph().root();
    let inside = full.strip_prefix(root).unwrap_or(&full);
    inside
        .components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// The path with `.` and `..` worked out, without asking the file system.
fn lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

/// The source file at `path`, relative to the config folder, or why it is not a source file.
pub(crate) fn source(session: &Session, path: &Path) -> Result<String, String> {
    let name = config_relative(session, path);
    if session.graph().file(&name).is_some() {
        return Ok(name);
    }
    let shown = path.display();
    let full = session.graph().root().join(&name);
    let why = if !full.exists() {
        "it does not exist".to_owned()
    } else if !archangel_graph::is_source(&full) {
        format!(
            "its extension is not a source extension: archangel reads .{}",
            archangel_graph::SOURCE_EXTENSIONS.join(", .")
        )
    } else {
        "it is ignored, by a .gitignore file or the config's ignore list".to_owned()
    };
    Err(format!("{shown} is not a source file, because {why}"))
}
