//! Scenarios from the rule-examples spec: the cost of an example does not grow with the repository.
#![cfg(test)]

use archangel_engine::Session;
use archangel_graph::Work;
use archangel_testkit::FixtureRepo;
use serde_json::json;

const EXAMPLES: usize = 50;

/// 1,000 files in the feature "x", whose imports an isolate rule evaluates, and a layers rule with
/// 50 rejected examples of one file and one import each. The isolate rule makes a check of the
/// whole graph cost evaluations, so the fault control can see one.
fn repo() -> FixtureRepo {
    let rejected: Vec<_> = (0..EXAMPLES)
        .map(|i| json!({ format!("src/shared/e{i}.ts"): "import '../features/x/index';\n" }))
        .collect();
    FixtureRepo::new()
        .synthetic("src/features/x", 999, 2)
        .file("src/features/x/index.ts", "export const x = 1;\n")
        .json_config(&json!({
            "elements": { "feature": "src/features/{feature}/**", "shared": "src/shared/**" },
            "rules": [
                {
                    "kind": "layers", "order": ["feature", "shared"],
                    "examples": { "rejected": rejected },
                },
                { "kind": "isolate", "element": "feature" },
            ],
        }))
}

/// The work of opening the session, and the work of its examples.
fn work() -> (Work, Work) {
    let repo = repo();
    let mut session = Session::open(repo.path(), None).expect("the session opens");
    let examples = session.run_examples();
    assert!(
        examples.runs.iter().all(|run| run.outcome.passed),
        "every example passes, so the counts are those of real checks: {:?}",
        examples.runs.iter().find(|run| !run.outcome.passed)
    );
    (*session.work(), examples.work)
}

/// The counts the scenario states, or what differs from them.
fn counts_match() -> Result<(), String> {
    let (open, examples) = work();
    let found = (
        open.files_parsed,
        examples.files_parsed,
        examples.resolutions,
        examples.rule_evaluations,
        examples.examples_checked,
    );
    let expected = (1000, 50, 50, 50, 50);
    if found == expected {
        Ok(())
    } else {
        Err(format!(
            "(repository parsed, example parsed, example resolutions, example evaluations, \
             examples checked) were {found:?}, expected {expected:?}"
        ))
    }
}

#[test]
fn examples_parse_only_their_own_files() {
    #[cfg(feature = "failpoints")]
    let _quiet = fail::FailScenario::setup();
    assert_eq!(
        counts_match(),
        Ok(()),
        "each example costs its own file and import only"
    );
}

#[cfg(feature = "failpoints")]
fn under_fault(point: &str) -> (Work, Work) {
    let scenario = fail::FailScenario::setup();
    fail::cfg(point, "return").unwrap();
    let found = work();
    scenario.teardown();
    found
}

#[cfg(feature = "failpoints")]
#[test]
fn example_work_count_rejects_a_reread_per_example() {
    let (_, examples) = under_fault("engine::reread_per_example");
    assert_ne!(
        examples.files_parsed, 50,
        "the count catches a reread per example"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn example_work_count_rejects_a_full_evaluation_per_example() {
    let (_, examples) = under_fault("engine::evaluate_graph_per_example");
    assert_ne!(
        examples.rule_evaluations, 50,
        "the count catches an evaluation of the whole graph per example"
    );
}

/// A layers rule with one accepted and one rejected example, which pass when `rejected` imports
/// the app from the shared layer, and with `unproven`, a rule without examples.
fn small(rejected: &str, unproven: bool) -> FixtureRepo {
    let mut rules = vec![json!({
        "kind": "layers", "order": ["app", "shared"],
        "examples": {
            "accepted": [{ "src/app/ok.ts": "import '../s';\n" }],
            "rejected": [{ "src/bad.ts": rejected }],
        },
    })];
    if unproven {
        rules.push(json!({ "name": "no-cycles", "files": "**", "noCycles": true }));
    }
    FixtureRepo::new()
        .file("src/app/main.ts", "")
        .file("src/s.ts", "")
        .json_config(&json!({
            "elements": { "app": "src/app/**", "shared": "src/**" },
            "rules": rules,
        }))
}

fn examples(repo: &FixtureRepo) -> archangel_engine::ExampleReport {
    Session::open(repo.path(), None)
        .expect("the session opens")
        .run_examples()
}

#[test]
fn the_examples_pass_when_each_passes_and_every_rule_is_proven() {
    #[cfg(feature = "failpoints")]
    let _quiet = fail::FailScenario::setup();
    let report = examples(&small("import './app/main';\n", false));
    assert_eq!(
        report
            .runs
            .iter()
            .map(|run| (run.number, run.outcome.passed))
            .collect::<Vec<_>>(),
        [(1, true), (1, true)],
        "(number, passed) of the accepted and the rejected example, each the first of its kind"
    );
    assert!(
        report.passed(),
        "every example passed and the rule is proven"
    );
}

#[test]
fn the_examples_fail_when_one_fails() {
    #[cfg(feature = "failpoints")]
    let _quiet = fail::FailScenario::setup();
    let report = examples(&small("import './s';\n", false));
    assert!(
        report.unproven.is_empty(),
        "the rule has a rejected example, so it is proven"
    );
    assert!(!report.passed(), "the rejected example breaks no rule");
}

#[test]
fn the_examples_fail_when_a_rule_is_unproven() {
    #[cfg(feature = "failpoints")]
    let _quiet = fail::FailScenario::setup();
    let report = examples(&small("import './app/main';\n", true));
    assert!(
        report.runs.iter().all(|run| run.outcome.passed),
        "every example passes"
    );
    assert!(
        !report.passed(),
        "the no-cycles rule has no rejected example: {:?}",
        report.unproven
    );
}

#[test]
fn examples_are_numbered_from_one_in_config_order() {
    #[cfg(feature = "failpoints")]
    let _quiet = fail::FailScenario::setup();
    let numbers: Vec<usize> = examples(&repo())
        .runs
        .iter()
        .map(|run| run.number)
        .collect();
    assert_eq!(
        numbers,
        (1..=EXAMPLES).collect::<Vec<_>>(),
        "the 50 rejected examples, numbered from 1"
    );
}

#[test]
fn the_test_command_writes_nothing() {
    #[cfg(feature = "failpoints")]
    let _quiet = fail::FailScenario::setup();
    let (_, examples) = work();
    assert_eq!(examples.files_written, 0, "no file is written");
}

#[cfg(feature = "failpoints")]
#[test]
fn written_files_count_rejects_examples_written_to_disk() {
    let (_, examples) = under_fault("engine::write_examples");
    assert_eq!(
        examples.files_written, 50,
        "the count catches each example's one file written"
    );
}
