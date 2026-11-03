//! Scenarios from the command-line spec about `archangel explain <file>:<line>`: which import the
//! line means, and what each rule says about it.
#![cfg(test)]

#[path = "support/cli.rs"]
mod support;

#[path = "support/fixtures.rs"]
mod fixtures;

use archangel_testkit::FixtureRepo;
use serde_json::{Value, json};
use support::run;

const CART_PAGE: &str = "src/features/cart/views/cart-page.tsx";

/// The lines of an explanation that name a rule, without the spaces that line them up.
fn line_of<'o>(output: &'o str, rule: &str) -> &'o str {
    output
        .lines()
        .find(|line| line.trim_start().starts_with(&format!("{rule} ")))
        .unwrap_or_else(|| panic!("no line for {rule} in:\n{output}"))
}

#[test]
fn explain_of_one_import_lists_the_rules_that_judged_and_skipped_it() {
    let found = run(
        &fixtures::fixture("small-app"),
        &["explain", &format!("{CART_PAGE}:2")],
    );
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    let out = &found.stdout;
    assert!(
        out.starts_with(&format!(
            "{CART_PAGE}:2  '../api/fetch-cart' -> src/features/cart/api/fetch-cart.ts\n"
        )),
        "{out}"
    );
    assert!(
        out.contains("file:    part feature (instance cart), kind views in feature-code\n"),
        "{out}"
    );
    assert!(
        out.contains("target:  part feature (instance cart), kind api in feature-code\n"),
        "{out}"
    );
    assert!(
        line_of(out, "app.feature").contains("permits: the same instance of feature"),
        "{out}"
    );
    assert!(
        line_of(out, "feature-code.views").contains(
            "refuses: api is not in its row (views may import only models, shared, and itself)"
        ),
        "{out}"
    );
    assert!(
        line_of(out, "app.shared").contains("the file is not in shared"),
        "{out}"
    );
    assert!(
        line_of(out, "feature-code.models").contains("the file has the kind views, not models"),
        "{out}"
    );
    assert!(
        line_of(out, "feature-code.api").contains("the file has the kind views, not api"),
        "{out}"
    );
}

// Windows refuses a colon in a file name.
#[cfg(not(windows))]
#[test]
fn explain_reads_the_line_after_the_last_colon() {
    let repo = FixtureRepo::new()
        .file("src/a:b.ts", "// first\nimport './c';\n")
        .file("src/c.ts", "")
        .json_config(&json!({ "elements": { "src": "src/**" }, "rules": [] }));
    let found = run(&repo, &["explain", "src/a:b.ts:2"]);
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    assert!(
        found
            .stdout
            .starts_with("src/a:b.ts:2  './c' -> src/c.ts\n"),
        "{}",
        found.stdout
    );
    let whole = run(&repo, &["explain", "src/a:b.ts"]);
    assert_eq!(
        whole.status, 0,
        "a path with a colon and no line: {}",
        whole.stderr
    );
    assert!(whole.stdout.starts_with("src/a:b.ts\n"), "{}", whole.stdout);
}

#[test]
fn explain_of_an_unresolved_import_says_so_and_names_no_target() {
    let repo = fixtures::fixture("small-app");
    repo.write(
        "src/features/cart/views/lost.tsx",
        "// first\n// second\nimport './missing';\n",
    );
    let found = run(&repo, &["explain", "src/features/cart/views/lost.tsx:3"]);
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    let out = &found.stdout;
    assert!(
        out.starts_with("src/features/cart/views/lost.tsx:3  './missing' does not resolve: "),
        "{out}"
    );
    assert!(!out.contains("target:"), "no target: {out}");
    for rule in ["app.feature", "feature-code.views"] {
        assert!(
            line_of(out, rule).contains("the import does not resolve"),
            "{rule}: {out}"
        );
    }
}

#[test]
fn explain_of_one_import_shows_a_rule_that_is_off_and_one_that_checks_no_import() {
    let repo = fixtures::fixture("small-app");
    let config = repo.read("archangel.config.ts").replacen(
        "rules: ({ mayImport }) => [",
        "off: ['feature-code.views'],\n  rules: ({ mayImport, rule }) => [\n    \
         rule('kebab-case-files', { files: 'src/**', mustBeNamed: 'kebab-case' }),",
        1,
    );
    repo.write("archangel.config.ts", &config);
    let found = run(&repo, &["explain", &format!("{CART_PAGE}:2")]);
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    let out = &found.stdout;
    assert!(
        line_of(out, "feature-code.views").contains("the rule is off"),
        "{out}"
    );
    assert!(
        line_of(out, "kebab-case-files").contains("mustBeNamed checks no import"),
        "{out}"
    );
}

#[test]
fn explain_of_one_import_names_the_area_that_skipped_it() {
    let repo = FixtureRepo::new()
        .file("apps/cli/src/main.ts", "import 'zod';\n")
        .file("apps/api/src/app.router.ts", "")
        .file("node_modules/zod/package.json", r#"{ "name": "zod" }"#)
        .file("node_modules/zod/index.js", "")
        .file(".gitignore", "node_modules/\n")
        .json_config(&json!({
            "parts": { "cli": "apps/cli/**", "api": "apps/api/**" },
            "rules": [{
                "name": "cli-uses-only-the-api-router", "files": "cli",
                "mayOnlyImport": "apps/api/src/app.router.ts", "within": "api",
            }],
        }));
    let found = run(&repo, &["explain", "apps/cli/src/main.ts:1"]);
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    assert!(
        line_of(&found.stdout, "cli-uses-only-the-api-router")
            .contains("zod is outside the area api"),
        "{}",
        found.stdout
    );
}

#[test]
fn explain_of_a_line_with_no_import_says_so() {
    let found = run(
        &fixtures::fixture("small-app"),
        &["explain", &format!("{CART_PAGE}:4")],
    );
    assert_eq!(found.status, 1, "{}{}", found.stdout, found.stderr);
    assert!(
        found
            .stdout
            .contains(&format!("line 4 of {CART_PAGE} has no import")),
        "{}",
        found.stdout
    );
}

#[test]
fn explain_of_a_line_in_a_file_that_does_not_parse_gives_parse_failed() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "import './b';\n\n\n\nconst = ;\n")
        .file("src/b.ts", "")
        .json_config(&json!({ "elements": { "src": "src/**" }, "rules": [] }));
    let found = run(&repo, &["explain", "src/a.ts:1"]);
    assert_eq!(found.status, 1, "{}{}", found.stdout, found.stderr);
    assert!(found.stdout.contains("parse-failed"), "{}", found.stdout);
    assert!(
        found
            .stdout
            .contains("archangel cannot explain a file that does not parse"),
        "{}",
        found.stdout
    );
}

#[test]
fn explain_of_a_line_inside_an_import_statement_explains_it() {
    let repo = FixtureRepo::new()
        .file(
            "src/a.ts",
            "import {\n  fetchCart } from './fetch-cart';\nconst other = 1;\n",
        )
        .file("src/fetch-cart.ts", "export const fetchCart = 1;\n")
        .json_config(&json!({ "elements": { "src": "src/**" }, "rules": [] }));
    for line in [1, 2] {
        let found = run(&repo, &["explain", &format!("src/a.ts:{line}")]);
        assert_eq!(
            found.status, 0,
            "line {line}: {}{}",
            found.stdout, found.stderr
        );
        assert!(
            found.stdout.contains("'./fetch-cart' -> src/fetch-cart.ts"),
            "line {line}: {}",
            found.stdout
        );
    }
    assert_eq!(
        run(&repo, &["explain", "src/a.ts:3"]).status,
        1,
        "line 3 has no import"
    );
}

#[test]
fn explain_of_a_line_with_two_imports_explains_each() {
    let repo = FixtureRepo::new()
        .file("src/x.ts", "import a from './a'; import b from './b';\n")
        .file("src/a.ts", "export default 1;\n")
        .file("src/b.ts", "export default 2;\n")
        .json_config(&json!({ "elements": { "src": "src/**" }, "rules": [] }));
    let found = run(&repo, &["explain", "src/x.ts:1"]);
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    assert!(
        found.stdout.contains("src/x.ts:1  './a' -> src/a.ts\n"),
        "{}",
        found.stdout
    );
    assert!(
        found.stdout.contains("src/x.ts:1  './b' -> src/b.ts\n"),
        "{}",
        found.stdout
    );
}

#[test]
fn explain_of_one_import_shows_the_exception_beside_the_rule() {
    let repo = fixtures::fixture("small-app");
    repo.write(
        "src/features/cart/views/other-page.tsx",
        "// archangel-allow feature-code.views: migration\n\
         import { saveCart } from '../api/save-cart';\n\
         export const other = saveCart;\n",
    );
    let found = run(
        &repo,
        &["explain", "src/features/cart/views/other-page.tsx:2"],
    );
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    let line = line_of(&found.stdout, "feature-code.views");
    assert!(line.contains("refuses: "), "{line}");
    assert!(
        line.contains("permitted by an exception: migration"),
        "{line}"
    );
}

#[test]
fn explain_of_one_import_shows_a_baseline_match_beside_the_rule() {
    let repo = fixtures::fixture("small-app");
    assert_eq!(
        run(&repo, &["baseline", "add"]).status,
        0,
        "the baseline is written"
    );
    let found = run(&repo, &["explain", &format!("{CART_PAGE}:2")]);
    let line = line_of(&found.stdout, "feature-code.views");
    assert!(line.contains("refuses: "), "{line}");
    assert!(line.contains("permitted by the baseline"), "{line}");
}

#[test]
fn explain_of_a_missing_file_is_refused() {
    let found = run(
        &fixtures::fixture("small-app"),
        &["explain", "src/missing.ts:1"],
    );
    assert_eq!(found.status, 2, "{}{}", found.stdout, found.stderr);
    assert!(found.stderr.contains("src/missing.ts"), "{}", found.stderr);
    assert!(found.stderr.contains("does not exist"), "{}", found.stderr);
}

#[test]
fn explain_finds_an_import_on_each_line_of_its_statement() {
    let repo = FixtureRepo::new()
        .file(
            "src/a.ts",
            "import data from './b.json'\n  with { type: 'json' };\nconst lazy = import(\n  './c'\n);\n",
        )
        .file("src/b.json", "{}")
        .file("src/c.ts", "")
        .json_config(&json!({ "elements": { "src": "src/**" }, "rules": [] }));
    for (line, target) in [(1, "b.json"), (2, "b.json"), (3, "c"), (4, "c"), (5, "c")] {
        let found = run(&repo, &["explain", &format!("src/a.ts:{line}")]);
        assert_eq!(
            found.status, 0,
            "line {line}: {}{}",
            found.stdout, found.stderr
        );
        assert!(
            found.stdout.contains(target),
            "line {line}: {}",
            found.stdout
        );
    }
}

#[test]
fn explain_of_a_name_with_nothing_before_the_colon_keeps_the_whole_name() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "")
        .json_config(&json!({ "elements": { "src": "src/**" }, "rules": [] }));
    let found = run(&repo, &["explain", ":5"]);
    assert_ne!(
        found.status, 0,
        "`:5` has no file before its colon, so it names a file that is not there: {}{}",
        found.stdout, found.stderr
    );
    assert!(
        found.stderr.contains(":5"),
        "{}{}",
        found.stdout,
        found.stderr
    );
}

#[test]
fn explain_of_a_line_with_no_import_as_json_says_so() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "const a = 1;\n")
        .json_config(&json!({ "elements": { "src": "src/**" }, "rules": [] }));
    let found = run(&repo, &["explain", "src/a.ts:1", "--format", "json"]);
    assert_eq!(found.status, 1, "{}{}", found.stdout, found.stderr);
    let document: Value = serde_json::from_str(&found.stdout).expect("JSON");
    assert_eq!(
        document["line"], 1,
        "the JSON names the line it was asked about, though no import is on it"
    );
    assert_eq!(
        document["imports"],
        json!([]),
        "a line with no import gives an empty list, not a missing key"
    );
    assert_eq!(document["message"], "line 1 of src/a.ts has no import");
}

#[test]
fn explain_of_a_file_that_does_not_parse_as_json_lists_only_its_own_failure() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "const = ;\n")
        .file("src/b.ts", "const = ;\n")
        .file("src/c.ts", "import './missing';\n")
        .json_config(&json!({ "elements": { "src": "src/**" }, "rules": [] }));
    let found = run(&repo, &["explain", "src/a.ts:1", "--format", "json"]);
    assert_eq!(found.status, 1, "{}{}", found.stdout, found.stderr);
    let document: Value = serde_json::from_str(&found.stdout).expect("JSON");
    let diagnostics = document["diagnostics"].as_array().expect("a list");
    assert_eq!(diagnostics.len(), 1, "{document}");
    assert_eq!(diagnostics[0]["code"], "parse-failed");
    assert_eq!(diagnostics[0]["file"], "src/a.ts");
}

#[test]
fn explain_of_one_import_puts_each_rule_under_its_verdict() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "import './b';\n")
        .file("src/b.ts", "")
        .file("lib/c.ts", "")
        .json_config(&json!({
            "elements": { "src": "src/**", "lib": "lib/**" },
            "rules": [
                { "name": "judges", "files": "src", "mayNotImport": "lib" },
                { "name": "skips", "files": "lib", "mayNotImport": "src" },
            ],
        }));
    let found = run(&repo, &["explain", "src/a.ts:1"]);
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    let out = &found.stdout;
    let at = |text: &str| {
        out.find(text)
            .unwrap_or_else(|| panic!("no {text} in:\n{out}"))
    };
    assert!(at("judged by:") < at("    judges"), "{out}");
    assert!(at("    judges") < at("skipped by:"), "{out}");
    assert!(at("skipped by:") < at("    skips"), "{out}");
}
