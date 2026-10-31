//! Scenarios from the exceptions-and-baselines spec: an exception permits one import to break one
//! named rule, must give its reason, and is an error once it permits nothing.
#![cfg(test)]

#[path = "support/check.rs"]
mod support;

use archangel_rules::{Code, Report};
use archangel_testkit::FixtureRepo;
use serde_json::json;

fn check(importer: &str) -> Report {
    let repo = FixtureRepo::new()
        .file("src/entities/canvas/index.ts", "export {};\n")
        .file(
            "src/entities/canvas/model/store.ts",
            "export const store = 1;\n",
        )
        .file("src/app/main.ts", "")
        .file("src/shared/a.ts", importer);
    let config = json!({
        "elements": { "app": "src/app/**", "entity": "src/entities/{entity}/**", "shared": "src/shared/**" },
        "rules": [
            { "kind": "layers", "order": ["app", "entity", "shared"] },
            { "kind": "entry", "element": "entity", "files": ["index.ts"] },
        ],
    });
    support::check(&config, &repo, None).0
}

fn codes(report: &Report) -> Vec<(Code, Option<String>)> {
    report
        .diagnostics
        .iter()
        .map(|d| (d.code, d.rule.clone()))
        .collect()
}

const PAST_ENTRY: &str = "import '../entities/canvas/model/store';\n";

#[test]
fn exception_permits_one_import_to_break_one_rule() {
    let report = check(&format!(
        "// archangel-allow entry(entity): the migration reads the raw store\n{PAST_ENTRY}"
    ));
    let rules: Vec<_> = codes(&report)
        .into_iter()
        .filter_map(|(_, rule)| rule)
        .collect();
    assert!(
        !rules.contains(&"entry(entity)".to_owned()),
        "the entry violation is permitted: {rules:?}"
    );
    assert_eq!(
        report.summary.exceptions.get("entry(entity)"),
        Some(&1),
        "the summary counts it"
    );
}

#[test]
fn exception_does_not_permit_other_rules() {
    let report = check(&format!(
        "// archangel-allow entry(entity): the migration reads the raw store\n{PAST_ENTRY}"
    ));
    assert_eq!(
        codes(&report),
        [(Code::Layers, Some("layers(app, entity, shared)".to_owned()))],
        "the layers violation stays"
    );
}

#[test]
fn consecutive_exceptions_apply_to_the_import_after_them() {
    let text = format!(
        "// archangel-allow entry(entity): the migration reads the raw store\n// archangel-allow layers(app, entity, shared): the migration runs before layering\n{PAST_ENTRY}"
    );
    assert_eq!(
        codes(&check(&text)),
        [],
        "both exceptions apply to the one import"
    );
}

#[test]
fn exception_in_a_block_comment_permits_the_import() {
    let report = check(&format!(
        "/* archangel-allow entry(entity): generated code */\n{PAST_ENTRY}"
    ));
    assert!(
        codes(&report).iter().all(|(code, _)| *code != Code::Entry),
        "the block comment permits the import"
    );
}

#[test]
fn exception_separated_by_a_blank_line_permits_nothing() {
    let report = check(&format!(
        "// archangel-allow entry(entity): too far away\n\n{PAST_ENTRY}"
    ));
    let found = codes(&report);
    assert!(
        found.contains(&(Code::Entry, Some("entry(entity)".to_owned()))),
        "the violation stays: {found:?}"
    );
    assert!(
        found.iter().any(|(code, _)| *code == Code::ExceptionUnused),
        "the exception is unused: {found:?}"
    );
}

#[test]
fn exception_without_a_reason_permits_nothing() {
    let found = codes(&check(&format!(
        "// archangel-allow entry(entity):\n{PAST_ENTRY}"
    )));
    assert!(
        found
            .iter()
            .any(|(code, _)| *code == Code::ExceptionWithoutReason),
        "{found:?}"
    );
    assert!(
        found.contains(&(Code::Entry, Some("entry(entity)".to_owned()))),
        "the violation stays: {found:?}"
    );
}

#[test]
fn exception_for_an_unknown_rule_is_exception_unknown_rule() {
    let report = check(&format!(
        "// archangel-allow entry(entitty): old code\n{PAST_ENTRY}"
    ));
    let unknown = report
        .diagnostics
        .iter()
        .find(|d| d.code == Code::ExceptionUnknownRule)
        .expect("an unknown-rule diagnostic");
    assert!(
        unknown.message.contains("entry(entitty)") && unknown.message.contains("entry(entity)"),
        "{}",
        unknown.message
    );
}

#[test]
fn exception_that_permits_nothing_is_exception_unused() {
    let report = check(
        "// archangel-allow entry(entity): the store was moved\nimport '../entities/canvas/index';\n",
    );
    let found = codes(&report);
    assert!(
        found.iter().any(|(code, _)| *code == Code::ExceptionUnused),
        "{found:?}"
    );
}

#[test]
fn an_exception_does_not_permit_a_violation_about_a_file() {
    let repo = FixtureRepo::new().file(
        "src/userProfile.ts",
        "// archangel-allow kebab-files: legacy\nexport const profile = 1;\n",
    );
    let config = json!({
        "elements": { "src": "src/**" },
        "rules": [{ "name": "kebab-files", "files": "src/**", "mustBeNamed": "kebab-case" }],
    });
    let report = support::check(&config, &repo, None).0;
    assert_eq!(
        codes(&report),
        [
            (Code::MustBeNamed, Some("kebab-files".to_owned())),
            (Code::ExceptionUnused, None)
        ],
        "the naming violation stands, and the comment permits nothing"
    );
    let unused = &report.diagnostics[1].message;
    assert!(
        unused.contains("baseline"),
        "the comment's diagnostic says the violation can only be baselined: {unused}"
    );
}

#[test]
fn an_exception_permits_a_transitive_violation_at_its_first_import() {
    let repo = FixtureRepo::new()
        .file(
            "src/client/a.ts",
            "// archangel-allow client-never-reaches-server-only: the chart is rendered on the server\nimport '../lib/b';\n",
        )
        .file("src/lib/b.ts", "import './c';\n")
        .file("src/lib/c.ts", "import 'server-only';\n")
        .file(
            "node_modules/server-only/package.json",
            r#"{ "name": "server-only", "main": "index.js" }"#,
        )
        .file("node_modules/server-only/index.js", "")
        .file(".gitignore", "node_modules/\n");
    let config = json!({
        "elements": { "client": "src/client/**", "lib": "src/lib/**" },
        "rules": [{
            "name": "client-never-reaches-server-only",
            "files": "client",
            "mayNotImport": { "packages": ["server-only"] },
            "transitively": true,
        }],
    });
    let report = support::check(&config, &repo, None).0;
    assert_eq!(codes(&report), [], "the exception permits the chain");
    assert_eq!(
        report
            .summary
            .exceptions
            .get("client-never-reaches-server-only"),
        Some(&1),
        "one exception is counted"
    );
}

#[test]
fn an_unused_exception_is_about_a_file_only_when_its_own_file_breaks_the_rule() {
    let repo = FixtureRepo::new()
        .file("src/userProfile.ts", "export const profile = 1;\n")
        .file(
            "src/a.ts",
            "// archangel-allow kebab-files: legacy\nexport const a = 1;\n",
        );
    let config = json!({
        "elements": { "src": "src/**" },
        "rules": [{ "name": "kebab-files", "files": "src/**", "mustBeNamed": "kebab-case" }],
    });
    let report = support::check(&config, &repo, None).0;
    let unused: Vec<(Option<&str>, &str)> = report
        .diagnostics
        .iter()
        .filter(|d| d.code == Code::ExceptionUnused)
        .map(|d| (d.file.as_deref(), d.message.as_str()))
        .collect();
    assert_eq!(
        unused,
        [(
            Some("src/a.ts"),
            "the exception for \"kebab-files\" permits nothing, because the next line does not \
             break that rule; delete it"
        )],
        "userProfile.ts breaks the rule, but a.ts, where the comment is, does not"
    );
}
