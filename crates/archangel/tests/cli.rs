//! Scenarios from the command-line, import-graph and architecture-rules specs that need the binary:
//! exit statuses, `check` with paths, `explain`, `why` and `--stats`.
#![cfg(test)]

#[path = "support/cli.rs"]
mod support;

#[path = "support/fixtures.rs"]
mod fixtures;

use archangel_testkit::FixtureRepo;
use serde_json::{Value, json};
use support::run;

fn elements() -> Value {
    json!({
        "app": "src/app/**",
        "feature": "src/features/{feature}/**",
        "shared": "src/shared/**",
    })
}

fn layered(extra: &[Value]) -> Value {
    let mut rules =
        vec![json!({ "kind": "layers", "name": "layers", "order": ["app", "feature", "shared"] })];
    rules.extend(extra.iter().cloned());
    json!({ "elements": elements(), "rules": rules })
}

/// A repository where every element has a file and nothing breaks a rule.
fn clean() -> FixtureRepo {
    FixtureRepo::new()
        .file(
            "src/app/main.ts",
            "import { login } from '../features/login/index';\n",
        )
        .file(
            "src/features/login/index.ts",
            "import { s } from '../../shared/s';\nexport const login = s;\n",
        )
        .file("src/shared/s.ts", "export const s = 1;\n")
        .json_config(&layered(&[]))
}

#[test]
fn clean_check_prints_a_summary_and_exits_zero() {
    let found = run(&clean(), &["check"]);
    assert_eq!(
        found.status, 0,
        "a clean check exits 0: {}{}",
        found.stdout, found.stderr
    );
    assert!(
        found.stdout.contains("3 files") && found.stdout.contains("2 imports"),
        "the summary counts files and imports: {}",
        found.stdout
    );
}

#[test]
fn unknown_command_suggests_the_nearest_and_exits_two() {
    let found = run(&clean(), &["chekc"]);
    assert_eq!(found.status, 2, "an unknown command exits 2");
    for word in ["'check'", "explain", "why", "baseline"] {
        assert!(
            found.stderr.contains(word),
            "the error lists {word}: {}",
            found.stderr
        );
    }
}

#[test]
fn help_and_version_print_to_standard_output_and_exit_zero() {
    let version = run(&clean(), &["--version"]);
    assert_eq!(
        (version.status, version.stdout, version.stderr),
        (
            0,
            format!("archangel {}\n", env!("CARGO_PKG_VERSION")),
            String::new()
        ),
        "the version is an answer, not an error"
    );
    let help = run(&clean(), &["--help"]);
    assert_eq!(
        (help.status, help.stderr.as_str()),
        (0, ""),
        "help is an answer, not an error"
    );
    assert!(
        help.stdout.starts_with(&format!(
            "{}\n\nUsage: archangel [OPTIONS] <COMMAND>\n",
            env!("CARGO_PKG_DESCRIPTION")
        )),
        "help on standard output: {}",
        help.stdout
    );
}

fn two_violations() -> FixtureRepo {
    FixtureRepo::new()
        .file("src/app/main.ts", "")
        .file("src/features/login/index.ts", "")
        .file("src/shared/a.ts", "import '../features/login/index';\n")
        .file("src/shared/b.ts", "import '../app/main';\n")
        .json_config(&layered(&[]))
}

#[test]
fn check_with_a_path_prints_only_that_files_diagnostics() {
    let found = run(&two_violations(), &["check", "src/shared/a.ts"]);
    assert_eq!(found.status, 1, "a violation exits 1");
    assert!(
        found.stdout.contains("src/shared/a.ts"),
        "a.ts is printed: {}",
        found.stdout
    );
    assert!(
        !found.stdout.contains("src/shared/b.ts"),
        "b.ts is not: {}",
        found.stdout
    );
}

#[test]
fn check_with_a_path_still_finds_cycles() {
    let repo = FixtureRepo::new()
        .file("src/shared/a.ts", "import './b';\n")
        .file("src/shared/b.ts", "import './a';\n")
        .json_config(&json!({ "elements": { "shared": "src/shared/**" }, "rules": [{ "name": "no-cycles", "files": "**", "noCycles": true }] }));
    let found = run(&repo, &["check", "src/shared/a.ts"]);
    assert_eq!(found.status, 1, "the cycle exits 1");
    assert!(
        found.stdout.contains("no-cycles"),
        "the cycle is printed: {}",
        found.stdout
    );
}

#[test]
fn path_that_is_not_a_source_file_exits_two_for_each_command() {
    let repo = clean()
        .file(".gitignore", "src/gen/\n")
        .file("src/gen/g.ts", "")
        .file("src/shared/style.css", "");
    let cases = [
        ("src/missing.ts", "does not exist"),
        ("src/gen/g.ts", "ignored"),
        ("src/shared/style.css", "extension"),
    ];
    for (path, why) in cases {
        for args in [
            vec!["check", path],
            vec!["explain", path],
            vec!["why", path, "react"],
        ] {
            let found = run(&repo, &args);
            assert_eq!(found.status, 2, "{args:?} exits 2: {}", found.stderr);
            assert!(
                found.stderr.contains("not a source file") && found.stderr.contains(why),
                "{args:?} says {path} is not a source file because it {why}: {}",
                found.stderr
            );
        }
    }
}

#[test]
fn element_empty_exits_one() {
    let repo = clean().json_config(&json!({
        "elements": { "app": "src/app/**", "feature": "src/features/{feature}/**", "shared": "src/shraed/**" },
        "rules": [{ "kind": "layers", "order": ["app", "feature", "shared"] }],
    }));
    let found = run(&repo, &["check"]);
    assert_eq!(found.status, 1, "element-empty exits 1");
    assert!(
        found.stdout.contains("element-empty") && found.stdout.contains("src/shraed/**"),
        "it names the pattern: {}",
        found.stdout
    );
}

#[test]
fn repository_with_no_source_files_reports_every_file_element_empty() {
    let repo = FixtureRepo::new().json_config(&layered(&[]));
    let found = run(&repo, &["check"]);
    assert_eq!(found.status, 1, "element-empty exits 1");
    assert_eq!(
        found.stdout.matches("error[element-empty]").count(),
        3,
        "one for each element: {}",
        found.stdout
    );
    assert!(
        found.stdout.contains("0 files"),
        "the summary counts zero files: {}",
        found.stdout
    );
}

#[test]
fn exception_unused_exits_one() {
    let repo = clean().file(
        "src/app/other.ts",
        "// archangel-allow layers: the old router\nimport '../shared/s';\n",
    );
    let found = run(&repo, &["check"]);
    assert_eq!(
        found.status, 1,
        "exception-unused exits 1: {}",
        found.stdout
    );
    assert!(
        found.stdout.contains("exception-unused"),
        "the code is printed: {}",
        found.stdout
    );
}

#[cfg(unix)]
#[test]
fn file_unreadable_exits_one() {
    let repo = clean()
        .file("src/private/p.ts", "")
        .unreadable("src/private");
    let found = run(&repo, &["check"]);
    assert_eq!(found.status, 1, "file-unreadable exits 1");
    assert!(
        found.stdout.contains("file-unreadable") && found.stdout.contains("permission"),
        "it says permission was refused: {}",
        found.stdout
    );
}

#[test]
fn import_unresolved_exits_one() {
    let repo = clean().file("src/shared/a.ts", "import './missing';\n");
    let found = run(&repo, &["check"]);
    assert_eq!(found.status, 1, "import-unresolved exits 1");
    assert!(
        found.stdout.contains("import-unresolved") && found.stdout.contains("./missing"),
        "it names the specifier: {}",
        found.stdout
    );
}

#[test]
fn parse_failed_exits_one() {
    let repo = clean().file(
        "src/shared/a.ts",
        "import './s';\nconst a = 1;\nconst = ;\n",
    );
    let found = run(&repo, &["check"]);
    assert_eq!(found.status, 1, "parse-failed exits 1");
    assert!(
        found.stdout.contains("parse-failed") && found.stdout.contains("line 3"),
        "it lists the error on line 3: {}",
        found.stdout
    );
}

#[test]
fn resolution_file_invalid_exits_one() {
    let repo = clean().file(
        "tsconfig.json",
        "{\n  \"compilerOptions\": {\n    \"strict\": true\n    \"baseUrl\": \".\"\n  }\n}\n",
    );
    let found = run(&repo, &["check"]);
    assert_eq!(found.status, 1, "resolution-file-invalid exits 1");
    assert_eq!(
        found
            .stdout
            .matches("error[resolution-file-invalid]")
            .count(),
        1,
        "one diagnostic for the tsconfig: {}",
        found.stdout
    );
    assert!(
        found.stdout.contains("tsconfig.json"),
        "it names the file: {}",
        found.stdout
    );
}

fn feature_files() -> FixtureRepo {
    FixtureRepo::new()
        .file("src/app/main.ts", "import '../features/login/a';\n")
        .file(
            "src/features/login/a.ts",
            "import './b';\nimport '../signup/c';\n",
        )
        .file("src/features/login/b.ts", "")
        .file("src/features/signup/c.ts", "")
        .file("src/shared/s.ts", "")
        .file("scripts/seed.ts", "")
        .json_config(&layered(&[
            json!({ "kind": "isolate", "name": "isolate", "element": "feature" }),
        ]))
}

#[test]
fn explain_prints_the_element_imports_and_broken_rules() {
    let found = run(&feature_files(), &["explain", "src/features/login/a.ts"]);
    assert_eq!(found.status, 0, "explain answers: {}", found.stderr);
    assert_eq!(
        found.stdout,
        "src/features/login/a.ts\n  \
         element: \"feature\", instance \"login\"\n  \
         selectors: none\n  \
         rules that apply: layers, isolate\n    \
         layers: files in feature may not import app\n    \
         layers: files in shared may not import app, feature\n    \
         isolate: files in feature may not import feature in a different instance\n  \
         imports:\n    \
         line 1  './b' -> src/features/login/b.ts (\"feature\", instance \"login\")\n    \
         line 2  '../signup/c' -> src/features/signup/c.ts (\"feature\", instance \"signup\")  breaks isolate\n  \
         imported by: src/app/main.ts\n",
        "the element instance, each import with where it resolves and what it breaks, and the importers"
    );
}

#[test]
fn explain_lists_selectors_and_rule_sentences() {
    let repo = feature_files()
        .file("src/features/login/a.test.ts", "")
        .json_config(&json!({
            "elements": { "app": "src/app/**", "feature": "src/features/{feature}/**" },
            "selectors": { "test": "**/*.test.ts" },
            "rules": [{ "kind": "isolate", "element": "feature" }],
        }));
    let found = run(&repo, &["explain", "src/features/login/a.test.ts"]);
    assert_eq!(found.status, 0, "explain answers: {}", found.stderr);
    assert!(
        found
            .stdout
            .contains("  element: \"feature\", instance \"login\"\n  selectors: test\n"),
        "the element with its capture, and the selector: {}",
        found.stdout
    );
    assert!(
        found.stdout.contains(
            "    isolate(feature): files in feature may not import feature in a different instance\n"
        ),
        "the isolate rule as its sentence: {}",
        found.stdout
    );
}

#[test]
fn explain_says_why_a_file_is_in_no_element() {
    let repo = feature_files().file("src/util.ts", "").json_config(&json!({
        "elements": { "app": "src/app/**", "service": "src/**/*.service.ts" },
        "rules": [],
    }));
    let found = run(&repo, &["explain", "src/util.ts"]);
    assert_eq!(found.status, 0, "explain answers: {}", found.stderr);
    assert_eq!(
        found.stdout,
        "src/util.ts\n  \
         element: no element; no file pattern matches it:\n    \
         \"app\" src/app/**: the path does not start with \"src/app/\"\n    \
         \"service\" src/**/*.service.ts: the path starts with \"src/\", but the rest does not match\n  \
         selectors: none\n  \
         rules that apply: none\n  \
         imports:\n  \
         imported by: none\n",
        "each pattern, with the part of the path it fails on"
    );
}

/// A rule of each kind, each naming its elements in a different field.
fn every_rule() -> FixtureRepo {
    FixtureRepo::new()
        .file("src/app/main.ts", "")
        .file("src/features/login/index.ts", "")
        .file("src/entities/user/index.ts", "")
        .file("src/shared/s.ts", "")
        .json_config(&json!({
            "elements": {
                "app": "src/app/**",
                "feature": "src/features/{feature}/**",
                "entity": "src/entities/{entity}/**",
                "shared": "src/shared/**",
            },
            "rules": [
                { "kind": "layers", "name": "layers", "order": ["app", "feature", "shared"] },
                { "kind": "isolate", "name": "isolate", "element": "feature" },
                { "kind": "entry", "name": "entry", "element": "entity", "files": ["index.ts"] },
                { "name": "only", "files": "shared", "mayOnlyBeImportedBy": ["feature"] },
                { "name": "forbid", "files": "app", "mayNotImport": "entity" },
                { "name": "cycles", "files": "src/**", "noCycles": true },
            ],
        }))
}

#[test]
fn explain_lists_only_the_rules_that_name_the_files_element() {
    let repo = every_rule();
    for (file, rules) in [
        ("src/app/main.ts", "layers, forbid, cycles"),
        (
            "src/features/login/index.ts",
            "layers, isolate, only, cycles",
        ),
        ("src/entities/user/index.ts", "entry, forbid, cycles"),
        ("src/shared/s.ts", "layers, only, cycles"),
    ] {
        let found = run(&repo, &["explain", file]);
        assert_eq!(found.status, 0, "explain answers: {}", found.stderr);
        let line = found
            .stdout
            .lines()
            .find(|line| line.starts_with("  rules that apply: "))
            .unwrap_or_default();
        assert_eq!(
            line,
            format!("  rules that apply: {rules}"),
            "{file}: a rule applies when any of its fields names the element, and no-cycles \
             applies everywhere: {}",
            found.stdout
        );
    }
}

#[test]
fn explain_lists_the_lines_of_computed_specifiers() {
    let repo = clean().file(
        "src/shared/load.ts",
        "export const a = 1;\nconst page = import(`./pages/${a}`);\nconst m = require(a);\n",
    );
    let found = run(&repo, &["explain", "src/shared/load.ts"]);
    assert_eq!(found.status, 0, "explain answers");
    assert!(
        found.stdout.contains("cannot follow")
            && found.stdout.contains("line 2")
            && found.stdout.contains("line 3"),
        "it lists both lines: {}",
        found.stdout
    );
}

fn chain() -> FixtureRepo {
    FixtureRepo::new()
        .file(
            "node_modules/drizzle-orm/package.json",
            r#"{ "name": "drizzle-orm", "main": "index.js" }"#,
        )
        .file("node_modules/drizzle-orm/index.js", "")
        .file(".gitignore", "node_modules/\n")
        .file("src/a.ts", "export const x = 1;\nimport './b';\n")
        .file("src/b.ts", "import './c';\n")
        .file("src/c.ts", "\n\nimport 'drizzle-orm';\n")
        .file("src/d.tsx", "import './e';\n")
        .file("src/e.tsx", "import './e.module.css';\n")
        .file("src/e.module.css", "")
        .file("src/lone.ts", "")
        .json_config(&json!({ "elements": { "src": "src/**" }, "rules": [{ "name": "no-cycles", "files": "**", "noCycles": true }] }))
}

#[test]
fn why_prints_the_shortest_chain_with_lines() {
    let found = run(&chain(), &["why", "src/a.ts", "drizzle-orm"]);
    assert_eq!(found.status, 0, "a chain exists: {}", found.stderr);
    let steps: Vec<&str> = found
        .stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();
    assert_eq!(steps.len(), 4, "four steps: {}", found.stdout);
    for (step, (name, line)) in steps.iter().zip([
        ("src/a.ts", "2"),
        ("src/b.ts", "1"),
        ("src/c.ts", "3"),
        ("drizzle-orm", ""),
    ]) {
        assert!(step.contains(name), "the step names {name}: {step}");
        if !line.is_empty() {
            assert!(
                step.contains(&format!(":{line}")),
                "{name} imports the next on line {line}: {step}"
            );
        }
    }
}

#[test]
fn why_reaches_a_file_that_is_not_a_source_file() {
    let found = run(&chain(), &["why", "src/d.tsx", "src/e.module.css"]);
    assert_eq!(found.status, 0, "a chain exists: {}", found.stderr);
    for name in ["src/d.tsx", "src/e.tsx", "src/e.module.css"] {
        assert!(
            found.stdout.contains(name),
            "the chain has {name}: {}",
            found.stdout
        );
    }
}

#[test]
fn why_without_a_chain_says_so_and_exits_one() {
    let found = run(&chain(), &["why", "src/lone.ts", "src/a.ts"]);
    assert_eq!(found.status, 1, "no chain exits 1");
    assert!(
        found.stdout.contains("does not reach"),
        "it says so: {}",
        found.stdout
    );
}

#[test]
fn stats_prints_work_counts_and_stage_durations() {
    let repo = (0..12).fold(FixtureRepo::new(), |repo, i| repo.file(&format!("src/shared/f{i}.ts"), ""))
        .json_config(&json!({ "elements": { "shared": "src/shared/**" }, "rules": [{ "name": "no-cycles", "files": "**", "noCycles": true }] }));
    let found = run(&repo, &["check", "--stats"]);
    assert_eq!(found.status, 0, "a clean check: {}", found.stdout);
    for line in ["files read: 12", "files parsed: 12"] {
        assert!(
            found.stdout.contains(line),
            "the stats have {line}: {}",
            found.stdout
        );
    }
    for stage in ["config", "graph", "rules"] {
        let has = found
            .stdout
            .lines()
            .any(|l| l.contains(stage) && l.contains("ms"));
        assert!(has, "the stats time the {stage} stage: {}", found.stdout);
    }
}

#[test]
fn explain_shows_a_one_word_brace_as_a_capture() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "")
        .json_config(&json!({ "parts": { "code": "src/**/*.{ts}" } }));
    let found = run(&repo, &["explain", "src/a.ts"]);
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    assert!(
        found.stdout.contains("part: \"code\", instance \"ts\""),
        "the part with the capture's value: {}",
        found.stdout
    );
}

#[test]
fn explain_shows_a_file_in_a_part_and_in_a_part_of_a_preset() {
    let repo = FixtureRepo::new()
        .file("apps/web/src/features/cart/a.ts", "")
        .file(
            "archangel.config.mjs",
            "export default {
               extends: [{
                 name: 'fsd-web',
                 root: 'apps/web/src',
                 parts: { features: 'features/{slice}/**' },
               }],
               parts: { web: 'apps/web/**' },
             };",
        );
    let found = run(&repo, &["explain", "apps/web/src/features/cart/a.ts"]);
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    for line in [
        "part: \"web\"\n",
        "part: \"fsd-web/features\", instance \"cart\"\n",
    ] {
        assert!(found.stdout.contains(line), "{line} in: {}", found.stdout);
    }
}

#[test]
fn two_parts_that_match_one_file_exit_one() {
    let repo = FixtureRepo::new()
        .file("src/features/login/session.model.ts", "")
        .json_config(&json!({
            "parts": { "model": "src/**/*.model.ts", "feature": "src/features/{feature}/**" },
        }));
    let found = run(&repo, &["check"]);
    assert_eq!(found.status, 1, "{}{}", found.stdout, found.stderr);
    assert!(found.stdout.contains("part-overlap"), "{}", found.stdout);
}

/// A table with one kind that has files and one that has none.
fn table_repo() -> FixtureRepo {
    FixtureRepo::new()
        .file("apps/api/a.entity.ts", "")
        .json_config(&json!({
            "parts": { "api": "apps/api/**" },
            "rules": [{
                "kind": "mayImport",
                "name": "domain",
                "rows": { "entity": [], "aggregate-root": [] },
                "kinds": "**/*.{kind}.ts",
            }],
        }))
}

#[test]
fn a_row_with_no_file_is_listed_in_the_summary_as_text_and_json() {
    let repo = table_repo();
    let text = run(&repo, &["check"]);
    assert_eq!(text.status, 0, "{}{}", text.stdout, text.stderr);
    assert!(
        text.stdout
            .contains("domain.aggregate-root has no file in this repository"),
        "{}",
        text.stdout
    );
    let json: Value =
        serde_json::from_str(&run(&repo, &["check", "--format", "json"]).stdout).unwrap();
    assert_eq!(
        json["summary"]["rowsWithNoFile"],
        json!(["domain.aggregate-root"]),
        "a row with no file is listed in the summary as text and json"
    );
}

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
fn explain_shows_a_table_row_as_its_sentences() {
    let found = run(
        &fixtures::fixture("enhance3-slice"),
        &["explain", "apps/api/src/board/domain/board.entity.ts"],
    );
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    let out = &found.stdout;
    assert!(out.contains("kind: entity in domain\n"), "{out}");
    assert!(out.contains("domain.entity"), "{out}");
    assert!(
        out.contains(
            "entity may import only value-object, domain-event, specification, policy, error, \
             types, consts, apps/api/src/shared/result.ts, and itself"
        ),
        "{out}"
    );
}

#[test]
fn explain_shows_the_second_sentence_of_a_row_that_may_not_import_itself() {
    let found = run(
        &fixtures::fixture("enhance3-slice"),
        &[
            "explain",
            "apps/api/src/board/domain/created.domain-event.ts",
        ],
    );
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    let out = &found.stdout;
    assert!(
        out.contains("domain-event may import only value-object, types, consts, apps/api/src/shared/result.ts\n"),
        "{out}"
    );
    assert!(out.contains("domain-event may not import itself"), "{out}");
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

fn because_repo(key: &str) -> FixtureRepo {
    FixtureRepo::new()
        .file("src/a/x.ts", "import '../b/y';\n")
        .file("src/b/y.ts", "")
        .json_config(&json!({
            "elements": { "a": "src/a/**", "b": "src/b/**" },
            "rules": [{ "name": "r", "files": "a", "mayNotImport": "b", key: "They stay apart." }],
        }))
}

#[test]
fn a_rule_written_with_because_prints_because() {
    let found = run(&because_repo("because"), &["check"]);
    assert!(
        found.stdout.contains("because: They stay apart."),
        "{}",
        found.stdout
    );
    assert!(!found.stdout.contains("reason:"), "{}", found.stdout);
}

#[test]
fn a_rule_written_with_reason_prints_reason() {
    let found = run(&because_repo("reason"), &["check"]);
    assert!(
        found.stdout.contains("reason: They stay apart."),
        "{}",
        found.stdout
    );
    assert!(!found.stdout.contains("because:"), "{}", found.stdout);
}

#[test]
fn a_row_error_says_what_to_do() {
    let found = run(&fixtures::fixture("small-app"), &["check"]);
    assert!(
        found.stdout.contains(
            "= help: remove the import, or move the code to a file whose row permits the import"
        ),
        "{}",
        found.stdout
    );
}

#[test]
fn an_unknown_name_in_a_config_with_no_names_does_not_list_nothing() {
    let repo = FixtureRepo::new().file("src/routes/a.ts", "").json_config(&json!({
        "rules": [
            { "kind": "mayImport", "name": "t", "rows": { "routes": [] }, "kinds": "src/{kind}/**" },
            { "name": "r", "files": "routes", "mustBeNamed": "kebab-case" },
        ],
    }));
    let found = run(&repo, &["check"]);
    assert_eq!(found.status, 2, "{}{}", found.stdout, found.stderr);
    let all = format!("{}{}", found.stdout, found.stderr);
    assert!(!all.contains("the declared names are ;"), "{all}");
    assert!(!all.contains("packages: ['routes']"), "{all}");
    assert!(all.contains("only in its table"), "{all}");
}
