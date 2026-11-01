//! Scenarios from the architecture-rules and selectors specs: an import costs the same however many
//! rules or selectors cannot apply to it, a transitive rule walks the graph once, and each work
//! count would catch the costly way.
#![cfg(test)]

#[path = "support/check.rs"]
mod support;

use archangel_graph::{GraphOptions, ImportGraph, Work};
use archangel_rules::{Architecture, Classes, Results};
use archangel_testkit::FixtureRepo;
use serde_json::{Value, json};

fn repo() -> FixtureRepo {
    FixtureRepo::new()
        .file("src/shared/a.ts", "import '../features/login/b';\n")
        .file("src/features/login/b.ts", "import '../../shared/c';\n")
        .file("src/shared/c.ts", "")
}

fn config(extra: usize) -> Value {
    let mut elements = json!({ "feature": "src/features/{feature}/**", "shared": "src/shared/**" });
    let mut rules = vec![
        json!({ "kind": "layers", "order": ["feature", "shared"] }),
        json!({ "kind": "isolate", "element": "feature" }),
        json!({ "kind": "entry", "element": "feature", "files": ["index.ts"] }),
        json!({ "files": "shared", "mayOnlyBeImportedBy": ["feature"] }),
        json!({ "files": "src/**", "noCycles": true }),
    ];
    for index in 0..extra {
        elements[format!("x{index}")] = json!(format!("other/x{index}/**"));
        rules.push(json!({ "files": format!("x{index}"), "mayNotImport": format!("x{index}"), "name": format!("f{index}") }));
    }
    json!({ "elements": elements, "rules": rules })
}

fn evaluations(extra: usize) -> u64 {
    let repo = repo();
    support::check(&config(extra), &repo, None)
        .1
        .rule_evaluations
}

/// Whether two counts of work are the same, or what differs.
fn same_work<T: PartialEq + std::fmt::Debug>(
    (few, many): (T, T),
    (few_name, many_name): (&str, &str),
) -> Result<(), String> {
    if few == many {
        Ok(())
    } else {
        Err(format!(
            "{few:?} with {few_name}, {many:?} with {many_name}"
        ))
    }
}

/// The counts the scenario states, or what differs from them.
fn counts_match() -> Result<(), String> {
    same_work((evaluations(0), evaluations(495)), ("5 rules", "500"))
}

#[test]
fn rule_evaluations_do_not_grow_with_rules_that_cannot_apply() {
    #[cfg(feature = "failpoints")]
    let _quiet = fail::FailScenario::setup();
    assert_eq!(counts_match(), Ok(()), "the same work for 5 rules and 500");
}

#[cfg(feature = "failpoints")]
#[test]
fn rule_evaluation_count_rejects_a_search_of_every_rule() {
    let scenario = fail::FailScenario::setup();
    fail::cfg("rules::search_every_rule", "return").unwrap();
    let result = counts_match();
    scenario.teardown();
    assert!(result.is_err(), "the count catches a search of every rule");
}

/// A repository of 20 files, each importing the next, under two configs: ten selectors, and the
/// same ten with 990 more that each match one file and that no rule uses.
fn selector_counts(extra: usize) -> (u64, u64, u64) {
    let mut repo = FixtureRepo::new();
    for index in 0..20 {
        repo = repo.file(
            &format!("src/f{index:02}.ts"),
            &format!("import './f{:02}';\n", (index + 1) % 20),
        );
    }
    let mut selectors = serde_json::Map::new();
    for index in 0..10 {
        selectors.insert(format!("s{index}"), json!(format!("src/f{index:02}.ts")));
    }
    for index in 0..extra {
        selectors.insert(
            format!("unused{index}"),
            json!(format!("src/f{:02}.ts", index % 20)),
        );
    }
    let config = json!({
        "elements": { "src": "src/**" },
        "selectors": selectors,
        "rules": [{ "name": "r", "files": "s0", "mayNotImport": "s1" }],
    });
    let work = support::check(&config, &repo, None).1;
    (
        work.files_classified,
        work.pattern_passes,
        work.rule_evaluations,
    )
}

/// The classification counts the scenario states, or what differs from them.
fn classification_matches() -> Result<(), String> {
    let (ten, thousand) = (selector_counts(0), selector_counts(990));
    if ten == thousand && ten.1 == ten.0 {
        Ok(())
    } else {
        Err(format!(
            "(files classified, pattern passes, rule evaluations) are {ten:?} with 10 selectors and {thousand:?} with 1,000"
        ))
    }
}

#[test]
fn classification_does_not_grow_with_selectors() {
    #[cfg(feature = "failpoints")]
    let _quiet = fail::FailScenario::setup();
    assert_eq!(
        classification_matches(),
        Ok(()),
        "one pattern pass per file, and the same work, for 10 selectors and 1,000"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn classification_count_rejects_a_pass_per_selector() {
    let scenario = fail::FailScenario::setup();
    fail::cfg("rules::match_per_selector", "return").unwrap();
    let result = classification_matches();
    let (ten, thousand) = (selector_counts(0), selector_counts(990));
    scenario.teardown();
    assert!(
        result.is_err(),
        "the count catches a pass of each selector's patterns"
    );
    assert!(
        thousand.1 > ten.1,
        "a pass per selector grows with the selectors: {} passes with 10, {} with 1,000",
        ten.1,
        thousand.1
    );
}

/// A graph of 10,000 files: a chain of 9,999 source files, each importing the next, and the last
/// importing the database file, under one transitive rule whose selector covers every file.
fn transitive_walk() -> (u64, usize) {
    let count = 10_000;
    let mut repo = FixtureRepo::new().file("db/store.ts", "");
    for index in 0..count - 2 {
        repo = repo.file(
            &format!("src/f{index:05}.ts"),
            &format!("import './f{:05}';\n", index + 1),
        );
    }
    repo = repo.file(
        &format!("src/f{:05}.ts", count - 2),
        "import '../db/store';\n",
    );
    let config = json!({
        "elements": { "src": "src/**", "db": "db/**" },
        "rules": [{ "name": "no-db", "files": "src/**", "mayNotImport": "db", "transitively": true }],
    });
    let (report, work) = support::check(&config, &repo, None);
    let violations = report
        .diagnostics
        .iter()
        .filter(|d| d.code.is_rule())
        .count();
    (work.transitive_files_visited, violations)
}

#[test]
fn a_transitive_walk_visits_each_file_once() {
    #[cfg(feature = "failpoints")]
    let _quiet = fail::FailScenario::setup();
    let (visited, violations) = transitive_walk();
    assert_eq!(
        violations, 9_999,
        "every source file reaches the database through its chain"
    );
    assert_eq!(
        visited, 10_000,
        "the walk visits each of the 10,000 files, which all reach the database, once"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn transitive_walk_count_rejects_a_walk_per_file() {
    let scenario = fail::FailScenario::setup();
    fail::cfg("rules::walk_per_file", "return").unwrap();
    let (visited, _) = transitive_walk();
    scenario.teardown();
    assert!(
        visited > 10_000,
        "the count catches a walk from each file, which visited {visited}"
    );
}

#[test]
fn classifying_every_file_again_classifies_each_once_and_keeps_its_class() {
    #[cfg(feature = "failpoints")]
    let _quiet = fail::FailScenario::setup();
    let repo = FixtureRepo::new()
        .file("src/a.ts", "")
        .file("src/b.ts", "")
        .bytes("src/logo.png", b"\x89PNG");
    let config = json!({
        "elements": { "src": "src/**", "lib": "lib/**" },
        "rules": [{ "name": "kebab", "files": "src/**", "mustBeNamed": "kebab-case" }],
    });
    let config = archangel_config::parse_json(&config.to_string()).expect("a valid config");
    let architecture = Architecture::compile(config);
    let mut work = Work::default();
    let graph = ImportGraph::build(repo.path(), &GraphOptions::default(), &mut work);
    let mut classes = Classes::new();
    classes.sync(&architecture, &graph, &mut work);
    let mut again = Work::default();
    classes.reclassify_all(&architecture, &graph, &mut again);
    assert_eq!(
        again.files_classified, 3,
        "the two source files and the image, the work of the fault control for a classification per edit"
    );
    let checks = architecture.check_files(&graph, &mut classes, None, &mut work);
    let structure = architecture.structure(&graph, &mut classes, &mut work);
    let results = Results {
        checks: &checks,
        structure: &structure,
        plugin: &[],
    };
    let report = architecture.assemble(&graph, &mut classes, results, None, &mut work);
    let empty: Vec<&str> = report
        .diagnostics
        .iter()
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(
        empty,
        [
            "the element \"lib\" matches nothing (lib/**), so no rule about it can fire; fix the \
          pattern or remove the element"
        ],
        "each file is counted once in its class, as before"
    );
}

/// The evaluations of a check where entity files have `imports` package imports, all outside the
/// area of the one rule that could judge them.
fn area_evaluations(imports: usize) -> u64 {
    let text = (0..imports).fold(String::new(), |text, i| {
        text + "import 'pkg"
            + &(i % 7).to_string()
            + "';
"
    });
    let mut repo = FixtureRepo::new()
        .file(".gitignore", "node_modules/\n")
        .file("src/entity/a.ts", &text)
        .file("src/value/v.ts", "")
        .file("apps/api/src/b.ts", "");
    for i in 0..7 {
        repo = repo
            .file(
                &format!("node_modules/pkg{i}/package.json"),
                &format!(r#"{{ "name": "pkg{i}", "main": "index.js" }}"#),
            )
            .file(&format!("node_modules/pkg{i}/index.js"), "");
    }
    let config = json!({
        "elements": { "entity": "src/entity/**", "value-object": "src/value/**" },
        "rules": [{ "files": "entity", "mayOnlyImport": ["value-object"], "within": "apps/api/src/**" }],
    });
    support::check(&config, &repo, None).1.rule_evaluations
}

fn area_counts_match() -> Result<(), String> {
    same_work(
        (area_evaluations(10), area_evaluations(1000)),
        ("10 imports", "1,000"),
    )
}

#[test]
fn imports_outside_an_area_cost_no_rule_evaluation() {
    #[cfg(feature = "failpoints")]
    let _quiet = fail::FailScenario::setup();
    assert_eq!(
        area_counts_match(),
        Ok(()),
        "the same work for 10 and 1,000"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn rule_evaluation_count_rejects_an_area_test_per_import() {
    let scenario = fail::FailScenario::setup();
    fail::cfg("rules::area_per_import", "return").unwrap();
    let result = area_counts_match();
    scenario.teardown();
    assert!(result.is_err(), "the count catches an area test per import");
}

/// The evaluations of `count` imports inside one captured part, under a `mayOnlyImport` rule.
fn captured_evaluations(count: usize) -> u64 {
    let mut repo = FixtureRepo::new().file("src/shared/s.ts", "");
    let mut text = String::new();
    for i in 0..count {
        text = text + "import './b" + &i.to_string() + "';\n";
        repo = repo.file(&format!("src/features/cart/b{i}.ts"), "");
    }
    repo = repo.file("src/features/cart/a.ts", &text);
    let config = json!({
        "parts": { "feature": "src/features/{feature}/**", "shared": "src/shared/**" },
        "rules": [{ "files": "feature", "mayOnlyImport": ["shared"] }],
    });
    support::check(&config, &repo, None).1.rule_evaluations
}

#[test]
fn imports_inside_a_captured_part_cost_one_evaluation_each() {
    #[cfg(feature = "failpoints")]
    let _quiet = fail::FailScenario::setup();
    assert_eq!(
        (captured_evaluations(10), captured_evaluations(30)),
        (10, 30),
        "one evaluation for each import inside the instance"
    );
}

/// One table of `rows` kind rows, and one import between two files of the first of them.
fn table_counts(rows: usize) -> (u64, u64) {
    let repo = FixtureRepo::new()
        .file("src/a.k0.ts", "import './b.k1';\n")
        .file("src/b.k1.ts", "");
    let mut cells = serde_json::Map::new();
    for row in 0..rows {
        cells.insert(format!("k{row}"), json!([]));
    }
    let config = json!({
        "parts": { "src": "src/**" },
        "rules": [{ "kind": "mayImport", "name": "t", "rows": cells, "kinds": "**/*.{kind}.ts" }],
    });
    let (_, work) = support::check(&config, &repo, None);
    (work.rule_evaluations, work.pattern_passes)
}

fn table_counts_match() -> Result<(), String> {
    same_work((table_counts(5), table_counts(50)), ("5 rows", "50"))
}

#[test]
fn rule_evaluations_do_not_grow_with_rows_that_cannot_apply() {
    #[cfg(feature = "failpoints")]
    let _quiet = fail::FailScenario::setup();
    assert_eq!(
        table_counts_match(),
        Ok(()),
        "the same work for 5 rows and 50"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn rule_evaluation_count_rejects_a_test_of_every_row() {
    let scenario = fail::FailScenario::setup();
    fail::cfg("rules::every_row", "return").unwrap();
    let result = table_counts_match();
    scenario.teardown();
    assert!(result.is_err(), "the count catches a test of every row");
}

#[test]
fn a_kind_pattern_adds_no_pattern_pass() {
    #[cfg(feature = "failpoints")]
    let _quiet = fail::FailScenario::setup();
    let repo = FixtureRepo::new()
        .file("src/a.entity.ts", "import './b.entity';\n")
        .file("src/b.entity.ts", "");
    let without = json!({ "parts": { "src": "src/**" } });
    let mut with = without.clone();
    with["rules"] = json!([{
        "kind": "mayImport", "name": "t", "rows": { "entity": [] }, "kinds": "**/*.{kind}.ts",
    }]);
    let (_, plain) = support::check(&without, &repo, None);
    let (_, kinds) = support::check(&with, &repo, None);
    assert_eq!(
        plain.pattern_passes, kinds.pattern_passes,
        "the same passes"
    );
    assert_eq!(
        plain.files_classified, kinds.files_classified,
        "the same files"
    );
}

#[test]
fn every_file_has_a_kind_costs_one_structure_evaluation_per_file_with_no_row() {
    #[cfg(feature = "failpoints")]
    let _quiet = fail::FailScenario::setup();
    let repo = FixtureRepo::new()
        .file("src/d/a.entity.ts", "import './b.entity';\n")
        .file("src/d/b.entity.ts", "")
        .file("src/d/one.ts", "")
        .file("src/d/two.ts", "")
        .file("src/d/three.ts", "");
    let config = |off: Value| {
        json!({
            "parts": { "src": "src/**" },
            "off": off,
            "rules": [{
                "kind": "mayImport",
                "name": "domain",
                "rows": { "entity": [] },
                "files": "src/d/**",
                "kinds": "**/*.{kind}.ts",
            }],
        })
    };
    let (_, on) = support::check(&config(json!([])), &repo, None);
    let (_, off) = support::check(
        &config(json!(["domain.every-file-has-a-kind"])),
        &repo,
        None,
    );
    assert_eq!(
        on.rule_evaluations, off.rule_evaluations,
        "no rule evaluation"
    );
    assert_eq!(
        on.structure_evaluations,
        off.structure_evaluations + 3,
        "one for each file with no row"
    );
}

/// N imports inside one feature, M between entities, and P between the entities of one context.
fn inside_rows(n: u64, m: u64, p: u64) -> FixtureRepo {
    let mut repo = FixtureRepo::new()
        .file("src/features/cart/target.ts", "")
        .file("src/entities/target.entity.ts", "")
        .file("apps/api/src/board/domain/target.entity.ts", "");
    for index in 0..n {
        repo = repo.file(
            &format!("src/features/cart/f{index}.ts"),
            "import './target';\n",
        );
    }
    for index in 0..m {
        repo = repo.file(
            &format!("src/entities/e{index}.entity.ts"),
            "import './target.entity';\n",
        );
    }
    for index in 0..p {
        repo = repo.file(
            &format!("apps/api/src/board/domain/c{index}.entity.ts"),
            "import './target.entity';\n",
        );
    }
    repo
}

#[test]
fn imports_inside_a_captured_row_cost_one_evaluation_each() {
    #[cfg(feature = "failpoints")]
    let _quiet = fail::FailScenario::setup();
    let (n, m, p) = (3, 4, 2);
    let repo = inside_rows(n, m, p);
    let first = json!({
        "kind": "mayImport",
        "name": "first",
        "rows": { "feature": [], "entity": [] },
        "kinds": "**/*.{kind}.ts",
    });
    let contexts = json!({
        "kind": "mayImport",
        "name": "contexts",
        "rows": { "entity": [] },
        "kinds": "apps/api/src/{context}/domain/**/*.{kind}.ts",
    });
    let parts = json!({ "feature": "src/features/{feature}/**" });
    let without = json!({ "parts": parts, "rules": [first] });
    let with = json!({ "parts": parts, "rules": [first, contexts] });
    let (_, one) = support::check(&without, &repo, None);
    let (_, two) = support::check(&with, &repo, None);
    assert_eq!(
        one.rule_evaluations, n,
        "N for the feature, 0 for the entities"
    );
    assert_eq!(
        two.rule_evaluations,
        n + p,
        "and P for the kind row with a capture"
    );
    assert_eq!(
        one.pattern_passes, two.pattern_passes,
        "no pattern per import"
    );
}
