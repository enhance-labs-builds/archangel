//! Scenarios from the diagnostics spec: text for people, JSON for programs, the same bytes every
//! time, and every diagnostic printed.
#![cfg(test)]

#[path = "support/cli.rs"]
mod support;

use archangel_testkit::FixtureRepo;
use serde_json::{Value, json};
use support::{Finished, command, run};

fn layered() -> Value {
    json!({
        "elements": {
            "app": "src/app/**",
            "feature": "src/features/{feature}/**",
            "entity": "src/entities/{entity}/**",
            "shared": "src/shared/**",
        },
        "rules": [
            { "kind": "layers", "name": "layers", "order": ["app", "feature", "entity", "shared"],
              "reason": "Shared code must not know about features" },
            { "kind": "entry", "name": "entry(entity)", "element": "entity", "files": ["index.ts"] },
        ],
    })
}

fn base() -> FixtureRepo {
    FixtureRepo::new()
        .file("src/app/main.ts", "")
        .file("src/features/login/index.ts", "")
        .file(
            "src/entities/canvas/index.ts",
            "export * from './model/store';\n",
        )
        .file(
            "src/entities/canvas/model/store.ts",
            "export const store = 1;\n",
        )
        .file("src/shared/s.ts", "")
        .json_config(&layered())
}

fn one_entry_violation() -> FixtureRepo {
    base().file(
        "src/features/login/a.ts",
        "import { store } from '../../entities/canvas/model/store';\n",
    )
}

#[test]
fn text_output_shows_the_source_line_and_a_mark() {
    let found = run(&one_entry_violation(), &["check"]);
    assert_eq!(found.status, 1, "the violation exits 1: {}", found.stderr);
    insta::assert_snapshot!(found.stdout);
}

#[test]
fn text_output_to_a_pipe_has_no_color() {
    let found = run(&one_entry_violation(), &["check"]);
    assert!(
        !found.stdout.contains('\u{1b}'),
        "no escape codes in a pipe: {:?}",
        found.stdout
    );
}

/// Runs `archangel check` on a pseudo-terminal, with extra environment variables.
#[cfg(unix)]
fn on_a_terminal(repo: &FixtureRepo, env: &[(&str, &str)]) -> String {
    let binary = assert_cmd::cargo::cargo_bin("archangel");
    let mut script = std::process::Command::new("script");
    if cfg!(target_os = "macos") {
        script.args(["-q", "/dev/null"]).arg(&binary).arg("check");
    } else {
        script
            .args(["-qec"])
            .arg(format!("{} check", binary.display()))
            .arg("/dev/null");
    }
    script
        .current_dir(repo.path())
        .env_remove("NO_COLOR")
        .envs(env.iter().copied());
    String::from_utf8_lossy(&script.output().unwrap().stdout).into_owned()
}

#[cfg(unix)]
#[test]
fn no_color_turns_color_off_on_a_terminal() {
    let repo = one_entry_violation();
    let colored = on_a_terminal(&repo, &[]);
    assert!(
        colored.contains('\u{1b}'),
        "a terminal gets color, so the control is real: {colored:?}"
    );
    let plain = on_a_terminal(&repo, &[("NO_COLOR", "1")]);
    assert!(plain.contains("entry"), "the check ran: {plain:?}");
    assert!(
        !plain.contains('\u{1b}'),
        "NO_COLOR turns color off: {plain:?}"
    );
}

fn json_run(repo: &FixtureRepo) -> (Finished, Value) {
    let found = run(repo, &["check", "--format", "json"]);
    let document = serde_json::from_str(&found.stdout)
        .unwrap_or_else(|error| panic!("one JSON document, not {:?}: {error}", found.stdout));
    (found, document)
}

#[test]
fn json_output_is_one_versioned_document() {
    let (_, document) = json_run(&one_entry_violation());
    assert_eq!(document["schemaVersion"], 1, "the schema version");
    assert_eq!(
        document["diagnostics"].as_array().map(Vec::len),
        Some(1),
        "one diagnostic: {document}"
    );
    assert!(document["summary"].is_object(), "a summary: {document}");
}

#[test]
fn json_summary_has_the_work_counts() {
    let repo = (0..12).fold(FixtureRepo::new(), |repo, i| repo.file(&format!("src/shared/f{i}.ts"), ""))
        .json_config(&json!({ "elements": { "shared": "src/shared/**" }, "rules": [{ "name": "no-cycles", "files": "**", "noCycles": true }] }));
    let (_, document) = json_run(&repo);
    let work = &document["summary"]["work"];
    assert_eq!(
        (work["filesRead"].as_u64(), work["filesParsed"].as_u64()),
        (Some(12), Some(12)),
        "{document}"
    );
}

#[test]
fn json_stats_give_the_duration_of_each_stage_and_step() {
    let found = run(
        &one_entry_violation(),
        &["check", "--format", "json", "--stats"],
    );
    let document: Value = serde_json::from_str(&found.stdout)
        .unwrap_or_else(|error| panic!("one JSON document, not {:?}: {error}", found.stdout));
    let durations = document["summary"]["durations"]
        .as_object()
        .unwrap_or_else(|| panic!("the durations: {document}"));
    assert_eq!(
        durations
            .iter()
            .map(|(name, value)| (name.as_str(), value.is_f64()))
            .collect::<Vec<_>>(),
        [
            ("configMs", true),
            ("graphMs", true),
            ("listFilesMs", true),
            ("readAndParseMs", true),
            ("resolveMs", true),
            ("rulesMs", true),
        ],
        "each stage and step, as a number of milliseconds: {document}"
    );
}

#[test]
fn text_summary_counts_permitted_unresolved_and_computed_imports() {
    let repo = FixtureRepo::new()
        .file(
            "src/a.ts",
            "import 'virtual:a';\nimport 'virtual:b';\nconst m = require(name);\n\
             const p = import(`./${name}`);\nexport const name = 'x';\n",
        )
        .file("src/b.ts", "")
        .json_config(&json!({
            "elements": { "src": "src/**" },
            "allowUnresolved": { "virtual:*": "made by the bundler" },
            "rules": [],
        }));
    let found = run(&repo, &["check"]);
    assert_eq!(found.status, 0, "nothing breaks a rule: {}", found.stderr);
    assert_eq!(
        found.stdout,
        "checked 2 files and 2 imports: no problems\n  \
         2 unresolved imports permitted by allowUnresolved\n  \
         2 imports with a computed specifier, which archangel cannot follow; `archangel explain` lists them\n",
        "what the config permitted and what archangel could not follow, after the count"
    );
}

#[test]
fn json_summary_counts_exceptions_and_baseline_per_rule() {
    let allow = "// archangel-allow layers: the old router\nimport '../app/main';\n";
    let repo = base()
        .file("src/shared/x1.ts", allow)
        .file("src/shared/x2.ts", allow)
        .file("src/shared/y1.ts", "import '../app/main';\n")
        .file("src/shared/y2.ts", "import '../app/main';\n")
        .file("src/shared/y3.ts", "import '../app/main';\n");
    assert_eq!(
        run(&repo, &["baseline", "add"]).status,
        0,
        "the baseline is written"
    );
    let (found, document) = json_run(&repo);
    assert_eq!(found.status, 0, "every violation is permitted: {document}");
    let summary = &document["summary"];
    assert_eq!(
        (
            summary["exceptions"]["layers"].as_u64(),
            summary["baselined"]["layers"].as_u64()
        ),
        (Some(2), Some(3)),
        "(exceptions, baselined) for layers: {document}"
    );
}

#[test]
fn diagnostics_are_sorted_by_path_then_line() {
    let repo = base()
        .file("src/shared/b.ts", "\nimport '../app/main';\n")
        .file(
            "src/shared/a.ts",
            "\nimport '../app/main';\n\n\n\n\n\n\nimport '../features/login/index';\n",
        );
    let (_, document) = json_run(&repo);
    let order: Vec<(String, u64)> = document["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            (
                d["file"].as_str().unwrap_or_default().to_owned(),
                d["line"].as_u64().unwrap_or(0),
            )
        })
        .collect();
    let expected = [
        ("src/shared/a.ts", 2),
        ("src/shared/a.ts", 9),
        ("src/shared/b.ts", 2),
    ]
    .map(|(f, l)| (f.to_owned(), l));
    assert_eq!(order, expected, "path, then line");
}

/// 500 files, 40 of which break the layers rule.
fn many() -> FixtureRepo {
    (0..500).fold(base(), |repo, i| {
        let text = if i % 25 == 0 {
            "import '../app/main';\nimport '../features/login/index';\n"
        } else {
            "import './s';\n"
        };
        repo.file(&format!("src/shared/m{i}.ts"), text)
    })
}

#[test]
fn output_is_the_same_with_one_thread_and_with_many() {
    let repo = many();
    let outputs: Vec<String> = ["1", "16"]
        .iter()
        .map(|threads| {
            let found: Finished = command(&repo)
                .arg("check")
                .env("RAYON_NUM_THREADS", threads)
                .output()
                .unwrap()
                .into();
            found.stdout
        })
        .collect();
    assert_eq!(
        outputs[0].matches("[layers]").count(),
        40,
        "40 violations: {}",
        outputs[0]
    );
    assert_eq!(
        outputs[0], outputs[1],
        "the same bytes with 1 thread and 16"
    );
}

/// 5,000 violations and one cycle of 300 files.
fn very_many() -> FixtureRepo {
    let violators = (0..5000).fold(base(), |repo, i| {
        repo.file(&format!("src/shared/v{i}.ts"), "import '../app/main';\n")
    });
    (0..300)
        .fold(violators, |repo, i| {
            repo.file(
                &format!("src/shared/c{i}.ts"),
                &format!("import './c{}';\n", (i + 1) % 300),
            )
        })
        .json_config(&json!({
            "elements": layered()["elements"],
            "rules": [layered()["rules"][0], { "name": "no-cycles", "files": "**", "noCycles": true }],
        }))
}

#[test]
fn output_is_never_truncated() {
    let repo = very_many();
    let (_, document) = json_run(&repo);
    let diagnostics = document["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 5001, "5,000 violations and one cycle");
    let cycle = diagnostics
        .iter()
        .find(|d| d["code"] == "no-cycles")
        .unwrap();
    assert_eq!(
        cycle["cycle"].as_array().map(Vec::len),
        Some(301),
        "all 300 files and back to the first"
    );
}

/// The elements of the fixtures of `test_command.rs`, with their rules.
fn example_repo(rules: &Value) -> FixtureRepo {
    FixtureRepo::new()
        .file("src/app/main.ts", "")
        .file("src/shared/s.ts", "")
        .json_config(&json!({
            "elements": { "app": "src/app/**", "shared": "src/shared/**" },
            "rules": rules,
        }))
}

/// Each fixture of this file and of `test_command.rs`, by name.
fn recorded_fixtures() -> Vec<(&'static str, FixtureRepo)> {
    let allow = "// archangel-allow layers: the old router\nimport '../app/main';\n";
    let permitted = base()
        .file("src/shared/x1.ts", allow)
        .file("src/shared/x2.ts", allow)
        .file("src/shared/y1.ts", "import '../app/main';\n")
        .file("src/shared/y2.ts", "import '../app/main';\n")
        .file("src/shared/y3.ts", "import '../app/main';\n");
    assert_eq!(
        run(&permitted, &["baseline", "add"]).status,
        0,
        "a baseline"
    );
    let unresolved = FixtureRepo::new()
        .file(
            "src/a.ts",
            "import 'virtual:a';\nimport 'virtual:b';\nconst m = require(name);\n\
             const p = import(`./${name}`);\nexport const name = 'x';\n",
        )
        .file("src/b.ts", "")
        .json_config(&json!({
            "elements": { "src": "src/**" },
            "allowUnresolved": { "virtual:*": "made by the bundler" },
            "rules": [],
        }));
    let cycles = (0..12)
        .fold(FixtureRepo::new(), |repo, i| {
            repo.file(&format!("src/shared/f{i}.ts"), "")
        })
        .json_config(&json!({
            "elements": { "shared": "src/shared/**" },
            "rules": [{ "name": "no-cycles", "files": "**", "noCycles": true }],
        }));
    let sorted = base()
        .file("src/shared/b.ts", "\nimport '../app/main';\n")
        .file(
            "src/shared/a.ts",
            "\nimport '../app/main';\n\n\n\n\n\n\nimport '../features/login/index';\n",
        );
    let layers = |examples: Value| json!({ "kind": "layers", "name": "layers", "order": ["app", "shared"], "examples": examples });
    let accepted = json!([{ "src/app/x.ts": "import '../shared/s';\n" }]);
    let ten: Vec<Value> = (0..9)
        .map(|i| json!({ format!("src/shared/e{i}.ts"): "import '../app/main';\n" }))
        .chain(std::iter::once(
            json!({ "src/shared/s.ts": "import '../app/main';\n" }),
        ))
        .collect();
    vec![
        ("base", base()),
        ("one_entry_violation", one_entry_violation()),
        ("work_counts", cycles),
        ("unresolved_and_computed", unresolved),
        ("exceptions_and_baseline", permitted),
        ("sorted", sorted),
        ("many", many()),
        ("very_many", very_many()),
        (
            "examples_passing",
            example_repo(&json!([
                layers(json!({
                    "accepted": accepted,
                    "rejected": [{ "src/shared/x.ts": "import '../app/main';\n" }],
                })),
                { "name": "forbid", "files": "app", "mayNotImport": "app", "examples": {
                    "accepted": [{ "src/app/y.ts": "import '../shared/s';\n" }],
                    "rejected": [{ "src/app/y.ts": "import './main';\n" }],
                } },
            ])),
        ),
        (
            "examples_one_failed_two_unproven",
            example_repo(&json!([
                layers(json!({
                    "accepted": accepted,
                    "rejected": [
                        { "src/shared/x.ts": "import '../app/main';\n" },
                        { "src/shared/y.ts": "import './s';\n" },
                    ],
                })),
                { "name": "forbid", "files": "app", "mayNotImport": "app" },
                { "name": "forbid2", "files": "shared", "mayNotImport": "app" },
            ])),
        ),
        (
            "examples_failed",
            example_repo(&json!([layers(json!({
                "rejected": [{ "src/shared/x.ts": "import './s';\n" }],
            }))])),
        ),
        (
            "examples_replace_a_file",
            example_repo(&json!([
                { "kind": "layers", "name": "layers", "order": ["app", "shared"] },
                { "name": "forbid", "files": "shared", "mayNotImport": "shared", "examples": {
                    "rejected": [{ "src/shared/s.ts": "import { main } from '../app/main';\n" }],
                } },
            ])),
        ),
        (
            "examples_unproven",
            example_repo(&json!([
                { "name": "forbid", "files": "app", "mayNotImport": "app", "examples": {
                    "accepted": [{ "src/app/y.ts": "import '../shared/s';\n" }],
                } },
            ])),
        ),
        (
            "examples_ten",
            example_repo(&json!([layers(json!({ "rejected": ten }))])),
        ),
    ]
}

/// Fixtures whose recorded output 0.2.0 changes, each with the reason. A change to a recorded
/// output is one of the two that the release names: the self default of `mayOnlyImport` on a part
/// with a capture, and the type mark under `verbatimModuleSyntax`. No fixture has either.
const CHANGED_BY_THE_EXEMPT_CASES: [(&str, &str); 0] = [];

/// Text too large to keep in a snapshot is recorded as its length and a digest, which still
/// compares it byte for byte.
fn recorded(text: &str) -> String {
    if text.len() <= 16 * 1024 {
        return text.to_owned();
    }
    let digest = text.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    });
    format!("{} bytes, FNV-1a {digest:016x}\n", text.len())
}

#[test]
fn configs_without_the_new_forms_give_the_recorded_output() {
    for (name, repo) in recorded_fixtures() {
        assert!(
            !CHANGED_BY_THE_EXEMPT_CASES.iter().any(|(n, _)| *n == name),
            "{name} is recorded, so its output must not change"
        );
        let mut all = String::new();
        for args in [
            &["check"][..],
            &["check", "--format", "json"],
            &["test"],
            &["test", "--format", "json"],
        ] {
            let found = run(&repo, args);
            if args.contains(&"json") {
                let document: Value = serde_json::from_str(&found.stdout)
                    .unwrap_or_else(|error| panic!("{name}: {error}: {}", found.stdout));
                assert!(
                    document["summary"].get("rowsWithNoFile").is_none(),
                    "{name}: a config without a table has no rowsWithNoFile: {document}"
                );
            }
            all.push_str(&format!(
                "$ archangel {}\nstatus: {}\n{}\n",
                args.join(" "),
                found.status,
                recorded(&found.stdout)
            ));
        }
        insta::assert_snapshot!(format!("recorded_{name}"), all);
    }
}
