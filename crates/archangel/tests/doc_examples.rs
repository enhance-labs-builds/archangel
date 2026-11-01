//! Scenarios from the documentation spec: every config example in the docs is a fixture that
//! archangel checks, so an example cannot disagree with what archangel does.
#![cfg(test)]

#[path = "support/cli.rs"]
mod support;

#[path = "support/fixtures.rs"]
mod fixtures;

use std::path::Path;

use archangel_testkit::FixtureRepo;
use fixtures::{examples_folder, fixture, fixture_names, markdown_files, read_text};
use support::run;

const COMMANDS: [&str; 3] = ["check", "test", "rules"];

/// Why a command's output differs from what the docs say: the fixture, the command and the first
/// line that differs.
fn compare(name: &str, command: &str, got: &str, expected: &str) -> Result<(), String> {
    if got == expected {
        return Ok(());
    }
    let got_lines: Vec<&str> = got.lines().collect();
    let expected_lines: Vec<&str> = expected.lines().collect();
    let difference = (0..got_lines.len().max(expected_lines.len()))
        .find(|at| got_lines.get(*at) != expected_lines.get(*at));
    Err(difference.map_or_else(
        || format!("the fixture {name}: `archangel {command}` differs in its line endings"),
        |at| {
            format!(
                "the fixture {name}: `archangel {command}` differs at line {}: the docs say {:?}, \
                 and archangel prints {:?}",
                at + 1,
                expected_lines.get(at),
                got_lines.get(at)
            )
        },
    ))
}

/// Each documented output of a fixture, compared with what archangel prints.
fn check_fixture(name: &str, repo: &FixtureRepo) {
    let folder = examples_folder().join(name).join("expected");
    for command in COMMANDS {
        let expected = read_text(&folder.join(format!("{command}.txt")));
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

/// Each rule fragment of a doc, with the line it starts on: a TypeScript block that starts with a
/// builder, and a piece of inline code that does.
fn fragments(text: &str) -> Vec<(usize, String)> {
    const BUILDERS: [&str; 5] = ["rule(", "mayImport(", "layers(", "isolate(", "entry("];
    // A placeholder, a rule's default name and a plugin's rule cannot load alone.
    let is_fragment = |code: &str| {
        BUILDERS.iter().any(|builder| code.starts_with(builder))
            && code.contains('\'')
            && !code.contains("...")
            && !code.contains("plugin:")
    };
    let mut found = Vec::new();
    let mut block: Option<(usize, String)> = None;
    let mut fenced = false;
    for (index, line) in text.lines().enumerate() {
        if line.starts_with("```") {
            match block.take() {
                Some((at, code)) if is_fragment(&code) => found.push((at, code)),
                Some(_) => {}
                None if line.trim_end() == "```ts" => block = Some((index + 1, String::new())),
                None => {}
            }
            fenced = !fenced;
            continue;
        }
        if let Some((_, code)) = &mut block {
            code.push_str(line);
            code.push('\n');
        } else if !fenced {
            found.extend(
                line.split('`')
                    .skip(1)
                    .step_by(2)
                    .filter(|code| is_fragment(code))
                    .map(|code| (index + 1, code.to_owned())),
            );
        }
    }
    found
}

/// A repository whose config holds one rule fragment, with each name that the docs use declared.
fn fragment_repo(fragment: &str) -> FixtureRepo {
    let parts: String = [
        "app",
        "shared",
        "view",
        "db",
        "api",
        "entity",
        "value-object",
        "error",
        "use-case",
    ]
    .iter()
    .map(|name| format!("'{name}': 'src/{name}/**',"))
    .collect();
    let parts = format!("{parts} feature: 'src/features/{{feature}}/**',");
    FixtureRepo::new().file("src/shared/a.ts", "").file(
        "archangel.config.mjs",
        &format!(
            "export default {{
               parts: {{ {parts} }},
               groups: {{ test: '**/*.test.ts', client: {{ directive: 'use client' }} }},
               externals: {{ driver: ['pg'] }},
               rules: ({{ rule, mayImport, layers, isolate, entry }}) => [{fragment}],
             }};\n"
        ),
    )
}

/// A problem for each rule fragment of a doc that does not load.
fn fragments_that_do_not_load(doc: &str, text: &str) -> Vec<String> {
    fragments(text)
        .into_iter()
        .filter_map(|(line, fragment)| {
            let found = run(&fragment_repo(&fragment), &["check"]);
            (found.status == 2).then(|| {
                format!(
                    "{doc}:{line}: this fragment does not load: {fragment}\n{}{}",
                    found.stdout, found.stderr
                )
            })
        })
        .collect()
}

/// The text of each fixture's config.
fn fixture_configs() -> Vec<String> {
    fixture_names()
        .iter()
        .map(|name| read_text(&examples_folder().join(name).join("archangel.config.ts")))
        .collect()
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
fn every_config_example_in_the_docs_is_a_checked_fixture() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let configs = fixture_configs();
    let mut problems = Vec::new();
    for doc in std::iter::once(root.join("README.md")).chain(markdown_files(&root.join("docs"))) {
        let text = read_text(&doc);
        let shown = doc.strip_prefix(&root).unwrap().display().to_string();
        problems.extend(blocks_without_a_fixture(&shown, &text, &configs));
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn a_fragment_that_does_not_load_is_named() {
    let text = "Write `rule('a', { files: 'shared', mayNotImport: 'react' })` in a config.\n";
    let problems = fragments_that_do_not_load("docs/rules.md", text);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].starts_with("docs/rules.md:1: this fragment does not load"),
        "{}",
        problems[0]
    );
}

#[test]
fn every_rule_fragment_in_the_docs_loads() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut problems = Vec::new();
    for doc in std::iter::once(root.join("README.md")).chain(markdown_files(&root.join("docs"))) {
        let text = read_text(&doc);
        let shown = doc.strip_prefix(&root).unwrap().display().to_string();
        problems.extend(fragments_that_do_not_load(&shown, &text));
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
fn the_readme_shows_the_small_app_and_its_error() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let readme = read_text(&root.join("README.md"));
    let small = examples_folder().join("small-app");
    let config = read_text(&small.join("archangel.config.ts"));
    let check = read_text(&small.join("expected/check.txt"));
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

#[test]
fn every_fixture_gives_its_documented_output() {
    for name in fixture_names() {
        check_fixture(&name, &fixture(&name));
    }
}
