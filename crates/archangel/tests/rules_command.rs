//! Scenarios from the command line spec: `archangel rules` prints every rule as a sentence with its
//! reach, and what no rule judges.
#![cfg(test)]

#[path = "support/cli.rs"]
mod support;

#[path = "support/fixtures.rs"]
mod fixtures;

use archangel_testkit::FixtureRepo;
use fixtures::fixture;
use serde_json::json;
use support::run;

/// The block of a rule in the output: its lines from the one that starts with its name.
fn block<'o>(output: &'o str, name: &str) -> &'o str {
    let start = output
        .find(&format!("  {name}: "))
        .or_else(|| output.find(&format!("\n{name}  ")))
        .unwrap_or_else(|| panic!("no block for {name} in:\n{output}"));
    let rest = &output[start + 1..];
    let end = rest
        .match_indices('\n')
        .map(|(at, _)| at)
        .nth(1)
        .unwrap_or(rest.len());
    &rest[..end]
}

#[test]
fn rules_prints_each_rule_as_a_sentence_with_its_reach() {
    let found = run(&fixture("small-app"), &["rules"]);
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    let views = block(&found.stdout, "feature-code.views");
    assert!(
        views.contains("views may import only models, shared, and itself"),
        "the sentence: {views}"
    );
    assert!(
        views.contains("2 files · 3 imports judged · 1 problem · 0 exceptions"),
        "the reach: {views}"
    );
    assert!(
        found.stdout.contains("A view shows a model."),
        "the because text: {}",
        found.stdout
    );
    for table in ["app", "feature-code"] {
        assert!(
            found
                .stdout
                .lines()
                .any(|line| line.starts_with(table) && line.contains("archangel.config.ts:")),
            "the file and line of {table}: {}",
            found.stdout
        );
    }
    for row in [
        "app.feature",
        "app.shared",
        "feature-code.models",
        "feature-code.api",
    ] {
        assert!(
            found.stdout.contains(&format!("  {row}: ")),
            "{row}: {}",
            found.stdout
        );
    }
}

#[test]
fn rules_prints_every_rule_with_no_elision() {
    let rules: Vec<_> = (0..300)
        .map(|index| json!({ "name": format!("rule-{index}"), "files": "src/**", "mustBeNamed": "kebab-case" }))
        .collect();
    let repo = FixtureRepo::new()
        .file("src/a.ts", "")
        .json_config(&json!({ "rules": rules }));
    let found = run(&repo, &["rules"]);
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    for index in 0..300 {
        assert!(
            found.stdout.contains(&format!("\nrule-{index}  "))
                || found.stdout.starts_with(&format!("rule-{index}  ")),
            "an entry for rule-{index}"
        );
    }
}

#[test]
fn rules_shows_a_row_with_no_file_as_checking_nothing() {
    let found = run(&fixture("enhance3-slice"), &["rules"]);
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    let row = block(&found.stdout, "domain.aggregate-root");
    assert!(
        row.contains("checks nothing: no file is in aggregate-root"),
        "{row}"
    );
}

#[test]
fn rules_names_the_config_file_of_a_plain_rule() {
    let repo = FixtureRepo::new().file("src/a.ts", "").json_config(&json!({
        "rules": [{ "name": "kebab", "files": "src/**", "mustBeNamed": "kebab-case" }],
    }));
    let found = run(&repo, &["rules"]);
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    let first = found.stdout.lines().next().unwrap();
    assert!(
        first.starts_with("kebab") && first.ends_with("archangel.config.json"),
        "the file and no line: {first}"
    );
}

#[test]
fn rules_names_the_helper_module_that_built_a_rule() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "")
        .file(
            "rules/domain.ts",
            "export const domain = (rule: any) => [\n\
             \x20 // the rules of the domain\n\
             \n\
             \x20 rule('entities-stay-pure', { files: 'src/**', mustBeNamed: 'kebab-case' }),\n\
             ];\n",
        )
        .file(
            "archangel.config.ts",
            "import { domain } from './rules/domain.ts';\n\
             export default { rules: ({ rule }: any) => domain(rule) };\n",
        );
    let found = run(&repo, &["rules"]);
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    let first = found.stdout.lines().next().unwrap();
    assert!(
        first.starts_with("entities-stay-pure") && first.ends_with("rules/domain.ts:4"),
        "the helper module and its line: {first}"
    );
}

#[test]
fn rules_names_the_preset_rule_that_a_config_rule_replaces() {
    let repo = FixtureRepo::new().file("src/a.ts", "").file(
        "archangel.config.mjs",
        "export default {
           extends: [{
             name: 'team',
             rules: [{ name: 'views-thin', files: 'src/**', mustBeNamed: 'kebab-case' }],
           }, {
             name: 'other',
             rules: [{ name: 'kept', files: 'src/**', mustBeNamed: 'kebab-case' }],
           }],
           rules: [{ name: 'team/views-thin', files: 'src/**', mustBeNamed: 'camelCase' }],
         };",
    );
    let found = run(&repo, &["rules"]);
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    assert!(
        found
            .stdout
            .contains("replaces the rule views-thin of the preset team"),
        "{}",
        found.stdout
    );
    assert!(
        found.stdout.contains("from the preset other"),
        "a preset rule names its preset: {}",
        found.stdout
    );
}

#[test]
fn rules_lists_the_files_and_imports_that_no_rule_judges() {
    let repo = FixtureRepo::new()
        .file("src/features/a.ts", "")
        .file("src/main.ts", "import './shared/a';\nimport 'react';\n")
        .file("src/shared/a.ts", "")
        .file("node_modules/react/package.json", r#"{ "name": "react" }"#)
        .file("node_modules/react/index.js", "")
        .file(".gitignore", "node_modules/\n")
        .json_config(&json!({
            "rules": [{ "name": "kebab", "files": "src/features/**", "mustBeNamed": "kebab-case" }],
        }));
    let found = run(&repo, &["rules"]);
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    for line in [
        "Files that no rule judges: 2\n  src/main.ts\n  src/shared/a.ts\n",
        "Imports between repository files that no rule judges: 1\n  src/main.ts:1 imports src/shared/a.ts\n",
        "Outside packages that no rule names: react (1 import).\n",
    ] {
        assert!(found.stdout.contains(line), "{line} in:\n{}", found.stdout);
    }
}

#[test]
fn rules_with_a_path_prints_the_rules_that_apply_there() {
    let found = run(&fixture("small-app"), &["rules", "src/shared"]);
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    assert!(found.stdout.contains("  app.shared: "), "{}", found.stdout);
    assert!(!found.stdout.contains("feature-code"), "{}", found.stdout);
}

#[test]
fn rules_with_a_path_that_holds_no_file_is_refused() {
    let found = run(&fixture("small-app"), &["rules", "src/nothing"]);
    assert_eq!(found.status, 2, "{}{}", found.stdout, found.stderr);
    assert!(
        found.stderr.contains("no source file is under src/nothing"),
        "{}",
        found.stderr
    );
}

#[test]
fn rules_counts_exceptions_and_baseline_matches_in_the_reach() {
    let repo = fixture("small-app");
    repo.write(
        "src/features/cart/views/other-page.tsx",
        "// archangel-allow feature-code.views: migration\n\
         import { saveCart } from '../api/save-cart';\n\
         export const other = saveCart;\n",
    );
    let added = run(&repo, &["baseline", "add"]);
    assert_eq!(added.status, 0, "{}{}", added.stdout, added.stderr);
    let found = run(&repo, &["rules"]);
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    let views = block(&found.stdout, "feature-code.views");
    assert!(
        views.contains("0 problems · 1 exception · 1 baseline match"),
        "{views}"
    );
}

#[test]
fn rules_exits_zero_whatever_the_check_finds() {
    let repo = fixture("small-app");
    assert_eq!(
        run(&repo, &["check"]).status,
        1,
        "the check finds a problem"
    );
    assert_eq!(run(&repo, &["rules"]).status, 0, "rules describes");
}

#[test]
fn rules_shows_a_rule_that_is_off() {
    let repo = FixtureRepo::new().file("src/a.ts", "").json_config(&json!({
        "rules": [
            { "name": "kept", "files": "src/**", "mustBeNamed": "kebab-case" },
            { "name": "gone", "files": "src/**", "mustBeNamed": "camelCase" },
        ],
        "off": ["gone"],
    }));
    let found = run(&repo, &["rules"]);
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    assert!(
        found
            .stdout
            .contains("Rules that `off` removes: 1\n  gone: "),
        "{}",
        found.stdout
    );
}

#[test]
fn rules_counts_folders_for_a_rule_about_folders() {
    let repo = FixtureRepo::new()
        .file("src/one/a.ts", "")
        .json_config(&json!({
            "rules": [{ "name": "kebab", "folders": "src/*", "mustBeNamed": "kebab-case" }],
        }));
    let found = run(&repo, &["rules"]);
    assert!(
        found.stdout.contains("1 folder · 0 problems"),
        "{}",
        found.stdout
    );
}

const ALIAS: &str = r#"{ "compilerOptions": { "baseUrl": ".", "paths": { "@/*": ["src/*"] } } }"#;

#[test]
fn reach_does_not_count_an_import_that_a_limit_skips() {
    let repo = FixtureRepo::new()
        .file("tsconfig.json", ALIAS)
        .file("src/a/x.ts", "import '@/b/y';\n")
        .file("src/b/y.ts", "")
        .json_config(&json!({
            "rules": [{
                "kind": "mayImport", "name": "t",
                "rows": { "a": [], "b": [] },
                "kinds": "src/{kind}/**",
                "specifier": "parent",
            }],
        }));
    let found = run(&repo, &["rules"]);
    assert!(
        block(&found.stdout, "t.a").contains("0 imports judged"),
        "{}",
        found.stdout
    );
    assert!(
        found.stdout.contains("src/a/x.ts:1 imports src/b/y.ts"),
        "the import is one that no rule judged: {}",
        found.stdout
    );
}

#[test]
fn reach_counts_an_import_that_no_cycles_follows() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "import './b';\n")
        .file("src/b.ts", "")
        .json_config(&json!({
            "elements": { "src": "src/**" },
            "rules": [{ "name": "c", "files": "src/**", "noCycles": true }],
        }));
    let found = run(&repo, &["rules"]);
    assert!(
        !found.stdout.contains("src/a.ts:1 imports src/b.ts"),
        "{}",
        found.stdout
    );
}

#[test]
fn a_table_stays_in_one_block_when_a_config_rule_replaces_a_row() {
    let repo = FixtureRepo::new()
        .file("src/a/x.ts", "")
        .file("src/b/y.ts", "")
        .file("src/c/z.ts", "")
        .file(
            "archangel.config.mjs",
            "export default {
               extends: [{
                 name: 'team', package: 'archangel-plugin-team', root: '',
                 rules: [{
                   kind: 'mayImport', name: 'roles',
                   rows: { a: [], b: [], c: [] },
                   kinds: 'src/{kind}/**',
                 }],
               }],
               rules: [{ name: 'team/roles.b', files: 'src/b/**', mayNotImport: 'src/a/**' }],
             };",
        );
    let found = run(&repo, &["rules"]);
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    assert_eq!(
        found.stdout.matches("team/roles  ").count(),
        1,
        "{}",
        found.stdout
    );
}

#[test]
fn a_table_counts_only_source_files() {
    let repo = FixtureRepo::new()
        .file("src/domain/a.ts", "")
        .file("src/domain/x.ts", "")
        .file("src/docs/r.md", "")
        .file("src/assets/s.css", "")
        .json_config(&json!({
            "rules": [{
                "kind": "mayImport", "name": "t",
                "rows": { "domain": [], "assets": [] },
                "kinds": "src/{kind}/**",
                "files": "src/**",
            }],
        }));
    let found = run(&repo, &["rules"]);
    assert!(
        block(&found.stdout, "t.every-file-has-a-kind").contains("2 files · 0 problems"),
        "{}",
        found.stdout
    );
    assert!(
        block(&found.stdout, "t.assets").contains("checks nothing"),
        "{}",
        found.stdout
    );
}

#[test]
fn a_table_without_files_says_what_it_covers_and_lists_the_rest() {
    let repo = FixtureRepo::new()
        .file("src/routes/a.ts", "")
        .file("src/index.ts", "")
        .json_config(&json!({
            "rules": [{
                "kind": "mayImport", "name": "t",
                "rows": { "routes": [] },
                "kinds": "src/{kind}/**",
            }],
        }));
    let found = run(&repo, &["rules"]);
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    let line = block(&found.stdout, "t.every-file-has-a-kind");
    assert!(
        line.contains("a file with a kind has a row; a file with no kind is not checked here"),
        "{}",
        found.stdout
    );
    assert!(line.contains("1 file · 0 problems"), "{}", found.stdout);
    assert!(
        found.stdout.contains("src/index.ts"),
        "the file with no kind is in the list of files that no rule judges: {}",
        found.stdout
    );
}
