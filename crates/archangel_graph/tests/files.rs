//! Scenarios from the import-graph spec: which files archangel reads, and how it tells the
//! developer about each path it does not enter.
#![cfg(test)]

#[path = "support/graph.rs"]
mod support;

use archangel_graph::{GraphOptions, GraphProblem, ImportGraph, Work};
use archangel_testkit::FixtureRepo;
use support::sources;

fn build(repo: &FixtureRepo, options: &GraphOptions) -> ImportGraph {
    ImportGraph::build(repo.path(), options, &mut Work::default())
}

#[test]
fn reads_source_files_that_git_does_not_ignore() {
    let repo = FixtureRepo::new()
        .file(".gitignore", "dist/\n")
        .file("src/a.ts", "")
        .file("src/b.d.ts", "")
        .file("dist/a.js", "");
    let graph = build(&repo, &GraphOptions::default());
    assert_eq!(
        sources(&graph),
        ["src/a.ts", "src/b.d.ts"],
        "dist is ignored"
    );
}

#[test]
fn reads_source_files_in_hidden_folders() {
    let repo = FixtureRepo::new().file(".storybook/main.ts", "");
    let graph = build(&repo, &GraphOptions::default());
    assert_eq!(
        sources(&graph),
        [".storybook/main.ts"],
        "hidden folders are read"
    );
}

#[test]
fn config_ignore_list_removes_files() {
    let repo = FixtureRepo::new()
        .file("fixtures/broken.ts", "")
        .file("src/a.ts", "");
    let options = GraphOptions {
        ignore: vec!["fixtures/".to_owned()],
    };
    let graph = build(&repo, &options);
    assert_eq!(
        sources(&graph),
        ["src/a.ts"],
        "the config's ignore list applies"
    );
}

#[test]
fn ignores_ignore_rules_from_outside_the_repository() {
    // A global Git ignore file comes from the user's own Git config, so the test points HOME at
    // a home folder whose Git config names one that ignores src/.
    let home = FixtureRepo::new().file("global-ignore", "src/\n");
    let gitconfig = format!(
        "[core]\n\texcludesFile = {}\n",
        home.join("global-ignore").display()
    );
    let home = home.file(".gitconfig", &gitconfig);
    let env = [
        ("HOME", home.path().as_os_str()),
        ("XDG_CONFIG_HOME", home.path().as_os_str()),
    ];
    if !archangel_testkit::in_child_with_env(
        "ignores_ignore_rules_from_outside_the_repository",
        &env,
    ) {
        return;
    }
    let repo = FixtureRepo::new()
        .file(".git/HEAD", "ref: refs/heads/main\n")
        .file("src/a.ts", "");
    let graph = build(&repo, &GraphOptions::default());
    assert_eq!(
        sources(&graph),
        ["src/a.ts"],
        "global ignore rules do not apply"
    );
}

#[test]
fn ignores_dot_ignore_and_git_info_exclude() {
    let repo = FixtureRepo::new()
        .file(".git/info/exclude", "src/\n")
        .file(".ignore", "src/\n")
        .file("src/a.ts", "");
    let graph = build(&repo, &GraphOptions::default());
    assert_eq!(
        sources(&graph),
        ["src/a.ts"],
        "machine-local ignore files do not apply"
    );
}

#[test]
fn git_file_in_a_worktree_marks_the_repository() {
    let repo = FixtureRepo::new()
        .file(".git", "gitdir: /elsewhere/.git/worktrees/x\n")
        .file(".gitignore", "dist/\n")
        .file("packages/web/archangel.config.json", "{}")
        .file("packages/web/src/a.ts", "")
        .file("packages/web/dist/a.js", "");
    let graph = ImportGraph::build(
        &repo.join("packages/web"),
        &GraphOptions::default(),
        &mut Work::default(),
    );
    assert_eq!(
        sources(&graph),
        ["src/a.ts"],
        "the worktree's top .gitignore applies"
    );
}

#[test]
fn gitignore_above_the_repository_is_not_used() {
    let repo = FixtureRepo::new()
        .file(".gitignore", "src/\n")
        .file("inner/archangel.config.json", "{}")
        .file("inner/src/a.ts", "");
    let graph = ImportGraph::build(
        &repo.join("inner"),
        &GraphOptions::default(),
        &mut Work::default(),
    );
    assert_eq!(
        sources(&graph),
        ["src/a.ts"],
        "a .gitignore above the repository does not apply"
    );
}

#[cfg(unix)]
#[test]
fn symbolic_link_to_a_folder_is_listed_and_not_followed() {
    let repo = FixtureRepo::new()
        .file("old/src/x.ts", "")
        .symlink("app/legacy", "../old/src");
    let graph = ImportGraph::build(
        &repo.join("app"),
        &GraphOptions::default(),
        &mut Work::default(),
    );
    assert!(
        sources(&graph).is_empty(),
        "nothing is read through the link"
    );
    assert_eq!(
        graph.links_not_followed(),
        ["legacy"],
        "the summary lists the link"
    );
}

#[cfg(unix)]
#[test]
fn symbolic_link_to_a_file_is_listed_and_not_followed() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "")
        .symlink("src/b.ts", "a.ts");
    let graph = build(&repo, &GraphOptions::default());
    assert_eq!(
        sources(&graph),
        ["src/a.ts"],
        "the link is not a source file"
    );
    assert_eq!(
        graph.links_not_followed(),
        ["src/b.ts"],
        "the summary lists the link"
    );
}

#[cfg(unix)]
#[test]
fn folder_that_cannot_be_opened_is_file_unreadable() {
    let repo = FixtureRepo::new()
        .file("src/private/a.ts", "")
        .unreadable("src/private");
    let graph = build(&repo, &GraphOptions::default());
    let found = graph.problems().iter().find_map(|problem| match problem {
        GraphProblem::FileUnreadable { path, reason } => Some((path.clone(), reason.clone())),
        _ => None,
    });
    let (path, reason) = found.expect("an unreadable-file problem");
    assert_eq!(path, "src/private", "the problem names the folder");
    assert!(
        reason.contains("permission"),
        "the reason says permission was refused: {reason}"
    );
}

#[test]
fn reads_utf16_files_with_a_byte_order_mark() {
    let text = "import './b';\n";
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
    let repo = FixtureRepo::new()
        .bytes("src/a.ts", &bytes)
        .file("src/b.ts", "");
    let graph = build(&repo, &GraphOptions::default());
    let facts = graph
        .file("src/a.ts")
        .and_then(|file| file.facts.as_ref())
        .expect("facts");
    assert_eq!(
        facts.imports.len(),
        1,
        "the import in the UTF-16 file is found"
    );
    assert_eq!(facts.imports[0].specifier, "./b", "the specifier is read");
}

#[test]
fn file_that_is_not_text_is_file_unreadable() {
    let repo = FixtureRepo::new().bytes("src/a.ts", &[0x66, 0xFF, 0xFE, 0xFD, 0x00, 0xC3]);
    let graph = build(&repo, &GraphOptions::default());
    let problem = graph.problems().iter().find_map(|problem| match problem {
        GraphProblem::FileUnreadable { path, reason } if path == "src/a.ts" => Some(reason.clone()),
        _ => None,
    });
    let reason = problem.expect("an unreadable-file problem for src/a.ts");
    assert!(
        reason.contains("UTF-8"),
        "the reason names the encoding: {reason}"
    );
}

#[cfg(unix)]
#[test]
fn file_larger_than_the_parser_limit_is_parse_failed() {
    let repo = FixtureRepo::new().file("src/big.ts", "");
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(repo.join("src/big.ts"))
        .unwrap();
    file.set_len(5 * 1024 * 1024 * 1024).unwrap();
    let mut work = Work::default();
    let graph = ImportGraph::build(repo.path(), &GraphOptions::default(), &mut work);
    let problem = graph.problems().iter().find_map(|problem| match problem {
        GraphProblem::ParseFailed { path, failure } if path == "src/big.ts" => {
            Some(failure.clone())
        }
        _ => None,
    });
    let failure = problem.expect("a parse problem for the large file");
    assert!(failure.too_large, "the problem says the file is too large");
    assert_eq!(work.bytes_read, 0, "the file is not read");
}

#[test]
fn reads_big_endian_utf16_files_with_a_byte_order_mark() {
    let text = "import './b';\n";
    let mut bytes = vec![0xFE, 0xFF];
    bytes.extend(text.encode_utf16().flat_map(u16::to_be_bytes));
    let repo = FixtureRepo::new()
        .bytes("src/a.ts", &bytes)
        .file("src/b.ts", "");
    let graph = build(&repo, &GraphOptions::default());
    let facts = graph
        .file("src/a.ts")
        .and_then(|file| file.facts.as_ref())
        .expect("facts");
    assert_eq!(
        facts
            .imports
            .iter()
            .map(|i| i.specifier.as_str())
            .collect::<Vec<_>>(),
        ["./b"],
        "the import in the big-endian UTF-16 file is read"
    );
}

#[test]
fn a_utf8_byte_order_mark_is_not_part_of_the_text() {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend(b"import './b';\n");
    let repo = FixtureRepo::new()
        .bytes("src/a.ts", &bytes)
        .file("src/b.ts", "");
    let graph = build(&repo, &GraphOptions::default());
    let import = graph
        .file("src/a.ts")
        .and_then(|file| file.facts.as_ref())
        .and_then(|facts| facts.imports.first())
        .expect("the import");
    assert_eq!(
        (import.line, import.column, import.span),
        (1, 8, (7, 12)),
        "the specifier's position counts from the first character after the mark"
    );
}

#[test]
fn ignore_line_that_is_not_gitignore_syntax_is_ignore_line_invalid() {
    let repo = FixtureRepo::new().file("src/a.ts", "");
    let options = GraphOptions {
        ignore: vec!["[z-a]".to_owned()],
    };
    let graph = build(&repo, &options);
    let found: Vec<&str> = graph
        .problems()
        .iter()
        .filter_map(|problem| match problem {
            GraphProblem::IgnoreLineInvalid { line, .. } => Some(line.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(found, ["[z-a]"], "the problem names the line");
}

#[test]
fn never_reads_git_or_node_modules_folders_even_when_nothing_ignores_them() {
    let repo = FixtureRepo::new()
        .file(".git/hooks/pre-commit.js", "")
        .file("node_modules/react/index.js", "")
        .file("src/a.ts", "");
    let graph = build(&repo, &GraphOptions::default());
    assert_eq!(
        sources(&graph),
        ["src/a.ts"],
        "neither folder holds source files"
    );
}

#[test]
fn lists_each_nested_repository_below_the_config_folder() {
    let repo = FixtureRepo::new()
        .file(".git/HEAD", "ref: refs/heads/main\n")
        .file("vendor/lib/.git/HEAD", "ref: refs/heads/main\n")
        .file("vendor/lib/a.ts", "")
        .file("src/a.ts", "");
    let graph = build(&repo, &GraphOptions::default());
    assert_eq!(
        graph.nested_repositories(),
        ["vendor/lib"],
        "the inner repository is listed, and the repository itself is not"
    );
}

#[test]
fn lists_package_manifests_and_every_dependency_they_name() {
    let repo = FixtureRepo::new()
        .file(
            "package.json",
            r#"{ "dependencies": { "react": "18" }, "devDependencies": { "vitest": "3" } }"#,
        )
        .file(
            "packages/ui/package.json",
            r#"{ "peerDependencies": { "react-dom": "18" }, "optionalDependencies": { "fsevents": "2" } }"#,
        )
        .file("packages/broken/package.json", "{ not json")
        .file("src/a.ts", "");
    let graph = build(&repo, &GraphOptions::default());
    assert_eq!(
        graph.manifests(),
        [
            "package.json",
            "packages/broken/package.json",
            "packages/ui/package.json"
        ],
        "every manifest, sorted"
    );
    assert_eq!(
        graph.manifest_dependencies().iter().collect::<Vec<_>>(),
        ["fsevents", "react", "react-dom", "vitest"],
        "all four dependency fields count, and the manifest that is not JSON names none"
    );
}

#[test]
fn would_walk_answers_as_the_walk_does() {
    let repo = FixtureRepo::new()
        .file(".gitignore", "dist/\n")
        .file("src/generated/.gitignore", "*.gen.ts\n")
        .file("src/a.ts", "");
    let options = GraphOptions {
        ignore: vec!["fixtures/".to_owned()],
    };
    let graph = build(&repo, &options);
    assert_eq!(
        graph.ignore(),
        options.ignore.as_slice(),
        "the graph keeps the list"
    );
    let cases = [
        ("src/new.ts", true),
        ("src/generated/types.ts", true),
        ("src/generated/types.gen.ts", false),
        ("src/readme.md", false),
        ("dist/a.ts", false),
        ("fixtures/a.ts", false),
        ("node_modules/react/index.js", false),
        (".git/hooks/a.js", false),
    ];
    let answers: Vec<(&str, bool)> = cases
        .iter()
        .map(|(path, _)| (*path, graph.would_walk(std::path::Path::new(path))))
        .collect();
    assert_eq!(answers, cases, "each path, relative to the config folder");
    assert!(
        graph.would_walk(&graph.root().join("src/new.ts")),
        "an absolute path below the config folder"
    );
    assert!(
        !graph.would_walk(std::path::Path::new("/elsewhere/a.ts")),
        "a path outside the config folder"
    );
}

#[test]
fn lists_the_files_that_are_not_source_where_the_walk_visits() {
    let repo = FixtureRepo::new()
        .file(".gitignore", "dist/\n")
        .file("src/a.ts", "")
        .file("src/logo.png", "")
        .file("dist/skipped.png", "");
    let mut graph = build(&repo, &GraphOptions::default());
    assert_eq!(
        graph.others(),
        [".gitignore", "src/logo.png"],
        "the files that are not source, sorted, without the ignored one"
    );
    let known = ["src/logo.png", "src/a.ts", "dist/skipped.png"].map(|path| graph.is_other(path));
    assert_eq!(
        known,
        [true, false, false],
        "only a listed path is another file"
    );
    let visited = ["src/new.png", "src/new.ts", "dist/new.png"]
        .map(|path| graph.would_visit(std::path::Path::new(path)));
    assert_eq!(
        visited,
        [true, true, false],
        "the walk would visit a new file of either kind, and not an ignored one"
    );
    let changes = [
        ("src/new.png", true),
        ("src/new.png", true),
        ("src/new.png", false),
        ("src/new.png", false),
    ]
    .map(|(path, present)| graph.set_other(path, present));
    assert_eq!(
        changes,
        [true, false, true, false],
        "adding and removing a path changes the list once each"
    );
    assert_eq!(
        graph.others(),
        [".gitignore", "src/logo.png"],
        "the list is as it was"
    );
}

#[test]
fn finds_the_nearest_manifest_and_the_packages_of_the_workspace() {
    let repo = FixtureRepo::new()
        .file("package.json", r#"{ "name": "root" }"#)
        .file("packages/ui/package.json", r#"{ "name": "ui" }"#)
        .file("packages/ui/src/a.ts", "")
        .file("src/b.ts", "");
    let graph = build(&repo, &GraphOptions::default());
    let nearest = ["packages/ui/src/a.ts", "src/b.ts"].map(|file| {
        graph
            .nearest_manifest(file)
            .map(|manifest| manifest.path.clone())
    });
    assert_eq!(
        nearest,
        [
            Some("packages/ui/package.json".to_owned()),
            Some("package.json".to_owned())
        ],
        "the manifest at or above each file's folder"
    );
    let packages = ["ui", "root", "react"].map(|name| graph.is_workspace_package(name));
    assert_eq!(
        packages,
        [true, true, false],
        "the names of the repository's manifests"
    );
}
