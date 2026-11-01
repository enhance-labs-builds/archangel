//! Scenarios from the architecture-rules spec: what each import condition and each of its options
//! permits and refuses.
#![cfg(test)]

#[path = "support/layered.rs"]
mod layered;
#[path = "support/check.rs"]
mod support;
#[path = "support/violations.rs"]
mod violations;

use layered::report;
use violations::rule_violations;

use archangel_rules::{Code, Report, ViolationId};
use archangel_testkit::FixtureRepo;
use serde_json::{Value, json};

fn report_with(config: &Value, repo: &FixtureRepo) -> Report {
    support::check(config, repo, None).0
}

/// The violations, as (importing file, specifier), in report order.
fn sites(report: &Report) -> Vec<(String, String)> {
    rule_violations(report)
        .into_iter()
        .map(|d| {
            (
                d.file.clone().unwrap_or_default(),
                d.specifier.clone().unwrap_or_default(),
            )
        })
        .collect()
}

fn site(file: &str, specifier: &str) -> (String, String) {
    (file.to_owned(), specifier.to_owned())
}

fn installed(repo: FixtureRepo, name: &str) -> FixtureRepo {
    repo.file(
        &format!("node_modules/{name}/package.json"),
        &format!(r#"{{ "name": "{name}", "main": "index.js" }}"#),
    )
    .file(&format!("node_modules/{name}/index.js"), "")
    .file(".gitignore", "node_modules/\n")
}

#[test]
fn may_not_import_rejects_an_import_into_its_side() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "import './helpers.test';\n")
        .file("src/helpers.test.ts", "");
    let config = json!({
        "elements": { "src": "src/**" },
        "selectors": { "test": "**/*.test.ts" },
        "rules": [{ "name": "no-tests", "files": { "not": "test" }, "mayNotImport": "test" }],
    });
    let found = report_with(&config, &repo);
    assert_eq!(
        sites(&found),
        [site("src/a.ts", "./helpers.test")],
        "production code may not import a test helper"
    );
    assert_eq!(
        rule_violations(&found)[0].code,
        Code::MayNotImport,
        "the violation's code is its condition"
    );
}

#[test]
fn may_not_import_rejects_a_package_and_permits_other_importers() {
    let repo = installed(
        FixtureRepo::new()
            .file("src/entities/user/a.ts", "import 'react';\n")
            .file("src/features/login/b.ts", "import 'react';\n"),
        "react",
    );
    let found = report(
        &json!([{ "files": "entity", "mayNotImport": { "packages": ["react"] } }]),
        &repo,
    );
    assert_eq!(
        sites(&found),
        [site("src/entities/user/a.ts", "react")],
        "only the entity's import of react breaks the rule"
    );
}

fn views(extra: &[(&str, &str)]) -> FixtureRepo {
    let mut repo = installed(
        FixtureRepo::new()
            .file("src/model/store.ts", "")
            .file("src/shared/ui.ts", "")
            .file("src/views/other.ts", ""),
        "react",
    );
    for (path, text) in extra {
        repo = repo.file(path, text);
    }
    repo
}

fn views_only_import() -> Value {
    json!([{ "name": "thin-views", "files": "view", "mayOnlyImport": ["shared", { "packages": ["react"] }] }])
}

#[test]
fn may_only_import_rejects_an_import_outside_its_list() {
    let repo = views(&[("src/views/a.ts", "import '../model/store';\n")]);
    let found = report(&views_only_import(), &repo);
    assert_eq!(
        sites(&found),
        [site("src/views/a.ts", "../model/store")],
        "a view may not import the model"
    );
    let help = rule_violations(&found)[0].help.clone().unwrap_or_default();
    assert!(
        help.contains("\"shared\"") && help.contains("react"),
        "the help names what a view may import: {help}"
    );
}

#[test]
fn may_only_import_permits_its_list_and_its_own_files() {
    let repo = views(&[(
        "src/views/a.ts",
        "import './other';\nimport 'react';\nimport '../shared/ui';\n",
    )]);
    assert_eq!(
        sites(&report(&views_only_import(), &repo)),
        [],
        "a view may import another view, react and shared code"
    );
}

#[test]
fn may_only_import_counts_imports_of_non_source_files() {
    let repo = views(&[
        ("src/views/a.ts", "import './button.css';\n"),
        ("src/views/button.css", ".button {}\n"),
    ]);
    assert_eq!(
        sites(&report(&views_only_import(), &repo)),
        [site("src/views/a.ts", "./button.css")],
        "the stylesheet is not in the rule's list"
    );
}

#[test]
fn an_unresolved_import_is_not_a_condition_violation() {
    let repo = views(&[("src/views/a.ts", "import './missing';\n")]);
    let found = report(&views_only_import(), &repo);
    assert_eq!(
        sites(&found),
        [],
        "an unresolved import breaks no condition"
    );
    assert!(
        found
            .diagnostics
            .iter()
            .any(|d| d.code == Code::ImportUnresolved),
        "it has its own diagnostic: {:?}",
        found.diagnostics
    );
}

fn db_rule() -> Value {
    json!([{ "name": "db-behind-api", "files": "db", "mayOnlyBeImportedBy": "api" }])
}

#[test]
fn may_only_be_imported_by_rejects_an_importer_not_in_its_list() {
    let repo = FixtureRepo::new()
        .file("web/a.ts", "import '../db/b';\n")
        .file("api/a.ts", "import '../db/b';\n")
        .file("db/b.ts", "");
    let found = report(&db_rule(), &repo);
    assert_eq!(
        sites(&found),
        [site("web/a.ts", "../db/b")],
        "web may not import db"
    );
    let help = rule_violations(&found)[0].help.clone().unwrap_or_default();
    assert!(
        help.contains("\"api\""),
        "the help names the permitted importer: {help}"
    );
}

#[test]
fn may_only_be_imported_by_rejects_an_importer_in_no_element() {
    let repo = FixtureRepo::new()
        .file("scripts/seed.ts", "import '../db/b';\n")
        .file("db/b.ts", "")
        .file("api/a.ts", "")
        .file("web/a.ts", "");
    assert_eq!(
        sites(&report(&db_rule(), &repo)),
        [site("scripts/seed.ts", "../db/b")],
        "a file in no element may not import db"
    );
}

#[test]
fn may_only_be_imported_by_permits_its_own_files() {
    let repo = FixtureRepo::new()
        .file("db/a.ts", "import './b';\n")
        .file("db/b.ts", "")
        .file("api/a.ts", "")
        .file("web/a.ts", "");
    assert_eq!(
        sites(&report(&db_rule(), &repo)),
        [],
        "db files may import each other"
    );
}

fn src_only(rules: &Value) -> Value {
    json!({ "elements": { "src": "src/**" }, "rules": rules })
}

#[test]
fn names_limits_a_rule_to_the_names_imported() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "import { env } from 'node:process';\n")
        .file("src/b.ts", "import { argv } from 'node:process';\n");
    let rules = json!([{ "files": "src/**", "mayNotImport": { "packages": ["node:process"] }, "names": ["env"] }]);
    assert_eq!(
        sites(&report_with(&src_only(&rules), &repo)),
        [site("src/a.ts", "node:process")],
        "only the import of env breaks the rule"
    );
}

#[test]
fn an_import_with_unseen_names_brings_in_every_name() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "import * as p from 'node:process';\n")
        .file("src/b.ts", "export * from 'node:process';\n")
        .file("src/c.ts", "await import('node:process');\n")
        .file("src/d.ts", "require('node:process');\n")
        .file("src/e.ts", "import 'node:process';\n");
    let rules = json!([{ "files": "src/**", "mayNotImport": { "packages": ["node:process"] }, "names": ["env"] }]);
    assert_eq!(
        sites(&report_with(&src_only(&rules), &repo)),
        [
            site("src/a.ts", "node:process"),
            site("src/b.ts", "node:process"),
            site("src/c.ts", "node:process"),
            site("src/d.ts", "node:process"),
        ],
        "a namespace import, an export *, a dynamic import and a require bring in every name; a side-effect import brings in none"
    );
}

#[test]
fn a_default_import_brings_in_default() {
    let repo = installed(
        FixtureRepo::new()
            .file("src/a.ts", "import _ from 'lodash';\n")
            .file("src/b.ts", "import { map } from 'lodash';\n"),
        "lodash",
    );
    let rules = json!([{ "files": "src/**", "mayNotImport": { "packages": ["lodash"] }, "names": ["default"] }]);
    assert_eq!(
        sites(&report_with(&src_only(&rules), &repo)),
        [site("src/a.ts", "lodash")],
        "the default import brings in default"
    );
}

fn aliased(repo: FixtureRepo) -> FixtureRepo {
    repo.file(
        "tsconfig.json",
        r#"{ "compilerOptions": { "baseUrl": ".", "paths": { "@/*": ["src/*"] } } }"#,
    )
}

#[test]
fn specifier_limits_a_rule_to_one_form_of_specifier() {
    let repo = aliased(
        FixtureRepo::new()
            .file("src/features/login/a.ts", "import '../billing/index.ts';\n")
            .file("src/features/login/b.ts", "import '@/features/billing';\n")
            .file("src/features/billing/index.ts", ""),
    );
    let config = json!({
        "elements": { "slice": "src/features/{slice}/**" },
        "rules": [{ "files": "slice", "mayNotImport": "slice", "instance": "different", "specifier": "relative" }],
    });
    assert_eq!(
        sites(&report_with(&config, &repo)),
        [site("src/features/login/a.ts", "../billing/index.ts")],
        "only the relative import breaks the rule"
    );
}

#[test]
fn specifier_forms_are_told_apart() {
    let repo = aliased(
        FixtureRepo::new()
            .file(
                "src/x/main.ts",
                "import './a';\nimport '../b';\nimport '@/c';\nimport '#d';\n",
            )
            .file("src/x/a.ts", "")
            .file("src/b.ts", "")
            .file("src/c.ts", "")
            .file("src/d.ts", "")
            .file(
                "package.json",
                r##"{ "name": "app", "imports": { "#d": "./src/d.ts" } }"##,
            ),
    );
    let rules: Vec<Value> = ["relative", "parent", "alias", "package"]
        .iter()
        .map(|form| json!({ "name": form, "files": "src/**", "mayNotImport": "src/**", "specifier": form }))
        .collect();
    let found = report_with(&src_only(&json!(rules)), &repo);
    let by_rule = |rule: &str| -> Vec<String> {
        rule_violations(&found)
            .into_iter()
            .filter(|d| d.rule.as_deref() == Some(rule))
            .map(|d| d.specifier.clone().unwrap_or_default())
            .collect()
    };
    assert_eq!(by_rule("relative"), ["./a", "../b"], "relative imports");
    assert_eq!(by_rule("parent"), ["../b"], "parent imports");
    assert_eq!(by_rule("alias"), ["@/c", "#d"], "alias imports");
    assert_eq!(
        by_rule("package"),
        Vec::<String>::new(),
        "no import here is a package import"
    );
}

fn features() -> FixtureRepo {
    FixtureRepo::new()
        .file(
            "src/features/login/a.ts",
            "import '../billing/b';\nimport './c';\nimport './internal/x';\n",
        )
        .file("src/features/login/c.ts", "")
        .file("src/features/login/internal/x.ts", "")
        .file(
            "src/features/billing/b.ts",
            "import '../login/internal/x';\n",
        )
}

#[test]
fn isolate_and_its_sentence_give_the_same_violations() {
    let repo = features();
    let sentence = sites(&report(
        &json!([{ "files": "feature", "mayNotImport": "feature", "instance": "different" }]),
        &repo,
    ));
    let isolate = sites(&report(
        &json!([{ "kind": "isolate", "element": "feature" }]),
        &repo,
    ));
    assert_eq!(
        sentence,
        [
            site("src/features/billing/b.ts", "../login/internal/x"),
            site("src/features/login/a.ts", "../billing/b"),
        ],
        "the sentence refuses the imports between two features"
    );
    assert_eq!(isolate, sentence, "isolate gives the same violations");
}

#[test]
fn instance_same_limits_a_rule_to_one_instance() {
    let found = report(
        &json!([{ "files": "feature", "mayNotImport": "**/internal/**", "instance": "same" }]),
        &features(),
    );
    assert_eq!(
        sites(&found),
        [site("src/features/login/a.ts", "./internal/x")],
        "only the import within one instance breaks the rule"
    );
}

#[test]
fn dependency_dev_selects_packages_listed_only_for_development() {
    let repo = installed(
        installed(
            FixtureRepo::new()
                .file(
                    "package.json",
                    r#"{ "name": "app", "dependencies": { "zod": "1" }, "devDependencies": { "vitest": "1", "zod": "1" } }"#,
                )
                .file("src/a.ts", "import 'vitest';\nimport 'zod';\n"),
            "vitest",
        ),
        "zod",
    );
    let rules =
        json!([{ "files": "src/**", "mayNotImport": { "packages": ["*"] }, "dependency": "dev" }]);
    assert_eq!(
        sites(&report_with(&src_only(&rules), &repo)),
        [site("src/a.ts", "vitest")],
        "only the package listed only for development breaks the rule"
    );
}

#[test]
fn dependency_undeclared_uses_the_nearest_manifest() {
    let repo = installed(
        FixtureRepo::new()
            .file(
                "package.json",
                r#"{ "name": "root", "dependencies": { "lodash": "1" } }"#,
            )
            .file("packages/a/package.json", r#"{ "name": "a" }"#)
            .file("packages/a/src/x.ts", "import 'lodash';\n"),
        "lodash",
    );
    let config = json!({
        "elements": { "packages": "packages/**" },
        "rules": [{ "files": "packages/**", "mayNotImport": { "packages": ["*"] }, "dependency": "undeclared" }],
    });
    let found = report_with(&config, &repo);
    assert_eq!(
        sites(&found),
        [site("packages/a/src/x.ts", "lodash")],
        "the nearest manifest does not list lodash"
    );
    let violation = rule_violations(&found)[0];
    let text = format!(
        "{} {}",
        violation.message,
        violation.help.clone().unwrap_or_default()
    );
    assert!(
        text.contains("packages/a/package.json"),
        "the violation names the nearest manifest: {text}"
    );
}

fn server_only(repo: FixtureRepo) -> FixtureRepo {
    installed(repo, "server-only")
}

fn client_rule(extra: &Value) -> Value {
    let mut rule = json!({
        "name": "client-stays-client",
        "files": { "directive": "use client" },
        "mayNotImport": { "packages": ["server-only"] },
        "transitively": true,
    });
    if let (Some(rule), Some(extra)) = (rule.as_object_mut(), extra.as_object()) {
        rule.extend(extra.clone());
    }
    json!({ "elements": { "app": "app/**", "lib": "lib/**" }, "rules": [rule] })
}

#[test]
fn a_transitive_rule_shows_the_shortest_chain() {
    let repo = server_only(
        FixtureRepo::new()
            .file("app/a.tsx", "'use client';\nimport '../lib/b';\n")
            .file("lib/b.ts", "import 'server-only';\n"),
    );
    let found = report_with(&client_rule(&json!({})), &repo);
    let violations = rule_violations(&found);
    assert_eq!(violations.len(), 1, "one violation: {violations:?}");
    assert_eq!(
        violations[0].file.as_deref(),
        Some("app/a.tsx"),
        "the violation is in the client file"
    );
    assert_eq!(
        violations[0].chain,
        ["app/a.tsx", "lib/b.ts", "server-only"],
        "it shows the chain"
    );
}

#[test]
fn a_tie_between_chains_goes_to_path_order() {
    let repo = server_only(
        FixtureRepo::new()
            .file(
                "app/a.tsx",
                "'use client';\nimport '../lib/c';\nimport '../lib/b';\n",
            )
            .file("lib/b.ts", "import 'server-only';\n")
            .file("lib/c.ts", "import 'server-only';\n"),
    );
    let found = report_with(&client_rule(&json!({})), &repo);
    let violations = rule_violations(&found);
    assert_eq!(violations.len(), 1, "one violation: {violations:?}");
    assert_eq!(
        violations[0].chain,
        ["app/a.tsx", "lib/b.ts", "server-only"],
        "the chain goes through the file first in path order"
    );
    assert_eq!(
        violations[0].specifier.as_deref(),
        Some("../lib/b"),
        "and is reported at that chain's first import"
    );
}

#[test]
fn a_transitive_rule_follows_runtime_imports_by_default() {
    let repo = server_only(
        FixtureRepo::new()
            .file(
                "app/a.tsx",
                "'use client';\nimport type { T } from '../lib/b';\n",
            )
            .file("lib/b.ts", "import 'server-only';\nexport type T = 1;\n"),
    );
    assert_eq!(
        rule_violations(&report_with(&client_rule(&json!({})), &repo)).len(),
        0,
        "a type import disappears when TypeScript compiles"
    );
}

#[test]
fn a_chain_stops_at_files_in_stop_at() {
    let repo = server_only(
        FixtureRepo::new()
            .file("app/a.tsx", "'use client';\nimport './actions';\n")
            .file("app/actions.ts", "'use server';\nimport 'server-only';\n"),
    );
    let config = client_rule(&json!({ "stopAt": { "directive": "use server" } }));
    assert_eq!(
        rule_violations(&report_with(&config, &repo)).len(),
        0,
        "the chain ends at the server action"
    );
}

#[test]
fn dependency_undeclared_permits_a_package_in_either_list() {
    let repo = installed(
        installed(
            installed(
                FixtureRepo::new()
                    .file(
                        "package.json",
                        r#"{ "name": "app", "dependencies": { "zod": "1" }, "devDependencies": { "vitest": "1" } }"#,
                    )
                    .file("src/a.ts", "import 'zod';\nimport 'vitest';\nimport 'lodash';\n"),
                "zod",
            ),
            "vitest",
        ),
        "lodash",
    );
    let rules = json!([{ "files": "src/**", "mayNotImport": { "packages": ["*"] }, "dependency": "undeclared" }]);
    assert_eq!(
        sites(&report_with(&src_only(&rules), &repo)),
        [site("src/a.ts", "lodash")],
        "zod and vitest are each listed once, and lodash in neither list"
    );
}

#[test]
fn a_package_of_the_workspace_is_a_package_specifier_though_it_resolves_to_a_file() {
    let repo = FixtureRepo::new()
        .file(
            "tsconfig.json",
            r#"{ "compilerOptions": { "baseUrl": ".", "paths": {
                "@acme/ui": ["packages/ui/index.ts"],
                "core/*": ["packages/core/*"],
                "@/*": ["app/*"]
            } } }"#,
        )
        .file("packages/ui/package.json", r#"{ "name": "@acme/ui" }"#)
        .file("packages/ui/index.ts", "")
        .file("packages/core/package.json", r#"{ "name": "core" }"#)
        .file("packages/core/utils.ts", "")
        .file("app/local.ts", "")
        .file(
            "app/main.ts",
            "import '@acme/ui';\nimport 'core/utils';\nimport '@/local';\nimport './local';\n",
        );
    let rules: Vec<Value> = ["package", "alias"]
        .iter()
        .map(|form| json!({ "name": form, "files": "app/**", "mayNotImport": "**", "specifier": form }))
        .collect();
    let config =
        json!({ "elements": { "app": "app/**", "packages": "packages/**" }, "rules": rules });
    let found = report_with(&config, &repo);
    let by_rule = |rule: &str| -> Vec<String> {
        rule_violations(&found)
            .into_iter()
            .filter(|d| d.rule.as_deref() == Some(rule))
            .map(|d| d.specifier.clone().unwrap_or_default())
            .collect()
    };
    assert_eq!(
        by_rule("package"),
        ["@acme/ui", "core/utils"],
        "a scoped and a plain name of the workspace's own manifests, with or without a subpath"
    );
    assert_eq!(by_rule("alias"), ["@/local"], "an alias names no manifest");
}

#[test]
fn two_inline_package_names_each_stand_for_their_own_package() {
    let repo = installed(
        installed(
            FixtureRepo::new().file("src/a.ts", "import 'react';\nimport 'lodash';\n"),
            "react",
        ),
        "lodash",
    );
    let rules = json!([
        { "name": "no-react", "files": "src/**", "mayNotImport": { "packages": ["react"] } },
        { "name": "no-lodash", "files": "src/**", "mayNotImport": { "packages": ["lodash"] } },
    ]);
    let found = report_with(&src_only(&rules), &repo);
    let broken: Vec<(Option<&str>, Option<&str>)> = rule_violations(&found)
        .into_iter()
        .map(|d| (d.rule.as_deref(), d.specifier.as_deref()))
        .collect();
    assert_eq!(
        broken,
        [
            (Some("no-react"), Some("react")),
            (Some("no-lodash"), Some("lodash")),
        ],
        "each rule refuses its own package and no other"
    );
}

#[test]
fn a_transitive_violation_is_the_occurrence_of_the_import_its_rule_counts() {
    let repo = server_only(
        FixtureRepo::new()
            .file(
                "app/a.tsx",
                "'use client';\nimport type { T } from '../lib/b';\nimport '../lib/b';\n",
            )
            .file("lib/b.ts", "import 'server-only';\nexport type T = 1;\n"),
    );
    let found = report_with(&client_rule(&json!({})), &repo);
    let ids: Vec<Option<&ViolationId>> = rule_violations(&found)
        .into_iter()
        .map(|d| d.id.as_ref())
        .collect();
    assert_eq!(
        ids,
        [Some(&ViolationId::Import {
            rule: "client-stays-client".to_owned(),
            file: "app/a.tsx".to_owned(),
            specifier: "../lib/b".to_owned(),
            occurrence: 1,
        })],
        "the runtime import is the second import of ../lib/b, after the type import"
    );
}

/// An entity file among the API's files, with the rule of an area under test.
fn entity_config(rule: &Value) -> Value {
    json!({
        "elements": { "entity": "apps/api/src/*/domain/**", "value-object": "apps/api/src/*/values/**" },
        "rules": [rule],
    })
}

#[test]
fn may_only_import_within_rejects_an_import_into_the_area_outside_its_list() {
    let repo = FixtureRepo::new()
        .file(
            "apps/api/src/canvas/domain/a.entity.ts",
            "import '../application/save.use-case';\n",
        )
        .file("apps/api/src/canvas/application/save.use-case.ts", "")
        .file("apps/api/src/canvas/values/v.ts", "");
    let config = entity_config(&json!({
        "name": "entities", "files": "entity", "mayOnlyImport": ["value-object"], "within": "apps/api/src/**",
    }));
    let found = report_with(&config, &repo);
    assert_eq!(
        sites(&found),
        [site(
            "apps/api/src/canvas/domain/a.entity.ts",
            "../application/save.use-case"
        )],
        "{found:?}"
    );
    let message = &rule_violations(&found)[0].message;
    assert!(message.contains("\"value-object\""), "{message}");
    assert!(message.contains("apps/api/src/**"), "{message}");
}

#[test]
fn may_only_import_within_does_not_judge_an_import_outside_the_area() {
    let repo = installed(
        FixtureRepo::new()
            .file(
                "apps/api/src/canvas/domain/a.entity.ts",
                "import 'zod';\nimport '../../../../web/src/shared/lib/format';\n",
            )
            .file("apps/web/src/shared/lib/format.ts", "")
            .file("apps/api/src/canvas/values/v.ts", ""),
        "zod",
    );
    let config = entity_config(&json!({
        "files": "entity", "mayOnlyImport": ["value-object"], "within": "apps/api/src/**",
    }));
    assert_eq!(sites(&report_with(&config, &repo)), [], "both are outside");
}

#[test]
fn may_only_import_within_judges_the_packages_in_its_area() {
    let repo = installed(
        installed(
            FixtureRepo::new()
                .file(
                    "apps/api/src/canvas/domain/a.entity.ts",
                    "import 'zod';\nimport 'lodash';\n",
                )
                .file("apps/api/src/canvas/values/v.ts", ""),
            "zod",
        ),
        "lodash",
    );
    let config = entity_config(&json!({
        "files": "entity",
        "mayOnlyImport": [{ "packages": ["zod"] }],
        "within": ["apps/api/src/**", { "packages": ["*"] }],
    }));
    assert_eq!(
        sites(&report_with(&config, &repo)),
        [site("apps/api/src/canvas/domain/a.entity.ts", "lodash")],
        "the area holds every package, and the list holds zod"
    );
}

#[test]
fn may_only_import_within_permits_its_own_files() {
    let repo = FixtureRepo::new()
        .file(
            "apps/api/src/canvas/domain/a.entity.ts",
            "import './b.entity';\n",
        )
        .file("apps/api/src/canvas/domain/b.entity.ts", "")
        .file("apps/api/src/canvas/values/v.ts", "");
    let config = entity_config(&json!({
        "files": "entity", "mayOnlyImport": ["value-object"], "within": "apps/api/src/**",
    }));
    assert_eq!(sites(&report_with(&config, &repo)), [], "its own files");
}

#[test]
fn may_only_import_within_combines_with_the_import_limits() {
    let repo = FixtureRepo::new()
        .file(
            "tsconfig.json",
            r#"{ "compilerOptions": { "baseUrl": ".", "paths": { "@api/*": ["apps/api/src/canvas/*"] } } }"#,
        )
        .file(
            "apps/api/src/canvas/domain/a.entity.ts",
            "import '../application/save.use-case';\nimport '@api/application/load.use-case';\n",
        )
        .file("apps/api/src/canvas/application/save.use-case.ts", "")
        .file("apps/api/src/canvas/application/load.use-case.ts", "")
        .file("apps/api/src/canvas/values/v.ts", "");
    let config = entity_config(&json!({
        "files": "entity", "mayOnlyImport": ["value-object"], "within": "apps/api/src/**", "specifier": "parent",
    }));
    assert_eq!(
        sites(&report_with(&config, &repo)),
        [site(
            "apps/api/src/canvas/domain/a.entity.ts",
            "../application/save.use-case"
        )],
        "only the import that starts with ../"
    );
}

#[test]
fn an_unresolved_import_is_not_a_violation_of_a_rule_with_an_area() {
    let repo = FixtureRepo::new()
        .file(
            "apps/api/src/canvas/domain/a.entity.ts",
            "import './missing';\n",
        )
        .file("apps/api/src/canvas/values/v.ts", "");
    let config = entity_config(&json!({
        "files": "entity", "mayOnlyImport": ["value-object"], "within": "apps/api/src/**",
    }));
    let found = report_with(&config, &repo);
    assert_eq!(sites(&found), [], "no violation of the rule");
    let unresolved = found
        .diagnostics
        .iter()
        .filter(|d| d.code == Code::ImportUnresolved)
        .count();
    assert_eq!(unresolved, 1, "{found:?}");
}

#[test]
fn may_only_be_imported_by_within_does_not_judge_an_importer_outside_the_area() {
    let repo = FixtureRepo::new()
        .file("scripts/seed.ts", "import '../db/client';\n")
        .file("apps/web/a.ts", "import '../../db/client';\n")
        .file("apps/api/a.ts", "import '../../db/client';\n")
        .file("db/client.ts", "");
    let config = json!({
        "elements": { "db": "db/**", "api": "apps/api/**", "web": "apps/web/**" },
        "rules": [{ "name": "db-for-api", "files": "db", "mayOnlyBeImportedBy": "api", "within": "apps/**" }],
    });
    let found = report_with(&config, &repo);
    assert_eq!(
        sites(&found),
        [site("apps/web/a.ts", "../../db/client")],
        "the script is outside the area"
    );
    let message = &rule_violations(&found)[0].message;
    assert!(message.contains("apps/**"), "{message}");
}

#[test]
fn may_only_import_within_an_externals_set_judges_its_packages() {
    let repo = installed(
        installed(
            FixtureRepo::new()
                .file(
                    "src/domain/a.ts",
                    "import 'pg';\nimport 'zod';\nimport './b';\n",
                )
                .file("src/domain/b.ts", "")
                .file(
                    "package.json",
                    r#"{ "name": "app", "dependencies": { "drizzle-orm": "1" } }"#,
                ),
            "pg",
        ),
        "zod",
    );
    let config = json!({
        "parts": { "domain": "src/domain/**" },
        "externals": { "driver": ["pg", "drizzle-orm"] },
        "rules": [{ "name": "drivers-in-infrastructure-only", "files": "domain", "mayOnlyImport": [], "within": "driver" }],
    });
    assert_eq!(
        sites(&report_with(&config, &repo)),
        [site("src/domain/a.ts", "pg")],
        "only the package in the set"
    );
    let found = report_with(&config, &repo);
    let message = &rule_violations(&found)[0].message;
    assert!(
        message.contains("may only import each other, within \"driver\""),
        "{message}"
    );
}

#[test]
fn may_only_import_without_an_area_judges_outside_packages() {
    let repo = installed(
        FixtureRepo::new()
            .file("src/view/a.ts", "import 'zod';\n")
            .file("src/shared/b.ts", ""),
        "zod",
    );
    let config = json!({
        "elements": { "view": "src/view/**", "shared": "src/shared/**" },
        "rules": [{ "files": "view", "mayOnlyImport": ["shared"] }],
    });
    assert_eq!(
        sites(&report_with(&config, &repo)),
        [site("src/view/a.ts", "zod")],
        "no area judges every import"
    );
}

#[test]
fn may_only_import_permits_its_own_files_in_the_same_instance() {
    let repo = FixtureRepo::new()
        .file(
            "src/features/cart/a.ts",
            "import './b';\nimport '../login/c';\n",
        )
        .file("src/features/cart/b.ts", "")
        .file("src/features/login/c.ts", "")
        .file("src/shared/s.ts", "");
    let config = json!({
        "parts": { "feature": "src/features/{feature}/**", "shared": "src/shared/**" },
        "rules": [{ "files": "feature", "mayOnlyImport": ["shared"] }],
    });
    let found = report_with(&config, &repo);
    assert_eq!(
        sites(&found),
        [site("src/features/cart/a.ts", "../login/c")],
        "the same instance is permitted, and the other is not"
    );
    let message = &rule_violations(&found)[0].message;
    assert!(message.contains("same instance"), "{message}");
}

#[test]
fn an_area_package_that_no_import_uses_and_no_manifest_lists_is_an_error() {
    let repo = FixtureRepo::new()
        .file("src/entity/a.ts", "")
        .file("src/value/v.ts", "");
    let config = json!({
        "elements": { "entity": "src/entity/**", "value-object": "src/value/**" },
        "rules": [{
            "name": "entities", "files": "entity", "mayOnlyImport": ["value-object"],
            "within": [{ "packages": ["zodd"] }],
        }],
    });
    let found = report_with(&config, &repo);
    let empty: Vec<_> = found
        .diagnostics
        .iter()
        .filter(|d| d.code == Code::ElementEmpty && d.message.contains("`zodd`"))
        .collect();
    assert_eq!(empty.len(), 1, "{found:?}");
    assert_eq!(empty[0].rule.as_deref(), Some("entities"), "{empty:?}");
}
