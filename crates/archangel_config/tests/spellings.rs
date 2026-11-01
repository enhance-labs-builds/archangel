//! Scenarios from the configuration spec: `parts`, `groups`, `because` and a rule named first mean
//! what `elements`, `selectors`, `reason` and `name` mean, and a config uses one spelling of a key.
#![cfg(test)]

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

use archangel_config::{Config, ConfigProblem, ProblemCode, from_loader, from_value};
use archangel_testkit::FixtureRepo;
use serde_json::{Value, json};

/// The loader's first answer for a JavaScript config, as Node gives it to the engine.
fn loaded(config: &str) -> Value {
    let repo = FixtureRepo::new().file("archangel.config.mjs", config);
    let mut child = Command::new("node")
        .args([
            "--no-warnings",
            "--input-type=module",
            "-e",
            archangel_config::LOADER,
        ])
        .arg(repo.join("archangel.config.mjs"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Node starts");
    let mut line = String::new();
    BufReader::new(child.stdout.take().expect("the answer"))
        .read_line(&mut line)
        .expect("one line");
    drop(child.stdin.take());
    child.wait().expect("Node stops");
    serde_json::from_str(&line).expect("the loader's answer is JSON")
}

/// What a config means: its elements, selectors and rules, without the spelling they came from.
fn meaning(config: &Config) -> String {
    let elements: Vec<_> = config
        .elements
        .iter()
        .map(|element| (&element.name, format!("{:?}", element.kind)))
        .collect();
    format!("{elements:?} {:?} {:?}", config.selectors, config.rules)
}

#[test]
fn the_new_spellings_mean_the_old_ones() {
    let new = loaded(
        "export default {
           parts: { feature: 'src/features/{feature}/**' },
           groups: { tests: '**/*.test.ts' },
           rules: ({ rule }) => [
             rule('no-tests-in-features', {
               files: 'feature', mayNotImport: 'tests',
               because: 'Production code never imports a test.',
             }),
           ],
         };",
    );
    let old = loaded(
        "export default {
           elements: { feature: 'src/features/{feature}/**' },
           selectors: { tests: '**/*.test.ts' },
           rules: ({ rule }) => [
             rule({
               name: 'no-tests-in-features',
               files: 'feature', mayNotImport: 'tests',
               reason: 'Production code never imports a test.',
             }),
           ],
         };",
    );
    let mut new = from_loader(&new).expect("the new spellings are valid");
    let mut old = from_loader(&old).expect("the old spellings are valid");
    // Each rule keeps the line of its own builder call, which is not part of what it means.
    for rule in new.rules.iter_mut().chain(old.rules.iter_mut()) {
        rule.source = None;
    }
    assert_eq!(meaning(&new), meaning(&old), "one meaning");
    assert_eq!(
        new.rules[0].reason.as_deref(),
        Some("Production code never imports a test."),
        "each violation carries the reason"
    );
    assert_eq!(
        new.rules[0].name, "no-tests-in-features",
        "the name is first"
    );
}

fn second_spelling(found: Result<Config, Vec<ConfigProblem>>, field: &str) {
    let problems = found.expect_err("two spellings of one key");
    assert_eq!(problems.len(), 1, "one problem: {problems:?}");
    assert_eq!(problems[0].code, ProblemCode::ConfigInvalid, "{problems:?}");
    assert_eq!(problems[0].field.as_deref(), Some(field), "{problems:?}");
    assert!(
        problems[0].message.contains("one spelling for each key"),
        "{}",
        problems[0].message
    );
}

#[test]
fn a_config_cannot_mix_two_spellings_of_one_key() {
    let parts = json!({ "feature": "src/features/{feature}/**" });
    second_spelling(
        from_value(&json!({ "parts": parts, "elements": { "shared": "src/shared/**" } })),
        "elements",
    );
    second_spelling(
        from_value(&json!({
            "parts": parts,
            "groups": { "tests": "**/*.test.ts" },
            "selectors": { "docs": "**/*.md" },
        })),
        "selectors",
    );
    second_spelling(
        from_value(&json!({
            "parts": parts,
            "rules": [{ "files": "src/**", "noCycles": true, "because": "a", "reason": "b" }],
        })),
        "rules[0].reason",
    );
    second_spelling(
        from_loader(&loaded(
            "export default {
               parts: { feature: 'src/features/{feature}/**' },
               rules: ({ rule }) => [
                 rule('a', { name: 'b', files: 'src/**', mayNotImport: 'feature' }),
               ],
             };",
        )),
        "rules[0].name",
    );
}

#[test]
fn a_preset_may_use_the_other_spelling() {
    let config = from_value(&json!({
        "extends": [{
            "name": "old",
            "root": "src",
            "elements": { "shared": "shared/**" },
            "rules": [{ "name": "a", "files": "shared", "noCycles": true, "reason": "old" }],
        }],
        "parts": { "app": "app/**" },
        "rules": [{ "name": "b", "files": "app", "noCycles": true, "because": "new" }],
    }))
    .expect("no problem with spellings");
    let reasons: Vec<_> = config.rules.iter().map(|r| r.reason.as_deref()).collect();
    assert_eq!(
        reasons,
        [Some("old"), Some("new")],
        "each rule keeps its reason"
    );
}
