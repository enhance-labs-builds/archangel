//! Scenarios from the configuration spec: which element a package is in, and when an element can
//! match nothing.
#![cfg(test)]

#[path = "support/codes.rs"]
mod codes;
#[path = "support/kept.rs"]
mod kept;
#[path = "support/check.rs"]
mod support;

use archangel_graph::{GraphOptions, ImportGraph, Work};
use archangel_rules::{Architecture, Code, Report};
use archangel_testkit::FixtureRepo;
use kept::{DiskChange, KeptCheck};
use serde_json::{Value, json};

fn empty_elements(report: &Report) -> Vec<String> {
    codes::messages(report, Code::ElementEmpty)
}

fn check(elements: &Value, repo: &FixtureRepo) -> Report {
    support::check(&json!({ "elements": elements }), repo, None).0
}

#[test]
fn builtin_package_is_in_the_node_prefix_element() {
    let repo = FixtureRepo::new().file("src/a.ts", "import 'fs';\n");
    let config = json!({
        "elements": { "src": "src/**", "node": { "packages": ["node:*"] } },
        "rules": [{ "files": "src", "mayNotImport": "node" }],
    });
    let (report, _) = support::check(&config, &repo, None);
    let forbid: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.code == Code::MayNotImport)
        .collect();
    assert_eq!(forbid.len(), 1, "fs is node:fs, in the element node");
}

#[test]
fn file_element_that_matches_nothing_is_element_empty() {
    let repo = FixtureRepo::new().file("src/shared/a.ts", "");
    let found = empty_elements(&check(&json!({ "shared": "src/shraed/**" }), &repo));
    assert_eq!(found.len(), 1, "one empty element");
    assert!(
        found[0].contains("\"shared\"") && found[0].contains("src/shraed/**"),
        "it names the element and pattern: {}",
        found[0]
    );
}

#[test]
fn package_element_listed_in_a_manifest_is_not_empty() {
    let repo = FixtureRepo::new()
        .file(
            "package.json",
            r#"{ "name": "app", "dependencies": { "react": "18.0.0" } }"#,
        )
        .file("src/a.ts", "");
    let found = empty_elements(&check(
        &json!({ "src": "src/**", "react": { "packages": ["react"] } }),
        &repo,
    ));
    assert_eq!(
        found,
        Vec::<String>::new(),
        "an installed package is a valid guard"
    );
}

#[test]
fn package_element_with_an_unknown_name_is_element_empty() {
    let repo = FixtureRepo::new()
        .file("package.json", r#"{ "name": "app" }"#)
        .file("src/a.ts", "");
    let found = empty_elements(&check(
        &json!({ "src": "src/**", "react": { "packages": ["recat"] } }),
        &repo,
    ));
    assert_eq!(found.len(), 1, "the misspelt package is reported");
    assert!(
        found[0].contains("\"react\"") && found[0].contains("recat"),
        "it names the element and package: {}",
        found[0]
    );
}

#[test]
fn package_element_of_node_built_ins_is_not_empty() {
    let repo = FixtureRepo::new()
        .file("package.json", r#"{ "name": "app" }"#)
        .file("src/a.ts", "");
    let found = empty_elements(&check(
        &json!({ "src": "src/**", "node": { "packages": ["node:fs", "node:path"] } }),
        &repo,
    ));
    assert_eq!(
        found,
        Vec::<String>::new(),
        "every Node has its built-in modules, though no manifest lists them; the dogfood found \
         the element reported empty"
    );
}

#[test]
fn an_inline_forbidden_package_may_be_uninstalled() {
    let repo = FixtureRepo::new().file("src/a.ts", "");
    let config = json!({
        "elements": { "src": "src/**" },
        "rules": [{ "files": "src/**", "mayNotImport": { "packages": ["pg"] } }],
    });
    let (report, _) = support::check(&config, &repo, None);
    assert_eq!(
        report.diagnostics,
        [],
        "forbidding a package before anyone installs it is the rule's passing state"
    );
}

#[test]
fn a_file_element_of_files_that_are_not_source_is_not_empty() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "")
        .bytes("assets/logo.png", b"\x89PNG");
    let report = check(&json!({ "src": "src/**", "assets": "assets/**" }), &repo);
    assert_eq!(
        empty_elements(&report),
        Vec::<String>::new(),
        "assets holds logo.png, though no import reaches it"
    );
}

#[test]
fn an_inline_permitted_package_that_nothing_names_is_element_empty() {
    let repo = FixtureRepo::new().file("src/a.ts", "");
    let config = json!({
        "elements": { "src": "src/**" },
        "rules": [{ "name": "only-react", "files": "src/**", "mayOnlyImport": { "packages": ["recat"] } }],
    });
    let (report, _) = support::check(&config, &repo, None);
    let empty = empty_elements(&report);
    assert_eq!(
        empty.len(),
        1,
        "one element-empty: {:?}",
        report.diagnostics
    );
    assert!(
        empty[0].contains("\"only-react\"") && empty[0].contains("`recat`"),
        "it names the rule and the package: {}",
        empty[0]
    );
}

#[test]
fn a_session_forgets_the_files_that_are_gone_and_classifies_only_the_new_ones() {
    let repo = FixtureRepo::new().file("src/a.ts", "").file("lib/b.ts", "");
    let config = json!({ "elements": { "src": "src/**", "lib": "lib/**", "app": "app/**" } });
    let mut session = KeptCheck::open(&config, repo);
    let work = session.change(&DiskChange {
        write: &[("app/c.ts", "")],
        remove: &["lib/b.ts"],
    });
    assert_eq!(
        work.files_classified, 1,
        "only app/c.ts is classified; the files that stayed keep their classes"
    );
    let report = session.report();
    let found = empty_elements(&report);
    assert_eq!(found.len(), 1, "one empty element: {found:?}");
    assert!(
        found[0].contains("\"lib\""),
        "lib lost its only file, and app gained one: {}",
        found[0]
    );
    assert_eq!(
        report.diagnostics,
        session.fresh().diagnostics,
        "as a fresh check finds"
    );
}

/// A preset as the loader sends it, with its root at the config folder.
fn preset(name: &str, elements: &Value) -> Value {
    json!({
        "name": name,
        "package": format!("archangel-plugin-{name}"),
        "root": "",
        "elements": elements,
        "rules": [],
    })
}

#[test]
fn a_preset_whose_file_elements_all_match_nothing_is_preset_empty() {
    let repo = FixtureRepo::new().file("src/a.ts", "");
    let config = archangel_config::from_value(&json!({
        "extends": [
            preset("nowhere", &json!({ "x": "nowhere/**", "y": "nothing/**" })),
            preset("half", &json!({ "src": "src/**", "z": "nothing/**" })),
            preset("packages", &json!({ "react": { "packages": ["react"] } })),
        ],
        "elements": { "own": "src/**" },
        "rules": [],
    }))
    .expect("a valid config");
    let mut work = Work::default();
    let graph = ImportGraph::build(repo.path(), &GraphOptions::default(), &mut work);
    let report = Architecture::compile(config).check(&graph, None, &mut work);
    assert_eq!(
        codes::messages(&report, Code::PresetEmpty),
        [
            "the preset \"nowhere\" matches no file under the config folder, so none of its rules \
          can fire; give it the folder it applies to as its root"
        ],
        "one file element that matches is enough, and a preset of packages alone matches no file \
         by design"
    );
}

#[test]
fn an_inline_permitted_package_is_named_by_an_import_a_manifest_or_node() {
    let repo = FixtureRepo::new()
        .file(
            "package.json",
            r#"{ "name": "app", "dependencies": { "react": "18.0.0" } }"#,
        )
        .file("src/a.ts", "import 'left-pad';\n")
        .file(
            "node_modules/left-pad/package.json",
            r#"{ "name": "left-pad", "main": "index.js" }"#,
        )
        .file("node_modules/left-pad/index.js", "")
        .file(".gitignore", "node_modules/\n");
    let only = |name: &str, package: &str| json!({ "name": name, "files": "src/**", "mayOnlyImport": { "packages": [package, "left-pad"] } });
    let config = json!({
        "elements": { "src": "src/**" },
        "rules": [
            only("built-in", "node:fs"),
            only("listed", "react"),
            only("misspelt", "reactt"),
        ],
    });
    let (report, _) = support::check(&config, &repo, None);
    assert_eq!(
        empty_elements(&report),
        [
            "the rule \"misspelt\" names the package `reactt`, which no import uses and no package \
          manifest lists, so the rule never permits it; fix the name"
        ],
        "left-pad is imported, react is listed, and node:fs is always there"
    );
}
