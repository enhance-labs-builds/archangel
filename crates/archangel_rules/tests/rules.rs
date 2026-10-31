//! Scenarios from the architecture-rules spec: what each shorthand kind and the no-cycles condition
//! permit and forbid.
#![cfg(test)]

#[path = "support/layered.rs"]
mod layered;
#[path = "support/check.rs"]
mod support;

use layered::report;

use archangel_rules::{Code, Report, Summary};
use archangel_testkit::FixtureRepo;
use serde_json::{Value, json};

/// The first violation of a rule kind.
fn first(report: &Report) -> &archangel_rules::Diagnostic {
    report
        .diagnostics
        .iter()
        .find(|d| d.code.is_rule())
        .expect("a violation")
}

/// The violations of rule kinds, as (code, importing file) pairs, in report order.
fn violations(report: &Report) -> Vec<(Code, String)> {
    report
        .diagnostics
        .iter()
        .filter(|d| d.code.is_rule())
        .map(|d| (d.code, d.file.clone().unwrap_or_default()))
        .collect()
}

#[test]
fn runtime_rule_ignores_type_imports() {
    let repo = FixtureRepo::new()
        .file("web/a.ts", "import type { Router } from '../api/router';\n")
        .file("api/router.ts", "export type Router = 1;\n");
    let found = report(
        &json!([{ "files": "web", "mayNotImport": "api", "imports": "runtime" }]),
        &repo,
    );
    assert_eq!(
        violations(&found),
        [],
        "a runtime rule does not see type imports"
    );
}

#[test]
fn rule_without_imports_option_checks_type_imports() {
    let repo = FixtureRepo::new()
        .file("web/a.ts", "import type { Router } from '../api/router';\n")
        .file("api/router.ts", "export type Router = 1;\n");
    let found = report(&json!([{ "files": "web", "mayNotImport": "api" }]), &repo);
    assert_eq!(
        violations(&found),
        [(Code::MayNotImport, "web/a.ts".to_owned())],
        "type imports count by default"
    );
}

fn layers() -> Value {
    json!([{ "kind": "layers", "order": ["app", "feature", "shared"] }])
}

#[test]
fn layers_rejects_an_import_toward_an_earlier_layer() {
    let repo = FixtureRepo::new()
        .file("src/shared/a.ts", "import '../features/login/b';\n")
        .file("src/features/login/b.ts", "");
    assert_eq!(
        violations(&report(&layers(), &repo)),
        [(Code::Layers, "src/shared/a.ts".to_owned())],
        "shared may not import feature"
    );
}

#[test]
fn layers_permits_imports_toward_later_layers_and_within_a_layer() {
    let repo = FixtureRepo::new()
        .file("src/app/a.ts", "import '../shared/s';\n")
        .file("src/shared/s.ts", "")
        .file("src/features/login/a.ts", "import '../billing/b';\n")
        .file("src/features/billing/b.ts", "");
    assert_eq!(
        violations(&report(&layers(), &repo)),
        [],
        "downward and same-layer imports are permitted"
    );
}

#[test]
fn layers_does_not_apply_to_elements_outside_its_list() {
    let repo = FixtureRepo::new()
        .file("scripts/a.ts", "import '../src/app/x';\n")
        .file("src/app/x.ts", "")
        .file("src/shared/a.ts", "import '../../scripts/b';\n")
        .file("scripts/b.ts", "");
    assert_eq!(
        violations(&report(&layers(), &repo)),
        [],
        "unlisted elements are not checked"
    );
}

fn isolate() -> Value {
    json!([{ "kind": "isolate", "element": "feature" }])
}

#[test]
fn isolate_rejects_an_import_between_two_instances() {
    let repo = FixtureRepo::new()
        .file("src/features/login/a.ts", "import '../billing/b';\n")
        .file("src/features/billing/b.ts", "");
    let found = report(&isolate(), &repo);
    assert_eq!(
        violations(&found),
        [(Code::Isolate, "src/features/login/a.ts".to_owned())],
        "two features may not import each other"
    );
    let message = &first(&found).message;
    assert!(
        message.contains("login") && message.contains("billing"),
        "the violation names both instances: {message}"
    );
}

#[test]
fn isolate_permits_imports_within_one_instance() {
    let repo = FixtureRepo::new()
        .file("src/features/login/a.ts", "import './model/b';\n")
        .file("src/features/login/model/b.ts", "");
    assert_eq!(
        violations(&report(&isolate(), &repo)),
        [],
        "a feature may import itself"
    );
}

fn entry() -> Value {
    json!([{ "kind": "entry", "element": "entity", "files": ["index.ts"] }])
}

fn canvas() -> FixtureRepo {
    FixtureRepo::new()
        .file("src/entities/canvas/index.ts", "export {};\n")
        .file(
            "src/entities/canvas/model/store.ts",
            "export const store = 1;\n",
        )
}

#[test]
fn entry_rejects_an_import_past_the_entry_file() {
    let repo = canvas().file("src/app/a.ts", "import '../entities/canvas/model/store';\n");
    let found = report(&entry(), &repo);
    assert_eq!(
        violations(&found),
        [(Code::Entry, "src/app/a.ts".to_owned())],
        "the import skips the entry file"
    );
    assert!(
        first(&found)
            .help
            .as_deref()
            .unwrap_or_default()
            .contains("index.ts"),
        "the help names the entry file"
    );
}

#[test]
fn entry_permits_an_import_through_the_entry_file() {
    let repo = canvas().file("src/app/a.ts", "import '../entities/canvas/index';\n");
    assert_eq!(
        violations(&report(&entry(), &repo)),
        [],
        "the entry file is permitted"
    );
}

#[test]
fn entry_rejects_a_sibling_instance_past_the_entry_file() {
    let repo = canvas().file(
        "src/entities/user/a.ts",
        "import '../canvas/model/store';\n",
    );
    assert_eq!(
        violations(&report(&entry(), &repo)),
        [(Code::Entry, "src/entities/user/a.ts".to_owned())],
        "a sibling instance is outside"
    );
}

#[test]
fn no_cycles_reports_two_files_that_import_each_other() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "import './b';\n")
        .file("src/b.ts", "import './a';\n");
    let found = report(&json!([{ "files": "src/**", "noCycles": true }]), &repo);
    assert_eq!(
        violations(&found),
        [(Code::NoCycles, "src/a.ts".to_owned())],
        "one violation for the cycle"
    );
    assert_eq!(
        first(&found).cycle,
        ["src/a.ts", "src/b.ts", "src/a.ts"],
        "the violation shows the cycle"
    );
}

#[test]
fn no_cycles_ignores_type_import_cycles_by_default() {
    let repo = FixtureRepo::new()
        .file(
            "src/a.ts",
            "import type { B } from './b';\nexport type A = 1;\n",
        )
        .file(
            "src/b.ts",
            "import type { A } from './a';\nexport type B = 1;\n",
        );
    assert_eq!(
        violations(&report(
            &json!([{ "files": "src/**", "noCycles": true }]),
            &repo
        )),
        [],
        "type cycles do not run"
    );
}

#[test]
fn no_cycles_with_all_imports_reports_type_import_cycles() {
    let repo = FixtureRepo::new()
        .file(
            "src/a.ts",
            "import type { B } from './b';\nexport type A = 1;\n",
        )
        .file(
            "src/b.ts",
            "import type { A } from './a';\nexport type B = 1;\n",
        );
    let found = report(
        &json!([{ "files": "src/**", "noCycles": true, "imports": "all" }]),
        &repo,
    );
    assert_eq!(
        violations(&found).len(),
        1,
        "with all imports the type cycle counts"
    );
}

#[test]
fn no_cycles_gives_one_violation_per_group() {
    let mut repo = FixtureRepo::new();
    for name in ["a", "b", "c", "d", "e"] {
        let imports: String = ["a", "b", "c", "d", "e"]
            .iter()
            .filter(|other| **other != name)
            .fold(String::new(), |text, other| {
                text + &format!("import './{other}';\n")
            });
        repo = repo.file(&format!("src/{name}.ts"), &imports);
    }
    let found = report(&json!([{ "files": "src/**", "noCycles": true }]), &repo);
    assert_eq!(violations(&found).len(), 1, "one violation for the group");
    assert_eq!(
        first(&found).group.len(),
        5,
        "the violation lists all five files"
    );
    assert_eq!(
        first(&found).cycle,
        ["src/a.ts", "src/b.ts", "src/a.ts"],
        "the shortest cycle, ties by path"
    );
}

#[test]
fn no_cycles_handles_a_chain_of_200000_files() {
    let count = 200_000;
    let mut graph = archangel_rules::CycleGraph::with_files(count);
    for index in 0..count {
        graph.add_import(index, (index + 1) % count);
    }
    let groups = std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || graph.groups())
        .unwrap()
        .join()
        .expect("the search does not overflow a 2 MB stack");
    assert_eq!(groups.len(), 1, "the chain is one group");
    assert_eq!(groups[0].len(), count, "the group holds every file");
}

#[test]
fn one_import_can_break_two_rules() {
    let repo = canvas().file(
        "src/shared/a.ts",
        "import '../entities/canvas/model/store';\n",
    );
    let rules = json!([
        { "kind": "layers", "order": ["app", "entity", "shared"] },
        { "kind": "entry", "element": "entity", "files": ["index.ts"] },
    ]);
    let codes: Vec<Code> = violations(&report(&rules, &repo))
        .into_iter()
        .map(|(code, _)| code)
        .collect();
    assert_eq!(
        codes,
        [Code::Entry, Code::Layers],
        "two violations, sorted by rule name"
    );
}

#[test]
fn rule_order_does_not_change_the_result() {
    let repo = canvas()
        .file(
            "src/shared/a.ts",
            "import '../entities/canvas/model/store';\nimport '../app/x';\n",
        )
        .file("src/app/x.ts", "");
    let forward = json!([
        { "kind": "layers", "order": ["app", "entity", "shared"] },
        { "kind": "entry", "element": "entity", "files": ["index.ts"] },
        { "files": "src/**", "noCycles": true },
    ]);
    let mut backward = forward.as_array().unwrap().clone();
    backward.reverse();
    let one = report(&forward, &repo);
    let two = report(&Value::Array(backward), &repo);
    assert_eq!(
        format!("{:?}", one.diagnostics),
        format!("{:?}", two.diagnostics),
        "the same diagnostics either way"
    );
}

#[test]
fn permitted_unresolved_specifier_is_counted_not_reported() {
    let repo = FixtureRepo::new().file("src/a.ts", "import 'astro:content';\n");
    let config = json!({
        "elements": { "src": "src/**" },
        "allowUnresolved": { "astro:*": "Astro makes these at build time" },
    });
    let (found, _) = support::check(&config, &repo, None);
    assert!(
        found
            .diagnostics
            .iter()
            .all(|d| d.code != Code::ImportUnresolved),
        "no unresolved diagnostic"
    );
    assert_eq!(
        found.summary.permitted_unresolved, 1,
        "the summary counts it"
    );
}

#[test]
fn summary_counts_parsed_files_their_imports_and_computed_specifiers() {
    let repo = FixtureRepo::new()
        .file(
            "src/a.ts",
            "import './b';\nimport './c';\nexport const load = (name: string) => import(name);\n",
        )
        .file("src/b.ts", "")
        .file("src/c.ts", "import {;\n");
    let (found, _) = support::check(&json!({ "elements": { "src": "src/**" } }), &repo, None);
    assert_eq!(
        found.summary,
        Summary {
            files: 2,
            imports: 2,
            unfollowed: 1,
            ..Summary::default()
        },
        "c.ts does not parse, so it adds no file and no imports"
    );
}

#[test]
fn unresolved_import_is_import_unresolved_and_says_how_to_permit_it() {
    let repo = FixtureRepo::new().file("src/a.ts", "import './missing';\n");
    let (found, _) = support::check(&json!({ "elements": { "src": "src/**" } }), &repo, None);
    let codes: Vec<Code> = found.diagnostics.iter().map(|d| d.code).collect();
    assert_eq!(codes, [Code::ImportUnresolved], "one diagnostic");
    let unresolved = &found.diagnostics[0];
    assert_eq!(
        (
            unresolved.file.as_deref(),
            unresolved.line,
            unresolved.specifier.as_deref()
        ),
        (Some("src/a.ts"), 1, Some("./missing")),
        "(file, line, specifier)"
    );
    assert_eq!(
        unresolved.message, "there is no file and no installed package for `./missing`",
        "the message"
    );
    assert_eq!(
        unresolved.help.as_deref(),
        Some(
            "fix the specifier, or add \"./missing\" to the config's allowUnresolved with the \
             reason it cannot be resolved"
        ),
        "the help"
    );
}

#[test]
fn import_that_a_broken_tsconfig_leaves_unresolved_is_reported_on_the_tsconfig_alone() {
    let repo = FixtureRepo::new()
        .file(
            "tsconfig.json",
            r#"{ "extends": "@tsconfig/node20/tsconfig.json" }"#,
        )
        .file("src/a.ts", "import './b';\n")
        .file("src/b.ts", "");
    let (found, _) = support::check(&json!({ "elements": { "src": "src/**" } }), &repo, None);
    let reported: Vec<(Code, Option<&str>, &str)> = found
        .diagnostics
        .iter()
        .map(|d| (d.code, d.file.as_deref(), d.message.as_str()))
        .collect();
    assert_eq!(
        reported,
        [(
            Code::ResolutionFileInvalid,
            Some("tsconfig.json"),
            "its `extends` names `@tsconfig/node20`, which is not installed or does not exist"
        )],
        "one diagnostic on the tsconfig, and no import-unresolved for the import it broke"
    );
}

#[test]
fn no_cycles_breaks_a_tie_between_shortest_cycles_by_path() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "import './b';\nimport './c';\n")
        .file("src/b.ts", "import './d';\n")
        .file("src/c.ts", "import './d';\n")
        .file("src/d.ts", "import './a';\n");
    let found = report(&json!([{ "files": "src/**", "noCycles": true }]), &repo);
    assert_eq!(
        first(&found).cycle,
        ["src/a.ts", "src/b.ts", "src/d.ts", "src/a.ts"],
        "through b.ts and through c.ts are equally short, and b.ts sorts first"
    );
}
