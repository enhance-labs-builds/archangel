//! Scenarios from the import-graph spec: a built graph follows edits, created and deleted files,
//! changed ignore lists and changed manifests, and does only the work each change needs.
#![cfg(test)]

#[path = "support/graph.rs"]
mod support;

use archangel_graph::{GraphOptions, GraphProblem, ImportGraph, Resolution, Work};
use archangel_testkit::FixtureRepo;
use support::sources;

fn build(repo: &FixtureRepo) -> ImportGraph {
    ImportGraph::build(repo.path(), &GraphOptions::default(), &mut Work::default())
}

fn three_files() -> FixtureRepo {
    FixtureRepo::new()
        .file("src/a.ts", "import './b';\n")
        .file("src/b.ts", "export const b = 1;\n")
        .file("src/c.ts", "export const c = 1;\n")
}

/// (read, parsed, imports found, resolutions)
fn counts(work: &Work) -> (u64, u64, u64, u64) {
    (
        work.files_read,
        work.files_parsed,
        work.imports_found,
        work.resolutions,
    )
}

#[test]
fn an_edit_parses_the_one_file_and_resolves_only_its_imports() {
    let repo = three_files();
    let mut graph = build(&repo);
    let mut work = Work::default();
    let changed = graph.replace_file(
        "src/b.ts",
        Some("import './a';\nimport './c';\n"),
        &mut work,
    );
    assert!(changed, "the new text is parsed");
    assert_eq!(
        counts(&work),
        (0, 1, 2, 2),
        "(read, parsed, imports, resolutions) for an edit in memory with two imports"
    );
    assert_eq!(
        graph.resolutions("src/b.ts"),
        Some(
            [
                Resolution::File("src/a.ts".to_owned()),
                Resolution::File("src/c.ts".to_owned()),
            ]
            .as_slice()
        ),
        "the edit's imports resolve"
    );
    assert_eq!(
        graph.importers("src/c.ts"),
        ["src/b.ts"],
        "c has a new importer"
    );
}

#[test]
fn an_edit_read_from_disk_reads_the_one_file() {
    let repo = three_files();
    let mut graph = build(&repo);
    repo.write("src/c.ts", "import './b';\n");
    let mut work = Work::default();
    assert!(
        graph.replace_file("src/c.ts", None, &mut work),
        "the saved text is parsed"
    );
    assert_eq!(
        counts(&work),
        (1, 1, 1, 1),
        "(read, parsed, imports, resolutions) for one saved file with one import"
    );
    assert_eq!(
        graph.importers("src/b.ts"),
        ["src/a.ts", "src/c.ts"],
        "the saved import is in the graph"
    );
}

#[test]
fn text_the_graph_already_has_costs_no_parse() {
    let repo = three_files();
    let mut graph = build(&repo);
    let mut work = Work::default();
    assert!(
        !graph.replace_file("src/a.ts", Some("import './b';\n"), &mut work),
        "the text is the text on disk"
    );
    assert_eq!(work, Work::default(), "nothing is parsed or resolved");
}

#[test]
fn a_file_with_a_syntax_error_saved_again_unchanged_costs_no_parse() {
    let repo = three_files().file("src/broken.ts", "const = ;\n");
    let mut graph = build(&repo);
    let mut work = Work::default();
    assert!(
        !graph.replace_file("src/broken.ts", Some("const = ;\n"), &mut work),
        "the broken text is the text the graph parsed"
    );
    assert_eq!(work, Work::default(), "nothing is parsed again");
    assert!(
        graph
            .problems()
            .iter()
            .any(|problem| matches!(problem, GraphProblem::ParseFailed { path, .. } if path == "src/broken.ts")),
        "the parse failure stays"
    );
}

#[test]
fn a_path_that_is_not_a_source_file_of_the_graph_costs_no_parse() {
    let repo = three_files();
    let mut graph = build(&repo);
    let mut work = Work::default();
    assert!(
        !graph.replace_file("src/new.ts", Some("import './a';\n"), &mut work),
        "the file is not in the graph"
    );
    assert_eq!(work, Work::default(), "nothing is parsed");
    assert_eq!(
        sources(&graph),
        ["src/a.ts", "src/b.ts", "src/c.ts"],
        "the graph does not gain the file"
    );
}

#[test]
fn a_created_file_is_parsed_once_and_every_import_resolved_again() {
    let repo = three_files();
    let mut graph = build(&repo);
    repo.write("src/d.ts", "import './c';\n");
    let mut work = Work::default();
    graph.apply_file_events(&["src/d.ts".to_owned()], &[], &mut work);
    assert_eq!(
        sources(&graph),
        ["src/a.ts", "src/b.ts", "src/c.ts", "src/d.ts"],
        "d is a source file"
    );
    assert_eq!(
        graph.importers("src/c.ts"),
        ["src/d.ts"],
        "d's import resolves"
    );
    assert_eq!(
        counts(&work),
        (1, 1, 1, 2),
        "(read, parsed, imports, resolutions): d is read and parsed, and both imports resolve again"
    );
}

#[test]
fn a_deleted_file_leaves_the_graph_and_its_importers_resolve_again() {
    let repo = three_files();
    let mut graph = build(&repo);
    repo.remove("src/b.ts");
    let mut work = Work::default();
    graph.apply_file_events(&[], &["src/b.ts".to_owned()], &mut work);
    assert_eq!(sources(&graph), ["src/a.ts", "src/c.ts"], "b is gone");
    assert!(
        matches!(
            graph.resolutions("src/a.ts"),
            Some([Resolution::Unresolved(_)])
        ),
        "a's import of b no longer resolves"
    );
    assert_eq!(
        counts(&work),
        (0, 0, 0, 1),
        "(read, parsed, imports, resolutions): nothing is parsed, and a's import resolves again"
    );
}

#[test]
fn a_file_created_and_deleted_in_one_batch_is_not_read() {
    let repo = three_files();
    let mut graph = build(&repo);
    let mut work = Work::default();
    graph.apply_file_events(
        &["src/gone.ts".to_owned()],
        &["src/gone.ts".to_owned()],
        &mut work,
    );
    assert_eq!(
        sources(&graph),
        ["src/a.ts", "src/b.ts", "src/c.ts"],
        "the file that no longer exists is not a source file"
    );
    assert!(
        graph.problems().is_empty(),
        "and it is not reported as unreadable"
    );
    assert_eq!(work.files_parsed, 0, "nothing is parsed");
}

#[test]
fn a_created_event_for_a_file_the_graph_has_does_not_add_it_twice() {
    let repo = three_files();
    let mut graph = build(&repo);
    let mut work = Work::default();
    graph.apply_file_events(&["src/a.ts".to_owned()], &[], &mut work);
    assert_eq!(
        sources(&graph),
        ["src/a.ts", "src/b.ts", "src/c.ts"],
        "a is listed once"
    );
    assert_eq!(work.files_parsed, 0, "a is not parsed again");
}

#[test]
fn resolving_again_after_a_tsconfig_change_uses_the_new_tsconfig_and_parses_nothing() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "import '@/b';\nimport './c';\n")
        .file("src/b.ts", "")
        .file("src/c.ts", "");
    let mut graph = build(&repo);
    assert!(
        matches!(
            graph.resolutions("src/a.ts"),
            Some([Resolution::Unresolved(_), Resolution::File(_)])
        ),
        "without a tsconfig the alias does not resolve"
    );
    repo.write(
        "tsconfig.json",
        r#"{ "compilerOptions": { "paths": { "@/*": ["./src/*"] } } }"#,
    );
    let mut work = Work::default();
    graph.resolve_all_again(&mut work);
    assert_eq!(
        graph.resolutions("src/a.ts"),
        Some(
            [
                Resolution::File("src/b.ts".to_owned()),
                Resolution::File("src/c.ts".to_owned()),
            ]
            .as_slice()
        ),
        "the alias resolves through the new tsconfig"
    );
    assert_eq!(
        counts(&work),
        (0, 0, 0, 2),
        "(read, parsed, imports, resolutions): every import resolves once, and nothing is parsed"
    );
}

#[test]
fn walking_again_with_a_new_ignore_list_removes_and_restores_files() {
    let repo = three_files();
    let mut graph = build(&repo);
    let ignore = vec!["src/c.ts".to_owned()];
    let mut work = Work::default();
    graph.walk_again(&ignore, &mut work);
    assert_eq!(sources(&graph), ["src/a.ts", "src/b.ts"], "c is ignored");
    assert_eq!(
        graph.ignore(),
        ignore.as_slice(),
        "the graph keeps the new list"
    );
    assert_eq!(
        (work.files_found, work.files_parsed),
        (2, 0),
        "(found, parsed): the walk finds two files and parses none"
    );
    let mut work = Work::default();
    graph.walk_again(&[], &mut work);
    assert_eq!(
        sources(&graph),
        ["src/a.ts", "src/b.ts", "src/c.ts"],
        "c is a source file again"
    );
    assert!(graph.ignore().is_empty(), "the list is empty again");
    assert_eq!(
        (work.files_found, work.files_read, work.files_parsed),
        (3, 1, 1),
        "(found, read, parsed): the walk finds three files and reads only c"
    );
}

#[test]
fn walking_again_finds_new_manifests_and_their_dependencies() {
    let repo = three_files().file(
        "package.json",
        r#"{ "name": "root", "dependencies": { "react": "18.0.0" } }"#,
    );
    let mut graph = build(&repo);
    repo.write(
        "packages/ui/package.json",
        r#"{ "name": "ui", "devDependencies": { "vitest": "3.0.0" } }"#,
    );
    graph.walk_again(&[], &mut Work::default());
    assert_eq!(
        graph.manifests(),
        ["package.json", "packages/ui/package.json"],
        "the new manifest is found"
    );
    assert_eq!(
        graph.manifest_dependencies().iter().collect::<Vec<_>>(),
        ["react", "vitest"],
        "its dependencies are read"
    );
}

#[test]
fn reading_the_manifests_again_picks_up_changed_dependencies() {
    let repo = three_files().file(
        "package.json",
        r#"{ "name": "root", "dependencies": { "react": "18.0.0" } }"#,
    );
    let mut graph = build(&repo);
    repo.write(
        "package.json",
        r#"{ "name": "root", "dependencies": { "preact": "10.0.0" } }"#,
    );
    graph.reread_manifests();
    assert_eq!(
        graph.manifest_dependencies().iter().collect::<Vec<_>>(),
        ["preact"],
        "the changed dependency replaces the old one"
    );
}

#[test]
fn reading_the_manifests_again_returns_the_ones_whose_lists_changed() {
    let repo = three_files()
        .file(
            "package.json",
            r#"{ "name": "root", "dependencies": { "react": "18.0.0" } }"#,
        )
        .file("packages/ui/package.json", r#"{ "name": "ui" }"#)
        .file("packages/kit/package.json", r#"{ "name": "kit" }"#);
    let mut graph = build(&repo);
    assert_eq!(
        graph.reread_manifests(),
        Vec::<String>::new(),
        "nothing changed"
    );
    repo.write(
        "package.json",
        r#"{ "name": "root", "dependencies": { "preact": "10.0.0" } }"#,
    );
    repo.remove("packages/kit/package.json");
    assert_eq!(
        graph.reread_manifests(),
        ["package.json", "packages/kit/package.json"],
        "the changed manifest and the one that is gone, and not the one that stayed the same"
    );
}

#[test]
fn an_edit_that_drops_an_import_takes_the_file_off_its_targets_importers() {
    let repo = three_files().file("src/c.ts", "import './b';\n");
    let mut graph = build(&repo);
    assert_eq!(
        graph.importers("src/b.ts"),
        ["src/a.ts", "src/c.ts"],
        "both files import b"
    );
    graph.replace_file("src/a.ts", Some(""), &mut Work::default());
    assert_eq!(
        graph.importers("src/b.ts"),
        ["src/c.ts"],
        "only the file that still imports b"
    );
}
