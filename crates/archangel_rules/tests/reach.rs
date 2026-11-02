//! Scenarios from the command-line spec about `archangel rules`: what each rule covered, and what no
//! rule judged.
#![cfg(test)]

use archangel_config::from_value;
use archangel_graph::{GraphOptions, ImportGraph, Work};
use archangel_rules::{Architecture, Reach, Report, UnjudgedImport};
use archangel_testkit::{FixtureRepo, table};
use serde_json::{Value, json};

/// What a rule covered: its name, files, folders, imports judged, and problems.
type Line = (String, u64, Option<u64>, u64, u64);

struct Found {
    reach: Reach,
    lines: Vec<Line>,
    report: Report,
}

fn reach_under(
    config: &Value,
    repo: &FixtureRepo,
    under: Option<&str>,
    config_file: &str,
) -> Found {
    let parsed = from_value(config).expect("a valid config");
    let graph = ImportGraph::build(repo.path(), &GraphOptions::default(), &mut Work::default());
    let architecture = Architecture::compile(parsed);
    let report = architecture.check(&graph, None, &mut Work::default());
    let reach = architecture.reach(&graph, &report, under, config_file);
    let lines = reach
        .rules
        .iter()
        .map(|rule| {
            (
                architecture.config().rules[rule.rule].name.clone(),
                rule.files,
                rule.folders,
                rule.imports_judged,
                rule.problems,
            )
        })
        .collect();
    Found {
        reach,
        lines,
        report,
    }
}

fn reach_of(config: &Value, repo: &FixtureRepo) -> Found {
    reach_under(config, repo, None, "")
}

fn line(name: &str, files: u64, imports: u64, problems: u64) -> Line {
    (name.to_owned(), files, None, imports, problems)
}

fn two_folders() -> FixtureRepo {
    FixtureRepo::new()
        .file("src/a.ts", "import './b';\nimport '../lib/c';\n")
        .file("src/b.ts", "")
        .file("lib/c.ts", "import '../src/b';\n")
        .file("lib/d.ts", "")
}

#[test]
fn each_rule_counts_its_files_and_the_imports_that_it_judged() {
    let config = json!({
        "elements": { "src": "src/**", "lib": "lib/**" },
        "rules": [
            { "name": "src-not-lib", "files": "src", "mayNotImport": "lib" },
            { "name": "lib-not-src", "files": "lib", "mayNotImport": "src" },
            { "name": "lib-only", "files": "lib", "mayOnlyBeImportedBy": ["src"] },
            { "name": "named", "files": "src/**", "mustBeNamed": "kebab-case" },
        ],
    });
    let found = reach_of(&config, &two_folders());
    assert_eq!(
        found.lines,
        [
            line("src-not-lib", 2, 2, 1),
            line("lib-not-src", 2, 1, 1),
            line("lib-only", 2, 1, 0),
            line("named", 2, 0, 0),
        ]
    );
    assert_eq!(found.reach.files_not_judged, Vec::<String>::new());
    assert_eq!(found.reach.imports_not_judged, []);
}

#[test]
fn a_rule_about_cycles_follows_only_imports_between_its_files() {
    let config = json!({
        "elements": { "src": "src/**", "lib": "lib/**" },
        "rules": [{ "name": "cycles", "files": "src", "noCycles": true }],
    });
    let found = reach_of(&config, &two_folders());
    assert_eq!(found.lines, [line("cycles", 2, 1, 0)]);
    assert_eq!(
        found.reach.imports_not_judged,
        [
            UnjudgedImport {
                from: "lib/c.ts".to_owned(),
                line: 1,
                to: "src/b.ts".to_owned()
            },
            UnjudgedImport {
                from: "src/a.ts".to_owned(),
                line: 2,
                to: "lib/c.ts".to_owned()
            },
        ]
    );
    assert_eq!(found.reach.files_not_judged, ["lib/c.ts", "lib/d.ts"]);
}

#[test]
fn a_rule_about_cycles_is_counted_with_a_second_condition_on_its_rule_only() {
    let config = json!({
        "elements": { "src": "src/**", "lib": "lib/**" },
        "rules": [
            { "name": "first", "files": "lib", "mustBeNamed": "kebab-case" },
            { "name": "cycles", "files": "src", "noCycles": true },
        ],
    });
    let found = reach_of(&config, &two_folders());
    assert_eq!(
        found.lines,
        [line("first", 2, 0, 0), line("cycles", 2, 1, 0)]
    );
}

#[test]
fn an_area_limits_the_imports_that_a_rule_judged() {
    let into_area = json!({
        "elements": { "src": "src/**", "lib": "lib/**" },
        "rules": [{ "name": "r", "files": "src", "mayOnlyImport": ["src"], "within": "src/**" }],
    });
    assert_eq!(
        reach_of(&into_area, &two_folders()).lines,
        [line("r", 2, 1, 0)]
    );

    let from_area = json!({
        "elements": { "src": "src/**", "lib": "lib/**" },
        "rules": [{ "name": "r", "files": "lib", "mayOnlyBeImportedBy": ["src"], "within": "lib/**" }],
    });
    assert_eq!(
        reach_of(&from_area, &two_folders()).lines,
        [line("r", 2, 0, 0)]
    );

    let from_inside = json!({
        "elements": { "src": "src/**", "lib": "lib/**" },
        "rules": [{ "name": "r", "files": "src", "mayOnlyBeImportedBy": ["src"], "within": "lib/**" }],
    });
    assert_eq!(
        reach_of(&from_inside, &two_folders()).lines,
        [line("r", 2, 1, 1)]
    );
}

#[test]
fn a_limit_keeps_the_imports_that_it_does_not_name_out_of_the_count() {
    let config = json!({
        "elements": { "src": "src/**", "lib": "lib/**" },
        "rules": [{ "name": "r", "files": "src", "mayNotImport": "lib", "specifier": "alias" }],
    });
    assert_eq!(
        reach_of(&config, &two_folders()).lines,
        [line("r", 2, 0, 0)]
    );
}

#[test]
fn a_rule_that_counts_type_imports_does_not_count_a_runtime_import() {
    let config = json!({
        "elements": { "src": "src/**", "lib": "lib/**" },
        "rules": [{ "name": "r", "files": "src", "mayNotImport": "lib", "imports": "types" }],
    });
    assert_eq!(
        reach_of(&config, &two_folders()).lines,
        [line("r", 2, 0, 0)]
    );
}

#[test]
fn an_instance_limit_counts_by_the_place_of_each_end() {
    let repo = FixtureRepo::new()
        .file("src/cart/a.ts", "import './b';\nimport '../login/c';\n")
        .file("src/cart/b.ts", "")
        .file("src/login/c.ts", "");
    let config = json!({
        "parts": { "feature": "src/{feature}/**" },
        "rules": [{ "name": "r", "files": "feature", "mayNotImport": "feature", "instance": "different" }],
    });
    assert_eq!(reach_of(&config, &repo).lines, [line("r", 3, 1, 1)]);
}

#[test]
fn a_path_limits_the_rules_the_files_and_the_imports() {
    let config = json!({
        "elements": { "src": "src/**", "lib": "lib/**", "tools": "tools/**" },
        "rules": [
            { "name": "of-src", "files": "src", "mayNotImport": "tools" },
            { "name": "of-lib", "files": "lib", "mayNotImport": "tools" },
        ],
    });
    let repo = two_folders()
        .file("srcs/x.ts", "import '../src/b';\n")
        .file("tools/t.ts", "");
    let found = reach_under(&config, &repo, Some("src"), "");
    assert_eq!(
        found.lines,
        [line("of-src", 2, 2, 0)],
        "a folder holds what starts with it and a slash"
    );
    assert_eq!(
        found.reach.imports_not_judged,
        [],
        "the import of srcs/x.ts is not under src"
    );

    let one_file = reach_under(&config, &repo, Some("lib/d.ts"), "");
    assert_eq!(
        one_file.lines,
        [line("of-lib", 2, 1, 0)],
        "the whole rule, for a file of the rule"
    );
    assert_eq!(one_file.reach.files_not_judged, Vec::<String>::new());

    let nothing = reach_under(&config, &repo, Some("srcs"), "");
    assert_eq!(nothing.lines, [], "no rule's files hold srcs/x.ts");
    assert_eq!(nothing.reach.files_not_judged, ["srcs/x.ts"]);
    assert_eq!(
        nothing.reach.imports_not_judged,
        [UnjudgedImport {
            from: "srcs/x.ts".to_owned(),
            line: 1,
            to: "src/b.ts".to_owned()
        }]
    );
}

#[test]
fn the_config_file_is_in_no_list_of_what_no_rule_judges() {
    let repo = FixtureRepo::new()
        .file("archangel.config.ts", "")
        .file("other.ts", "");
    let config = json!({ "elements": { "src": "src/**" }, "rules": [] });
    let found = reach_under(&config, &repo, None, "archangel.config.ts");
    assert_eq!(found.reach.files_not_judged, ["other.ts"]);
}

#[test]
fn a_package_that_no_rule_names_is_counted_with_each_import() {
    let repo = FixtureRepo::new()
        .file(
            "src/a.ts",
            "import 'react';\nimport 'react';\nimport 'pg';\nimport 'zod';\n",
        )
        .file("lib/b.ts", "import 'other';\n")
        .file(".gitignore", "node_modules/\n")
        .file("node_modules/react/index.js", "")
        .file("node_modules/pg/index.js", "")
        .file("node_modules/zod/index.js", "")
        .file("node_modules/other/index.js", "");
    let config = json!({
        "elements": { "src": "src/**", "lib": "lib/**" },
        "externals": { "driver": ["pg"], "area": ["zod"] },
        "rules": [
            { "name": "r", "files": "src", "mayNotImport": "driver" },
            { "name": "a", "files": "src", "mayOnlyImport": ["src"], "within": "area" },
            { "name": "c", "files": "src", "noCycles": true },
        ],
    });
    let found = reach_under(&config, &repo, Some("src"), "");
    assert_eq!(found.reach.packages_not_named, [("react".to_owned(), 2)]);
}

#[test]
fn a_rule_about_folders_counts_folders_and_applies_under_its_folders() {
    let config = json!({
        "elements": { "src": "src/**", "lib": "lib/**" },
        "rules": [
            { "name": "folders", "folders": "src/*", "mustBeNamed": "kebab-case" },
            { "name": "two", "sentences": [
                { "folders": "src/*", "mustBeNamed": "kebab-case" },
                { "folders": "lib/*", "mustBeNamed": "kebab-case" },
            ] },
        ],
    });
    let repo = FixtureRepo::new()
        .file("src/x/y/a.ts", "")
        .file("src/z/b.ts", "")
        .file("lib/q/c.ts", "");
    let found = reach_of(&config, &repo);
    let covered: Vec<(&str, Option<u64>, u64)> = found
        .lines
        .iter()
        .map(|line| (line.0.as_str(), line.2, line.1))
        .collect();
    assert_eq!(covered, [("folders", Some(2), 0), ("two", Some(3), 0)]);

    let under = reach_under(&config, &repo, Some("lib"), "");
    let names: Vec<&str> = under.lines.iter().map(|line| line.0.as_str()).collect();
    assert_eq!(names, ["two"], "the first rule covers no folder under lib");
    assert_eq!(
        under.reach.files_not_judged,
        ["lib/q/c.ts"],
        "a folder rule judges no file"
    );
}

fn kinds_table(options: &Value) -> Value {
    let mut options = options.clone();
    options["kinds"] = json!("**/*.{kind}.ts");
    json!({ "rules": [table("t", &json!({ "a": [], "b": [] }), &options), table("u", &json!({ "z": [] }), &json!({ "kinds": "other/{kind}/**" }))] })
}

#[test]
fn a_table_own_rule_counts_the_files_of_the_table() {
    let repo = FixtureRepo::new()
        .file("src/x.a.ts", "import './y.a';\n")
        .file("src/y.a.ts", "")
        .file("src/z.b.ts", "")
        .file("other/z/w.ts", "");
    let found = reach_of(&kinds_table(&json!({})), &repo);
    assert_eq!(
        found.lines,
        [
            line("t.a", 2, 1, 0),
            line("t.b", 1, 0, 0),
            line("t.every-file-has-a-kind", 3, 0, 0),
            line("u.z", 1, 0, 0),
            line("u.every-file-has-a-kind", 1, 0, 0),
        ]
    );
}

#[test]
fn a_table_own_rule_applies_with_a_row_of_its_table_under_the_path() {
    let repo = FixtureRepo::new()
        .file("src/x.a.ts", "")
        .file("other/z/w.ts", "");
    let found = reach_under(&kinds_table(&json!({})), &repo, Some("src"), "");
    let names: Vec<&str> = found.lines.iter().map(|line| line.0.as_str()).collect();
    assert_eq!(names, ["t.a", "t.every-file-has-a-kind"]);
}

#[test]
fn problems_exceptions_and_counts_come_from_the_report_of_the_check() {
    let repo = FixtureRepo::new()
        .file(
            "src/a.ts",
            "import '../lib/b';\n// archangel-allow r: moving\nimport '../lib/c';\n",
        )
        .file("lib/b.ts", "")
        .file("lib/c.ts", "");
    let config = json!({
        "elements": { "src": "src/**", "lib": "lib/**" },
        "rules": [{ "name": "r", "files": "src", "mayNotImport": "lib" }],
    });
    let found = reach_of(&config, &repo);
    assert_eq!(found.lines, [line("r", 1, 2, 1)]);
    assert_eq!(found.reach.rules[0].exceptions, 1);
    assert_eq!(found.report.diagnostics.len(), 1);
    assert_eq!(found.reach.rules[0].rule, 0);
}

#[test]
fn a_cycle_rule_judges_only_an_import_with_both_ends_in_its_files() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "import './b';\nimport '../lib/c';\n")
        .file("src/b.ts", "")
        .file("lib/c.ts", "import '../src/b';\n");
    let config = json!({
        "parts": { "src": "src/**", "lib": "lib/**" },
        "rules": [{ "name": "cycles", "files": "src", "noCycles": true }],
    });
    let found = reach_under(&config, &repo, None, "archangel.config.json");
    assert_eq!(found.lines[0].3, 1, "{:?}", found.lines);
}
