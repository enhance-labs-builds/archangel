//! Scenarios from the exceptions-and-baselines spec: a baseline permits the violations it records,
//! up to their count, and reports what it no longer matches.
#![cfg(test)]

#[path = "support/check.rs"]
mod support;

use archangel_rules::{Baseline, BaselineEntry, Code, Report};
use archangel_testkit::FixtureRepo;
use serde_json::{Value, json};

fn config() -> Value {
    json!({
        "elements": { "feature": "src/features/{feature}/**", "shared": "src/shared/**" },
        "rules": [{ "kind": "layers", "order": ["feature", "shared"] }, { "name": "no-cycles", "files": "**", "noCycles": true }],
    })
}

fn check(repo: &FixtureRepo, baseline: Option<&Baseline>) -> Report {
    support::check(&config(), repo, baseline).0
}

fn codes(report: &Report) -> Vec<Code> {
    report.diagnostics.iter().map(|d| d.code).collect()
}

fn violating(text: &str) -> FixtureRepo {
    FixtureRepo::new()
        .file("src/shared/a.ts", text)
        .file("src/features/login/b.ts", "")
}

#[test]
fn baseline_entry_matches_after_lines_move() {
    let before = violating("\n\n\nimport '../features/login/b';\n");
    let baseline = Baseline::from_report(&check(&before, None));
    let after = violating(&format!(
        "{}import '../features/login/b';\n",
        "// moved\n".repeat(13)
    ));
    assert_eq!(
        codes(&check(&after, Some(&baseline))),
        [],
        "the entry still matches"
    );
}

#[test]
fn violation_beyond_the_recorded_count_fails() {
    let baseline =
        Baseline::from_report(&check(&violating("import '../features/login/b';\n"), None));
    let report = check(
        &violating("import '../features/login/b';\nimport '../features/login/b';\n"),
        Some(&baseline),
    );
    assert_eq!(codes(&report), [Code::Layers], "the second import fails");
    assert_eq!(
        report.summary.baselined.get("layers(feature, shared)"),
        Some(&1),
        "one is matched"
    );
}

#[test]
fn fixed_violation_makes_the_entry_baseline_stale() {
    let baseline =
        Baseline::from_report(&check(&violating("import '../features/login/b';\n"), None));
    let report = check(&violating(""), Some(&baseline));
    let stale = report
        .diagnostics
        .iter()
        .find(|d| d.code == Code::BaselineStale)
        .expect("a stale entry");
    assert!(
        stale.message.contains("archangel baseline prune"),
        "it names the command: {}",
        stale.message
    );
}

#[test]
fn entry_for_a_removed_rule_is_baseline_stale() {
    let text = r#"{ "entries": [{ "rule": "entry(widget)", "file": "src/a.ts", "specifier": "./b", "count": 1 }] }"#;
    let baseline = Baseline::parse(text).expect("the baseline parses");
    let report = check(&violating(""), Some(&baseline));
    let stale = report
        .diagnostics
        .iter()
        .find(|d| d.code == Code::BaselineStale)
        .expect("a stale entry");
    assert!(
        stale.message.contains("entry(widget)") && stale.message.contains("does not exist"),
        "{}",
        stale.message
    );
}

#[test]
fn cycle_group_that_gains_a_file_is_a_new_violation() {
    let two = FixtureRepo::new()
        .file("src/a.ts", "import './b';\n")
        .file("src/b.ts", "import './a';\n");
    let baseline = Baseline::from_report(&check(&two, None));
    let three = FixtureRepo::new()
        .file("src/a.ts", "import './b';\n")
        .file("src/b.ts", "import './c';\n")
        .file("src/c.ts", "import './a';\n");
    let report = check(&three, Some(&baseline));
    assert!(
        codes(&report).contains(&Code::NoCycles),
        "the larger group is a new violation"
    );
    assert!(
        codes(&report).contains(&Code::BaselineStale),
        "the old entry is stale"
    );
}

fn entry(rule: &str, file: &str, count: u32) -> BaselineEntry {
    BaselineEntry {
        rule: rule.to_owned(),
        file: Some(file.to_owned()),
        specifier: Some("./b".to_owned()),
        files: Vec::new(),
        path: None,
        count,
    }
}

#[test]
fn prune_lowers_each_count_to_the_violations_left_and_drops_fixed_entries() {
    let recorded = Baseline {
        entries: vec![
            entry("forbid", "src/a.ts", 3),
            entry("forbid", "src/b.ts", 1),
            entry("forbid", "src/c.ts", 2),
        ],
    };
    let current = Baseline {
        entries: vec![
            entry("forbid", "src/a.ts", 1),
            entry("forbid", "src/b.ts", 4),
            entry("forbid", "src/d.ts", 1),
        ],
    };
    assert_eq!(
        recorded.pruned(&current),
        Baseline {
            entries: vec![
                entry("forbid", "src/a.ts", 1),
                entry("forbid", "src/b.ts", 1)
            ],
        },
        "a.ts is lowered to 1, b.ts keeps its 1, c.ts was fixed, and d.ts is never added"
    );
}

#[test]
fn baseline_is_written_as_sorted_pretty_json_ending_in_a_newline() {
    let baseline = Baseline {
        entries: vec![
            BaselineEntry {
                rule: "no-cycles".to_owned(),
                file: None,
                specifier: None,
                files: vec!["src/a.ts".to_owned(), "src/b.ts".to_owned()],
                path: None,
                count: 1,
            },
            entry("forbid", "src/a.ts", 2),
        ],
    };
    let text = baseline.to_json();
    assert_eq!(
        text,
        r#"{
  "entries": [
    {
      "rule": "forbid",
      "file": "src/a.ts",
      "specifier": "./b",
      "count": 2
    },
    {
      "rule": "no-cycles",
      "files": [
        "src/a.ts",
        "src/b.ts"
      ],
      "count": 1
    }
  ]
}
"#,
        "the entries sorted by rule, each without the fields it does not use"
    );
    let mut sorted = baseline.clone();
    sorted.entries.reverse();
    assert_eq!(
        Baseline::parse(&text),
        Ok(sorted),
        "the written text reads back as the sorted entries"
    );
}

#[test]
fn a_folder_violations_identity_is_its_rule_and_path() {
    let config = json!({
        "elements": { "src": "src/**" },
        "rules": [{ "name": "kebab-folders", "folders": "src/**", "mustBeNamed": "kebab-case" }],
    });
    let repo = FixtureRepo::new().file("src/userProfile/a.ts", "");
    let first = support::check(&config, &repo, None).0;
    let baseline = Baseline::from_report(&first);
    assert_eq!(
        baseline.entries,
        [BaselineEntry {
            rule: "kebab-folders".into(),
            file: None,
            specifier: None,
            files: Vec::new(),
            path: Some("src/userProfile".into()),
            count: 1,
        }],
        "the entry is the rule and the folder's path"
    );
    let grown = FixtureRepo::new()
        .file("src/userProfile/a.ts", "")
        .file("src/userProfile/b.ts", "")
        .bytes("src/userProfile/c.png", b"\x89PNG");
    let second = support::check(&config, &grown, Some(&baseline)).0;
    assert_eq!(
        codes(&second),
        [],
        "the baseline still matches after files are added to the folder: {:?}",
        second.diagnostics
    );
    assert_eq!(
        second.summary.baselined.get("kebab-folders"),
        Some(&1),
        "the baseline matched the folder"
    );
}
