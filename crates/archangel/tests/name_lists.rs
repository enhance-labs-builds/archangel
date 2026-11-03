//! A scenario from the structure-rules spec, end to end: a `mustBeNamed` rule with a list, as
//! `archangel check` and `archangel rules` give it.
#![cfg(test)]

#[path = "support/cli.rs"]
mod support;

use archangel_testkit::FixtureRepo;
use serde_json::json;
use support::run;

#[test]
fn check_and_rules_give_a_list_of_names() {
    let repo = FixtureRepo::new()
        .file("src/ui/cart.view.tsx", "")
        .file("src/ui/cart.tsx", "")
        .json_config(&json!({
            "rules": [{
                "name": "ui-files", "files": "src/ui/*", "mustBeNamed": ["*.view", "*.styles"],
                "because": "a file in ui says what it is",
            }],
        }));
    let found = run(&repo, &["check"]);
    assert_eq!(
        found.status, 1,
        "cart.tsx breaks the rule: {}{}",
        found.stdout, found.stderr
    );
    assert!(
        found
            .stdout
            .contains("the full name of cart.tsx is cart, and it must be *.view or *.styles"),
        "the output teaches the names the rule permits: {}",
        found.stdout
    );
    assert!(
        !found.stdout.contains("cart.view.tsx"),
        "the entry that matches permits the other file: {}",
        found.stdout
    );
    let rules = run(&repo, &["rules"]);
    assert!(
        rules
            .stdout
            .contains("files in src/ui/* must have full names that match *.view or *.styles"),
        "`archangel rules` reads the list as a sentence: {}",
        rules.stdout
    );
}
