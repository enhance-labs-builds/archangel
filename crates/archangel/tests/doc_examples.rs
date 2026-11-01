//! Scenarios from the documentation spec: every config example in the docs is a fixture that
//! archangel checks, so an example cannot disagree with what archangel does.
#![cfg(test)]

#[path = "support/cli.rs"]
mod support;

#[path = "support/fixtures.rs"]
mod fixtures;

use std::path::Path;

use archangel_testkit::FixtureRepo;
use fixtures::{examples_folder, fixture};
use support::run;

const COMMANDS: [&str; 3] = ["check", "test", "rules"];

/// Why a command's output differs from what the docs say: the fixture, the command and the first
/// line that differs.
fn compare(name: &str, command: &str, got: &str, expected: &str) -> Result<(), String> {
    if got == expected {
        return Ok(());
    }
    let mut got_lines = got.lines();
    let mut expected_lines = expected.lines();
    let mut number = 0;
    loop {
        number += 1;
        let (one, other) = (got_lines.next(), expected_lines.next());
        if one != other {
            return Err(format!(
                "the fixture {name}: `archangel {command}` differs at line {number}: the docs say \
                 {other:?}, and archangel prints {one:?}"
            ));
        }
        if one.is_none() {
            return Err(format!(
                "the fixture {name}: `archangel {command}` differs in its line endings"
            ));
        }
    }
}

/// Each documented output of a fixture, compared with what archangel prints.
fn check_fixture(name: &str, repo: &FixtureRepo) {
    let folder = examples_folder().join(name).join("expected");
    for command in COMMANDS {
        let expected = std::fs::read_to_string(folder.join(format!("{command}.txt")))
            .unwrap_or_else(|e| panic!("{name}: {command}: {e}"));
        let found = run(repo, &[command]);
        if let Err(difference) = compare(name, command, &found.stdout, &expected) {
            panic!("{difference}\n{}{}", found.stdout, found.stderr);
        }
    }
}

/// The start line of each TypeScript block with a whole config, and its text.
fn config_blocks(text: &str) -> Vec<(usize, String)> {
    let mut blocks = Vec::new();
    let mut open: Option<(usize, String)> = None;
    for (index, line) in text.lines().enumerate() {
        match &mut open {
            None if line.trim_end() == "```ts" => open = Some((index + 1, String::new())),
            None => {}
            Some(_) if line.trim_end() == "```" => {
                let (at, block) = open.take().unwrap_or_default();
                if block.contains("defineConfig(") {
                    blocks.push((at, block));
                }
            }
            Some((_, block)) => {
                block.push_str(line);
                block.push('\n');
            }
        }
    }
    blocks
}

/// The text of each fixture's config.
fn fixture_configs() -> Vec<String> {
    let mut configs = Vec::new();
    for entry in std::fs::read_dir(examples_folder()).unwrap() {
        let config = entry.unwrap().path().join("archangel.config.ts");
        if config.is_file() {
            configs.push(std::fs::read_to_string(config).unwrap());
        }
    }
    configs
}

/// A problem for each config block of a doc whose text is the text of no fixture.
fn blocks_without_a_fixture(doc: &str, text: &str, configs: &[String]) -> Vec<String> {
    config_blocks(text)
        .into_iter()
        .filter(|(_, block)| !configs.contains(block))
        .map(|(line, _)| format!("{doc}:{line}: this config block is the text of no fixture"))
        .collect()
}

#[test]
fn a_fixture_whose_output_differs_fails() {
    let found = run(&fixture("small-app"), &["check"]);
    assert_eq!(found.status, 1, "the config refuses one of its imports");
    let difference = compare("small-app", "check", &found.stdout, "no problem\n")
        .expect_err("the docs say no problem");
    assert!(
        difference.starts_with("the fixture small-app: `archangel check` differs at line 1"),
        "it names the fixture and the first line that differs: {difference}"
    );
}

#[test]
fn a_config_block_with_no_fixture_is_named() {
    let text = "# Rules\n\n```ts\nexport default defineConfig({ parts: {} });\n```\n";
    let problems = blocks_without_a_fixture("docs/rules.md", text, &[]);
    assert_eq!(
        problems,
        ["docs/rules.md:3: this config block is the text of no fixture"],
        "the test names the file and the line of the block"
    );
}

#[test]
#[ignore = "passes when task 8.2 writes the docs from the fixtures"]
fn every_config_example_in_the_docs_is_a_checked_fixture() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let configs = fixture_configs();
    let mut docs = vec![root.join("README.md")];
    for entry in std::fs::read_dir(root.join("docs")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|extension| extension == "md") {
            docs.push(path);
        }
    }
    let mut problems = Vec::new();
    for doc in docs {
        let text = std::fs::read_to_string(&doc).unwrap();
        let shown = doc.strip_prefix(&root).unwrap().display().to_string();
        problems.extend(blocks_without_a_fixture(&shown, &text, &configs));
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn the_small_app_gives_its_documented_output() {
    let repo = fixture("small-app");
    let check = run(&repo, &["check"]);
    assert_eq!(check.status, 1, "{}{}", check.stdout, check.stderr);
    assert_eq!(
        check.stdout.matches("error[").count(),
        1,
        "one violation: {}",
        check.stdout
    );
    assert!(
        check.stdout.contains("rule: feature-code.views"),
        "{}",
        check.stdout
    );
    let test = run(&repo, &["test"]);
    assert_eq!(test.status, 0, "{}{}", test.stdout, test.stderr);
    assert!(!test.stdout.contains("rule-unproven"), "{}", test.stdout);
    check_fixture("small-app", &repo);
}

#[test]
#[ignore = "passes when task 8.2 writes the README from the fixture"]
fn the_readme_shows_the_small_app_and_its_error() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let readme = std::fs::read_to_string(root.join("README.md")).unwrap();
    let small = examples_folder().join("small-app");
    let config = std::fs::read_to_string(small.join("archangel.config.ts")).unwrap();
    let check = std::fs::read_to_string(small.join("expected/check.txt")).unwrap();
    assert!(
        config_blocks(&readme)
            .iter()
            .any(|(_, block)| *block == config),
        "the README holds the config of the small app"
    );
    assert!(
        readme.contains(&format!("```\n{check}```\n")),
        "the README holds the error of the small app"
    );
}

#[test]
fn the_enhance3_slice_gives_its_documented_output() {
    let repo = fixture("enhance3-slice");
    let check = run(&repo, &["check"]);
    assert_eq!(check.status, 1, "{}{}", check.stdout, check.stderr);
    assert_eq!(
        check.stdout.matches("error[").count(),
        2,
        "one violation of each table: {}",
        check.stdout
    );
    for rule in ["rule: workspace.contract", "rule: domain.entity"] {
        assert!(check.stdout.contains(rule), "{rule}: {}", check.stdout);
    }
    let test = run(&repo, &["test"]);
    assert_eq!(test.status, 0, "{}{}", test.stdout, test.stderr);
    check_fixture("enhance3-slice", &repo);
}
