//! Scenarios from the export-rules spec: names a file must export, kinds of export it may not have,
//! and files that may only pass on what other files export.
#![cfg(test)]

#[path = "support/check.rs"]
mod support;
#[path = "support/violations.rs"]
mod violations;

use std::collections::BTreeSet;

use archangel_graph::{GraphOptions, ImportGraph, Work};
use archangel_rules::{
    Code, ExportedNames, Exports, Report, exported_names, exports_name, star_importers,
};
use archangel_testkit::FixtureRepo;
use serde_json::{Value, json};
use violations::rule_violations;

fn report(rule: &Value, repo: &FixtureRepo) -> Report {
    let config = json!({ "elements": { "src": "src/**" }, "rules": [rule] });
    support::check(&config, repo, None).0
}

/// The violations, as (code, file, line), in report order.
fn sites(report: &Report) -> Vec<(Code, String, u32)> {
    rule_violations(report)
        .into_iter()
        .map(|d| (d.code, d.file.clone().unwrap_or_default(), d.line))
        .collect()
}

fn site(code: Code, file: &str, line: u32) -> (Code, String, u32) {
    (code, file.to_owned(), line)
}

fn views() -> Value {
    json!({ "name": "views-export-their-component", "files": "**/*.view.tsx", "mustExport": "{FileName}" })
}

#[test]
fn must_export_rejects_a_file_without_the_name() {
    let repo = FixtureRepo::new().file(
        "src/button.view.tsx",
        "export const ButtonView = () => null;\n",
    );
    let report = report(&views(), &repo);
    let violations = rule_violations(&report);
    assert_eq!(
        violations
            .iter()
            .map(|d| (d.code, d.file.as_deref().unwrap_or_default()))
            .collect::<Vec<_>>(),
        [(Code::MustExport, "src/button.view.tsx")],
        "the view exports no Button"
    );
    let message = &violations[0].message;
    assert!(
        message.contains("`Button`") && message.contains("`ButtonView`"),
        "the message names the name it needs and the names it has: {message}"
    );
}

#[test]
fn a_re_export_counts_as_an_export() {
    let repo = FixtureRepo::new()
        .file("src/card.view.tsx", "export { Card } from './card.impl';\n")
        .file("src/card.impl.ts", "export const Card = 1;\n");
    assert_eq!(
        sites(&report(&views(), &repo)),
        [],
        "the view passes Card on"
    );
}

#[test]
fn export_star_names_count_as_exports() {
    let repo = FixtureRepo::new()
        .file("src/card.view.tsx", "export * from './card.impl';\n")
        .file("src/card.impl.ts", "export const Card = 1;\n");
    assert_eq!(
        sites(&report(&views(), &repo)),
        [],
        "the view passes Card on through export *"
    );
}

#[test]
fn an_export_star_into_a_package_is_exports_unknown() {
    let repo = FixtureRepo::new()
        .file("src/chip.view.tsx", "export * from '@acme/chips';\n")
        .file(
            "node_modules/@acme/chips/package.json",
            r#"{ "name": "@acme/chips", "main": "index.js" }"#,
        )
        .file("node_modules/@acme/chips/index.js", "")
        .file(".gitignore", "node_modules/\n");
    let report = report(&views(), &repo);
    assert_eq!(sites(&report), [], "no violation of the rule");
    let unknown: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.code == Code::ExportsUnknown)
        .collect();
    assert_eq!(
        unknown
            .iter()
            .map(|d| d.file.as_deref().unwrap_or_default())
            .collect::<Vec<_>>(),
        ["src/chip.view.tsx"],
        "one exports-unknown for the file: {:?}",
        report.diagnostics
    );
    assert!(
        unknown[0].message.contains("@acme/chips"),
        "it names the package: {}",
        unknown[0].message
    );
}

#[test]
fn an_unparsed_file_gets_no_export_violation() {
    let repo = FixtureRepo::new().file("src/broken.view.tsx", "export const = ;\n");
    let report = report(&views(), &repo);
    assert_eq!(sites(&report), [], "no violation of the rule");
    assert!(
        report.diagnostics.iter().any(
            |d| d.code == Code::ParseFailed && d.file.as_deref() == Some("src/broken.view.tsx")
        ),
        "parse-failed for the file: {:?}",
        report.diagnostics
    );
}

#[test]
fn may_not_export_rejects_a_default_export() {
    let repo = FixtureRepo::new().file(
        "src/app.tsx",
        "export const a = 1;\n\nexport default function App() {}\n",
    );
    let rule = json!({ "name": "no-default", "files": "src/**", "mayNotExport": "default" });
    assert_eq!(
        sites(&report(&rule, &repo)),
        [site(Code::MayNotExport, "src/app.tsx", 3)],
        "one violation at the line of the default export"
    );
}

#[test]
fn may_not_export_rejects_a_star_export() {
    let repo = FixtureRepo::new()
        .file(
            "src/index.ts",
            "export { a } from './a';\nexport * from './model';\n",
        )
        .file("src/a.ts", "export const a = 1;\n")
        .file("src/model.ts", "export const m = 1;\n");
    let rule = json!({ "name": "no-star", "files": "**/index.ts", "mayNotExport": "star" });
    assert_eq!(
        sites(&report(&rule, &repo)),
        [site(Code::MayNotExport, "src/index.ts", 2)],
        "one violation at the line of export *"
    );
}

#[test]
fn namespace_star_and_re_exported_default_count() {
    let repo = FixtureRepo::new()
        .file("src/model.ts", "export const m = 1;\n")
        .file("src/app.ts", "export default 1;\n")
        .file("src/stars.ts", "export * as model from './model';\n")
        .file("src/defaults.ts", "export { default } from './app';\n");
    let config = json!({
        "elements": { "src": "src/**" },
        "rules": [
            { "name": "no-star", "files": "src/stars.ts", "mayNotExport": "star" },
            { "name": "no-default", "files": "src/defaults.ts", "mayNotExport": "default" },
        ],
    });
    let found = support::check(&config, &repo, None).0;
    assert_eq!(
        rule_violations(&found)
            .iter()
            .map(|d| (
                d.rule.as_deref().unwrap_or_default(),
                d.file.as_deref().unwrap_or_default()
            ))
            .collect::<Vec<_>>(),
        [
            ("no-default", "src/defaults.ts"),
            ("no-star", "src/stars.ts")
        ],
        "export * as is a star export, and export {{ default }} from is a default export"
    );
}

#[test]
fn may_not_export_rejects_names_that_match() {
    let repo = FixtureRepo::new().file(
        "src/state.ts",
        "export const internalState = 1;\nexport const state = 2;\n",
    );
    let rule = json!({ "name": "no-internals", "files": "src/**", "mayNotExport": "internal*" });
    let report = report(&rule, &repo);
    assert_eq!(
        sites(&report),
        [site(Code::MayNotExport, "src/state.ts", 1)],
        "one violation, for internalState"
    );
    assert!(
        rule_violations(&report)[0]
            .message
            .contains("`internalState`"),
        "the message names the export"
    );
}

fn barrels() -> Value {
    json!({ "name": "barrels-only-re-export", "files": "**/index.ts", "mustOnlyReExport": true })
}

#[test]
fn must_only_re_export_rejects_a_declaration() {
    let repo = FixtureRepo::new()
        .file(
            "src/index.ts",
            "export { a } from './a';\nexport function b() {}\n",
        )
        .file("src/a.ts", "export const a = 1;\n");
    let report = report(&barrels(), &repo);
    assert_eq!(
        sites(&report),
        [site(Code::MustOnlyReExport, "src/index.ts", 2)],
        "one violation at the declaration of b"
    );
    assert!(
        rule_violations(&report)[0].message.contains("`b`"),
        "the message names b: {}",
        rule_violations(&report)[0].message
    );
}

#[test]
fn an_import_then_export_is_a_re_export() {
    let repo = FixtureRepo::new()
        .file("src/index.ts", "import { a } from './a';\nexport { a };\n")
        .file("src/a.ts", "export const a = 1;\n");
    assert_eq!(
        sites(&report(&barrels(), &repo)),
        [],
        "the barrel declares nothing of its own"
    );
}

#[test]
fn a_type_declaration_is_the_files_own() {
    let repo = FixtureRepo::new().file("src/index.ts", "export type Id = string;\n");
    let report = report(&barrels(), &repo);
    assert_eq!(
        sites(&report),
        [site(Code::MustOnlyReExport, "src/index.ts", 1)],
        "the type alias is the barrel's own"
    );
    assert!(
        rule_violations(&report)[0].message.contains("`Id`"),
        "the message names Id"
    );
}

#[test]
fn may_not_export_star_tells_export_star_from_export_star_as() {
    let repo = FixtureRepo::new()
        .file(
            "src/index.ts",
            "export * from './a';\nexport * as b from './b';\n",
        )
        .file("src/a.ts", "export const a = 1;\n")
        .file("src/b.ts", "export const b = 1;\n");
    let rule = json!({ "name": "no-star", "files": "src/index.ts", "mayNotExport": "star" });
    let messages: Vec<String> = rule_violations(&report(&rule, &repo))
        .into_iter()
        .map(|d| d.message.clone())
        .collect();
    assert_eq!(
        messages,
        [
            "src/index.ts may not have an `export *`",
            "src/index.ts may not have `export * as b`",
        ],
        "each kind of star export is named as written"
    );
}

/// The graph of a barrel that passes on two files through `export *`, names one export of a
/// third, and passes on a package; a root barrel over it; and a file that only imports.
fn barrel_graph() -> (FixtureRepo, ImportGraph) {
    let repo = FixtureRepo::new()
        .file(
            "src/index.ts",
            "import { x } from 'pkg';\nexport * from './a';\nexport { c } from './c';\nexport * from 'pkg';\nexport default 1;\n",
        )
        .file("src/a.ts", "export const a = 1;\nexport default 2;\n")
        .file("src/c.ts", "export const c = 1;\nexport const hidden = 1;\n")
        .file("src/root.ts", "export * from './index';\n")
        .file("src/user.ts", "import { a } from './a';\nexport const user = a;\n");
    let graph = ImportGraph::build(repo.path(), &GraphOptions::default(), &mut Work::default());
    (repo, graph)
}

#[test]
fn a_file_exports_its_own_names_and_those_its_export_stars_pass_on() {
    let (_repo, graph) = barrel_graph();
    assert_eq!(
        exported_names(&graph, "src/root.ts"),
        ExportedNames {
            names: BTreeSet::from(["a".to_owned(), "c".to_owned()]),
            unknown: vec!["pkg".to_owned()],
        },
        "export * passes on no default, `export {{ c }}` passes on c alone, and the package's \
         names are unknown once"
    );
}

#[test]
fn whether_a_file_exports_a_name_is_yes_no_or_unknown() {
    let (_repo, graph) = barrel_graph();
    assert_eq!(
        ["*", "a", "default", "hidden"].map(|name| exports_name(&graph, "src/index.ts", name)),
        [
            Exports::Yes,
            Exports::Yes,
            Exports::Yes,
            Exports::Unknown(vec!["pkg".to_owned()]),
        ],
        "a namespace needs no name, and a name it does not show may come from the package"
    );
    assert_eq!(
        ["*", "default", "missing"].map(|name| exports_name(&graph, "src/c.ts", name)),
        [Exports::Yes, Exports::No, Exports::No],
        "with every chain followed, a missing name is not exported"
    );
    assert_eq!(
        exports_name(&graph, "src/root.ts", "default"),
        Exports::No,
        "export * never passes on default, so the package cannot supply one"
    );
}

#[test]
fn the_barrels_of_a_file_are_those_that_pass_it_on_through_export_star() {
    let (_repo, graph) = barrel_graph();
    assert_eq!(
        star_importers(&graph, &["src/a.ts"]),
        ["src/index.ts", "src/root.ts"],
        "index.ts passes a.ts on, and root.ts passes index.ts on; user.ts only imports it"
    );
    assert_eq!(
        star_importers(&graph, &["src/c.ts"]),
        Vec::<String>::new(),
        "`export {{ c }}` names what it passes on, so an edit to c.ts changes no barrel's names"
    );
}
