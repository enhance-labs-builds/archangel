//! Scenarios from the selectors spec: a file is in its element and in every selector it matches,
//! a rule side names packages through package elements, and a selector that matches nothing is an
//! error where emptiness means a mistake.
#![cfg(test)]

#[path = "support/codes.rs"]
mod codes;
#[path = "support/kept.rs"]
mod kept;
#[path = "support/check.rs"]
mod support;

use std::collections::BTreeMap;

use archangel_graph::{GraphOptions, ImportGraph, Work};
use archangel_rules::{Architecture, Code, PluginPlace, Report};
use archangel_testkit::FixtureRepo;
use kept::{DiskChange, KeptCheck};
use serde_json::{Value, json};

/// The compiled config and the graph of the repository.
fn compile(config: &Value, repo: &FixtureRepo) -> (Architecture, ImportGraph) {
    let config = archangel_config::parse_json(&config.to_string()).expect("a valid config");
    let options = GraphOptions {
        ignore: config.ignore.clone(),
    };
    let graph = ImportGraph::build(repo.path(), &options, &mut Work::default());
    (Architecture::compile(config), graph)
}

/// The declared selectors that a file is in.
fn selectors_of(config: &Value, repo: &FixtureRepo, path: &str) -> Vec<String> {
    let (architecture, graph) = compile(config, repo);
    architecture.selectors_of(&graph, path)
}

fn report(config: &Value, repo: &FixtureRepo) -> Report {
    support::check(config, repo, None).0
}

/// The violations of rules, as (rule, importing file, specifier), in report order.
fn violations(report: &Report) -> Vec<(String, String, String)> {
    report
        .diagnostics
        .iter()
        .filter(|d| d.code.is_rule())
        .map(|d| {
            (
                d.rule.clone().unwrap_or_default(),
                d.file.clone().unwrap_or_default(),
                d.specifier.clone().unwrap_or_default(),
            )
        })
        .collect()
}

fn empty_selectors(report: &Report) -> Vec<String> {
    codes::messages(report, Code::SelectorEmpty)
}

#[test]
fn a_file_is_in_its_element_and_every_matching_selector() {
    let repo = FixtureRepo::new().file("src/features/login/a.test.ts", "");
    let config = json!({
        "elements": { "feature": "src/features/{feature}/**" },
        "selectors": { "test": "**/*.test.ts", "features": "src/features/**", "views": "**/*.view.tsx" },
    });
    let (architecture, graph) = compile(&config, &repo);
    let class = architecture
        .matcher()
        .classify_file("src/features/login/a.test.ts")
        .expect("the file is in an element");
    assert_eq!(
        architecture.config().elements[class.element.index()].name,
        "feature",
        "the file is in the element feature"
    );
    assert_eq!(
        architecture.selectors_of(&graph, "src/features/login/a.test.ts"),
        ["test", "features"],
        "the file is in every selector it matches, in declaration order, and in no other"
    );
}

#[test]
fn all_any_and_not_combine_selectors() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "")
        .file("src/a.test.ts", "");
    let config = json!({
        "elements": { "src": "src/**" },
        "selectors": {
            "test": "**/*.test.ts",
            "production": { "all": ["src/**", { "not": "test" }] },
            "either": { "any": ["test", "**/*.view.ts"] },
        },
    });
    assert_eq!(
        selectors_of(&config, &repo, "src/a.ts"),
        ["production"],
        "src/a.ts is production code"
    );
    assert_eq!(
        selectors_of(&config, &repo, "src/a.test.ts"),
        ["test", "either"],
        "src/a.test.ts is a test and not production code"
    );
}

#[test]
fn not_any_element_selects_unclassified_files() {
    let repo = FixtureRepo::new()
        .file("src/app/a.ts", "")
        .file("src/stray.ts", "");
    let config = json!({
        "elements": { "app": "src/app/**" },
        "selectors": { "stray": { "not": { "element": "*" } } },
    });
    assert_eq!(
        selectors_of(&config, &repo, "src/stray.ts"),
        ["stray"],
        "a file in no element is in the selector"
    );
    assert!(
        selectors_of(&config, &repo, "src/app/a.ts").is_empty(),
        "a file in an element is not"
    );
}

#[test]
fn an_instance_selector_selects_one_capture_value() {
    let repo = FixtureRepo::new()
        .file("src/features/login/a.ts", "")
        .file("src/features/billing/b.ts", "");
    let config = json!({
        "elements": { "feature": "src/features/{feature}/**" },
        "selectors": { "login": { "element": "feature", "capture": { "feature": "login" } } },
    });
    assert_eq!(
        selectors_of(&config, &repo, "src/features/login/a.ts"),
        ["login"],
        "the login file is in the instance selector"
    );
    assert!(
        selectors_of(&config, &repo, "src/features/billing/b.ts").is_empty(),
        "the billing file is not"
    );
}

fn client_config() -> Value {
    json!({
        "elements": { "app": "app/**" },
        "selectors": { "client": { "directive": "use client" } },
    })
}

#[test]
fn a_directive_selector_selects_files_by_their_first_statement() {
    let repo = FixtureRepo::new()
        .file("app/a.tsx", "'use client';\nexport const A = 1;\n")
        .file("app/b.tsx", "export const B = 1;\n");
    assert_eq!(
        selectors_of(&client_config(), &repo, "app/a.tsx"),
        ["client"],
        "the file that starts with the directive is a client file"
    );
    assert!(
        selectors_of(&client_config(), &repo, "app/b.tsx").is_empty(),
        "the file without it is not"
    );
}

#[test]
fn a_directive_counts_only_as_the_first_statement() {
    let repo = FixtureRepo::new()
        .file("app/c.tsx", "\"use client\";\nexport const C = 1;\n")
        .file(
            "app/d.tsx",
            "import x from './x';\n'use client';\nexport default x;\n",
        )
        .file("app/x.ts", "export default 1;\n");
    assert_eq!(
        selectors_of(&client_config(), &repo, "app/c.tsx"),
        ["client"],
        "a directive in double quotes counts"
    );
    assert!(
        selectors_of(&client_config(), &repo, "app/d.tsx").is_empty(),
        "a directive after an import is not the first statement"
    );
}

#[test]
fn a_file_that_does_not_parse_has_no_directive() {
    let repo = FixtureRepo::new()
        .file("app/a.tsx", "'use client';\nexport const = ;\n")
        .file("app/b.tsx", "'use client';\n");
    let found = report(&client_config(), &repo);
    assert!(
        found
            .diagnostics
            .iter()
            .any(|d| d.code == Code::ParseFailed && d.file.as_deref() == Some("app/a.tsx")),
        "the file that does not parse gets parse-failed: {:?}",
        found.diagnostics
    );
    assert!(
        selectors_of(&client_config(), &repo, "app/a.tsx").is_empty(),
        "and is in no directive selector"
    );
}

#[test]
fn node_star_selects_built_in_modules_with_and_without_the_prefix() {
    let repo = FixtureRepo::new()
        .file("src/web/a.ts", "import 'fs';\n")
        .file("src/web/b.ts", "import 'node:path';\n");
    let config = json!({
        "elements": { "web": "src/web/**" },
        "rules": [{ "name": "no-node", "files": "src/web/**", "mayNotImport": { "packages": ["node:*"] } }],
    });
    assert_eq!(
        violations(&report(&config, &repo)),
        [
            (
                "no-node".to_owned(),
                "src/web/a.ts".to_owned(),
                "fs".to_owned()
            ),
            (
                "no-node".to_owned(),
                "src/web/b.ts".to_owned(),
                "node:path".to_owned()
            ),
        ],
        "one violation for each built-in module, with and without its prefix"
    );
}

fn installed(repo: FixtureRepo, name: &str) -> FixtureRepo {
    repo.file(
        &format!("node_modules/{name}/package.json"),
        &format!(r#"{{ "name": "{name}", "main": "index.js" }}"#),
    )
    .file(&format!("node_modules/{name}/index.js"), "")
}

#[test]
fn a_star_package_name_selects_every_package() {
    let repo = installed(
        installed(
            FixtureRepo::new()
                .file(
                    "src/domain/a.ts",
                    "import 'zod';\nimport '@acme/ui';\nimport './b';\n",
                )
                .file("src/domain/b.ts", "")
                .file(".gitignore", "node_modules/\n"),
            "zod",
        ),
        "@acme/ui",
    );
    let config = json!({
        "elements": { "domain": "src/domain/**" },
        "rules": [{ "name": "pure", "files": "src/domain/**", "mayNotImport": { "packages": ["*"] } }],
    });
    assert_eq!(
        violations(&report(&config, &repo)),
        [
            (
                "pure".to_owned(),
                "src/domain/a.ts".to_owned(),
                "zod".to_owned()
            ),
            (
                "pure".to_owned(),
                "src/domain/a.ts".to_owned(),
                "@acme/ui".to_owned()
            ),
        ],
        "one violation for each package and none for the relative import"
    );
}

#[test]
fn a_selector_that_matches_nothing_is_an_error() {
    let repo = FixtureRepo::new().file("src/a.view.tsx", "");
    let config = json!({
        "elements": { "src": "src/**" },
        "selectors": { "views": "**/*.viewz.tsx" },
    });
    let found = empty_selectors(&report(&config, &repo));
    assert_eq!(found.len(), 1, "one empty selector: {found:?}");
    assert!(
        found[0].contains("\"views\"") && found[0].contains("**/*.viewz.tsx"),
        "it names the selector and its pattern: {}",
        found[0]
    );
}

#[test]
fn an_inline_pattern_that_matches_nothing_is_an_error() {
    let repo = FixtureRepo::new().file("src/next/b.ts", "");
    let config = json!({
        "elements": { "src": "src/**" },
        "rules": [{ "name": "legacy", "files": "src/legasy/**", "mayNotImport": "src/next/**" }],
    });
    let found = empty_selectors(&report(&config, &repo));
    assert_eq!(found.len(), 1, "one empty inline pattern: {found:?}");
    assert!(
        found[0].contains("\"legacy\"") && found[0].contains("src/legasy/**"),
        "it names the rule and the pattern: {}",
        found[0]
    );
}

#[test]
fn an_inline_pattern_only_inside_not_may_match_nothing() {
    let repo = FixtureRepo::new()
        .file("src/app/a.ts", "")
        .file("src/next/b.ts", "");
    let exempt = json!({ "not": ["src/generated/**", { "any": ["src/vendor/**", "src/app/**"] }] });
    let config = json!({
        "elements": { "src": "src/**" },
        "rules": [
            { "name": "shared-exemptions", "files": { "all": ["src/**", exempt] },
              "mayNotImport": { "all": ["src/next/**", { "not": "src/legacy/**" }] } },
            { "name": "also-outside", "files": "src/**",
              "mayNotImport": { "any": ["src/vendor/**", { "not": "src/vendor/**" }] } },
        ],
    });
    let found = empty_selectors(&report(&config, &repo));
    assert_eq!(
        found,
        [
            "the rule \"also-outside\" names the pattern src/vendor/**, which matches no file, so \
          the rule cannot fire there; fix the pattern"
        ],
        "src/generated/**, src/vendor/** and src/legacy/** appear only inside not in the first \
         rule, and the second rule also names src/vendor/** outside not"
    );
}

#[test]
fn a_plugin_rule_treats_patterns_inside_not_as_every_other_rule_does() {
    let repo = FixtureRepo::new().file("src/app/a.ts", "");
    let files = json!({ "all": ["src/**", { "not": ["src/generated/**", "src/vendor/**"] }] });
    let config = json!({
        "elements": { "app": "src/app/**" },
        "plugins": { "team": { "schemaVersion": 1, "conditions": {
            "no-barrels": { "description": "", "reads": ["exports"], "optionsSchema": null },
        } } },
        "rules": [
            { "name": "by-plugin", "files": files, "plugin": "team/no-barrels" },
            { "name": "by-structure", "files": files, "mustNotExist": true },
        ],
    });
    // A plugin rule needs the loader's description of its plugin, which a JSON config cannot carry.
    let architecture =
        Architecture::compile(archangel_config::from_value(&config).expect("a valid config"));
    let graph = ImportGraph::build(repo.path(), &GraphOptions::default(), &mut Work::default());
    let found = architecture.check(&graph, None, &mut Work::default());
    assert_eq!(
        empty_selectors(&found),
        Vec::<String>::new(),
        "src/generated/** and src/vendor/** match nothing, but only inside not, for a plugin rule \
         as for a structure rule"
    );
}

#[test]
fn an_empty_must_not_exist_selector_passes() {
    let repo = FixtureRepo::new().file("src/app/a.ts", "");
    let config = json!({
        "elements": { "app": "src/app/**" },
        "rules": [{
            "name": "no-strays",
            "files": { "all": ["src/**", { "not": { "element": "*" } }] },
            "mustNotExist": true,
        }],
    });
    let found = report(&config, &repo);
    assert_eq!(
        violations(&found),
        [],
        "no file is outside an element, so no violation"
    );
    assert_eq!(
        empty_selectors(&found),
        Vec::<String>::new(),
        "and an empty mustNotExist selector is how the rule passes"
    );
}

#[test]
fn a_string_with_a_slash_is_an_inline_pattern() {
    let repo = FixtureRepo::new()
        .file("src/legacy/a.ts", "import '../next/b';\n")
        .file("src/next/b.ts", "");
    let config = json!({
        "elements": { "src": "src/**" },
        "rules": [{ "name": "legacy", "files": "src/legacy/**", "mayNotImport": "src/next/**" }],
    });
    assert_eq!(
        violations(&report(&config, &repo)),
        [(
            "legacy".to_owned(),
            "src/legacy/a.ts".to_owned(),
            "../next/b".to_owned()
        )],
        "the two inline patterns select the two files"
    );
}

#[test]
fn a_selector_of_files_no_import_reaches_is_not_empty() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "")
        .bytes("src/assets/logo.png", b"\x89PNG");
    let config = json!({
        "elements": { "src": "src/**" },
        "selectors": { "images": "**/*.png" },
        "rules": [{ "name": "no-images-in-assets", "files": "src/assets/**", "mustNotExist": true }],
    });
    let (report, _) = support::check(&config, &repo, None);
    assert_eq!(
        empty_selectors(&report),
        Vec::<String>::new(),
        "logo.png is in images, though no import reaches it"
    );
}

#[test]
fn the_sentences_about_a_file_are_those_with_it_in_their_files_or_their_side() {
    let repo = FixtureRepo::new()
        .file("src/a/x.ts", "")
        .file("src/b/y.ts", "")
        .file("src/c/z.ts", "");
    let config = json!({
        "elements": { "src": "src/**" },
        "rules": [
            { "name": "a", "files": "src/a/**", "mayNotImport": "src/b/**" },
            { "name": "c", "files": "src/c/**", "mayNotImport": "src/a/**" },
            { "name": "b", "files": "src/b/**", "mayNotImport": "src/c/**" },
        ],
    });
    let (architecture, graph) = compile(&config, &repo);
    assert_eq!(
        architecture.sentences_about(&graph, "src/a/x.ts"),
        [(0, 0), (1, 0)],
        "x.ts is in the files of the first rule and on the side of the second, and not in the third"
    );
}

#[test]
fn a_name_is_from_a_preset_when_its_prefix_is_a_presets_name() {
    let config = archangel_config::from_value(&json!({
        "extends": [{
            "name": "fsd",
            "package": "archangel-plugin-fsd",
            "root": "",
            "elements": { "shared": "src/shared/**" },
            "rules": [],
        }],
        "elements": { "app": "src/app/**" },
        "rules": [],
    }))
    .expect("a valid config");
    let architecture = Architecture::compile(config);
    assert_eq!(
        ["fsd/shared", "other/shared", "app"].map(|name| architecture.from_preset(name)),
        [true, false, false],
        "only the prefix of a preset that the config extends"
    );
}

/// One client file among plain ones, and a rule over the client files.
fn one_client_file() -> FixtureRepo {
    FixtureRepo::new()
        .file(
            "app/Dialog.tsx",
            "'use client';\nexport const dialog = 1;\n",
        )
        .file("app/page.tsx", "export const page = 1;\n")
}

#[test]
fn a_selector_that_an_edit_empties_is_an_error() {
    let mut config = client_config();
    config["rules"] = json!([{ "name": "pascal", "files": "client", "mustBeNamed": "PascalCase" }]);
    let mut session = KeptCheck::open(&config, one_client_file());
    assert_eq!(
        empty_selectors(&session.report()),
        Vec::<String>::new(),
        "Dialog.tsx is a client file"
    );
    session.change(&DiskChange {
        write: &[("app/Dialog.tsx", "export const dialog = 1;\n")],
        remove: &[],
    });
    let report = session.report();
    let found = empty_selectors(&report);
    assert_eq!(
        found.len(),
        1,
        "the client selector is empty now: {found:?}"
    );
    assert!(
        found[0].contains("\"client\""),
        "it names the selector: {}",
        found[0]
    );
    assert_eq!(
        report.diagnostics,
        session.fresh().diagnostics,
        "as a fresh check finds"
    );
}

/// Three rules over two selectors: one that is not a plugin rule and covers every file, and two
/// plugin rules, over the features and over shared.
fn plugin_rules() -> Architecture {
    let config = archangel_config::from_value(&json!({
        "elements": { "shared": "src/shared/**", "feature": "src/features/{feature}/**" },
        "selectors": { "test": "**/*.test.ts", "story": "**/*.stories.ts" },
        "plugins": { "team": { "schemaVersion": 1, "conditions": {
            "no-barrels": { "description": "", "reads": ["exports"], "optionsSchema": null },
        } } },
        "rules": [
            { "name": "no-stories", "files": "**", "mayNotImport": "story" },
            { "name": "feature-barrels", "files": "feature", "plugin": "team/no-barrels" },
            { "name": "shared-barrels", "files": "shared", "plugin": "team/no-barrels" },
        ],
    }))
    .expect("a valid config");
    Architecture::compile(config)
}

fn plugin_graph() -> (FixtureRepo, ImportGraph) {
    let repo = FixtureRepo::new()
        .file("src/features/login/a.test.ts", "")
        .file("src/shared/b.ts", "");
    let graph = ImportGraph::build(repo.path(), &GraphOptions::default(), &mut Work::default());
    (repo, graph)
}

#[test]
fn the_plugin_rules_are_the_rules_that_use_a_plugin_condition() {
    let rules: Vec<(usize, String)> = plugin_rules()
        .plugin_rules()
        .map(|(rule, used)| (rule, used.full_name()))
        .collect();
    assert_eq!(
        rules,
        [
            (1, "team/no-barrels".to_owned()),
            (2, "team/no-barrels".to_owned())
        ],
        "by their index in the config"
    );
}

#[test]
fn plugin_rules_see_a_repository_files_element_captures_selectors_and_their_own_rules() {
    let architecture = plugin_rules();
    let (_repo, graph) = plugin_graph();
    let mut classes = archangel_rules::Classes::new();
    classes.sync(&architecture, &graph, &mut Work::default());
    assert_eq!(
        architecture.plugin_place(&classes, "src/features/login/a.test.ts"),
        PluginPlace {
            element: Some("feature".to_owned()),
            captures: [("feature".to_owned(), "login".to_owned())].into(),
            selectors: vec!["test".to_owned()],
            rules: vec![1],
        },
        "from the class the session keeps, with only the plugin rules whose files hold it"
    );
}

#[test]
fn plugin_rules_see_an_example_file_as_its_own_path_and_text_place_it() {
    let architecture = plugin_rules();
    let (_repo, graph) = plugin_graph();
    let mut work = Work::default();
    let example = [(
        "src/shared/c.stories.ts".to_owned(),
        "export const c = 1;\n".to_owned(),
    )]
    .into();
    let overlay = graph.overlay(&example, &mut work);
    assert_eq!(
        architecture.example_plugin_place(&overlay.files[0], &mut work),
        PluginPlace {
            element: Some("shared".to_owned()),
            captures: BTreeMap::new(),
            selectors: vec!["story".to_owned()],
            rules: vec![2],
        },
        "an example's file, classified on its own"
    );
}

#[test]
fn an_area_pattern_that_matches_nothing_is_an_error() {
    let repo = FixtureRepo::new()
        .file("src/entity/a.ts", "")
        .file("src/value/b.ts", "");
    let config = json!({
        "elements": { "entity": "src/entity/**", "value-object": "src/value/**" },
        "rules": [{
            "name": "entities", "files": "entity", "mayOnlyImport": ["value-object"],
            "within": "apps/apii/src/**",
        }],
    });
    let found = report(&config, &repo);
    let empty: Vec<_> = found
        .diagnostics
        .iter()
        .filter(|d| d.code == Code::SelectorEmpty)
        .collect();
    assert_eq!(empty.len(), 1, "{found:?}");
    assert_eq!(empty[0].rule.as_deref(), Some("entities"), "{empty:?}");
    assert!(empty[0].message.contains("apps/apii/src/**"), "{empty:?}");
}
