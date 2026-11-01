//! Scenarios from the command-line, import-graph and architecture-rules specs that need the binary:
//! exit statuses, `check` with paths, `explain`, `why` and `--stats`.
#![cfg(test)]

#[path = "support/cli.rs"]
mod support;

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
        found.stdout.contains("element: \"code\", instance \"ts\""),
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
        "element: \"web\"\n",
        "element: \"fsd-web/features\", instance \"cart\"\n",
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
