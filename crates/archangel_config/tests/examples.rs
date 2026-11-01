//! Scenarios from the rule-examples spec about loading: a one-line example becomes a file set, and
//! it is refused where the text of a file decides the verdict.
#![cfg(test)]

use archangel_config::{ConfigProblem, ExampleFiles, ProblemCode, parse_json};
use archangel_testkit::table;
use serde_json::{Value, json};

fn config_with(rules: &Value) -> Value {
    json!({
        "parts": { "api": "apps/api/**", "shared": "src/shared/**" },
        "groups": { "client": { "directive": "use client" } },
        "rules": rules,
    })
}

/// The rejected examples of a rule with these strings.
fn rejected_of(strings: &[&str]) -> Vec<ExampleFiles> {
    let rules = json!([{
        "name": "r", "files": "src/**", "mustBeNamed": "kebab-case",
        "examples": { "rejected": strings },
    }]);
    let config = parse_json(&config_with(&rules).to_string()).expect("a valid config");
    config.rules[0].examples.rejected.clone()
}

fn files(pairs: &[(&str, &str)]) -> ExampleFiles {
    pairs
        .iter()
        .map(|(path, text)| ((*path).to_owned(), (*text).to_owned()))
        .collect()
}

fn problems(rules: &Value) -> Vec<ConfigProblem> {
    parse_json(&config_with(rules).to_string()).expect_err("the config has a problem")
}

#[test]
fn a_target_is_a_file_when_it_has_a_source_extension() {
    let found = rejected_of(&[
        "a.ts imports src/b.tsx",
        "a.ts imports react",
        "a.ts imports @aws-sdk/client-s3",
        "a.ts imports node:fs",
    ]);
    assert_eq!(
        found,
        [
            files(&[("a.ts", "import './src/b.tsx';\n"), ("src/b.tsx", "")]),
            files(&[("a.ts", "import 'react';\n")]),
            files(&[("a.ts", "import '@aws-sdk/client-s3';\n")]),
            files(&[("a.ts", "import 'node:fs';\n")]),
        ],
        "a target with a source extension is a file, and any other target is a package"
    );
}

#[test]
fn a_chain_gives_each_file_its_imports_and_a_single_path_is_empty() {
    let found = rejected_of(&[
        "a.ts imports b.ts imports a.ts",
        "src/app/ui.ts",
        "src/x/a.ts imports src/y/b.ts",
    ]);
    assert_eq!(
        found,
        [
            files(&[
                ("a.ts", "import './b.ts';\n"),
                ("b.ts", "import './a.ts';\n")
            ]),
            files(&[("src/app/ui.ts", "")]),
            files(&[("src/x/a.ts", "import '../y/b.ts';\n"), ("src/y/b.ts", "")]),
        ],
        "each path is a file with one import for each import that the string gives it"
    );
}

#[test]
fn a_string_with_any_other_shape_is_refused_with_the_form() {
    for text in [
        "a.ts import b.ts",
        "a.ts imports",
        "../a.ts imports b.ts",
        "a.ts imports react imports b.ts",
        "/a.ts",
        "",
        "a.css imports b.ts",
        "a.ts imports ./b.ts",
    ] {
        let rules = json!([{
            "name": "r", "files": "src/**", "mustBeNamed": "kebab-case",
            "examples": { "rejected": [text] },
        }]);
        let found = problems(&rules);
        assert_eq!(found.len(), 1, "one problem for `{text}`: {found:?}");
        assert_eq!(found[0].code, ProblemCode::ConfigInvalid, "`{text}`");
        assert_eq!(
            found[0].field.as_deref(),
            Some("rules[0].examples.rejected[0]"),
            "the message names the field of `{text}`"
        );
        assert!(
            found[0]
                .message
                .contains("<file> imports <file or package>"),
            "the message of `{text}` gives the form: {}",
            found[0].message
        );
    }
}

#[test]
fn a_one_liner_is_refused_where_file_text_decides_the_verdict() {
    let plain = json!({ "name": "r", "files": "api", "mayNotImport": { "packages": ["react"] } });
    let with = |key: &str, value: Value| {
        let mut rule = plain.clone();
        rule[key] = value;
        rule
    };
    let cases = [
        with("names", json!(["useEffect"])),
        with("specifier", json!("relative")),
        with("dependency", json!("dev")),
        with("imports", json!("types")),
        json!({ "name": "r", "files": "api", "mustExport": "{FileName}" }),
        json!({ "name": "r", "files": "api", "mayNotExport": "default" }),
        json!({ "name": "r", "files": "api", "mustOnlyReExport": true }),
        json!({ "name": "r", "files": { "directive": "use client" }, "mayNotImport": { "packages": ["react"] } }),
        json!({ "name": "r", "files": "client", "mayNotImport": { "packages": ["react"] } }),
        json!({
            "name": "r", "files": "api", "mayNotImport": { "packages": ["react"] }, "transitively": true,
            "stopAt": { "directive": "use server" },
        }),
    ];
    for mut rule in cases {
        rule["examples"] = json!({ "rejected": ["apps/api/a.ts imports react"] });
        let found = problems(&json!([rule.clone()]));
        assert_eq!(found.len(), 1, "one problem for {rule}: {found:?}");
        let message = &found[0].message;
        assert_eq!(found[0].code, ProblemCode::ConfigInvalid, "for {rule}");
        assert!(
            message.contains("\"r\"") && message.contains("file set"),
            "the message names the rule and says to write a file set: {message}"
        );
    }
}

#[test]
fn a_one_liner_on_a_narrowed_table_is_refused() {
    let rules = json!([table(
        "domain",
        &json!({ "entity": [] }),
        &json!({
            "kinds": "**/*.{kind}.ts",
            "imports": "types",
            "examples": { "rejected": ["a.entity.ts imports b.use-case.ts"] },
        }),
    )]);
    let found = problems(&rules);
    assert_eq!(found.len(), 1, "one problem: {found:?}");
    assert!(
        found[0].message.contains("\"domain\"") && found[0].message.contains("file set"),
        "the message names the table and says to write a file set: {}",
        found[0].message
    );
}

#[test]
fn a_file_set_stays_valid_on_a_rule_that_refuses_a_one_liner() {
    let rules = json!([{
        "name": "r", "files": "api", "mayNotImport": { "packages": ["react"] }, "imports": "types",
        "examples": { "rejected": [{ "apps/api/a.ts": "import type { A } from 'react';\n" }] },
    }]);
    parse_json(&config_with(&rules).to_string()).expect("a file set is what the rule asks for");
}
