//! Scenarios from the diagnostics spec: what a violation says, and its identity.
#![cfg(test)]

#[path = "support/check.rs"]
mod support;
#[path = "support/violations.rs"]
mod violations;

use violations::rule_violations;

use archangel_rules::Code;
use archangel_testkit::FixtureRepo;
use serde_json::{Value, json};

fn layered(rules: &Value) -> Value {
    json!({
        "elements": { "app": "src/app/**", "feature": "src/features/{feature}/**", "entity": "src/entities/{entity}/**", "shared": "src/shared/**" },
        "rules": rules,
    })
}

#[test]
fn layers_violation_says_what_shared_may_import() {
    let repo = FixtureRepo::new()
        .file("src/shared/a.ts", "import '../features/login/b';\n")
        .file("src/features/login/b.ts", "");
    let rules = json!([{ "kind": "layers", "order": ["app", "feature", "shared"], "reason": "Shared code must not know about features" }]);
    let report = support::check(&layered(&rules), &repo, None).0;
    let found = rule_violations(&report);
    assert_eq!(found.len(), 1, "one violation");
    assert_eq!(
        (found[0].file.as_deref(), found[0].line),
        (Some("src/shared/a.ts"), 1),
        "file and line"
    );
    assert_eq!(
        found[0].reason.as_deref(),
        Some("Shared code must not know about features"),
        "the reason"
    );
    let help = found[0].help.as_deref().unwrap_or_default();
    assert!(
        help.contains("\"shared\" may import only \"shared\""),
        "what shared may import: {help}"
    );
}

#[test]
fn entry_violation_names_the_entry_file_to_use() {
    let repo = FixtureRepo::new()
        .file(
            "tsconfig.json",
            r#"{ "compilerOptions": { "paths": { "@/*": ["./src/*"] } } }"#,
        )
        .file("src/entities/ledger/index.ts", "export {};\n")
        .file(
            "src/entities/ledger/model/store.ts",
            "export const store = 1;\n",
        )
        .file("src/app/a.ts", "import '@/entities/ledger/model/store';\n");
    let report = support::check(
        &layered(&json!([{ "kind": "entry", "element": "entity", "files": ["index.ts"] }])),
        &repo,
        None,
    )
    .0;
    let help = rule_violations(&report)[0].help.clone().unwrap_or_default();
    assert!(
        help.contains("src/entities/ledger/index.ts"),
        "it names the entry file: {help}"
    );
}

#[test]
fn violation_shows_the_rules_documentation_link() {
    let repo = FixtureRepo::new()
        .file("src/shared/a.ts", "import '../app/b';\n")
        .file("src/app/b.ts", "");
    let rules = json!([{ "files": "shared", "mayNotImport": "app", "link": "docs/architecture.md#entities" }]);
    let report = support::check(&layered(&rules), &repo, None).0;
    assert_eq!(
        rule_violations(&report)[0].link.as_deref(),
        Some("docs/architecture.md#entities"),
        "the link"
    );
}

#[test]
fn every_code_has_a_page_that_names_it_and_says_what_it_means() {
    for code in Code::ALL {
        assert!(
            code.docs().starts_with(&format!("# {}\n\n", code.as_str())),
            "the page of {} opens with its name",
            code.as_str()
        );
        assert!(
            code.summary().ends_with('.') && !code.summary().contains('\n'),
            "the page of {} says what it means in one sentence: {:?}",
            code.as_str(),
            code.summary()
        );
        assert_eq!(
            Code::from_name(code.as_str()),
            Some(code),
            "the name of {} reads back",
            code.as_str()
        );
    }
    assert_eq!(Code::from_name("lyers"), None, "a misspelt name is no code");
}

#[test]
fn every_code_reads_as_its_json_spelling() {
    let differ: Vec<(String, &str)> = Code::ALL
        .iter()
        .map(|code| {
            (
                serde_json::to_value(code)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_owned(),
                code.as_str(),
            )
        })
        .filter(|(json, text)| json != text)
        .collect();
    assert_eq!(
        differ,
        Vec::new(),
        "(JSON, text) pairs that differ; keep NAMES in declaration order"
    );
}

#[test]
fn violation_identity_survives_moved_lines() {
    let config = layered(&json!([{ "files": "shared", "mayNotImport": "app" }]));
    let before = FixtureRepo::new()
        .file("src/shared/a.ts", "import '../app/b';\n")
        .file("src/app/b.ts", "");
    let after = FixtureRepo::new()
        .file("src/shared/a.ts", "\n\n\n\n\nimport '../app/b';\n")
        .file("src/app/b.ts", "");
    let one = support::check(&config, &before, None).0;
    let two = support::check(&config, &after, None).0;
    assert_eq!(
        rule_violations(&one)[0].id,
        rule_violations(&two)[0].id,
        "the identity does not hold a line"
    );
}

#[test]
fn two_imports_of_one_specifier_have_different_identities() {
    let config = layered(&json!([{ "files": "shared", "mayNotImport": "app" }]));
    let repo = FixtureRepo::new()
        .file(
            "src/shared/a.ts",
            "import '../app/b';\nimport '../app/b';\n",
        )
        .file("src/app/b.ts", "");
    let report = support::check(&config, &repo, None).0;
    let found = rule_violations(&report);
    assert_eq!(found.len(), 2, "two violations");
    assert_ne!(found[0].id, found[1].id, "different identities");
}

#[test]
fn diagnostics_are_sorted_by_file_line_column_and_rule() {
    let config = json!({
        "elements": { "app": "src/app/**", "feature": "src/features/{feature}/**", "shared": "src/shared/**", "scripts": "scripts/**" },
        "rules": [
            { "kind": "layers", "name": "a-layers", "order": ["app", "feature", "shared"] },
            { "name": "b-forbid", "files": "shared", "mayNotImport": "app" },
        ],
    });
    let repo = FixtureRepo::new()
        .file("src/app/x.ts", "")
        .file("src/features/f/y.ts", "")
        .file(
            "src/shared/a.ts",
            "import '../features/f/y'; import '../app/x';\nimport '../app/x';\n",
        )
        .file("src/shared/b.ts", "import '../app/x';\n");
    let report = support::check(&config, &repo, None).0;
    let order: Vec<(Option<&str>, u32, u32, Code)> = report
        .diagnostics
        .iter()
        .map(|d| (d.file.as_deref(), d.line, d.column, d.code))
        .collect();
    let a = Some("src/shared/a.ts");
    let b = Some("src/shared/b.ts");
    assert_eq!(
        order,
        [
            (None, 0, 0, Code::ElementEmpty),
            (a, 1, 8, Code::Layers),
            (a, 1, 34, Code::Layers),
            (a, 1, 34, Code::MayNotImport),
            (a, 2, 8, Code::Layers),
            (a, 2, 8, Code::MayNotImport),
            (b, 1, 8, Code::Layers),
            (b, 1, 8, Code::MayNotImport),
        ],
        "a diagnostic with no file first, then by file, line, column and rule name, whatever the \
         messages say"
    );
}

#[test]
fn each_shorthand_page_shows_its_sentences() {
    for (code, sentence) in [
        (
            Code::Isolate,
            "rule({ files: 'feature', mayNotImport: 'feature', instance: 'different' })",
        ),
        (
            Code::Layers,
            "rule({ files: 'shared', mayNotImport: ['app', 'feature'] })",
        ),
        (
            Code::Entry,
            "mayOnlyBeImportedBy: { element: 'entity', capture: { entity: 'ledger' } }",
        ),
        (Code::MayImport, "mayOnlyImport: ['models', 'shared'],"),
        (
            Code::MayImport,
            "rule({ files: 'port', mayNotImport: 'port' })",
        ),
    ] {
        assert!(
            code.docs().contains(sentence),
            "the {} page shows its sentence {sentence}",
            code.as_str()
        );
    }
}

#[test]
fn a_sentence_reads_as_its_condition_its_values_and_its_options() {
    let config = archangel_config::parse_json(
        &json!({
            "elements": { "src": "src/**", "db": "db/**" },
            "rules": [
                { "name": "no-db", "files": "src/**", "mayNotImport": "db", "transitively": true },
                { "name": "kebab", "files": "src/**/*.ts", "mustBeNamed": "kebab-case" },
                { "name": "index", "folders": "src/*", "mustContain": ["index.ts", "index.tsx"] },
            ],
        })
        .to_string(),
    )
    .expect("a valid config");
    let architecture = archangel_rules::Architecture::compile(config);
    let texts: Vec<String> = (0..3)
        .map(|rule| architecture.sentence_text(rule, 0))
        .collect();
    assert_eq!(
        texts,
        [
            "files in src/** may not import db, directly or through other files",
            "files in src/**/*.ts must have names in kebab-case",
            "each folder in src/* must have a file that matches index.ts or index.tsx",
        ],
        "each sentence in the words `archangel explain` prints"
    );
}

#[test]
fn a_plugin_violation_is_about_a_path_only_when_it_names_no_import() {
    let with_specifier = |code: Code, specifier: Option<&str>| {
        let mut diagnostic = archangel_rules::Diagnostic::new(code, "a violation");
        diagnostic.specifier = specifier.map(str::to_owned);
        diagnostic.is_about_a_path()
    };
    assert_eq!(
        [
            with_specifier(Code::Plugin, None),
            with_specifier(Code::Plugin, Some("./a")),
            with_specifier(Code::MayNotImport, None),
            with_specifier(Code::MustBeNamed, None),
        ],
        [true, false, false, true],
        "a plugin's violation without a specifier is about its file, as a naming violation is"
    );
}

#[test]
fn an_entry_sentence_in_a_rule_of_several_sentences_names_its_entry_files() {
    let repo = FixtureRepo::new()
        .file("src/entities/ledger/index.ts", "export {};\n")
        .file(
            "src/entities/ledger/model/store.ts",
            "export const store = 1;\n",
        )
        .file("src/app/a.ts", "import '../entities/ledger/model/store';\n");
    let rules = json!([{
        "name": "public-api",
        "sentences": [{ "kind": "entry", "element": "entity", "files": ["index.ts"] }],
    }]);
    let report = support::check(&layered(&rules), &repo, None).0;
    let found = rule_violations(&report);
    assert_eq!(found.len(), 1, "one violation: {found:?}");
    assert_eq!(
        found[0].code,
        Code::Entry,
        "the entry code, not may-not-import"
    );
    assert!(
        found[0].message.contains("past its entry files"),
        "the entry message: {}",
        found[0].message
    );
    let help = found[0].help.clone().unwrap_or_default();
    assert!(
        help.contains("src/entities/ledger/index.ts"),
        "it names the entry file: {help}"
    );
}

#[test]
fn a_may_not_import_message_names_the_limits_of_its_rule() {
    let repo = FixtureRepo::new()
        .file("src/app/a.ts", "import { b } from '../shared/b';\n")
        .file("src/shared/b.ts", "export const b = 1;\n");
    let rules = json!([{ "name": "aliases", "files": "app", "mayNotImport": "shared", "specifier": "relative" }]);
    let report = support::check(&layered(&rules), &repo, None).0;
    let found = rule_violations(&report);
    assert_eq!(found.len(), 1, "one violation: {found:?}");
    assert!(
        found[0]
            .message
            .ends_with("may not import \"shared\" through a relative specifier"),
        "the message says that only a relative import breaks the rule: {}",
        found[0].message
    );
}

#[test]
fn a_package_pattern_reads_as_words() {
    let repo = FixtureRepo::new()
        .file("package.json", r#"{ "name": "app" }"#)
        .file(
            "node_modules/zod/package.json",
            r#"{ "name": "zod", "main": "index.js" }"#,
        )
        .file("node_modules/zod/index.js", "")
        .file(
            "node_modules/@aws-sdk/s3/package.json",
            r#"{ "name": "@aws-sdk/s3", "main": "index.js" }"#,
        )
        .file("node_modules/@aws-sdk/s3/index.js", "")
        .file("src/app/a.ts", "import 'zod';\n")
        .file("src/shared/b.ts", "import '@aws-sdk/s3';\n");
    let rules = json!([
        { "name": "declared", "files": "app", "mayNotImport": { "packages": ["*"] }, "dependency": "undeclared" },
        { "name": "no-aws", "files": "shared", "mayNotImport": { "packages": ["@aws-sdk/*"] } },
    ]);
    let report = support::check(&layered(&rules), &repo, None).0;
    let found = rule_violations(&report);
    let messages: Vec<&str> = found.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(
        messages,
        [
            "src/app/a.ts may not import \"zod\", because files in \"app\" may not import any package that package.json does not list",
            "src/shared/b.ts may not import \"@aws-sdk/s3\", because files in \"shared\" may not import a package in `@aws-sdk`",
        ],
        "a pattern of packages reads as words, not as the pattern"
    );
}

#[test]
fn every_page_uses_the_words_of_the_config() {
    for code in Code::ALL {
        // Code is quoted between backticks or fenced, and the title is the code's name, such as
        // `element-empty`, so only the prose counts.
        let mut fenced = false;
        for line in code.docs().lines() {
            if line.starts_with("```") {
                fenced = !fenced;
                continue;
            }
            if fenced || line.starts_with("# ") {
                continue;
            }
            let mut prose: String = line.split('`').step_by(2).collect::<Vec<_>>().join(" ");
            // A link's target is a file name, not a word of the page.
            while let Some(start) = prose.find("](") {
                let end = prose[start..]
                    .find(')')
                    .map_or(prose.len(), |at| start + at + 1);
                prose.replace_range(start..end, " ");
            }
            for word in prose.split(|c: char| !c.is_ascii_alphabetic()) {
                assert!(
                    !matches!(
                        word.to_ascii_lowercase().as_str(),
                        "element" | "elements" | "selector" | "selectors"
                    ),
                    "the page of {} says {word:?}; say part, group or set of files: {line}",
                    code.as_str()
                );
            }
        }
    }
}
