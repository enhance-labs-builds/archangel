//! Scenarios from the structure-rules spec: names, places, files that every instance or folder must
//! have, and files that must not exist, over every file the walk visits.
#![cfg(test)]

#[path = "support/kept.rs"]
mod kept;
#[path = "support/check.rs"]
mod support;
#[path = "support/violations.rs"]
mod violations;

use archangel_rules::{Code, FileName, Report};
use archangel_testkit::FixtureRepo;
use kept::{DiskChange, KeptCheck};
use serde_json::{Value, json};
use violations::rule_violations;

fn report(config: &Value, repo: &FixtureRepo) -> Report {
    support::check(config, repo, None).0
}

/// The violations, as (code, file, message), in report order.
fn found(report: &Report) -> Vec<(Code, String, String)> {
    rule_violations(report)
        .into_iter()
        .map(|d| {
            (
                d.code,
                d.file.clone().unwrap_or_default(),
                d.message.clone(),
            )
        })
        .collect()
}

/// A config of elements and one rule.
fn one_rule(elements: &Value, rule: &Value) -> Value {
    json!({ "elements": elements, "rules": [rule] })
}

fn kebab_folders() -> Value {
    one_rule(
        &json!({ "src": "src/**" }),
        &json!({ "name": "kebab-folders", "folders": "src/**", "mustBeNamed": "kebab-case" }),
    )
}

#[test]
fn structure_rules_see_folders_without_source_files() {
    let repo = FixtureRepo::new()
        .file("src/app.ts", "")
        .bytes("src/userAssets/logo.png", b"\x89PNG")
        .bytes("src/userAssets/icon.png", b"\x89PNG");
    let found = found(&report(&kebab_folders(), &repo));
    assert_eq!(
        found.len(),
        1,
        "one violation for the folder of images: {found:?}"
    );
    assert_eq!(
        (found[0].0, found[0].1.as_str()),
        (Code::MustBeNamed, "src/userAssets"),
        "the folder that holds only images is checked"
    );
}

#[test]
fn a_file_has_a_name_and_a_full_name() {
    let names = ["src/button.view.tsx", "src/auth.test.ts"].map(|path| {
        let name = FileName::of(path);
        (name.name.to_owned(), name.full.to_owned())
    });
    assert_eq!(
        names,
        [
            ("button".to_owned(), "button.view".to_owned()),
            ("auth".to_owned(), "auth.test".to_owned()),
        ],
        "the name ends at the first dot, the full name before the last extension"
    );
}

#[test]
fn a_folder_name_is_checked_once_against_its_style() {
    let repo = FixtureRepo::new()
        .file("src/userProfile/a.ts", "")
        .file("src/userProfile/b.ts", "");
    let (report, work) = support::check(&kebab_folders(), &repo, None);
    let found = found(&report);
    assert_eq!(
        found.len(),
        1,
        "one violation however many files the folder holds, and none for src: {found:?}"
    );
    assert_eq!(
        found[0].1, "src/userProfile",
        "the violation is the folder's"
    );
    assert!(
        found[0].2.contains("user-profile"),
        "the message suggests the kebab-case name: {}",
        found[0].2
    );
    assert_eq!(
        work.folder_evaluations, 1,
        "src/userProfile is looked at once, and src not at all"
    );
}

#[test]
fn a_style_checks_the_name_up_to_the_first_dot() {
    let repo = FixtureRepo::new()
        .file("src/components/Button.tsx", "")
        .file("src/components/Card.stories.tsx", "")
        .file("src/components/icon-button.tsx", "");
    let config = json!({
        "elements": { "components": "src/components/**" },
        "rules": [{ "name": "pascal", "files": "src/components/**/*.tsx", "mustBeNamed": "PascalCase" }],
    });
    let found = found(&report(&config, &repo));
    assert_eq!(
        found
            .iter()
            .map(|(code, file, _)| (*code, file.as_str()))
            .collect::<Vec<_>>(),
        [(Code::MustBeNamed, "src/components/icon-button.tsx")],
        "Card.stories.tsx is named Card, which is PascalCase"
    );
    assert!(
        found[0].2.contains("IconButton.tsx"),
        "the message suggests IconButton.tsx: {}",
        found[0].2
    );
}

#[test]
fn a_name_pattern_uses_the_folders_name() {
    let repo = FixtureRepo::new()
        .file("src/features/login/login.test.ts", "")
        .file("src/features/login/auth.test.ts", "");
    let config = json!({
        "elements": { "feature": "src/features/{feature}/**" },
        "rules": [{
            "name": "tests-named-for-their-folder",
            "files": "src/features/*/*.test.ts",
            "mustBeNamed": "{folder-name}.test",
        }],
    });
    let found = found(&report(&config, &repo));
    assert_eq!(
        found
            .iter()
            .map(|(_, file, _)| file.as_str())
            .collect::<Vec<_>>(),
        ["src/features/login/auth.test.ts"],
        "login.test.ts is named for its folder"
    );
    assert!(
        found[0].2.contains("login.test"),
        "the message says the full name must be login.test: {}",
        found[0].2
    );
}

#[test]
fn must_contain_rejects_an_instance_without_the_file() {
    let repo = FixtureRepo::new()
        .file("src/features/login/index.ts", "")
        .file("src/features/login/a.ts", "")
        .file("src/features/billing/a.ts", "");
    let found = found(&report(
        &one_rule(
            &json!({ "slice": "src/features/{slice}/**" }),
            &json!({ "name": "public-api", "files": "slice", "mustContain": "index.ts" }),
        ),
        &repo,
    ));
    assert_eq!(
        found
            .iter()
            .map(|(code, file, _)| (*code, file.as_str()))
            .collect::<Vec<_>>(),
        [(Code::MustContain, "src/features/billing")],
        "billing has no index.ts"
    );
    assert!(
        found[0].2.contains("\"billing\"") && found[0].2.contains("src/features/billing/index.ts"),
        "the message names the instance and the file it needs: {}",
        found[0].2
    );
}

#[test]
fn must_contain_rejects_a_folder_without_the_file() {
    let repo = FixtureRepo::new()
        .file("packages/a/README.md", "# a\n")
        .file("packages/a/index.ts", "")
        .file("packages/b/index.ts", "");
    let config = json!({
        "elements": { "packages": "packages/**" },
        "rules": [{ "name": "readme", "folders": "packages/*", "mustContain": "README.md" }],
    });
    let found = found(&report(&config, &repo));
    assert_eq!(
        found
            .iter()
            .map(|(code, file, _)| (*code, file.as_str()))
            .collect::<Vec<_>>(),
        [(Code::MustContain, "packages/b")],
        "packages/a has its README, which is not a source file"
    );
}

#[test]
fn must_not_exist_reports_a_file_in_no_element() {
    let repo = FixtureRepo::new()
        .file("src/app/main.ts", "")
        .file("src/shared/a.ts", "")
        .file("src/stray.ts", "");
    let config = json!({
        "elements": { "app": "src/app/**", "shared": "src/shared/**" },
        "rules": [{
            "name": "every-file-has-a-place",
            "files": { "all": ["src/**", { "not": { "element": "*" } }] },
            "mustNotExist": true,
        }],
    });
    let found = found(&report(&config, &repo));
    assert_eq!(
        found
            .iter()
            .map(|(code, file, _)| (*code, file.as_str()))
            .collect::<Vec<_>>(),
        [(Code::MustNotExist, "src/stray.ts")],
        "only the file in no element"
    );
    assert!(
        found[0].2.contains("\"app\"") && found[0].2.contains("\"shared\""),
        "the message lists every declared element: {}",
        found[0].2
    );
}

#[test]
fn must_not_exist_reports_a_forbidden_folder() {
    let repo = FixtureRepo::new()
        .file("src/features/login/utils/a.ts", "")
        .file("src/features/login/model/b.ts", "");
    let config = json!({
        "elements": { "feature": "src/features/{feature}/**" },
        "rules": [{
            "name": "no-utils",
            "files": "src/*/*/{utils,helpers}/**",
            "mustNotExist": true,
            "reason": "Group segments by purpose",
        }],
    });
    let report = report(&config, &repo);
    let violations = rule_violations(&report);
    assert_eq!(
        violations
            .iter()
            .map(|d| d.file.as_deref().unwrap_or_default())
            .collect::<Vec<_>>(),
        ["src/features/login/utils/a.ts"],
        "only the file in the reserved folder"
    );
    assert_eq!(
        violations[0].reason.as_deref(),
        Some("Group segments by purpose"),
        "the violation shows the reason"
    );
}

#[test]
fn a_folders_pattern_that_matches_no_folder_is_selector_empty() {
    let repo = FixtureRepo::new().file("src/a.ts", "");
    let config = json!({
        "elements": { "src": "src/**" },
        "rules": [{ "name": "readme", "folders": "modules/*", "mustContain": "README.md" }],
    });
    let report = report(&config, &repo);
    let empty: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.code == Code::SelectorEmpty)
        .collect();
    assert_eq!(
        empty.len(),
        1,
        "one selector-empty: {:?}",
        report.diagnostics
    );
    assert!(
        empty[0].message.contains("\"readme\"") && empty[0].message.contains("modules/*"),
        "it names the rule and the pattern: {}",
        empty[0].message
    );
}

#[test]
fn a_folder_that_must_not_exist_is_one_violation() {
    let repo = FixtureRepo::new()
        .file("src/app/main.ts", "")
        .file("src/processes/checkout/a.ts", "")
        .file("src/processes/checkout/b.ts", "");
    let config = one_rule(
        &json!({ "src": "src/**" }),
        &json!({ "name": "no-processes", "folders": "src/processes", "mustNotExist": true }),
    );
    let found = found(&report(&config, &repo));
    let at: Vec<(Code, &str)> = found.iter().map(|(c, f, _)| (*c, f.as_str())).collect();
    assert_eq!(
        at,
        [(Code::MustNotExist, "src/processes")],
        "one violation for the folder, not one for each file"
    );
}

#[test]
fn must_contain_accepts_any_of_several_patterns() {
    let repo = FixtureRepo::new()
        .file("src/features/a/model.ts", "")
        .file("src/features/b/ui/button.ts", "")
        .file("src/features/c/helpers.ts", "");
    let config = one_rule(
        &json!({ "src": "src/**" }),
        &json!({ "name": "has-a-segment", "folders": "src/features/*",
                 "mustContain": ["{ui,model}.*", "{ui,model}/**"] }),
    );
    let found = found(&report(&config, &repo));
    let at: Vec<&str> = found.iter().map(|(_, f, _)| f.as_str()).collect();
    assert_eq!(
        at,
        ["src/features/c"],
        "a file segment and a folder segment each satisfy the rule: {found:?}"
    );
}

#[test]
fn a_folder_pattern_with_an_exclamation_mark_leaves_folders_out() {
    let repo = FixtureRepo::new()
        .file("src/shared/api/client.ts", "")
        .file("src/shared/ui/button.ts", "")
        .file("src/shared/lib/format.ts", "")
        .file("src/shared/config/index.ts", "");
    let config = one_rule(
        &json!({ "src": "src/**" }),
        &json!({ "name": "public-api", "folders": ["src/shared/*", "!src/shared/{ui,lib}"],
                 "mustContain": "index.*" }),
    );
    let found = found(&report(&config, &repo));
    let at: Vec<&str> = found.iter().map(|(_, f, _)| f.as_str()).collect();
    assert_eq!(at, ["src/shared/api"], "ui and lib are left out: {found:?}");
}

#[test]
fn a_rule_of_several_sentences_is_broken_by_each_of_them() {
    let repo = FixtureRepo::new()
        .file("src/app/ui/root.ts", "")
        .file("src/app/ui.tsx", "");
    let config = json!({
        "elements": { "app": "src/app/**" },
        "rules": [{ "name": "no-ui-in-app", "sentences": [
            { "folders": "src/app/ui", "mustNotExist": true },
            { "files": "src/app/ui.*", "mustNotExist": true },
        ] }],
    });
    let found = found(&report(&config, &repo));
    let at: Vec<(Code, &str)> = found.iter().map(|(c, f, _)| (*c, f.as_str())).collect();
    assert_eq!(
        at,
        [
            (Code::MustNotExist, "src/app/ui"),
            (Code::MustNotExist, "src/app/ui.tsx"),
        ],
        "each sentence of the one rule reports under its name: {found:?}"
    );
}

#[test]
fn a_folder_named_against_its_style_is_told_its_whole_new_path() {
    let repo = FixtureRepo::new().file("src/userAssets/logo.ts", "");
    let found = found(&report(&kebab_folders(), &repo));
    assert_eq!(
        found,
        [(
            Code::MustBeNamed,
            "src/userAssets".to_owned(),
            "the folder src/userAssets is not kebab-case; in kebab-case it is src/user-assets"
                .to_owned()
        )],
        "the new name keeps the folder's parent"
    );
}

#[test]
fn a_folder_name_pattern_refuses_only_the_folders_it_does_not_match() {
    let repo = FixtureRepo::new()
        .file("src/ui-kit/a.ts", "")
        .file("src/tools/b.ts", "");
    let config = one_rule(
        &json!({ "src": "src/**" }),
        &json!({ "name": "kits", "folders": "src/*", "mustBeNamed": "*-kit" }),
    );
    let found = found(&report(&config, &repo));
    assert_eq!(
        found,
        [(
            Code::MustBeNamed,
            "src/tools".to_owned(),
            "the folder src/tools must be named *-kit".to_owned()
        )],
        "ui-kit matches the pattern and tools does not"
    );
}

/// The verdict of a full-name pattern on each name, as whether a file of that full name passes.
fn named(pattern: &str, names: &[&str]) -> Vec<bool> {
    let mut repo = FixtureRepo::new();
    for name in names {
        repo = repo.file(&format!("src/{name}.ts"), "");
    }
    let config = one_rule(
        &json!({ "src": "src/**" }),
        &json!({ "name": "named", "files": "src/*.ts", "mustBeNamed": pattern }),
    );
    let refused: Vec<String> = found(&report(&config, &repo))
        .into_iter()
        .map(|(_, file, _)| file)
        .collect();
    names
        .iter()
        .map(|name| !refused.contains(&format!("src/{name}.ts")))
        .collect()
}

#[test]
fn a_name_pattern_with_stars_needs_its_texts_in_order_and_apart() {
    assert_eq!(
        named("use*", &["useAuth", "fetchUse"]),
        [true, false],
        "the text before the first star starts the name"
    );
    assert_eq!(
        named("*Store", &["userStore", "StoreFront"]),
        [true, false],
        "the text after the last star ends the name"
    );
    assert_eq!(
        named("xy*yx", &["xyyx", "xyx"]),
        [true, false],
        "the first and last texts may meet, but may not overlap"
    );
    assert_eq!(
        named("a*ab", &["aab", "ab"]),
        [true, false],
        "a name shorter than the first and last texts together is refused"
    );
    assert_eq!(
        named("use*Store", &["useUserStore", "useStoreX"]),
        [true, false],
        "the last text is not looked for in the middle"
    );
    assert_eq!(
        named("x*ab*ab*y", &["xabQaby", "xabQy"]),
        [true, false],
        "each middle text is looked for after the one before it"
    );
}

#[test]
fn must_not_exist_names_the_elements_only_for_a_file_in_none() {
    let repo = FixtureRepo::new()
        .file("src/old/a.ts", "")
        .file("lib/b.ts", "");
    let config = json!({
        "elements": { "src": "src/**" },
        "rules": [{ "name": "no-old", "files": ["src/old/**", "lib/**"], "mustNotExist": true }],
    });
    let messages: Vec<String> = found(&report(&config, &repo))
        .into_iter()
        .map(|(_, _, message)| message)
        .collect();
    assert_eq!(
        messages,
        [
            "lib/b.ts must not exist, because it is in src/old/** | lib/**, and it is in no element; \
             the elements are \"src\""
                .to_owned(),
            "src/old/a.ts must not exist, because it is in src/old/** | lib/**".to_owned(),
        ],
        "only the file in no element is told the elements"
    );
}

#[test]
fn a_folder_pattern_whose_folders_each_hold_one_file_is_not_empty() {
    let repo = FixtureRepo::new()
        .file("src/features/login/a.ts", "")
        .file("src/features/billing/b.ts", "");
    let config = one_rule(
        &json!({ "src": "src/**" }),
        &json!({ "name": "kebab", "folders": "src/features/*", "mustBeNamed": "kebab-case" }),
    );
    let report = report(&config, &repo);
    assert_eq!(
        report.diagnostics,
        [],
        "each folder of the pattern counts, whatever it holds"
    );
}

#[test]
fn structure_rules_count_one_evaluation_per_file_folder_and_instance_they_cover() {
    let repo = FixtureRepo::new()
        .file("src/features/login/index.ts", "export const login = 1;\n")
        .file(
            "src/features/billing/index.ts",
            "export const billing = 1;\n",
        )
        .file("src/features/billing/a.ts", "");
    let config = json!({
        "elements": { "feature": "src/features/{feature}/**" },
        "rules": [
            { "name": "kebab", "files": "src/**", "mustBeNamed": "kebab-case" },
            { "name": "index", "files": "feature", "mustContain": "index.ts" },
            { "name": "exports", "files": "**/index.ts", "mustExport": "{folder-name}" },
            { "name": "folders", "folders": "src/features/*", "mustBeNamed": "kebab-case" },
        ],
    });
    let (report, work) = support::check(&config, &repo, None);
    assert_eq!(
        report.diagnostics,
        [],
        "every file, folder and instance passes"
    );
    assert_eq!(
        (
            work.structure_evaluations,
            work.export_evaluations,
            work.folder_evaluations
        ),
        (5, 2, 2),
        "three files and two instances, two index files, and two folders"
    );
}

/// Rules of every kind about files, folders and instances. The element of the instances is not
/// the first, so that nothing passes for being element 0.
fn every_structure_rule() -> Value {
    json!({
        "elements": { "shared": "src/shared/**", "feature": "src/features/{feature}/**" },
        "selectors": { "client": { "directive": "use client" } },
        "rules": [
            { "name": "index", "files": "feature", "mustContain": "index.ts" },
            { "name": "readme", "folders": "src/shared/*", "mustContain": "README.md" },
            { "name": "legacy", "folders": "src/legacy/*", "mustBeNamed": "kebab-case" },
            { "name": "kebab", "files": { "all": ["src/**/*.ts", { "not": "client" }] },
              "mustBeNamed": "kebab-case" },
            { "name": "exports", "files": "src/features/*/index.ts", "mustExport": "feature" },
            { "name": "no-stray", "files": "src/stray/**", "mustNotExist": true },
            { "name": "client", "files": "client", "mustBeNamed": "PascalCase" },
        ],
    })
}

fn before_the_change() -> FixtureRepo {
    FixtureRepo::new()
        .file("src/features/login/index.ts", "export * from './model';\n")
        .file("src/features/login/model.ts", "export const feature = 1;\n")
        .file(
            "src/features/billing/index.ts",
            "export const feature = 2;\n",
        )
        .file("src/features/billing/a.ts", "")
        .file("src/shared/ui/README.md", "# ui\n")
        .file("src/shared/ui/button.ts", "export const button = 1;\n")
        .file(
            "src/shared/ui/Dialog.ts",
            "'use client';\nexport const dialog = 1;\n",
        )
        .file("src/shared/old/legacy.ts", "")
        .file("src/legacy/one/a.ts", "")
}

/// The violations and the empty selectors, as (code, file or rule), in report order.
fn outcome(report: &Report) -> Vec<(Code, String)> {
    report
        .diagnostics
        .iter()
        .map(|d| {
            let at = d
                .file
                .clone()
                .or_else(|| d.rule.clone())
                .unwrap_or_default();
            (d.code, at)
        })
        .collect()
}

#[test]
fn a_session_brought_up_to_date_reports_what_a_fresh_check_reports() {
    let mut session = KeptCheck::open(&every_structure_rule(), before_the_change());
    assert_eq!(
        outcome(&session.report()),
        [(Code::MustContain, "src/shared/old".to_owned())],
        "before the change, only the shared folder without a README"
    );
    session.change(&DiskChange {
        write: &[
            ("src/features/login/model.ts", "export const other = 1;\n"),
            (
                "src/shared/ui/button.ts",
                "'use client';\nexport const button = 1;\n",
            ),
            ("src/shared/ui/userCard.ts", ""),
            ("src/shared/data/x.ts", ""),
            ("src/stray/a.ts", ""),
        ],
        remove: &[
            "src/features/billing/index.ts",
            "src/shared/old/legacy.ts",
            "src/legacy/one/a.ts",
        ],
    });
    let fresh = session.fresh();
    assert_eq!(
        outcome(&fresh),
        [
            (Code::SelectorEmpty, "legacy".to_owned()),
            (Code::MustContain, "src/features/billing".to_owned()),
            (Code::MustExport, "src/features/login/index.ts".to_owned()),
            (Code::MustContain, "src/shared/data".to_owned()),
            (Code::MustBeNamed, "src/shared/ui/button.ts".to_owned()),
            (Code::MustBeNamed, "src/shared/ui/userCard.ts".to_owned()),
            (Code::MustNotExist, "src/stray/a.ts".to_owned()),
        ],
        "after it, a violation from each kind of change"
    );
    assert_eq!(
        session.report().diagnostics,
        fresh.diagnostics,
        "the session evaluated again everything the change touched"
    );
}

#[test]
fn an_edit_evaluates_the_structure_rules_of_the_file_it_touched_and_its_barrels() {
    let mut session = KeptCheck::open(&every_structure_rule(), before_the_change());
    let work = session.change(&DiskChange {
        write: &[("src/features/login/model.ts", "export const other = 1;\n")],
        remove: &[],
    });
    assert_eq!(
        (
            work.structure_evaluations,
            work.export_evaluations,
            work.folder_evaluations
        ),
        (0, 1, 0),
        "the edit keeps every class, so only the export rule of the index that passes model.ts on"
    );
    assert_eq!(
        session.report().diagnostics,
        session.fresh().diagnostics,
        "and the index's violation is found"
    );
}

#[test]
fn an_instance_holds_only_the_files_of_its_own_element() {
    let repo = FixtureRepo::new()
        .file("src/features/login/button.stories.ts", "")
        .file("src/features/login/button.ts", "");
    let config = json!({
        "elements": {
            "story": "src/features/{feature}/*.stories.ts",
            "feature": "src/features/{feature}/**",
        },
        "rules": [{ "name": "stories", "files": "feature", "mustContain": "*.stories.ts" }],
    });
    let at: Vec<(Code, String)> = found(&report(&config, &repo))
        .into_iter()
        .map(|(code, file, _)| (code, file))
        .collect();
    assert_eq!(
        at,
        [(Code::MustContain, "src/features/login".to_owned())],
        "the story file below the instance's root is in the element story, not feature"
    );
}
