//! Scenarios from the presets spec that need the binary and Node: the Feature-Sliced Design and
//! Next.js presets that the npm package ships, composed into a config with `extends`.
#![cfg(test)]

#[path = "support/cli.rs"]
mod support;

use std::path::Path;

use archangel_testkit::FixtureRepo;
use serde_json::Value;
use support::run;

/// The npm package's entry file as a `file:` URL, which a config imports the presets from.
fn package_url() -> String {
    let entry = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../npm/archangel/index.js");
    let plain = dunce::canonicalize(entry)
        .unwrap()
        .display()
        .to_string()
        .replace('\\', "/");
    if plain.starts_with('/') {
        format!("file://{plain}")
    } else {
        format!("file:///{plain}")
    }
}

/// Writes `archangel.config.mjs`, whose default export is `body`, a JavaScript object that may
/// call `fsd` and `nextjs`.
fn with_config(repo: FixtureRepo, body: &str) -> FixtureRepo {
    let url = package_url();
    repo.file(
        "archangel.config.mjs",
        &format!("import {{ fsd, nextjs }} from '{url}';\nexport default {body};\n"),
    )
}

/// A small Feature-Sliced Design app under `root`: shared code, an entity that one feature uses,
/// a page and the app.
fn fsd_app(repo: FixtureRepo, root: &str) -> FixtureRepo {
    [
        ("shared/ui/button/index.ts", "export const Button = 1;\n"),
        ("entities/user/index.ts", "export { User } from './model/user';\n"),
        ("entities/user/model/user.ts", "export const User = 1;\n"),
        ("features/login/index.ts", "export { login } from './model/login';\n"),
        (
            "features/login/model/login.ts",
            "import { User } from '../../../entities/user';\nexport const login = User;\n",
        ),
        ("pages/home/index.ts", "export { Home } from './ui/home';\n"),
        (
            "pages/home/ui/home.ts",
            "import { login } from '../../../features/login';\nimport { Button } from '../../../shared/ui/button';\nexport const Home = [login, Button];\n",
        ),
        (
            "app/main.ts",
            "import { Home } from '../pages/home';\nexport const main = Home;\n",
        ),
    ]
    .iter()
    .fold(repo, |repo, (path, text)| {
        repo.file(&format!("{root}/{path}"), text)
    })
}

fn document(stdout: &str) -> Value {
    serde_json::from_str(stdout).unwrap_or_else(|e| panic!("one JSON document: {e}\n{stdout}"))
}

/// The diagnostics of a check, as (code, rule, file).
fn diagnostics(repo: &FixtureRepo) -> Vec<(String, String, String)> {
    let found = run(repo, &["check", "--format", "json"]);
    assert!(
        found.status <= 1,
        "the check ran: {}{}",
        found.stdout,
        found.stderr
    );
    document(&found.stdout)["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|d| {
            let text = |key: &str| d[key].as_str().unwrap_or_default().to_owned();
            (text("code"), text("rule"), text("file"))
        })
        .collect()
}

#[test]
fn off_drops_a_preset_rule() {
    let on = with_config(
        fsd_app(FixtureRepo::new(), "src"),
        "{ extends: [fsd({ root: 'src' })] }",
    );
    assert!(
        diagnostics(&on)
            .iter()
            .any(|(_, rule, _)| rule == "fsd/insignificant-slice"),
        "without off the repository has an insignificant slice: {:?}",
        diagnostics(&on)
    );
    let off = with_config(
        fsd_app(FixtureRepo::new(), "src"),
        "{ extends: [fsd({ root: 'src' })], off: ['fsd/insignificant-slice'] }",
    );
    let found = diagnostics(&off);
    assert!(
        found
            .iter()
            .all(|(_, rule, _)| rule != "fsd/insignificant-slice"),
        "no diagnostic names the rule: {found:?}"
    );
}

#[test]
fn one_preset_twice_under_two_roots() {
    let repo = fsd_app(fsd_app(FixtureRepo::new(), "apps/web/src"), "apps/admin/src")
        .file("apps/web/src/features/cart/model/cart.ts", "export const cart = 1;\n")
        .file("apps/admin/src/features/audit/model/audit.ts", "export const audit = 1;\n")
        .file(
            "apps/web/src/features/login/model/admin.ts",
            "import { login } from '../../../../../admin/src/features/login';\nexport const admin = login;\n",
        );
    let repo = with_config(
        repo,
        "{ extends: [fsd({ root: 'apps/web/src' }), fsd({ root: 'apps/admin/src', name: 'fsd-admin' })] }",
    );
    let found = diagnostics(&repo);
    assert!(
        found.iter().any(|(_, rule, file)| rule == "fsd/public-api"
            && file == "apps/web/src/features/cart"),
        "fsd/public-api applies to the web app: {found:?}"
    );
    assert!(
        found
            .iter()
            .any(|(_, rule, file)| rule == "fsd-admin/public-api"
                && file == "apps/admin/src/features/audit"),
        "fsd-admin/public-api applies to the admin app: {found:?}"
    );
    for (_, rule, file) in &found {
        let app = if rule.starts_with("fsd-admin/") {
            "apps/admin/"
        } else {
            "apps/web/"
        };
        assert!(
            file.starts_with(app),
            "{rule} reports only in its own app, not {file}"
        );
        assert_ne!(
            file, "apps/web/src/features/login/model/admin.ts",
            "the import between apps breaks no rule: {rule}"
        );
    }
}

#[test]
fn a_presets_unused_layers_are_listed_not_failed() {
    let repo = with_config(
        fsd_app(FixtureRepo::new(), "src"),
        "{ extends: [fsd({ root: 'src' })] }",
    );
    let found = run(&repo, &["check", "--format", "json"]);
    let checked = document(&found.stdout);
    let codes: Vec<&str> = checked["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|d| d["code"].as_str())
        .collect();
    assert!(
        !codes
            .iter()
            .any(|c| *c == "selector-empty" || *c == "element-empty"),
        "no empty-element or empty-selector diagnostic: {codes:?}"
    );
    let listed: Vec<&str> = checked["summary"]["presetNamesMatchingNothing"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    for name in ["fsd/widgets", "fsd/processes"] {
        assert!(
            listed.contains(&name),
            "the summary lists {name}: {listed:?}"
        );
    }
}

#[test]
fn a_preset_that_matches_nothing_is_preset_empty() {
    let repo = with_config(
        fsd_app(FixtureRepo::new(), "src"),
        "{ extends: [fsd({ root: 'scr' })] }",
    );
    let found = run(&repo, &["check", "--format", "json"]);
    assert_eq!(
        found.status, 1,
        "an empty preset fails the check: {}",
        found.stdout
    );
    let checked = document(&found.stdout);
    let empty: Vec<&Value> = checked["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|d| d["code"] == "preset-empty")
        .collect();
    assert_eq!(empty.len(), 1, "one preset-empty: {}", found.stdout);
    let message = empty[0]["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("\"fsd\"") && message.contains("scr"),
        "the message names the preset and its root: {message}"
    );
}

#[test]
fn preset_examples_are_checked_under_the_presets_root() {
    let repo = with_config(
        fsd_app(FixtureRepo::new(), "apps/web/src"),
        "{ extends: [fsd({ root: 'apps/web/src', strict: true })] }",
    );
    let found = run(&repo, &["test", "--format", "json"]);
    let tested = document(&found.stdout);
    assert_eq!(
        found.status, 0,
        "every preset rule passes: {}",
        found.stdout
    );
    let runs = tested["examples"].as_array().cloned().unwrap_or_default();
    let rules: std::collections::BTreeSet<&str> =
        runs.iter().filter_map(|r| r["rule"].as_str()).collect();
    assert_eq!(
        rules.len(),
        21,
        "each of the 21 rules has examples: {rules:?}"
    );
    for example in &runs {
        for file in example["files"].as_array().into_iter().flatten() {
            let file = file.as_str().unwrap_or_default();
            assert!(
                file.starts_with("apps/web/src/"),
                "{} places {file} under the preset's root",
                example["rule"]
            );
        }
    }
}

/// A Next.js app with a client component, `app/page.tsx`, whose helper imports `imported`.
fn nextjs_app(imported: &str) -> FixtureRepo {
    let repo = FixtureRepo::new()
        .file(
            "package.json",
            r#"{ "name": "web", "dependencies": { "next": "15.0.0", "server-only": "0.0.1" } }"#,
        )
        .file("node_modules/server-only/package.json", r#"{ "name": "server-only", "main": "index.js" }"#)
        .file("node_modules/server-only/index.js", "")
        .file(
            "app/page.tsx",
            "'use client';\nimport { helper } from './helper';\nexport default function Page() { return helper; }\n",
        )
        .file(
            "app/helper.ts",
            &format!("import '{imported}';\nexport const helper = 1;\n"),
        );
    with_config(repo, "{ extends: [nextjs()] }")
}

/// The one violation of `nextjs/client-stays-client`, with its chain.
fn client_violation(repo: &FixtureRepo) -> Value {
    let found = run(repo, &["check", "--format", "json"]);
    let checked = document(&found.stdout);
    let violations: Vec<&Value> = checked["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|d| d["rule"] == "nextjs/client-stays-client")
        .collect();
    assert_eq!(violations.len(), 1, "one violation: {}", found.stdout);
    assert_eq!(found.status, 1, "the violation fails the check");
    violations[0].clone()
}

#[test]
fn nextjs_refuses_server_only_code_reached_from_a_client_component() {
    let violation = client_violation(&nextjs_app("server-only"));
    assert_eq!(violation["file"], "app/page.tsx");
    let chain = violation["chain"].to_string();
    assert!(
        chain.contains("app/helper.ts") && chain.contains("server-only"),
        "the chain runs from the component to server-only: {chain}"
    );
}

#[test]
fn nextjs_refuses_node_modules_reached_from_a_client_component() {
    let violation = client_violation(&nextjs_app("node:fs"));
    assert_eq!(violation["file"], "app/page.tsx");
    let chain = violation["chain"].to_string();
    assert!(
        chain.contains("app/helper.ts") && chain.contains("node:fs"),
        "the chain runs from the component to node:fs: {chain}"
    );
}

#[test]
fn nextjs_permits_server_actions_in_client_components() {
    let repo = nextjs_app("server-only")
        .file(
            "app/page.tsx",
            "'use client';\nimport { save } from './actions';\nexport default function Page() { return save; }\n",
        )
        .file(
            "app/actions.ts",
            "'use server';\nimport 'server-only';\nimport { readFile } from 'node:fs';\nexport async function save() { return readFile; }\n",
        );
    let found = run(&repo, &["check", "--format", "json"]);
    let rules: Vec<String> = document(&found.stdout)["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|d| d["rule"].as_str().map(str::to_owned))
        .collect();
    assert!(
        !rules.iter().any(|r| r == "nextjs/client-stays-client"),
        "a server action breaks no rule: {}",
        found.stdout
    );
}

/// A preset that declares a table, with the rest of the config's own text.
fn with_preset_table(repo: FixtureRepo, kinds: &str, own: &str) -> FixtureRepo {
    with_config(
        repo,
        &format!(
            "{{ extends: [{{ name: 'fsd-x', parts: {{ src: 'src/**' }}, rules: ({{ mayImport }}) => \
             [mayImport('roles', {{ entity: [] }}, {{ kinds: '{kinds}' }})] }}], {own} }}"
        ),
    )
}

#[test]
fn a_presets_table_row_is_named_with_the_preset_prefix() {
    let files = || {
        FixtureRepo::new()
            .file("src/b.use-case.ts", "")
            .file("src/a.entity.ts", "import './b.use-case';\n")
    };
    let repo = with_preset_table(files(), "**/*.{kind}.ts", "");
    let found = diagnostics(&repo);
    assert!(
        found
            .iter()
            .any(|(_, rule, _)| rule == "fsd-x/roles.entity"),
        "{found:?}"
    );
    let repo = with_preset_table(
        files(),
        "**/*.{kind}.ts",
        "rules: ({ rule }) => [rule('fsd-x/roles.entity', { files: 'src/**/*.entity.ts', mustBeNamed: 'kebab-case' })],",
    );
    let found = diagnostics(&repo);
    assert!(
        !found
            .iter()
            .any(|(_, rule, _)| rule == "fsd-x/roles.entity"),
        "a config rule of that name replaces the row: {found:?}"
    );
}

#[test]
fn a_presets_table_with_no_file_is_listed_not_failed() {
    let repo = with_preset_table(
        FixtureRepo::new().file("src/plain.ts", ""),
        "**/*.{kind}.ts",
        "",
    );
    let found = run(&repo, &["check"]);
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    assert!(
        found.stdout.contains(
            "fsd-x/roles matches no file: no file matches its kind pattern **/*.{kind}.ts"
        ),
        "{}",
        found.stdout
    );
    let checked = document(&run(&repo, &["check", "--format", "json"]).stdout);
    let listed: Vec<&str> = checked["summary"]["presetNamesMatchingNothing"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    assert!(listed.contains(&"fsd-x/roles"), "{listed:?}");
}
