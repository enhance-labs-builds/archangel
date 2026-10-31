//! Files held in memory, parsed and resolved over the repository without a change to it.
#![cfg(test)]

use std::collections::BTreeMap;

use archangel_graph::{GraphOptions, ImportGraph, Resolution, Work};
use archangel_testkit::FixtureRepo;

fn graph(repo: &FixtureRepo) -> ImportGraph {
    ImportGraph::build(repo.path(), &GraphOptions::default(), &mut Work::default())
}

fn files(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(path, text)| ((*path).to_owned(), (*text).to_owned()))
        .collect()
}

#[test]
fn overlay_files_resolve_to_each_other_in_a_folder_that_exists_only_in_memory() {
    let repo = FixtureRepo::new().file("src/shared/a.ts", "export const a = 1;\n");
    let graph = graph(&repo);
    let overlay = graph.overlay(
        &files(&[
            (
                "src/new/one.ts",
                "import { two } from './two';\nimport { a } from '../shared/a';\n",
            ),
            ("src/new/two.ts", "export const two = 2;\n"),
        ]),
        &mut Work::default(),
    );
    assert_eq!(
        overlay.resolutions[0],
        [
            Resolution::File("src/new/two.ts".to_owned()),
            Resolution::File("src/shared/a.ts".to_owned()),
        ],
        "one.ts imports a file in memory and a file on disk"
    );
    assert!(
        !repo.join("src/new").exists(),
        "the folder src/new must not be created on disk"
    );
}

#[test]
fn an_import_of_a_folder_that_exists_only_in_memory_resolves_to_its_index() {
    let repo = FixtureRepo::new().file("src/a.ts", "export const a = 1;\n");
    let graph = graph(&repo);
    let overlay = graph.overlay(
        &files(&[
            ("src/c.ts", "import { x } from './new';\n"),
            ("src/new/index.ts", "export const x = 1;\n"),
        ]),
        &mut Work::default(),
    );
    assert_eq!(
        overlay.resolutions[0],
        [Resolution::File("src/new/index.ts".to_owned())],
        "the folder in memory is a folder"
    );
}

#[test]
fn an_overlay_file_replaces_the_repository_file_at_its_path() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "export const a = 1;\n")
        .file("src/b.ts", "export const b = 1;\n");
    let graph = graph(&repo);
    let overlay = graph.overlay(
        &files(&[("src/a.ts", "import { b } from './b';\n")]),
        &mut Work::default(),
    );
    assert_eq!(
        overlay.resolutions[0],
        [Resolution::File("src/b.ts".to_owned())],
        "the file in memory is the one parsed"
    );
    assert_eq!(
        std::fs::read_to_string(repo.join("src/a.ts")).unwrap(),
        "export const a = 1;\n",
        "the file on disk keeps its bytes"
    );
    assert_eq!(
        graph
            .file("src/a.ts")
            .and_then(|f| f.facts.as_ref())
            .map(|f| f.imports.len()),
        Some(0),
        "the graph keeps the repository's file"
    );
}

#[test]
fn an_overlay_parses_only_its_own_files() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "export const a = 1;\n")
        .file("src/b.ts", "export const b = 1;\n");
    let graph = graph(&repo);
    let mut work = Work::default();
    let overlay = graph.overlay(
        &files(&[("src/c.ts", "import { a } from './a';\n")]),
        &mut work,
    );
    assert_eq!(
        (work.files_parsed, work.resolutions, work.files_read),
        (1, 1, 0),
        "(parsed, resolutions, read from disk) for one file in memory with one import"
    );
    assert_eq!(
        overlay.files.len(),
        1,
        "the overlay holds only its own file"
    );
}

#[test]
fn reaching_lists_every_file_that_imports_a_path_through_other_files() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "export const a = 1;\n")
        .file("src/b.ts", "import { a } from './a';\n")
        .file("src/c.ts", "import './b';\n")
        .file("src/d.ts", "export const d = 1;\n");
    let graph = graph(&repo);
    assert_eq!(
        graph.importers("src/a.ts"),
        ["src/b.ts"],
        "the direct importer"
    );
    assert_eq!(
        graph.reaching(&["src/a.ts"]),
        ["src/b.ts", "src/c.ts"],
        "b imports a, and c imports b; d imports nothing"
    );
}

#[test]
fn an_overlays_manifests_are_those_the_walk_would_visit() {
    let repo = FixtureRepo::new().file("src/a.ts", "export const a = 1;\n");
    let graph = graph(&repo);
    let overlay = graph.overlay(
        &files(&[
            (
                "package.json",
                r#"{ "name": "app", "devDependencies": { "vitest": "1" } }"#,
            ),
            ("src/package.json", "not json"),
            (
                "node_modules/vitest/package.json",
                r#"{ "name": "vitest" }"#,
            ),
        ]),
        &mut Work::default(),
    );
    let mut found: Vec<(&str, &str, Vec<&str>)> = overlay
        .manifests
        .iter()
        .map(|(folder, manifest)| {
            let development = manifest.development.iter().map(String::as_str).collect();
            (folder.as_str(), manifest.path.as_str(), development)
        })
        .collect();
    found.sort();
    assert_eq!(
        found,
        [
            ("", "package.json", vec!["vitest"]),
            ("src", "src/package.json", vec![]),
        ],
        "a manifest that does not parse lists nothing, and one in node_modules is not a manifest"
    );
    let manifests = graph.manifests_with(Some(&overlay));
    assert_eq!(
        manifests.nearest("src/a.ts").map(|m| m.path.as_str()),
        Some("src/package.json"),
        "the nearest manifest of a file in src is the overlay's src/package.json"
    );
}
