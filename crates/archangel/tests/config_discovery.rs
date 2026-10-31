//! Scenarios from the configuration spec that need the binary: finding the config, loading it
//! through Node, and stopping with status 2 when it cannot be used.
#![cfg(test)]

#[path = "support/cli.rs"]
mod support;

use archangel_testkit::FixtureRepo;
use serde_json::json;
use support::{Finished, command, run};

const ELEMENTS: &str = "{ app: 'src/app/**', shared: 'src/shared/**' }";

fn json_config() -> serde_json::Value {
    json!({
        "elements": { "app": "src/app/**", "shared": "src/shared/**" },
        "rules": [{ "kind": "layers", "order": ["app", "shared"] }],
    })
}

fn sources(repo: FixtureRepo) -> FixtureRepo {
    repo.file("src/app/main.ts", "import '../shared/s';\n")
        .file("src/shared/s.ts", "")
}

fn typescript(body: &str) -> FixtureRepo {
    sources(FixtureRepo::new()).file("archangel.config.ts", body)
}

fn typescript_config(before: &str) -> String {
    format!(
        "{before}\nexport default {{ elements: {ELEMENTS}, rules: ({{ layers }}: any) => [layers(['app', 'shared'])] }};\n"
    )
}

/// Runs `archangel check` with only `folder` on the search path.
#[cfg(unix)]
fn check_with_path(repo: &FixtureRepo, folder: &std::path::Path) -> Finished {
    command(repo)
        .arg("check")
        .env("PATH", folder)
        .output()
        .unwrap()
        .into()
}

/// A folder on the search path whose `node` prints `version`.
#[cfg(unix)]
fn fake_node(repo: FixtureRepo, version: &str) -> FixtureRepo {
    use std::os::unix::fs::PermissionsExt;
    let repo = repo.file(".bin/node", &format!("#!/bin/sh\necho {version}\n"));
    std::fs::set_permissions(
        repo.join(".bin/node"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    repo
}

#[test]
fn uses_the_config_in_the_nearest_parent_folder() {
    let repo = sources(FixtureRepo::new())
        .file("src/shared/bad.ts", "import '../app/main';\n")
        .file("packages/web/readme.md", "")
        .json_config(&json_config());
    let found = command(&repo)
        .current_dir(repo.join("packages/web"))
        .arg("check")
        .output()
        .unwrap();
    let found = Finished::from(found);
    assert_eq!(
        found.status, 1,
        "the top config finds the violation: {}",
        found.stdout
    );
    assert!(
        found.stdout.contains("src/shared/bad.ts"),
        "the path is relative to the top folder: {}",
        found.stdout
    );
}

#[test]
fn config_option_replaces_the_search() {
    let repo = sources(FixtureRepo::new())
        .json_config(&json!({ "elements": { "x": "nowhere/**" }, "rules": [] }))
        .file("archangel.strict.json", &json_config().to_string());
    let found = run(&repo, &["check", "--config", "archangel.strict.json"]);
    assert_eq!(
        found.status, 0,
        "the strict config is clean, and the other would be element-empty: {}",
        found.stdout
    );
}

#[test]
fn no_config_stops_with_config_missing() {
    let repo = sources(FixtureRepo::new());
    let found = run(&repo, &["check"]);
    assert_eq!(found.status, 2, "no config exits 2");
    assert!(
        found.stderr.contains("config-missing") && found.stderr.contains("archangel.config.ts"),
        "it names the files it looked for: {}",
        found.stderr
    );
}

#[test]
fn json_config_needs_no_node() {
    #[cfg(unix)]
    {
        let repo = sources(FixtureRepo::new()).json_config(&json_config());
        let found = check_with_path(&repo, &repo.join("no-such-folder"));
        assert_eq!(
            found.status, 0,
            "the check runs without Node: {}{}",
            found.stdout, found.stderr
        );
    }
}

#[test]
fn json_config_that_does_not_parse_exits_two() {
    let text = "{\n  \"elements\": {\n    \"app\": \"src/app/**\",\n    \"shared\": \"src/shared/**\"\n  },\n  \"rules\": []\n  \"ignore\": []\n}\n";
    let repo = sources(FixtureRepo::new()).file("archangel.config.json", text);
    let found = run(&repo, &["check"]);
    assert_eq!(found.status, 2, "a config that does not parse exits 2");
    assert!(
        found.stderr.contains("config-invalid")
            && found.stderr.contains("archangel.config.json:7:"),
        "it gives the line and column: {}",
        found.stderr
    );
}

#[test]
fn invalid_config_exits_two() {
    let repo = sources(FixtureRepo::new()).json_config(&json!({
        "elements": { "app": "src/app/**", "shared": "src/shared/**" },
        "rules": [{ "kind": "layers", "order": ["app", "shared"], "reasn": "typo" }],
    }));
    let found = run(&repo, &["check"]);
    assert_eq!(found.status, 2, "an unknown field exits 2");
    assert!(
        found.stderr.contains("config-invalid") && found.stderr.contains("rules[0].reasn"),
        "it names the field: {}",
        found.stderr
    );
}

#[test]
fn invalid_pattern_exits_two() {
    let repo = sources(FixtureRepo::new()).json_config(&json!({
        "elements": { "ui": "src/ui**/x" },
        "rules": [{ "name": "no-cycles", "files": "**", "noCycles": true }],
    }));
    let found = run(&repo, &["check"]);
    assert_eq!(found.status, 2, "an invalid pattern exits 2");
    assert!(
        found.stderr.contains("pattern-invalid")
            && found.stderr.contains("\"ui\"")
            && found.stderr.contains("src/ui**/x"),
        "it names the element and the pattern: {}",
        found.stderr
    );
}

#[test]
fn rule_with_an_unknown_element_exits_two() {
    let repo = sources(FixtureRepo::new()).json_config(&json!({
        "elements": { "entity": "src/app/**", "react": { "packages": ["react"] } },
        "rules": [{ "files": "entity", "mayNotImport": "reactt" }],
    }));
    let found = run(&repo, &["check"]);
    assert_eq!(found.status, 2, "an unknown element exits 2");
    assert!(
        found.stderr.contains("rule-unknown-element")
            && found.stderr.contains("reactt")
            && found.stderr.contains("react"),
        "it names the element and the declared ones: {}",
        found.stderr
    );
}

#[test]
fn typescript_config_is_evaluated_once_per_check() {
    let repo = typescript(&typescript_config(
        "import { appendFileSync } from 'node:fs';\nappendFileSync(new URL('./counter.txt', import.meta.url), 'x\\n');",
    ));
    let repo = (0..20).fold(repo, |repo, i| {
        repo.file(&format!("src/shared/f{i}.ts"), "")
    });
    let found = run(&repo, &["check"]);
    assert_eq!(
        found.status, 0,
        "the check runs: {}{}",
        found.stdout, found.stderr
    );
    let counter = repo.read("counter.txt");
    assert_eq!(counter.lines().count(), 1, "Node evaluated the config once");
}

#[cfg(unix)]
#[test]
fn typescript_config_without_node_stops_with_config_needs_node() {
    let repo = typescript(&typescript_config(""));
    let found = check_with_path(&repo, &repo.join("no-such-folder"));
    assert_eq!(found.status, 2, "no Node exits 2");
    assert!(
        found.stderr.contains("config-needs-node") && found.stderr.contains("JSON"),
        "it says a JSON config needs no Node: {}",
        found.stderr
    );
}

#[cfg(unix)]
#[test]
fn typescript_config_with_an_old_node_names_both_versions() {
    let repo = fake_node(typescript(&typescript_config("")), "v20.11.0");
    let found = check_with_path(&repo, &repo.join(".bin"));
    assert_eq!(found.status, 2, "an old Node exits 2");
    assert!(
        found.stderr.contains("20.11.0") && found.stderr.contains("22.18"),
        "it names both versions: {}",
        found.stderr
    );
}

#[test]
fn config_that_throws_stops_with_its_own_message() {
    let repo = typescript("throw new Error('bad element list');\nexport default {};\n");
    let found = run(&repo, &["check"]);
    assert_eq!(found.status, 2, "a config that throws exits 2");
    assert!(
        found.stderr.contains("config-load-failed") && found.stderr.contains("bad element list"),
        "it gives the config's own message: {}",
        found.stderr
    );
}

#[test]
fn typescript_config_without_a_default_export_exits_two() {
    let repo = typescript(&format!(
        "export const config = {{ elements: {ELEMENTS}, rules: [] }};\n"
    ));
    let found = run(&repo, &["check"]);
    assert_eq!(found.status, 2, "no default export exits 2");
    assert!(
        found.stderr.contains("config-load-failed") && found.stderr.contains("default export"),
        "it says so: {}",
        found.stderr
    );
}

#[test]
fn config_with_an_enum_names_the_construct_and_the_alternative() {
    let repo = typescript(&typescript_config("enum Layer { App, Shared }"));
    let found = run(&repo, &["check"]);
    assert_eq!(found.status, 2, "an enum exits 2");
    assert!(
        found.stderr.contains("enum") && found.stderr.contains("object"),
        "it names the enum and the constant object: {}",
        found.stderr
    );
}

#[test]
fn config_output_does_not_break_the_load() {
    let quiet = run(&typescript(&typescript_config("")), &["check"]);
    let loud = run(
        &typescript(&typescript_config("console.log('hello from the config');")),
        &["check"],
    );
    assert_eq!(
        quiet.status, 0,
        "the check runs: {}{}",
        quiet.stdout, quiet.stderr
    );
    assert_eq!(
        (loud.status, loud.stdout.clone()),
        (quiet.status, quiet.stdout.clone()),
        "the same result"
    );
}
