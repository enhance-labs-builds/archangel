//! Scenarios from the rule-examples spec: each example is judged by its own rule, in memory,
//! against the repository.
#![cfg(test)]

use archangel_graph::{GraphOptions, ImportGraph, Work};
use archangel_rules::{Architecture, Code, ExampleKind, ExampleOutcome};
use archangel_testkit::FixtureRepo;
use serde_json::{Value, json};

fn elements() -> Value {
    json!({
        "app": "src/app/**",
        "feature": "src/features/{feature}/**",
        "entity": "src/entities/{entity}/**",
        "shared": "src/shared/**",
    })
}

/// Every example of every rule, in config order, with the work the examples took.
fn examples(rules: &Value, repo: &FixtureRepo) -> (Vec<ExampleOutcome>, Work) {
    examples_of(&json!({ "elements": elements(), "rules": rules }), repo)
}

/// Every example of every rule of a config, in config order, with the work the examples took.
fn examples_of(config: &Value, repo: &FixtureRepo) -> (Vec<ExampleOutcome>, Work) {
    let config = archangel_config::parse_json(&config.to_string()).expect("a valid config");
    let graph = ImportGraph::build(repo.path(), &GraphOptions::default(), &mut Work::default());
    let architecture = Architecture::compile(config);
    let mut work = Work::default();
    let mut outcomes = Vec::new();
    for (rule, definition) in architecture.config().rules.iter().enumerate() {
        let sets = [
            (ExampleKind::Accepted, &definition.examples.accepted),
            (ExampleKind::Rejected, &definition.examples.rejected),
        ];
        for (kind, files) in sets
            .into_iter()
            .flat_map(|(k, all)| all.iter().map(move |f| (k, f)))
        {
            let overlay = graph.overlay(files, &mut work);
            outcomes.push(architecture.check_example(&graph, &overlay, rule, kind, &mut work));
        }
    }
    (outcomes, work)
}

fn passed(outcomes: &[ExampleOutcome]) -> Vec<bool> {
    outcomes.iter().map(|o| o.passed).collect()
}

fn login() -> FixtureRepo {
    FixtureRepo::new().file("src/features/login/index.ts", "export const login = 1;\n")
}

#[test]
fn example_file_can_import_a_repository_file() {
    let rules = json!([{
        "kind": "layers", "order": ["app", "feature", "shared"],
        "examples": { "rejected": [{ "src/shared/x.ts": "import '../features/login/index';\n" }] },
    }]);
    let (outcomes, _) = examples(&rules, &login());
    assert_eq!(
        passed(&outcomes),
        [true],
        "layers rejects shared importing login: {outcomes:?}"
    );
}

#[test]
fn example_file_replaces_a_repository_file_for_that_example_only() {
    let repo = login().file("src/shared/a.ts", "export const a = 1;\n");
    let rules = json!([{
        "kind": "layers", "order": ["app", "feature", "shared"],
        "examples": {
            "rejected": [{ "src/shared/a.ts": "import '../features/login/index';\n" }],
            "accepted": [{ "src/app/main.ts": "import '../shared/a';\n" }],
        },
    }]);
    let (outcomes, _) = examples(&rules, &repo);
    assert_eq!(
        passed(&outcomes),
        [true, true],
        "(accepted, rejected): the replaced a.ts breaks the rule, and app importing a.ts does not: \
         {outcomes:?}"
    );
    assert_eq!(
        std::fs::read_to_string(repo.join("src/shared/a.ts")).unwrap(),
        "export const a = 1;\n",
        "the repository file keeps its bytes"
    );
}

#[test]
fn example_with_a_syntax_error_fails() {
    let rules = json!([{
        "kind": "layers", "order": ["app", "feature", "shared"],
        "examples": { "rejected": [{ "src/shared/x.ts": "import { from '../features/login/index';\n" }] },
    }]);
    let (outcomes, _) = examples(&rules, &login());
    assert_eq!(
        passed(&outcomes),
        [false],
        "a syntax error never passes: {outcomes:?}"
    );
    let parse = outcomes[0]
        .diagnostics
        .iter()
        .find(|d| d.code == Code::ParseFailed)
        .map(|d| d.file.as_deref());
    assert_eq!(
        parse,
        Some(Some("src/shared/x.ts")),
        "parse-failed names the example's file"
    );
}

#[test]
fn accepted_example_with_a_syntax_error_in_one_file_fails() {
    let rules = json!([{
        "kind": "isolate", "element": "feature",
        "examples": { "accepted": [{
            "src/features/login/a.ts": "import './index';\n",
            "src/features/login/b.ts": "import {;\n",
        }] },
    }]);
    let (outcomes, _) = examples(&rules, &login());
    assert_eq!(
        passed(&outcomes),
        [false],
        "a.ts alone would pass, so only b.ts's syntax error can fail it: {outcomes:?}"
    );
}

fn two_file_cycle() -> Value {
    json!([{
        "name": "no-cycles", "files": "**", "noCycles": true,
        "examples": { "rejected": [{
            "src/x/a.ts": "import './b';\n",
            "src/x/b.ts": "import './a';\n",
        }] },
    }])
}

/// A repository of many files, none of them in the example's cycle.
fn bystanders() -> FixtureRepo {
    (0..20).fold(FixtureRepo::new(), |repo, i| {
        repo.file(
            &format!("src/other/f{i}.ts"),
            &format!("import './f{}';\n", (i + 1) % 20),
        )
    })
}

#[test]
fn cycle_example_searches_only_its_own_files() {
    #[cfg(feature = "failpoints")]
    let _quiet = fail::FailScenario::setup();
    let (outcomes, work) = examples(&two_file_cycle(), &bystanders());
    assert_eq!(
        passed(&outcomes),
        [true],
        "the rule finds the two-file cycle: {outcomes:?}"
    );
    assert_eq!(
        work.cycle_files_visited, 2,
        "the search visits the two example files only"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn cycle_search_count_rejects_a_whole_graph_search_per_example() {
    let scenario = fail::FailScenario::setup();
    fail::cfg("rules::search_whole_graph_per_example", "return").unwrap();
    let (_, work) = examples(&two_file_cycle(), &bystanders());
    scenario.teardown();
    assert_ne!(
        work.cycle_files_visited, 2,
        "the count catches a search of the whole graph"
    );
}

#[test]
fn replaced_file_cycle_through_a_repository_file_is_found() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "export const a = 1;\n")
        .file("src/b.ts", "import './a';\n");
    let rules = json!([{
        "name": "no-cycles", "files": "**", "noCycles": true,
        "examples": { "accepted": [{ "src/a.ts": "import './b';\n" }] },
    }]);
    let (outcomes, _) = examples(&rules, &repo);
    assert_eq!(
        passed(&outcomes),
        [false],
        "the cycle through b.ts fails the example"
    );
    let cycle = outcomes[0]
        .diagnostics
        .iter()
        .find(|d| d.code == Code::NoCycles)
        .map(|d| d.cycle.clone());
    assert_eq!(
        cycle,
        Some(vec![
            "src/a.ts".to_owned(),
            "src/b.ts".to_owned(),
            "src/a.ts".to_owned()
        ]),
        "the cycle runs through the repository file"
    );
}

#[test]
fn rejected_example_caught_by_another_rule_fails_and_names_it() {
    let repo = FixtureRepo::new().file("src/entities/canvas/index.ts", "export const c = 1;\n");
    let rules = json!([
        { "kind": "layers", "name": "layers", "order": ["app", "feature", "entity", "shared"] },
        {
            "kind": "entry", "name": "entry", "element": "entity", "files": ["index.ts"],
            "examples": { "rejected": [{ "src/shared/x.ts": "import '../entities/canvas/index';\n" }] },
        },
    ]);
    let (outcomes, _) = examples(&rules, &repo);
    assert_eq!(
        passed(&outcomes),
        [false],
        "entry does not reject an import of index.ts"
    );
    let failure = outcomes[0].failure.as_ref().expect("a failure");
    assert_eq!(failure.code, Code::ExampleFailed, "the failure's code");
    assert!(
        failure.message.contains("\"layers\"") && failure.message.contains("\"entry\""),
        "the message names both rules: {}",
        failure.message
    );
}

fn isolate(accepted: &Value) -> Value {
    json!([{ "kind": "isolate", "element": "feature", "examples": { "accepted": [accepted] } }])
}

#[test]
fn accepted_example_that_its_rule_never_checks_proves_nothing() {
    let repo = FixtureRepo::new().file("src/shared/b.ts", "export const b = 1;\n");
    let (outcomes, _) = examples(
        &isolate(&json!({ "src/shared/a.ts": "import './b';\n" })),
        &repo,
    );
    assert_eq!(passed(&outcomes), [false], "isolate never looks at shared");
    assert_eq!(
        outcomes[0].failure.as_ref().map(|d| d.code),
        Some(Code::ExampleProvesNothing),
        "the failure's code"
    );
}

#[test]
fn accepted_example_that_its_rule_checks_passes() {
    let (outcomes, _) = examples(
        &isolate(&json!({ "src/features/login/a.ts": "import './index';\n" })),
        &login(),
    );
    assert_eq!(
        passed(&outcomes),
        [true],
        "login importing login is checked and allowed: {outcomes:?}"
    );
}

#[test]
fn rule_without_a_rejected_example_is_rule_unproven() {
    let config = archangel_config::parse_json(
        &json!({
            "elements": elements(),
            "rules": [
                { "name": "f", "files": "shared", "mayNotImport": "app",
                  "examples": { "accepted": [{ "src/shared/a.ts": "" }] } },
                { "name": "no-cycles", "files": "**", "noCycles": true,
                  "examples": { "rejected": [{ "src/a.ts": "import './a';\n" }] } },
            ],
        })
        .to_string(),
    )
    .expect("a valid config");
    let unproven: Vec<(Code, Option<String>)> = Architecture::compile(config)
        .unproven_rules()
        .into_iter()
        .map(|d| (d.code, d.rule))
        .collect();
    assert_eq!(
        unproven,
        [(Code::RuleUnproven, Some("f".to_owned()))],
        "only the forbid rule"
    );
}

/// The failure of each example, as (code, message), or `None` for one that passed.
fn failures(outcomes: &[ExampleOutcome]) -> Vec<Option<(Code, String)>> {
    outcomes
        .iter()
        .map(|o| o.failure.as_ref().map(|d| (d.code, d.message.clone())))
        .collect()
}

fn proves_nothing(rule: &str) -> (Code, String) {
    (
        Code::ExampleProvesNothing,
        format!(
            "\"{rule}\" never looked at an import of this example, so accepting it proves nothing; \
             give the example an import between the elements the rule names"
        ),
    )
}

#[test]
fn accepted_layers_example_with_one_end_outside_the_order_proves_nothing() {
    let repo = FixtureRepo::new().file("src/shared/b.ts", "export const b = 1;\n");
    let rules = json!([{
        "kind": "layers", "name": "layers", "order": ["app", "feature", "shared"],
        "examples": { "accepted": [{ "src/x.ts": "import './shared/b';\n" }] },
    }]);
    let (outcomes, _) = examples(&rules, &repo);
    assert_eq!(
        failures(&outcomes),
        [Some(proves_nothing("layers"))],
        "layers looks only at imports between two of its layers"
    );
}

#[test]
fn accepted_isolate_example_that_leaves_the_element_proves_nothing() {
    let repo = FixtureRepo::new().file("src/shared/b.ts", "export const b = 1;\n");
    let rules = json!([{
        "kind": "isolate", "name": "isolate", "element": "feature",
        "examples": { "accepted": [{ "src/features/login/a.ts": "import '../../shared/b';\n" }] },
    }]);
    let (outcomes, _) = examples(&rules, &repo);
    assert_eq!(
        failures(&outcomes),
        [Some(proves_nothing("isolate"))],
        "isolate looks only at imports from one instance of its element to another"
    );
}

#[test]
fn accepted_entry_example_through_the_entry_file_passes() {
    let repo = FixtureRepo::new().file("src/entities/canvas/index.ts", "export const c = 1;\n");
    let rules = json!([{
        "kind": "entry", "name": "entry", "element": "entity", "files": ["index.ts"],
        "examples": { "accepted": [{ "src/app/a.ts": "import '../entities/canvas/index';\n" }] },
    }]);
    let (outcomes, _) = examples(&rules, &repo);
    assert_eq!(
        failures(&outcomes),
        [None],
        "entry looks at every import into its element"
    );
}

#[test]
fn accepted_cycle_example_whose_files_import_each_other_without_a_cycle_passes() {
    let rules = json!([{
        "name": "no-cycles", "files": "**", "noCycles": true,
        "examples": { "accepted": [{ "src/x/a.ts": "import './b';\n", "src/x/b.ts": "" }] },
    }]);
    let (outcomes, _) = examples(&rules, &FixtureRepo::new());
    assert_eq!(
        failures(&outcomes),
        [None],
        "the search looked at a.ts importing b.ts"
    );
}

#[test]
fn accepted_cycle_example_whose_own_files_import_nothing_proves_nothing() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "export const a = 1;\n")
        .file("src/b.ts", "import './a';\n");
    let rules = json!([{
        "name": "cycles", "files": "**", "noCycles": true,
        "examples": { "accepted": [{ "src/a.ts": "export const a = 2;\n" }] },
    }]);
    let (outcomes, _) = examples(&rules, &repo);
    assert_eq!(
        failures(&outcomes),
        [Some((
            Code::ExampleProvesNothing,
            "\"cycles\" never looked at an import of this example, so accepting it proves \
             nothing; give the example an import between files in the rule's files"
                .to_owned()
        ))],
        "only the repository's b.ts has an import the search looked at"
    );
}

#[test]
fn accepted_cycle_example_is_not_failed_by_a_repository_cycle_outside_its_files() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "export const a = 1;\n")
        .file("src/b.ts", "import './a';\nimport './c';\n")
        .file("src/c.ts", "import './b';\n");
    let rules = json!([{
        "name": "cycles", "files": "**", "noCycles": true,
        "examples": { "accepted": [{ "src/a.ts": "import './z';\n", "src/z.ts": "" }] },
    }]);
    let (outcomes, _) = examples(&rules, &repo);
    assert_eq!(
        failures(&outcomes),
        [None],
        "the cycle of b.ts and c.ts is not the example's"
    );
    let groups: Vec<&[String]> = outcomes[0]
        .diagnostics
        .iter()
        .map(|d| d.group.as_slice())
        .collect();
    assert_eq!(
        groups,
        [["src/b.ts".to_owned(), "src/c.ts".to_owned()]],
        "the search still found the repository's cycle"
    );
}

#[test]
fn an_accepted_structure_example_passes_and_non_source_files_are_not_parsed() {
    let config = json!({
        "elements": { "slice": "src/features/{slice}/**" },
        "rules": [{
            "name": "slices-have-a-readme",
            "files": "slice",
            "mustContain": "README.md",
            "examples": {
                "accepted": [{ "src/features/x/README.md": "# x", "src/features/x/a.ts": "" }],
                "rejected": [{ "src/features/y/a.ts": "" }],
            },
        }],
    });
    let repo = FixtureRepo::new()
        .file("src/features/login/README.md", "# login\n")
        .file("src/features/login/a.ts", "");
    let (outcomes, work) = examples_of(&config, &repo);
    assert_eq!(
        passed(&outcomes),
        [true, true],
        "the instance with its README is accepted, the one without it rejected: {outcomes:?}"
    );
    assert_eq!(
        work.files_parsed, 2,
        "each example parses its one source file, and README.md is not parsed"
    );
}

#[test]
fn accepted_example_that_another_rule_of_its_kind_checks_proves_nothing() {
    let rules = json!([
        { "name": "shared", "files": "shared", "mayNotImport": "app",
          "examples": { "accepted": [{ "src/features/login/a.ts": "import './index';\n" }] } },
        { "name": "feature", "files": "feature", "mayNotImport": "app" },
    ]);
    let (outcomes, _) = examples(&rules, &login());
    assert_eq!(
        failures(&outcomes),
        [Some(proves_nothing("shared"))],
        "only the second rule looks at imports from a feature"
    );
}

#[test]
fn accepted_example_of_a_folder_rule_sees_the_repository_files_in_its_folder() {
    let rules = json!([{
        "name": "index", "folders": "src/features/*", "mustContain": "index.ts",
        "examples": { "accepted": [{ "src/features/login/a.ts": "" }] },
    }]);
    let (outcomes, _) = examples(&rules, &login());
    assert_eq!(
        failures(&outcomes),
        [None],
        "the folder login holds the repository's index.ts beside the example's file"
    );
}

#[test]
fn rejected_example_of_an_export_rule_is_judged_by_its_own_text() {
    let rules = json!([{
        "name": "exports", "files": "src/features/*/index.ts", "mustExport": "{folder-name}",
        "examples": { "rejected": [{ "src/features/login/index.ts": "export const other = 1;\n" }] },
    }]);
    let (outcomes, _) = examples(&rules, &login());
    assert_eq!(
        failures(&outcomes),
        [None],
        "the example's index.ts exports no `login`, though the repository's does"
    );
}

/// A transitive rule of runtime imports over the client files, which the repository's own client
/// file breaks through its import.
fn client_reaches_server(accepted: &Value) -> (Vec<ExampleOutcome>, Work) {
    let repo = FixtureRepo::new()
        .file("src/client/b.ts", "import '../server/db';\n")
        .file("src/server/db.ts", "export const db = 1;\n")
        .file("src/server/types.ts", "export type Row = number;\n");
    let config = json!({
        "elements": { "client": "src/client/**", "server": "src/server/**" },
        "rules": [{
            "name": "client", "files": "client", "mayNotImport": "src/server/db.ts",
            "transitively": true, "imports": "runtime",
            "examples": { "accepted": [accepted] },
        }],
    });
    examples_of(&config, &repo)
}

#[test]
fn accepted_transitive_example_proves_nothing_through_the_imports_of_the_repository() {
    let (outcomes, _) = client_reaches_server(&json!({ "src/client/a.ts": "" }));
    assert_eq!(
        failures(&outcomes),
        [Some(proves_nothing("client"))],
        "the example's file imports nothing; only the repository's b.ts does"
    );
}

#[test]
fn accepted_transitive_example_proves_nothing_through_imports_its_rule_does_not_count() {
    let (outcomes, _) = client_reaches_server(&json!({
        "src/client/a.ts": "import type { Row } from '../server/types';\n",
    }));
    assert_eq!(
        failures(&outcomes),
        [Some(proves_nothing("client"))],
        "the rule counts runtime imports, and the example has only a type import"
    );
}

#[test]
fn accepted_transitive_example_with_an_import_its_rule_counts_passes() {
    let (outcomes, _) = client_reaches_server(&json!({
        "src/client/a.ts": "import '../server/types';\n",
    }));
    assert_eq!(
        failures(&outcomes),
        [None],
        "the walk follows the example's import, and it reaches no database"
    );
}

/// The package `vitest`, installed, as files of a repository or of an example.
fn vitest() -> [(&'static str, &'static str); 2] {
    [
        (
            "node_modules/vitest/package.json",
            r#"{ "name": "vitest", "main": "index.js" }"#,
        ),
        ("node_modules/vitest/index.js", ""),
    ]
}

/// A manifest that lists `vitest` under `devDependencies`.
const VITEST_FOR_DEVELOPMENT: &str = r#"{ "name": "app", "devDependencies": { "vitest": "1" } }"#;

/// For a rejected example of a `dependency: 'dev'` rule beside a `dependency: 'undeclared'` rule,
/// each violation the example gives, as (rule, message).
fn dependency_violations(repo: &FixtureRepo, example: &Value) -> Vec<(String, String)> {
    let forbid = |name: &str, kind: &str, examples: Value| {
        json!({ "name": name, "files": "src", "mayNotImport": { "packages": ["*"] },
                "dependency": kind, "examples": examples })
    };
    let config = json!({
        "elements": { "src": "src/**" },
        "rules": [
            forbid("dev", "dev", json!({ "rejected": [example] })),
            forbid("undeclared", "undeclared", json!({})),
        ],
    });
    let (outcomes, _) = examples_of(&config, repo);
    outcomes
        .iter()
        .flat_map(|o| &o.diagnostics)
        .map(|d| (d.rule.clone().unwrap_or_default(), d.message.clone()))
        .collect()
}

fn imports_vitest(manifests: &[(&str, &str)]) -> Value {
    let installed = vitest();
    let files = manifests
        .iter()
        .chain(&installed)
        .map(|(path, text)| ((*path).to_owned(), Value::from(*text)))
        .chain([(
            "src/a.ts".to_owned(),
            Value::from("import { x } from 'vitest';\n"),
        )]);
    Value::Object(files.collect())
}

fn listed_only_for_development(manifest: &str) -> Vec<(String, String)> {
    vec![(
        "dev".to_owned(),
        format!(
            "src/a.ts may not import \"vitest\", because files in \"src\" may not import the \
             package `*` that {manifest} lists only under devDependencies"
        ),
    )]
}

#[test]
fn an_example_sees_its_own_manifest() {
    let repo = FixtureRepo::new()
        .file("package.json", r#"{ "name": "app" }"#)
        .file("src/b.ts", "export {};\n");
    for manifest in ["package.json", "src/package.json"] {
        let example = imports_vitest(&[(manifest, VITEST_FOR_DEVELOPMENT)]);
        assert_eq!(
            dependency_violations(&repo, &example),
            listed_only_for_development(manifest),
            "the example's own {manifest} lists vitest for development"
        );
    }
}

#[test]
fn an_example_without_a_manifest_sees_the_repository_manifest() {
    let repo = vitest()
        .into_iter()
        .fold(FixtureRepo::new(), |repo, (path, text)| {
            repo.file(path, text)
        })
        .file(".gitignore", "node_modules/\n")
        .file("package.json", VITEST_FOR_DEVELOPMENT)
        .file("src/b.ts", "export {};\n");
    let example = json!({ "src/a.ts": "import { x } from 'vitest';\n" });
    assert_eq!(
        dependency_violations(&repo, &example),
        listed_only_for_development("package.json"),
        "the repository's package.json lists vitest for development"
    );
}

#[test]
fn an_example_manifest_replaces_the_repository_manifest_at_its_path_and_the_nearest_wins() {
    let repo = FixtureRepo::new()
        .file(
            "package.json",
            r#"{ "name": "app", "dependencies": { "vitest": "1" } }"#,
        )
        .file("src/b.ts", "export {};\n");
    let replaced = imports_vitest(&[("package.json", VITEST_FOR_DEVELOPMENT)]);
    assert_eq!(
        dependency_violations(&repo, &replaced),
        listed_only_for_development("package.json"),
        "the example's package.json replaces the repository's, which lists vitest for production"
    );
    let nearer = imports_vitest(&[("src/package.json", r#"{ "name": "src" }"#)]);
    assert_eq!(
        dependency_violations(&repo, &nearer),
        [(
            "undeclared".to_owned(),
            "src/a.ts may not import \"vitest\", because files in \"src\" may not import the \
             package `*` that src/package.json does not list"
                .to_owned()
        )],
        "the example's src/package.json is nearer than the repository's package.json"
    );
}

/// The end of the message for an accepted example that a rule never looked at.
fn looked_at(what: &str, add: &str) -> String {
    format!("never looked at {what} of this example, so accepting it proves nothing; {add}")
}

/// For `mustNotExist`, which every file or folder it looks at breaks.
fn cannot_accept(what: &str) -> String {
    format!(
        "never looked at a {what} of this example, and every {what} it looks at breaks it, so no \
         accepted example can prove it; keep only rejected examples"
    )
}

const ADD_A_FILE: &str = "give the example a file in the rule's files";

/// A rule about files, folders or instances of each kind that does not cover `src/app/a.ts`, and
/// the end of the message that an accepted example of only that file gets.
fn structure_rules_that_never_look_at_src_app() -> Vec<(Value, String)> {
    let folder = "give the example a file in a folder the rule names";
    let instance = "give the example a file in an instance of the element the rule names";
    vec![
        (
            json!({ "files": "src/legacy/**", "mustNotExist": true }),
            cannot_accept("file"),
        ),
        (
            json!({ "folders": "src/legacy/*", "mustNotExist": true }),
            cannot_accept("folder"),
        ),
        (
            json!({ "files": "src/shared/**", "mustBeNamed": "kebab-case" }),
            looked_at("a file", ADD_A_FILE),
        ),
        (
            json!({ "folders": "src/shared/*", "mustBeNamed": "kebab-case" }),
            looked_at("a folder", folder),
        ),
        (
            json!({ "folders": "src/shared/*", "mustContain": "index.ts" }),
            looked_at("a folder", folder),
        ),
        (
            json!({ "files": "feature", "mustContain": "index.ts" }),
            looked_at("an instance", instance),
        ),
    ]
}

/// An export or import rule of each kind that does not cover `src/app/a.ts`, and the end of the
/// message that an accepted example of only that file gets.
fn other_rules_that_never_look_at_src_app() -> Vec<(Value, String)> {
    let between_files = "give the example an import between files in the rule's files";
    let between_elements = "give the example an import between the elements the rule names";
    vec![
        (
            json!({ "files": "src/shared/**", "mustExport": "{FileName}" }),
            looked_at("a file", ADD_A_FILE),
        ),
        (
            json!({ "files": "src/shared/**", "mayNotExport": "default" }),
            looked_at("a file", ADD_A_FILE),
        ),
        (
            json!({ "files": "src/shared/**", "mustOnlyReExport": true }),
            looked_at("a file", ADD_A_FILE),
        ),
        (
            json!({ "files": "src/shared/**", "noCycles": true }),
            looked_at("an import", between_files),
        ),
        (
            json!({ "files": "shared", "mayNotImport": "app" }),
            looked_at("an import", between_elements),
        ),
    ]
}

#[test]
fn an_accepted_example_its_rule_never_checks_is_told_what_the_rule_looks_at() {
    let cases = structure_rules_that_never_look_at_src_app()
        .into_iter()
        .chain(other_rules_that_never_look_at_src_app());
    for (mut rule, expected) in cases {
        rule["name"] = json!("r");
        rule["examples"] = json!({ "accepted": [{ "src/app/a.ts": "export const a = 1;\n" }] });
        let (outcomes, _) = examples(&json!([rule]), &login());
        assert_eq!(
            failures(&outcomes),
            [Some((
                Code::ExampleProvesNothing,
                format!("\"r\" {expected}")
            ))],
            "{rule}"
        );
    }
}

#[test]
fn an_accepted_example_outside_the_area_proves_nothing() {
    let repo = FixtureRepo::new()
        .file(".gitignore", "node_modules/\n")
        .file(
            "node_modules/zod/package.json",
            r#"{ "name": "zod", "main": "index.js" }"#,
        )
        .file("node_modules/zod/index.js", "")
        .file("src/value/v.ts", "")
        .file("apps/api/src/a.ts", "");
    let config = json!({
        "elements": { "entity": "src/entity/**", "value-object": "src/value/**" },
        "rules": [{
            "name": "entities", "files": "entity", "mayOnlyImport": ["value-object"],
            "within": "apps/api/src/**",
            "examples": { "accepted": [{ "src/entity/a.ts": "import 'zod';\n" }] },
        }],
    });
    let (outcomes, _) = examples_of(&config, &repo);
    let failure = outcomes[0].failure.as_ref().expect("the example fails");
    assert_eq!(failure.code, Code::ExampleProvesNothing, "{failure:?}");
    assert!(failure.message.contains("apps/api/src/**"), "{failure:?}");
    assert!(
        failure.message.contains("from the rule's files into"),
        "{failure:?}"
    );
}
