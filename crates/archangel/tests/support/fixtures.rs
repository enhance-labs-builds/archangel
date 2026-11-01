//! Shared by the tests that run a documented fixture: a copy of it, with the packages installed
//! that its config and its manifest name.
#![cfg(test)]
// @justified intent
// Each test crate includes this file and uses only some of its helpers.
#![allow(dead_code, reason = "each test crate uses only some of these helpers")]

use std::path::{Path, PathBuf};

use archangel_testkit::FixtureRepo;

#[path = "install.rs"]
mod install;

/// The folder of the documented fixtures.
pub(crate) fn examples_folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/examples")
}

/// A copy of the fixture at `folder`, without its expected output.
pub(crate) fn copy_of_folder(folder: &Path) -> FixtureRepo {
    let mut repo = FixtureRepo::new();
    let mut pending = vec![folder.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(&next).unwrap() {
            let path = entry.unwrap().path();
            let inside = path.strip_prefix(folder).unwrap();
            if path.is_dir() {
                if inside != Path::new("expected") {
                    pending.push(path);
                }
            } else {
                let text = std::fs::read_to_string(&path).unwrap();
                repo = repo.file(&inside.to_string_lossy().replace('\\', "/"), &text);
            }
        }
    }
    install_packages(&repo);
    repo
}

/// A copy of a documented fixture, by its name.
pub(crate) fn fixture(name: &str) -> FixtureRepo {
    copy_of_folder(&examples_folder().join(name))
}

/// Writes the package that a config imports, and an empty package for each dependency of the
/// fixture's manifest.
fn install_packages(repo: &FixtureRepo) {
    install::install_archangel(repo);
    repo.write(".gitignore", "node_modules/\n");
    let manifest: serde_json::Value = serde_json::from_str(&repo.read("package.json")).unwrap();
    let dependencies = manifest["dependencies"].as_object().unwrap();
    for name in dependencies.keys() {
        repo.write(
            &format!("node_modules/{name}/package.json"),
            &format!(r#"{{ "name": "{name}", "main": "index.js" }}"#),
        );
        repo.write(&format!("node_modules/{name}/index.js"), "");
    }
}
