//! Scenarios from the command line spec: a starter config, color, shell completions and the manual.
#![cfg(test)]

#[path = "support/cli.rs"]
mod support;

use archangel_testkit::FixtureRepo;
use serde_json::json;
use support::run;

fn sources() -> FixtureRepo {
    FixtureRepo::new()
        .file("src/app/main.ts", "import '../shared/s';\n")
        .file("src/features/login/index.ts", "")
        .file("src/shared/s.ts", "")
        .file("src/assets/logo.svg", "<svg/>")
        .file("node_modules/react/index.js", "")
        .file(".gitignore", "generated/\n")
        .file("src/generated/api.ts", "")
}

#[test]
fn init_names_an_element_for_each_folder_of_source_files() {
    let repo = sources();
    let found = run(&repo, &["init"]);
    assert_eq!(found.status, 0, "init writes the config: {}", found.stderr);
    let config: serde_json::Value =
        serde_json::from_str(&repo.read("archangel.config.json")).unwrap();
    assert_eq!(
        config["elements"],
        json!({ "app": "src/app/**", "features": "src/features/**", "shared": "src/shared/**" }),
        "one element per folder in src with a source file; not assets, not ignored folders"
    );
    assert!(
        found
            .stdout
            .contains("with 3 elements: app, features, shared"),
        "it says what it wrote: {}",
        found.stdout
    );
}

#[test]
fn a_fresh_config_checks_and_proves_itself() {
    for args in [&["init"][..], &["init", "--typescript"][..]] {
        let repo = sources();
        assert_eq!(run(&repo, args).status, 0, "{args:?} writes a config");
        if args.len() == 2 {
            install_archangel(&repo);
        }
        let checked = run(&repo, &["check"]);
        assert_eq!(
            checked.status, 0,
            "{args:?}: the starter config checks clean: {}{}",
            checked.stdout, checked.stderr
        );
        let tested = run(&repo, &["test"]);
        assert_eq!(
            tested.status, 0,
            "{args:?}: its rule proves itself: {}{}",
            tested.stdout, tested.stderr
        );
    }
}

#[test]
fn init_names_the_folders_at_the_top_when_src_has_none() {
    let repo = FixtureRepo::new()
        .file("index.ts", "")
        .file("lib/deep/a.ts", "")
        .file("tools/b.js", "")
        .file("docs/x.md", "");
    let found = run(&repo, &["init"]);
    assert_eq!(found.status, 0, "init writes the config: {}", found.stderr);
    let config: serde_json::Value =
        serde_json::from_str(&repo.read("archangel.config.json")).unwrap();
    assert_eq!(
        config["elements"],
        json!({ "lib": "lib/**", "tools": "tools/**" }),
        "a folder at the top with a source file at any depth; not a file at the top, not docs"
    );
    assert_eq!(
        found.stdout,
        "wrote archangel.config.json with 2 elements: lib, tools\n\
         Order the elements with a layers rule, then run `archangel check`; `archangel docs layers` shows how.\n",
        "it says what it wrote and what to do next"
    );
}

/// The package's own entry and the module it re-exports, standing in for the `archangel` that a
/// TypeScript config imports.
fn install_archangel(repo: &FixtureRepo) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    repo.write(
        "node_modules/@enhance-eng/archangel/package.json",
        r#"{ "name": "@enhance-eng/archangel", "type": "module", "main": "index.js" }"#,
    );
    let copied = [
        (
            "npm/archangel/index.js",
            "node_modules/@enhance-eng/archangel/index.js",
        ),
        (
            "npm/archangel/plugin.js",
            "node_modules/@enhance-eng/archangel/plugin.js",
        ),
        (
            "npm/archangel/presets/fsd.js",
            "node_modules/@enhance-eng/archangel/presets/fsd.js",
        ),
        (
            "npm/archangel/presets/fsd-conditions.js",
            "node_modules/@enhance-eng/archangel/presets/fsd-conditions.js",
        ),
        (
            "npm/archangel/presets/nextjs.js",
            "node_modules/@enhance-eng/archangel/presets/nextjs.js",
        ),
        (
            "node_modules/pluralize/package.json",
            "node_modules/pluralize/package.json",
        ),
        (
            "node_modules/pluralize/pluralize.js",
            "node_modules/pluralize/pluralize.js",
        ),
    ];
    for (from, to) in copied {
        let text =
            std::fs::read_to_string(root.join(from)).unwrap_or_else(|e| panic!("{from}: {e}"));
        repo.write(to, &text);
    }
}

#[test]
fn init_typescript_quotes_only_the_names_that_are_not_identifiers() {
    let repo = ["app", "_internal", "$lib", "2d", "my-feature"]
        .iter()
        .fold(FixtureRepo::new(), |repo, name| {
            repo.file(&format!("src/{name}/index.ts"), "")
        });
    let found = run(&repo, &["init", "--typescript"]);
    assert_eq!(found.status, 0, "init writes the config: {}", found.stderr);
    assert_eq!(
        repo.read("archangel.config.ts"),
        "import { defineConfig } from '@enhance-eng/archangel';\n\
         \n\
         export default defineConfig({\n  \
         elements: {\n    \
         $lib: 'src/$lib/**',\n    \
         '2d': 'src/2d/**',\n    \
         _internal: 'src/_internal/**',\n    \
         app: 'src/app/**',\n    \
         'my-feature': 'src/my-feature/**',\n  \
         },\n  \
         rules: ({ rule }) => [\n    \
         rule({\n      \
         name: 'no-cycles',\n      \
         files: '**',\n      \
         noCycles: true,\n      \
         examples: {\n        \
         accepted: [{ 'cycle/a.ts': 'import \"./b\";\\n', 'cycle/b.ts': 'export {};\\n' }],\n        \
         rejected: [{ 'cycle/a.ts': 'import \"./b\";\\n', 'cycle/b.ts': 'import \"./a\";\\n' }],\n      \
         },\n    \
         }),\n  \
         ],\n\
         });\n",
        "a name that starts with a letter, _ or $ and goes on with letters, digits, _ or $ is bare"
    );
    install_archangel(&repo);
    for name in ["2d", "my-feature", "$lib"] {
        let explained = run(&repo, &["explain", &format!("src/{name}/index.ts")]);
        assert!(
            explained
                .stdout
                .contains(&format!("  element: \"{name}\"\n")),
            "the config loads, with the element named {name}: {}{}",
            explained.stdout,
            explained.stderr
        );
    }
}

#[test]
fn init_never_writes_over_a_config() {
    let repo = sources().json_config(&json!({ "elements": { "app": "src/app/**" }, "rules": [] }));
    let before = repo.read("archangel.config.json");
    let found = run(&repo, &["init", "--typescript"]);
    assert_eq!(found.status, 2, "an existing config stops it");
    assert!(
        found
            .stderr
            .contains("archangel.config.json already exists"),
        "it says why: {}",
        found.stderr
    );
    assert_eq!(
        repo.read("archangel.config.json"),
        before,
        "the config is unchanged"
    );
    assert!(
        !repo.join("archangel.config.ts").exists(),
        "and no second config is written"
    );
}

#[test]
fn init_with_no_source_folders_says_so() {
    let found = run(&FixtureRepo::new().file("README.md", ""), &["init"]);
    assert_eq!(found.status, 2, "nothing to name elements after");
    assert!(
        found.stderr.contains("found no folder of source files"),
        "it says why: {}",
        found.stderr
    );
}

#[test]
fn color_always_colors_a_pipe_and_never_colors_nothing() {
    let repo = sources()
        .file("src/shared/bad.ts", "import '../app/main';\n")
        .json_config(&json!({
            "elements": { "app": "src/app/**", "shared": "src/shared/**" },
            "rules": [{ "kind": "layers", "order": ["app", "shared"] }],
        }));
    assert!(
        run(&repo, &["check", "--color", "always"])
            .stdout
            .contains('\u{1b}'),
        "always colors a pipe"
    );
    assert!(
        !run(&repo, &["check", "--color", "never"])
            .stdout
            .contains('\u{1b}'),
        "never does not"
    );
}

#[test]
fn completions_print_a_script_for_each_shell() {
    for (shell, marker) in [
        ("bash", "complete -F"),
        ("zsh", "#compdef archangel"),
        ("fish", "complete -c archangel"),
        ("powershell", "Register-ArgumentCompleter"),
    ] {
        let found = run(&FixtureRepo::new(), &["completions", shell]);
        assert_eq!(found.status, 0, "{shell}: {}", found.stderr);
        assert!(found.stdout.contains(marker), "{shell} gets its own script");
        assert!(
            found.stdout.contains("baseline"),
            "{shell}: the script knows the commands"
        );
    }
}

#[test]
fn man_prints_a_roff_page_naming_every_command() {
    let found = run(&FixtureRepo::new(), &["man"]);
    assert_eq!(found.status, 0, "{}", found.stderr);
    assert!(
        found.stdout.starts_with(".ie \\n(.g .ds Aq"),
        "roff: {}",
        &found.stdout[..80]
    );
    for command in [
        "check", "test", "explain", "why", "watch", "baseline", "lsp", "init", "docs",
    ] {
        assert!(found.stdout.contains(command), "the page names {command}");
    }
}
