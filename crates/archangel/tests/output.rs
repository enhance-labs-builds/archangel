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

#[test]
fn output_is_never_truncated() {
    let violators = (0..5000).fold(base(), |repo, i| {
        repo.file(&format!("src/shared/v{i}.ts"), "import '../app/main';\n")
    });
    let repo = (0..300)
        .fold(violators, |repo, i| {
            repo.file(
                &format!("src/shared/c{i}.ts"),
                &format!("import './c{}';\n", (i + 1) % 300),
            )
        })
        .json_config(&json!({
            "elements": layered()["elements"],
            "rules": [layered()["rules"][0], { "name": "no-cycles", "files": "**", "noCycles": true }],
        }));
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
