//! Scenarios from the structure-rules spec: the verdict, the message and the words of a
//! `mustBeNamed` rule that lists the names it permits.
#![cfg(test)]

#[path = "support/check.rs"]
mod support;
#[path = "support/violations.rs"]
mod violations;

use archangel_graph::{GraphOptions, ImportGraph, Work};
use archangel_rules::{Architecture, Code, ExampleKind};
use archangel_testkit::FixtureRepo;
use serde_json::{Value, json};
use violations::rule_violations;

fn config_of(rule: &Value) -> Value {
    json!({ "elements": { "src": "src/**" }, "rules": [rule] })
}

/// A repository of one empty file at each path.
fn repo_of(paths: &[&str]) -> FixtureRepo {
    paths
        .iter()
        .fold(FixtureRepo::new(), |repo, path| repo.file(path, ""))
}

/// The violations of a rule over these files, as (code, file, message, help).
fn judged(rule: &Value, paths: &[&str]) -> Vec<(Code, String, String, String)> {
    let report = support::check(&config_of(rule), &repo_of(paths), None).0;
    rule_violations(&report)
        .into_iter()
        .map(|d| {
            (
                d.code,
                d.file.clone().unwrap_or_default(),
                d.message.clone(),
                d.help.clone().unwrap_or_default(),
            )
        })
        .collect()
}

/// The files or folders that a `mustBeNamed` value of a rule on `src/*` refuses.
fn refused(scope: &str, value: &Value, paths: &[&str]) -> Vec<String> {
    let rule = json!({ "name": "named", scope: "src/*", "mustBeNamed": value });
    judged(&rule, paths)
        .into_iter()
        .map(|(_, file, _, _)| file)
        .collect()
}

#[test]
fn a_file_passes_when_one_entry_matches_and_the_message_lists_each_entry() {
    let rule = json!({
        "name": "ui", "files": "src/ui/*",
        "mustBeNamed": ["*.view", "*.container", "*.styles"],
    });
    let paths = [
        "src/ui/cart.view.tsx",
        "src/ui/cart.container.tsx",
        "src/ui/cart.styles.ts",
        "src/ui/cart.tsx",
    ];
    assert_eq!(
        judged(&rule, &paths),
        [(
            Code::MustBeNamed,
            "src/ui/cart.tsx".to_owned(),
            "the full name of cart.tsx is cart, and it must be *.view, *.container or *.styles"
                .to_owned(),
            "rename the file so that its name without the extension is *.view, *.container or \
             *.styles"
                .to_owned(),
        )],
        "the first, the middle and the last entry each permit a file, and the file none permits \
         gets a message that names it, its full name and each entry"
    );
}

#[test]
fn a_star_entry_matches_the_whole_full_name() {
    assert_eq!(
        refused(
            "files",
            &json!(["*.view", "*.styles"]),
            &[
                "src/cart.view.tsx",
                "src/view.tsx",
                "src/cart.view.extra.tsx"
            ]
        ),
        ["src/cart.view.extra.tsx", "src/view.tsx"],
        "the last extension is not in the full name, `view` has no dot for the entry to match, \
         and `.extra` means that the full name does not end in `.view`"
    );
}

#[test]
fn a_list_of_one_entry_is_the_single_value() {
    for single in ["PascalCase", "*.view"] {
        let with = |value: Value| {
            judged(
                &json!({ "name": "r", "files": "src/*", "mustBeNamed": value }),
                &["src/my-card.ts"],
            )
        };
        let (list, value) = (with(json!([single])), with(json!(single)));
        assert_eq!(list.len(), 1, "the file breaks {single}: {list:?}");
        assert_eq!(list, value, "a list of {single} has the single message");
    }
}

#[test]
fn a_style_entry_checks_the_name_up_to_the_first_dot() {
    assert_eq!(
        refused(
            "files",
            &json!(["PascalCase", "*.view"]),
            &[
                "src/Card.stories.tsx",
                "src/cart.view.tsx",
                "src/cart.stories.tsx"
            ]
        ),
        ["src/cart.stories.tsx"],
        "a style reads `Card` and a pattern reads `cart.view`, each as for a single value"
    );
}

#[test]
fn a_style_entry_says_what_it_checks_in_the_message() {
    let rule = json!({ "name": "r", "files": "src/*", "mustBeNamed": ["PascalCase", "*.view"] });
    let found = judged(&rule, &["src/my-card.ts"]);
    assert_eq!(
        found[0].2,
        "the full name of my-card.ts is my-card, and it must be in PascalCase up to the first dot \
         or *.view",
        "a style reads less of the name than a pattern, so the message says so"
    );
}

#[test]
fn a_placeholder_entry_is_written_out_in_the_message() {
    let rule = json!({
        "name": "r", "files": "src/*/*",
        "mustBeNamed": ["{folder-name}.test", "{FileName}.view"],
    });
    let found = judged(&rule, &["src/login/login.test.ts", "src/login/auth.ts"]);
    assert_eq!(
        found.len(),
        1,
        "login.test matches the first entry: {found:?}"
    );
    assert_eq!(
        found[0].2, "the full name of auth.ts is auth, and it must be login.test or Auth.view",
        "the message gives names the author can use"
    );
}

#[test]
fn a_folder_passes_when_one_entry_matches_and_the_message_lists_each_entry() {
    let rule =
        json!({ "name": "kits", "folders": "src/*", "mustBeNamed": ["*-kit", "PascalCase"] });
    let paths = ["src/ui-kit/a.ts", "src/Tools/b.ts", "src/my_tools/c.ts"];
    assert_eq!(
        judged(&rule, &paths),
        [(
            Code::MustBeNamed,
            "src/my_tools".to_owned(),
            "the folder src/my_tools must be named *-kit or in PascalCase".to_owned(),
            "rename the folder so that its name is *-kit or in PascalCase".to_owned(),
        )],
        "a pattern and a style each permit a folder, and the folder neither permits names each"
    );
}

#[test]
fn a_folder_with_one_pattern_entry_is_told_its_whole_new_path() {
    let rule = json!({ "name": "kits", "folders": "src/*", "mustBeNamed": ["*-kit"] });
    assert_eq!(
        judged(&rule, &["src/tools/a.ts"])[0].3,
        "name it src/*-kit",
        "one name can be given as a whole path, as for a single value"
    );
}

#[test]
fn a_folder_entry_reads_the_whole_folder_name() {
    assert_eq!(
        refused(
            "folders",
            &json!(["*.kit", "PascalCase"]),
            &["src/ui.kit/a.ts", "src/Card.stories/b.ts"]
        ),
        ["src/Card.stories"],
        "a folder has no extension, so a dot is part of its name for a style and a pattern"
    );
}

#[test]
fn a_list_reads_as_a_sentence() {
    let sentence = |scope: &str, value: Value| {
        let rule = json!({ "name": "r", scope: "src/*", "mustBeNamed": value });
        let config = archangel_config::parse_json(&config_of(&rule).to_string()).expect("valid");
        Architecture::compile(config).sentence_text(0, 0)
    };
    assert_eq!(
        sentence("files", json!(["*.view", "*.container", "*.styles"])),
        "files in src/* must have full names that match *.view, *.container or *.styles",
        "patterns read the full name"
    );
    assert_eq!(
        sentence("files", json!(["camelCase", "PascalCase"])),
        "files in src/* must have names in camelCase or PascalCase",
        "styles read the name"
    );
    assert_eq!(
        sentence("files", json!(["*.view", "PascalCase"])),
        "files in src/* must have names in PascalCase, or full names that match *.view",
        "a mix says which part of the name each entry reads"
    );
    assert_eq!(
        sentence("folders", json!(["*-kit", "PascalCase"])),
        "folders in src/* must have names in PascalCase, or names that match *-kit",
        "a folder has one name"
    );
}

/// Whether each example of a rule passed, accepted examples first.
fn examples_passed(rule: &Value, repo: &FixtureRepo) -> Vec<bool> {
    let config = archangel_config::parse_json(&config_of(rule).to_string()).expect("valid");
    let graph = ImportGraph::build(repo.path(), &GraphOptions::default(), &mut Work::default());
    let architecture = Architecture::compile(config);
    let examples = &architecture.config().rules[0].examples;
    let accepted = examples.accepted.iter().map(|f| (ExampleKind::Accepted, f));
    let rejected = examples.rejected.iter().map(|f| (ExampleKind::Rejected, f));
    let mut work = Work::default();
    let mut passed = Vec::new();
    for (kind, files) in accepted.chain(rejected) {
        let overlay = graph.overlay(files, &mut work);
        let outcome = architecture.check_example(&graph, &overlay, 0, kind, &mut work);
        passed.push(outcome.passed);
    }
    passed
}

#[test]
fn examples_prove_a_rule_with_a_list() {
    let rule = json!({
        "name": "ui", "files": "src/ui/*", "mustBeNamed": ["*.view", "*.styles"],
        "examples": {
            "accepted": ["src/ui/cart.view.tsx imports src/lib/a.ts", { "src/ui/cart.styles.ts": "" }],
            "rejected": ["src/ui/cart.tsx imports src/lib/a.ts", { "src/ui/cart.tsx": "" }],
        },
    });
    assert_eq!(
        examples_passed(&rule, &repo_of(&["src/lib/a.ts"])),
        [true, true, true, true],
        "a list judges one-line examples and sets of files as it judges a file"
    );
}

#[test]
fn an_example_that_a_list_permits_cannot_be_rejected() {
    let rule = json!({
        "name": "ui", "files": "src/ui/*", "mustBeNamed": ["*.view", "*.styles"],
        "examples": { "rejected": [{ "src/ui/cart.styles.ts": "" }] },
    });
    assert_eq!(
        examples_passed(&rule, &FixtureRepo::new()),
        [false],
        "the last entry permits the file, so the example fails and proves the rule reads each entry"
    );
}
