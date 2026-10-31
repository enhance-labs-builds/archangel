//! The differential test from the import-graph spec: on fixture repositories, archangel resolves
//! every import to the same file as the TypeScript compiler.
#![cfg(test)]

use std::path::{Path, PathBuf};
use std::process::Command;

use archangel_graph::{GraphOptions, ImportGraph, Resolution, Work};

fn fixtures() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/resolution");
    let mut found: Vec<PathBuf> = std::fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect();
    found.sort();
    found
}

/// Archangel's repository-file resolutions, as (importing file, specifier, resolved file).
fn ours(fixture: &Path) -> Vec<(String, String, Option<String>)> {
    let graph = ImportGraph::build(fixture, &GraphOptions::default(), &mut Work::default());
    let mut rows = Vec::new();
    for file in graph.files() {
        let Some(facts) = &file.facts else { continue };
        let resolutions = graph.resolutions(&file.path).unwrap();
        for (import, resolution) in facts.imports.iter().zip(resolutions) {
            let resolved = match resolution {
                Resolution::File(path) => Some(path.clone()),
                Resolution::Package(_) => continue,
                Resolution::Unresolved(_) => None,
            };
            rows.push((file.path.clone(), import.specifier.clone(), resolved));
        }
    }
    rows
}

/// The TypeScript compiler's resolutions of the same imports.
fn typescript(fixture: &Path, rows: &[(String, String, Option<String>)]) -> Vec<Option<String>> {
    let script =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/resolve-with-typescript.mjs");
    let requests: Vec<[&str; 2]> = rows
        .iter()
        .map(|(file, spec, _)| [file.as_str(), spec.as_str()])
        .collect();
    let output = Command::new("node")
        .arg(script)
        .arg(fixture)
        .arg(serde_json::to_string(&requests).unwrap())
        .output()
        .expect("node runs the TypeScript resolver");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("the script prints a JSON list")
}

#[test]
fn resolutions_agree_with_the_typescript_compiler() {
    let mut checked = 0;
    for fixture in fixtures() {
        let rows = ours(&fixture);
        let theirs = typescript(&fixture, &rows);
        for ((file, specifier, ours), theirs) in rows.iter().zip(&theirs) {
            assert_eq!(
                ours,
                theirs,
                "{} imports {specifier} in {}",
                file,
                fixture.display()
            );
            checked += 1;
        }
    }
    assert!(
        checked >= 20,
        "the fixtures hold enough imports to mean something: {checked}"
    );
}
