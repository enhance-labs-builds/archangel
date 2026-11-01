//! Scenarios from the configuration spec: loading a config from TypeScript through Node, and from
//! JSON.
#![cfg(test)]

use archangel_config::{ConfigProblem, ProblemCode, RuleKind, discover};
use archangel_engine::load_config as load;
use archangel_testkit::FixtureRepo;

const TYPESCRIPT_CONFIG: &str = r"
type Config = { elements: Record<string, string>; rules: unknown };
const config: Config = {
  elements: { app: 'src/app/**', shared: 'src/shared/**' },
  rules: ({ layers }: { layers: (order: string[]) => unknown }) => [layers(['app', 'shared'])],
};
export default config;
";

const JSON_CONFIG: &str = r#"{
  "elements": { "app": "src/app/**", "shared": "src/shared/**" },
  "rules": [{ "kind": "layers", "order": ["app", "shared"] }]
}"#;

#[test]
fn typescript_config_with_a_rules_function_loads() {
    let repo = FixtureRepo::new().file("archangel.config.ts", TYPESCRIPT_CONFIG);
    let config = load(&repo.join("archangel.config.ts")).expect("the config loads");
    assert_eq!(config.elements.len(), 2, "both elements load");
    assert_eq!(config.rules.len(), 1, "the rules function returns one rule");
    assert!(
        matches!(config.rules[0].kind, RuleKind::Layers { .. }),
        "the rule is a layers rule"
    );
}

#[test]
fn json_and_typescript_configs_with_equal_content_load_equal() {
    let repo = FixtureRepo::new()
        .file("archangel.config.ts", TYPESCRIPT_CONFIG)
        .file("archangel.config.json", JSON_CONFIG);
    let mut from_ts = load(&repo.join("archangel.config.ts")).expect("the TypeScript config loads");
    let from_json = load(&repo.join("archangel.config.json")).expect("the JSON config loads");
    // Only a builder call has a line, which is not part of what the rule means.
    assert!(from_ts.rules[0].source.is_some(), "the builder's line");
    from_ts.rules[0].source = None;
    assert_eq!(
        format!("{from_ts:?}"),
        format!("{from_json:?}"),
        "the two configs are equal"
    );
}

#[test]
fn json_config_that_does_not_parse_is_config_invalid_with_its_line() {
    let text = "{\n  \"elements\": {\n    \"app\": \"src/app/**\"\n  },\n  \"rules\": []\n  \"ignore\": []\n}\n";
    let repo = FixtureRepo::new().file("archangel.config.json", text);
    let problems = load(&repo.join("archangel.config.json")).expect_err("the config is refused");
    assert_eq!(problems.len(), 1, "one problem");
    assert_eq!(
        problems[0].code,
        ProblemCode::ConfigInvalid,
        "text that does not parse is invalid"
    );
    let (line, column) = problems[0].position.expect("the problem has a position");
    assert_eq!(line, 6, "the missing comma is before line 6");
    assert!(
        column > 0,
        "columns count from 1, as in every other diagnostic"
    );
}

#[test]
fn discover_finds_the_nearest_config_at_or_above_the_folder() {
    let repo = FixtureRepo::new()
        .file("archangel.config.json", JSON_CONFIG)
        .file("packages/web/archangel.config.json", JSON_CONFIG)
        .file("packages/web/archangel.config.ts", TYPESCRIPT_CONFIG)
        .file("packages/web/src/main.ts", "")
        .file("packages/api/src/main.ts", "");
    assert_eq!(
        [
            discover(&repo.join("packages/web/src")),
            discover(&repo.join("packages/web")),
            discover(&repo.join("packages/api/src")),
        ],
        [
            Some(repo.join("packages/web/archangel.config.ts")),
            Some(repo.join("packages/web/archangel.config.ts")),
            Some(repo.join("archangel.config.json")),
        ],
        "the nearest folder with a config wins, and in one folder TypeScript comes before JSON"
    );
}

#[test]
fn discover_finds_nothing_in_a_folder_with_no_config_at_or_above_it() {
    let repo = FixtureRepo::new().file("src/main.ts", "");
    assert_eq!(
        discover(&repo.join("src")),
        None,
        "no config anywhere above"
    );
}

/// The problem with a TypeScript config, from `load`.
fn typescript_problem(text: &str) -> ConfigProblem {
    let repo = FixtureRepo::new().file("archangel.config.ts", text);
    let mut problems = load(&repo.join("archangel.config.ts")).expect_err("the config is refused");
    assert_eq!(problems.len(), 1, "one problem: {problems:?}");
    let problem = problems.remove(0);
    assert_eq!(
        (problem.code, &problem.field, problem.position),
        (ProblemCode::ConfigLoadFailed, &None, None),
        "a load failure is about the whole file: {problem:?}"
    );
    problem
}

/// Node's own message, which archangel puts in parentheses after its advice.
fn node_message<'m>(message: &'m str, advice: &str) -> &'m str {
    message
        .strip_prefix(advice)
        .and_then(|rest| rest.strip_prefix(" ("))
        .and_then(|rest| rest.strip_suffix(')'))
        .unwrap_or_else(|| panic!("`{message}` gives the advice `{advice}` and Node's message"))
}

#[test]
fn a_config_without_a_default_export_says_how_to_write_one() {
    assert_eq!(
        typescript_problem("export const config = { elements: { app: 'src/**' } };\n").message,
        "the config has no default export; write `export default defineConfig({ ... })`",
        "the problem names the export to write"
    );
}

#[test]
fn a_config_that_throws_gives_its_own_message() {
    assert_eq!(
        typescript_problem("throw new Error('bad element list');\nexport default {};\n").message,
        "the config threw an error while it loaded: bad element list",
        "the config's own message"
    );
}

#[test]
fn a_config_with_an_enum_names_the_constant_object_to_use_instead() {
    let problem = typescript_problem("enum Layer { App }\nexport default {};\n");
    let node = node_message(
        &problem.message,
        "the config declares an enum, which Node cannot run because it only strips types; use a \
         constant object with `as const` instead",
    );
    assert!(
        node.contains("enum"),
        "Node's message names the enum: {node}"
    );
}

#[test]
fn a_config_with_typescript_node_cannot_strip_says_to_write_plain_javascript() {
    let problem = typescript_problem("namespace N { export const a = 1; }\nexport default {};\n");
    let node = node_message(
        &problem.message,
        "the config uses TypeScript that Node cannot strip, such as a namespace or a parameter \
         property; write it as plain JavaScript with types",
    );
    assert!(
        node.contains("namespace"),
        "Node's message names the namespace: {node}"
    );
}

#[test]
fn a_config_importing_a_missing_file_explains_how_node_finds_imports() {
    let problem = typescript_problem("import layers from './layers';\nexport default layers;\n");
    let node = node_message(
        &problem.message,
        "the config imports a file Node cannot find; Node needs the `.ts` extension on relative \
         imports, and does not use tsconfig path aliases",
    );
    assert!(
        node.contains("layers"),
        "Node's message names the file: {node}"
    );
}

/// A folder whose `node` prints `version` for `--version` and runs the real Node, named by
/// `ARCHANGEL_REAL_NODE`, for anything else.
#[cfg(unix)]
fn fake_node(version: &str) -> FixtureRepo {
    use std::os::unix::fs::PermissionsExt;
    let bin = FixtureRepo::new().file(
        "node",
        &format!(
            "#!/bin/sh\nif [ \"$1\" = --version ]; then echo {version}; else exec \"$ARCHANGEL_REAL_NODE\" \"$@\"; fi\n"
        ),
    );
    std::fs::set_permissions(bin.join("node"), std::fs::Permissions::from_mode(0o755)).unwrap();
    bin
}

/// Runs `test` again with only a fake `node` of `version` on the search path; true in that run.
#[cfg(unix)]
fn with_node_version(test: &str, version: &str) -> bool {
    let bin = fake_node(version);
    let real = std::env::var_os("PATH")
        .and_then(|path| {
            std::env::split_paths(&path)
                .map(|folder| folder.join("node"))
                .find(|candidate| candidate.is_file())
        })
        .expect("a real Node on the search path");
    archangel_testkit::in_child_with_env(
        test,
        &[
            ("PATH", bin.path().as_os_str()),
            ("ARCHANGEL_REAL_NODE", real.as_os_str()),
        ],
    )
}

#[cfg(unix)]
#[test]
fn a_node_older_than_22_18_is_refused_before_it_runs_the_config() {
    if !with_node_version(
        "a_node_older_than_22_18_is_refused_before_it_runs_the_config",
        "v22.17.9",
    ) {
        return;
    }
    let repo = FixtureRepo::new().file("archangel.config.ts", TYPESCRIPT_CONFIG);
    let problems = load(&repo.join("archangel.config.ts")).expect_err("the config is refused");
    assert_eq!(
        problems,
        [ConfigProblem {
            code: ProblemCode::ConfigNeedsNode,
            message: "the `node` on the search path is version 22.17.9, and archangel needs 22.18 \
                      or later to run a TypeScript config. A JSON config, archangel.config.json, \
                      needs no Node."
                .to_owned(),
            field: None,
            position: None,
        }],
        "the last minor version before 22.18 is too old, and the problem names both versions"
    );
}

#[cfg(unix)]
#[test]
fn node_22_18_runs_a_typescript_config() {
    if !with_node_version("node_22_18_runs_a_typescript_config", "v22.18.0") {
        return;
    }
    let repo = FixtureRepo::new().file("archangel.config.ts", TYPESCRIPT_CONFIG);
    let config = load(&repo.join("archangel.config.ts")).expect("22.18 is new enough");
    assert_eq!(config.rules.len(), 1, "the config loads through Node");
}
