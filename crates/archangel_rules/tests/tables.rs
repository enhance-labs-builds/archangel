//! Scenarios from the import-tables spec: each row of a table is one rule, a table fails closed for
//! repository files, and it judges an outside package only when it names its externals set.
#![cfg(test)]

#[path = "support/tables.rs"]
mod support;

use archangel_rules::{Diagnostic, Report};
use archangel_testkit::{FixtureRepo, table};
use serde_json::{Value, json};
use support::{of, repo_of, report_of, slice, slice_repo, slice_table, violated, with_packages};

/// The config of the small app in the design.
fn small_app() -> Value {
    json!({
        "parts": { "feature": "src/features/{feature}/**", "shared": "src/shared/**" },
        "rules": [
            table("app", &json!({ "feature": ["shared"], "shared": [] }), &json!({})),
            table(
                "feature-code",
                &json!({
                    "views": ["models", "shared"],
                    "models": ["api", "shared"],
                    "api": ["shared"],
                }),
                &json!({ "kinds": "src/features/*/{kind}/**" }),
            ),
        ],
    })
}

fn small_repo() -> FixtureRepo {
    repo_of(&[
        "src/main.ts",
        "src/shared/money.ts",
        "src/features/cart/views/cart-page.tsx",
        "src/features/cart/models/cart.ts",
        "src/features/cart/api/fetch-cart.ts",
    ])
}

fn codes(report: &Report) -> Vec<&'static str> {
    report.diagnostics.iter().map(|d| d.code.as_str()).collect()
}

fn message_of(report: &Report, rule: &str) -> String {
    let found = of(report, rule);
    assert_eq!(
        found.len(),
        1,
        "one violation of {rule}: {:?}",
        report.diagnostics
    );
    found[0].message.clone()
}

#[test]
fn a_row_refuses_an_import_that_it_does_not_list() {
    let repo = small_repo().file(
        "src/features/cart/views/cart-page.tsx",
        "import '../api/fetch-cart';\n",
    );
    let report = report_of(&small_app(), &repo);
    assert_eq!(
        violated(&report),
        ["feature-code.views"],
        "{:?}",
        report.diagnostics
    );
    let message = message_of(&report, "feature-code.views");
    assert!(
        message.contains("views may import only models, shared"),
        "{message}"
    );
}

#[test]
fn a_row_permits_an_import_of_a_name_that_it_lists() {
    let repo = small_repo().file(
        "src/features/cart/views/cart-page.tsx",
        "import '../models/cart';\nimport '../../../shared/money';\n",
    );
    let report = report_of(&small_app(), &repo);
    assert_eq!(
        violated(&report),
        [] as [&str; 0],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn a_cell_means_its_name_in_the_whole_repository() {
    let repo = slice_repo()
        .file("apps/api/src/board/application/draft.value-object.ts", "")
        .file(
            "apps/api/src/board/domain/board.entity.ts",
            "import '../application/draft.value-object';\n",
        );
    let report = report_of(&slice(), &repo);
    assert_eq!(
        violated(&report),
        [] as [&str; 0],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn an_unresolved_import_is_not_a_violation_of_a_row() {
    let repo = small_repo().file(
        "src/features/cart/views/cart-page.tsx",
        "import './missing';\n",
    );
    let report = report_of(&small_app(), &repo);
    assert_eq!(
        violated(&report),
        [] as [&str; 0],
        "{:?}",
        report.diagnostics
    );
    assert_eq!(
        codes(&report),
        ["import-unresolved"],
        "an unresolved import is not a violation of a row"
    );
}

#[test]
fn a_target_with_a_kind_and_no_row_is_named_by_its_kind() {
    let repo = slice_repo().file(
        "apps/api/src/board/domain/board.entity.ts",
        "import '../application/save.use-case';\n",
    );
    let report = report_of(&slice(), &repo);
    assert_eq!(
        violated(&report),
        ["domain.entity"],
        "{:?}",
        report.diagnostics
    );
    let message = message_of(&report, "domain.entity");
    assert!(message.contains("no row for use-case"), "{message}");
}

#[test]
fn a_target_without_a_kind_is_named_by_its_part() {
    let repo = slice_repo().file(
        "apps/api/src/board/domain/board.entity.ts",
        "import '../../../../../packages/contract/src/canvas';\n",
    );
    let report = report_of(&slice(), &repo);
    let message = message_of(&report, "domain.entity");
    assert!(message.contains("no row for contract"), "{message}");
}

#[test]
fn a_target_in_no_part_is_named_by_its_path() {
    let repo = small_repo().file(
        "src/features/cart/models/cart.ts",
        "import '../../../main';\n",
    );
    let report = report_of(&small_app(), &repo);
    let message = message_of(&report, "app.feature");
    assert!(
        message.contains("no row for src/main.ts, which is in no part"),
        "{message}"
    );
}

/// A table of two groups, whose first row lists an externals set.
fn driver_config(everyone: Option<&Value>) -> Value {
    let mut options = json!({});
    if let Some(cells) = everyone {
        options["everyRowMayImport"] = cells.clone();
    }
    json!({
        "parts": { "src": "src/**" },
        "groups": { "domain": "src/domain/**", "infrastructure": "src/infra/**" },
        "externals": { "driver": ["pg"], "orm": ["mysql2"] },
        "rules": [table(
            "app",
            &json!({ "infrastructure": ["domain", "driver", "orm"], "domain": [] }),
            &options,
        )],
    })
}

#[test]
fn a_table_judges_only_the_outside_packages_that_it_names() {
    let repo = with_packages(
        FixtureRepo::new()
            .file(
                "src/domain/a.ts",
                "import 'pg/lib/client.js';\nimport 'zod';\n",
            )
            .file("src/infra/b.ts", "import 'pg';\nimport 'mysql2';\n"),
        &["pg", "zod", "mysql2"],
    );
    let report = report_of(&driver_config(None), &repo);
    assert_eq!(
        violated(&report),
        ["app.domain"],
        "{:?}",
        report.diagnostics
    );
    assert_eq!(
        of(&report, "app.domain")[0].specifier.as_deref(),
        Some("pg/lib/client.js"),
        "a table judges only the outside packages that it names"
    );
}

#[test]
fn a_table_that_names_no_externals_set_ignores_outside_packages() {
    let repo = with_packages(
        small_repo().file("src/features/cart/views/cart-page.tsx", "import 'react';\n"),
        &["react"],
    );
    let report = report_of(&small_app(), &repo);
    assert_eq!(
        violated(&report),
        [] as [&str; 0],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn a_workspace_package_is_judged_by_the_part_of_its_file() {
    let repo = slice_repo()
        .file(
            "tsconfig.json",
            r#"{ "compilerOptions": { "baseUrl": ".", "paths": { "@enhance/contract": ["packages/contract/src/index.ts"] } } }"#,
        )
        .file("packages/contract/package.json", r#"{ "name": "@enhance/contract" }"#)
        .file("packages/contract/src/index.ts", "")
        .file("packages/canvas-format/src/format.ts", "import '@enhance/contract';\n");
    let config = json!({
        "parts": {
            "api": "apps/api/**",
            "contract": "packages/contract/**",
            "canvas-format": "packages/canvas-format/**",
        },
        "rules": [table(
            "workspace",
            &json!({ "api": ["contract"], "contract": [], "canvas-format": [] }),
            &json!({}),
        )],
    });
    let report = report_of(&config, &repo);
    let message = message_of(&report, "workspace.canvas-format");
    assert!(message.contains("contract"), "{message}");
}

#[test]
fn a_row_permits_its_own_name_only_in_the_same_instance() {
    let repo = small_repo()
        .file("src/features/login/models/session.ts", "")
        .file(
            "src/features/cart/models/cart.ts",
            "import '../api/fetch-cart';\nimport '../../login/models/session';\n",
        );
    let report = report_of(&small_app(), &repo);
    let found = of(&report, "app.feature");
    assert_eq!(found.len(), 1, "{:?}", report.diagnostics);
    assert_eq!(
        found[0].specifier.as_deref(),
        Some("../../login/models/session"),
        "a row permits its own name only in the same instance"
    );
}

#[test]
fn a_cell_that_names_the_row_permits_every_instance() {
    let repo = FixtureRepo::new()
        .file("src/shared/a.ts", "")
        .file("src/features/login/b.ts", "")
        .file("src/features/cart/c.ts", "import '../login/b';\n");
    let config = json!({
        "parts": { "feature": "src/features/{feature}/**", "shared": "src/shared/**" },
        "rules": [table("app", &json!({ "feature": ["feature", "shared"], "shared": [] }), &json!({}))],
    });
    let report = report_of(&config, &repo);
    assert_eq!(
        violated(&report),
        [] as [&str; 0],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn may_not_import_itself_removes_the_permission() {
    let repo = slice_repo()
        .file("apps/api/src/board/domain/renamed.domain-event.ts", "")
        .file(
            "apps/api/src/board/domain/created.domain-event.ts",
            "import './renamed.domain-event';\n",
        );
    let report = report_of(&slice(), &repo);
    assert_eq!(
        violated(&report),
        ["domain.domain-event"],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn a_row_with_a_capture_that_may_not_import_itself_gives_one_violation() {
    let repo = repo_of(&["src/shared/s.ts", "src/features/login/b.ts"])
        .file("src/features/cart/c.ts", "import '../login/b';\n");
    let config = json!({
        "parts": { "feature": "src/features/{feature}/**", "shared": "src/shared/**" },
        "rules": [table(
            "app",
            &json!({ "feature": [], "shared": [] }),
            &json!({ "mayNotImportItself": ["feature"] }),
        )],
    });
    let report = report_of(&config, &repo);
    assert_eq!(
        violated(&report),
        ["app.feature"],
        "one import gives one violation, though two sentences of the row refuse it"
    );
}

#[test]
fn a_kind_may_import_its_own_kind_by_default() {
    let repo = slice_repo()
        .file("apps/api/src/board/domain/column.entity.ts", "")
        .file(
            "apps/api/src/board/domain/board.entity.ts",
            "import './column.entity';\n",
        );
    let report = report_of(&slice(), &repo);
    assert_eq!(
        violated(&report),
        [] as [&str; 0],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn another_capture_in_a_kind_pattern_makes_an_instance() {
    let repo = FixtureRepo::new()
        .file("apps/api/src/board/domain/column.entity.ts", "")
        .file("apps/api/src/canvas/domain/canvas.entity.ts", "")
        .file(
            "apps/api/src/board/domain/board.entity.ts",
            "import './column.entity';\nimport '../../canvas/domain/canvas.entity';\n",
        );
    let config = json!({
        "parts": { "api": "apps/api/**" },
        "rules": [table(
            "t",
            &json!({ "entity": [] }),
            &json!({ "kinds": "apps/api/src/{context}/domain/**/*.{kind}.ts" }),
        )],
    });
    let report = report_of(&config, &repo);
    let found = of(&report, "t.entity");
    assert_eq!(found.len(), 1, "{:?}", report.diagnostics);
    assert_eq!(
        found[0].specifier.as_deref(),
        Some("../../canvas/domain/canvas.entity"),
        "another capture in a kind pattern makes an instance"
    );
}

#[test]
fn every_row_may_import_what_every_row_may_import_lists() {
    let repo = slice_repo().file(
        "apps/api/src/board/domain/money.value-object.ts",
        "import '../../shared/result';\n",
    );
    let report = report_of(&slice(), &repo);
    assert_eq!(
        violated(&report),
        [] as [&str; 0],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn every_row_may_import_an_externals_set() {
    let repo = with_packages(
        FixtureRepo::new()
            .file("src/domain/a.ts", "import 'pg';\nimport 'mysql2';\n")
            .file("src/infra/b.ts", ""),
        &["pg", "mysql2"],
    );
    let config = driver_config(Some(&json!(["driver"])));
    let report = report_of(&config, &repo);
    let found = of(&report, "app.domain");
    assert_eq!(found.len(), 1, "{:?}", report.diagnostics);
    assert_eq!(
        found[0].specifier.as_deref(),
        Some("mysql2"),
        "every row may import an externals set"
    );
}

/// One table of the rows, over a repository whose every file imports the target, so that a
/// violation shows which row a file is in.
fn with_target(files: &[&str], kinds: &str, rows: &Value) -> Report {
    let mut repo = FixtureRepo::new().file("z/target.js", "");
    for file in files {
        let depth = file.matches('/').count();
        repo = repo.file(
            file,
            &format!("import '{}z/target.js';\n", "../".repeat(depth)),
        );
    }
    let config = json!({
        "parts": { "repository": "**" },
        "rules": [table("t", rows, &json!({ "kinds": kinds }))],
    });
    report_of(&config, &repo)
}

/// Each violation of a row, as its file and its rule, sorted by file.
fn judged(report: &Report) -> Vec<String> {
    let mut found: Vec<String> = report
        .diagnostics
        .iter()
        .filter(|d| d.code.is_rule())
        .map(|d| {
            format!(
                "{} {}",
                d.file.as_deref().unwrap_or_default(),
                d.rule.as_deref().unwrap_or_default()
            )
        })
        .collect();
    found.sort();
    found
}

#[test]
fn the_kind_is_the_name_before_the_extension() {
    let report = with_target(
        &["a/board.entity.ts", "a/board.entity.test.ts", "a/board.ts"],
        "**/*.{kind}.ts",
        &json!({ "entity": [], "test": [] }),
    );
    assert_eq!(
        judged(&report),
        [
            "a/board.entity.test.ts t.test",
            "a/board.entity.ts t.entity"
        ],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn a_declaration_file_counts_d_ts_as_its_extension() {
    let report = with_target(
        &["a/board.entity.d.ts", "a/env.d.ts"],
        "**/*.{kind}.ts",
        &json!({ "entity": [] }),
    );
    assert_eq!(
        judged(&report),
        ["a/board.entity.d.ts t.entity"],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn a_kind_pattern_by_folder_starts_at_the_config_folder() {
    let report = with_target(
        &[
            "src/domain/entities/board.ts",
            "packages/x/src/domain/entities/a.ts",
        ],
        "src/domain/{kind}/**",
        &json!({ "entities": [] }),
    );
    assert_eq!(
        judged(&report),
        ["src/domain/entities/board.ts t.entities"],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn the_kind_capture_in_two_places_takes_one_name() {
    let report = with_target(
        &["src/entitys/board.entity.ts", "src/entitys/board.policy.ts"],
        "**/{kind}s/*.{kind}.ts",
        &json!({ "entity": [], "policy": [] }),
    );
    assert_eq!(
        judged(&report),
        ["src/entitys/board.entity.ts t.entity"],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn a_table_can_mix_groups_and_parts() {
    let repo = FixtureRepo::new()
        .file("src/shared/clock.ts", "")
        .file("src/infra/UserRepository.ts", "")
        .file(
            "src/application/save.ts",
            "import '../shared/clock';\nimport '../infra/UserRepository';\n",
        );
    let config = json!({
        "parts": { "shared": "src/shared/**" },
        "groups": { "repository": "**/*Repository.ts", "use-case": "src/application/**" },
        "rules": [table(
            "t",
            &json!({ "shared": [], "repository": [], "use-case": ["repository", "shared"] }),
            &json!({}),
        )],
    });
    let report = report_of(&config, &repo);
    assert_eq!(
        violated(&report),
        [] as [&str; 0],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn a_file_outside_the_tables_files_is_not_judged() {
    let repo = slice_repo().file(
        "apps/api/src/board/application/draft.entity.ts",
        "import './save.use-case';\n",
    );
    let report = report_of(&slice(), &repo);
    assert_eq!(
        violated(&report),
        [] as [&str; 0],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn the_default_files_of_a_table_are_the_files_its_kind_pattern_matches() {
    let repo = small_repo().file("src/features/cart/index.ts", "import '../../main';\n");
    let report = report_of(&small_app(), &repo);
    assert_eq!(
        of(&report, "feature-code.views").len(),
        0,
        "the default files of a table are the files its kind pattern matches"
    );
    assert_eq!(
        violated(&report)
            .into_iter()
            .filter(|rule| rule.starts_with("feature-code"))
            .count(),
        0,
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn a_table_with_an_area_judges_only_targets_in_the_area() {
    let repo = slice_repo().file(
        "apps/api/src/board/domain/board.entity.ts",
        "import '../../../../../packages/contract/src/canvas';\nimport '../application/save.use-case';\n",
    );
    let mut config = slice();
    config["rules"][0] = slice_table(&json!({ "within": "api" }));
    let report = report_of(&config, &repo);
    let found = of(&report, "domain.entity");
    assert_eq!(found.len(), 1, "{:?}", report.diagnostics);
    assert_eq!(
        found[0].specifier.as_deref(),
        Some("../application/save.use-case"),
        "the import outside the area is not judged"
    );
}

#[test]
fn the_default_files_of_a_table_of_parts_are_the_files_of_its_rows() {
    let repo = FixtureRepo::new()
        .file("apps/web/a.ts", "")
        .file("apps/api/b.ts", "")
        .file("scripts/seed.ts", "import '../apps/api/b';\n");
    let config = json!({
        "parts": { "web": "apps/web/**", "api": "apps/api/**" },
        "rules": [table("t", &json!({ "web": ["api"], "api": [] }), &json!({}))],
    });
    let report = report_of(&config, &repo);
    assert_eq!(
        violated(&report),
        [] as [&str; 0],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn a_folder_kind_may_hold_a_dot() {
    let repo = small_repo().file("src/features/cart/api.v2/fetch-cart.ts", "");
    let report = report_of(&small_app(), &repo);
    let message = message_of(&report, "feature-code.every-file-has-a-kind");
    assert!(message.contains("no row for api.v2"), "{message}");
}

#[test]
fn a_row_without_within_judges_every_repository_file_and_no_unnamed_package() {
    let repo = with_packages(
        small_repo().file(
            "src/features/cart/views/cart-page.tsx",
            "import 'react';\nimport '../../../main';\n",
        ),
        &["react"],
    );
    let report = report_of(&small_app(), &repo);
    let found = of(&report, "feature-code.views");
    assert_eq!(found.len(), 1, "{:?}", report.diagnostics);
    assert_eq!(
        found[0].specifier.as_deref(),
        Some("../../../main"),
        "a row without within judges every repository file and no unnamed package"
    );
}

#[test]
fn a_file_with_no_kind_is_refused_by_default() {
    let repo = slice_repo().file("apps/api/src/board/domain/board.ts", "");
    let report = report_of(&slice(), &repo);
    let message = message_of(&report, "domain.every-file-has-a-kind");
    assert!(
        message.contains("board.ts")
            && message.contains("no kind")
            && message.contains("**/*.{kind}.ts"),
        "{message}"
    );
}

#[test]
fn a_file_whose_kind_has_no_row_is_refused() {
    let repo = small_repo().file("src/features/cart/hooks/use-cart.ts", "");
    let report = report_of(&small_app(), &repo);
    let message = message_of(&report, "feature-code.every-file-has-a-kind");
    assert!(message.contains("no row for hooks"), "{message}");
    assert!(
        message.contains("add a row")
            && message.contains("rename the file")
            && message.contains("`not`"),
        "{message}"
    );
}

#[test]
fn a_file_whose_kind_has_no_row_names_the_closest_row() {
    let repo = FixtureRepo::new()
        .file("apps/api/src/board/domain/board.aggregate-root.ts", "")
        .file("apps/api/src/board/domain/a.aggregate-rot.ts", "");
    let config = json!({
        "rules": [table(
            "domain",
            &json!({ "aggregate-rot": [] }),
            &json!({ "files": "apps/api/src/*/domain/**", "kinds": "**/*.{kind}.ts" }),
        )],
        "parts": { "api": "apps/api/**" },
    });
    let report = report_of(&config, &repo);
    let message = message_of(&report, "domain.every-file-has-a-kind");
    assert!(message.contains("aggregate-rot"), "{message}");
}

#[test]
fn files_left_out_of_a_table_need_no_kind() {
    let repo = FixtureRepo::new()
        .file("apps/api/src/board/domain/index.ts", "")
        .file("apps/api/src/board/domain/board.entity.test.ts", "")
        .file("apps/api/src/board/domain/env.d.ts", "")
        .file("apps/api/src/board/domain/board.entity.ts", "");
    let config = json!({
        "parts": { "api": "apps/api/**" },
        "groups": { "tests": "**/*.test.ts" },
        "rules": [table(
            "domain",
            &json!({ "entity": [] }),
            &json!({
                "files": { "all": ["apps/api/src/*/domain/**", { "not": ["tests", "**/index.ts", "**/*.d.ts"] }] },
                "kinds": "**/*.{kind}.ts",
            }),
        )],
    });
    let report = report_of(&config, &repo);
    assert_eq!(
        violated(&report),
        [] as [&str; 0],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn a_file_with_no_kind_is_outside_a_table_without_files() {
    let repo = FixtureRepo::new()
        .file("apps/api/src/board/application/save.use-case.ts", "")
        .file("apps/api/src/board/domain/a.entity.ts", "")
        .file(
            "apps/api/src/board/domain/board.ts",
            "import '../application/save.use-case';\n",
        );
    let config = json!({
        "parts": { "api": "apps/api/**" },
        "rules": [table("t", &json!({ "entity": [] }), &json!({ "kinds": "**/*.{kind}.ts" }))],
    });
    let report = report_of(&config, &repo);
    let judged: Vec<_> = violated(&report)
        .into_iter()
        .filter(|rule| rule != "t.every-file-has-a-kind")
        .collect();
    assert_eq!(judged, [] as [&str; 0], "{:?}", report.diagnostics);
    assert!(
        of(&report, "t.every-file-has-a-kind")
            .iter()
            .all(|d| d.file.as_deref() != Some("apps/api/src/board/domain/board.ts")),
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn a_file_in_two_rows_of_a_table_is_an_error() {
    let repo = FixtureRepo::new()
        .file("src/features/cart/cart.model.ts", "")
        .file("src/x.model.ts", "");
    let config = json!({
        "parts": { "src": "src/**" },
        "groups": { "model": "src/**/*.model.ts", "feature": "src/features/**" },
        "rules": [table("t", &json!({ "model": [], "feature": [] }), &json!({}))],
    });
    let report = report_of(&config, &repo);
    let found: Vec<&Diagnostic> = report
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str() == "table-row-overlap")
        .collect();
    assert_eq!(found.len(), 1, "{:?}", report.diagnostics);
    let message = &found[0].message;
    assert_eq!(
        found[0].file.as_deref(),
        Some("src/features/cart/cart.model.ts"),
        "a file in two rows of a table is an error"
    );
    for word in ["model", "feature", "`not`", "table's files", "baseline"] {
        assert!(message.contains(word), "{word}: {message}");
    }
}

#[test]
fn a_row_key_that_is_a_group_means_the_group() {
    let repo = FixtureRepo::new()
        .file("apps/api/src/shared/types/a.ts", "")
        .file("apps/api/src/board/domain/id.types.ts", "");
    let config = json!({
        "parts": { "api": "apps/api/**" },
        "groups": { "types": "apps/api/src/shared/types/**" },
        "rules": [table(
            "t",
            &json!({ "types": [] }),
            &json!({ "files": "apps/api/src/board/domain/**", "kinds": "**/*.{kind}.ts" }),
        )],
    });
    let report = report_of(&config, &repo);
    let message = message_of(&report, "t.every-file-has-a-kind");
    assert!(
        message.contains("id.types.ts")
            && message.contains("the row `types`")
            && message.contains("apps/api/src/shared/types/**")
            && message.contains("not the kind"),
        "{message}"
    );
}

#[test]
fn a_rule_named_with_a_dot_or_a_space_judges_its_kind_and_takes_an_exception() {
    let repo = FixtureRepo::new()
        .file("src/main.ts", "")
        .file("src/shared/s.ts", "")
        .file("src/features/c/api.v2/a.ts", "import '../../../main';\n")
        .file(
            "src/features/c/my views/b.ts",
            "// archangel-allow feature-code.my views: reason\nimport '../../../main';\n",
        );
    let config = json!({
        "parts": { "shared": "src/shared/**" },
        "rules": [table(
            "feature-code",
            &json!({ "api.v2": ["shared"], "my views": ["shared"] }),
            &json!({ "kinds": "src/features/*/{kind}/**" }),
        )],
    });
    let report = report_of(&config, &repo);
    assert_eq!(
        violated(&report),
        ["feature-code.api.v2"],
        "{:?}",
        report.diagnostics
    );
    assert_eq!(
        report.summary.exceptions.get("feature-code.my views"),
        Some(&1),
        "a rule named with a dot or a space judges its kind and takes an exception"
    );
}

#[test]
fn a_file_in_no_row_of_a_table_of_parts_is_told_the_rows() {
    let repo = FixtureRepo::new()
        .file("src/features/cart/a.ts", "")
        .file("src/shared/b.ts", "")
        .file("src/stray.ts", "");
    let config = json!({
        "parts": { "feature": "src/features/{feature}/**", "shared": "src/shared/**" },
        "rules": [table(
            "app",
            &json!({ "feature": [], "shared": [] }),
            &json!({ "files": "src/**" }),
        )],
    });
    let report = report_of(&config, &repo);
    let found = of(&report, "app.every-file-has-a-kind");
    assert_eq!(found.len(), 1, "{:?}", report.diagnostics);
    assert_eq!(
        found[0].file.as_deref(),
        Some("src/stray.ts"),
        "a file in no row of a table of parts is told the rows"
    );
    let message = &found[0].message;
    for word in ["feature", "shared", "`not`"] {
        assert!(message.contains(word), "{word}: {message}");
    }
}

fn domain_with_two_rows(off: &[&str]) -> Value {
    json!({
        "parts": { "api": "apps/api/**" },
        "off": off,
        "rules": [table(
            "domain",
            &json!({ "entity": [], "policy": [] }),
            &json!({ "files": "apps/api/src/*/domain/**", "kinds": "**/*.{kind}.ts" }),
        )],
    })
}

fn domain_repo() -> FixtureRepo {
    FixtureRepo::new()
        .file("apps/api/src/board/application/save.use-case.ts", "")
        .file(
            "apps/api/src/board/domain/a.entity.ts",
            "import '../application/save.use-case';\n",
        )
        .file(
            "apps/api/src/board/domain/a.policy.ts",
            "import '../application/save.use-case';\n",
        )
        .file("apps/api/src/board/domain/plain.ts", "")
}

#[test]
fn off_drops_one_row_of_a_table() {
    let report = report_of(&domain_with_two_rows(&["domain.policy"]), &domain_repo());
    assert_eq!(
        violated(&report),
        ["domain.entity", "domain.every-file-has-a-kind"],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn a_table_name_in_off_drops_every_rule_of_the_table() {
    let config = domain_with_two_rows(&["domain"]);
    let parsed = archangel_config::parse_json(&config.to_string()).expect("a valid config");
    let mut off: Vec<&str> = parsed.off.iter().map(|rule| rule.name.as_str()).collect();
    off.sort_unstable();
    assert_eq!(
        off,
        [
            "domain.entity",
            "domain.every-file-has-a-kind",
            "domain.policy"
        ],
        "the three are off"
    );
    let report = report_of(&config, &domain_repo());
    assert_eq!(
        violated(&report),
        [] as [&str; 0],
        "{:?}",
        report.diagnostics
    );
    assert_eq!(
        codes(&report),
        [] as [&str; 0],
        "no table-empty for a table that is off"
    );
}

#[test]
fn an_exception_names_a_row_of_a_table() {
    let repo = slice_repo().file(
        "apps/api/src/board/domain/board.entity.ts",
        "// archangel-allow domain.entity: migration in progress\nimport '../application/save.use-case';\n",
    );
    let report = report_of(&slice(), &repo);
    assert_eq!(
        violated(&report),
        [] as [&str; 0],
        "{:?}",
        report.diagnostics
    );
    assert_eq!(
        report.summary.exceptions.get("domain.entity"),
        Some(&1),
        "an exception names a row of a table"
    );
}

#[test]
fn a_row_with_no_file_is_listed_in_the_summary() {
    let repo = slice_repo()
        .file("apps/api/src/board/domain/board.entity.ts", "")
        .file("apps/api/src/board/domain/board.entity.test.ts", "");
    let report = report_of(&slice(), &repo);
    assert_eq!(codes(&report), [] as [&str; 0], "{:?}", report.diagnostics);
    let listed = report.summary.rows_with_no_file.clone().unwrap_or_default();
    assert!(
        listed.contains(&"domain.aggregate-root".to_owned()),
        "{listed:?}"
    );
    assert!(!listed.contains(&"domain.entity".to_owned()), "{listed:?}");
}

#[test]
fn a_row_that_names_an_empty_part_gives_one_diagnostic() {
    let repo = FixtureRepo::new().file("packages/other/a.ts", "");
    let config = json!({
        "parts": { "ui": "packages/uii/**", "other": "packages/other/**" },
        "rules": [table("t", &json!({ "ui": [] }), &json!({}))],
    });
    let report = report_of(&config, &repo);
    assert_eq!(
        codes(&report),
        ["element-empty"],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn a_table_whose_rows_match_no_file_is_table_empty() {
    let repo = FixtureRepo::new().file("apps/api/src/a.entity.ts", "");
    let config = json!({
        "parts": { "api": "apps/api/**" },
        "rules": [table("domain", &json!({ "entity": [] }), &json!({ "kinds": "**/*.{kind}.tz" }))],
    });
    let report = report_of(&config, &repo);
    assert_eq!(codes(&report), ["table-empty"], "{:?}", report.diagnostics);
    let message = &report.diagnostics[0].message;
    for word in [
        "domain",
        "**/*.{kind}.tz",
        "check the pattern",
        "remove the table",
    ] {
        assert!(message.contains(word), "{word}: {message}");
    }
}

#[test]
fn a_table_without_a_row_is_table_empty() {
    let repo = FixtureRepo::new().file("apps/api/src/a.entity.ts", "");
    let config = json!({
        "parts": { "api": "apps/api/**" },
        "rules": [table("domain", &json!({}), &json!({ "kinds": "**/*.{kind}.ts" }))],
    });
    let report = report_of(&config, &repo);
    let found: Vec<&Diagnostic> = report
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str() == "table-empty")
        .collect();
    assert_eq!(found.len(), 1, "{:?}", report.diagnostics);
    let message = &found[0].message;
    for word in ["domain", "no row", "add a row", "remove the table"] {
        assert!(message.contains(word), "{word}: {message}");
    }
}

#[test]
fn files_that_match_nothing_are_selector_empty_not_table_empty() {
    let repo = FixtureRepo::new().file("apps/api/src/board/domain/a.entity.ts", "");
    let config = json!({
        "parts": { "api": "apps/api/**" },
        "rules": [
            table("t", &json!({ "entity": [] }), &json!({
                "files": "apps/api/src/*/domains/**", "kinds": "**/*.{kind}.ts",
            })),
        ],
    });
    let report = report_of(&config, &repo);
    assert_eq!(
        codes(&report),
        ["selector-empty"],
        "{:?}",
        report.diagnostics
    );
    assert!(
        report.diagnostics[0]
            .message
            .contains("apps/api/src/*/domains/**"),
        "files that match nothing are selector empty not table empty"
    );
}

#[test]
fn an_area_that_matches_nothing_is_selector_empty_not_table_empty() {
    let repo = FixtureRepo::new().file("apps/api/src/board/domain/a.entity.ts", "");
    let config = json!({
        "parts": { "api": "apps/api/**" },
        "rules": [
            table("t", &json!({ "entity": [], "policy": [] }), &json!({
                "within": "apps/apii/**", "kinds": "**/*.{kind}.ts",
            })),
        ],
    });
    let report = report_of(&config, &repo);
    assert_eq!(
        codes(&report),
        ["selector-empty"],
        "{:?}",
        report.diagnostics
    );
}

/// The report of a check whose config file is at `config_file`.
fn report_with_config_file(config: &Value, repo: &FixtureRepo, config_file: &str) -> Report {
    let parsed = archangel_config::from_value(config).expect("a valid config");
    let mut work = archangel_graph::Work::default();
    let graph = archangel_graph::ImportGraph::build(
        repo.path(),
        &archangel_graph::GraphOptions::default(),
        &mut work,
    );
    archangel_rules::Architecture::compile(parsed)
        .with_config_file(config_file)
        .check(&graph, None, &mut work)
}

/// The report of a check of a config with a preset, which only a loaded config can have.
fn preset_report(config: &Value, repo: &FixtureRepo) -> Report {
    let parsed = archangel_config::from_value(config).expect("a valid config");
    let mut work = archangel_graph::Work::default();
    let graph = archangel_graph::ImportGraph::build(
        repo.path(),
        &archangel_graph::GraphOptions::default(),
        &mut work,
    );
    archangel_rules::Architecture::compile(parsed).check(&graph, None, &mut work)
}

fn in_a_preset(name: &str, rules: &Value) -> Value {
    json!({
        "extends": [{ "name": name, "package": format!("archangel-plugin-{name}"), "root": "", "rules": rules }],
        "rules": [],
    })
}

#[test]
fn a_preset_that_is_only_tables_and_has_no_file_is_preset_empty() {
    let repo = repo_of(&["src/a.entity.ts"]);
    let config = in_a_preset(
        "team",
        &json!([table(
            "roles",
            &json!({ "entity": [], "policy": [] }),
            &json!({ "kinds": "**/*.{kind}.tz" }),
        )]),
    );
    let report = preset_report(&config, &repo);
    assert!(
        codes(&report).contains(&"preset-empty"),
        "a table is a part of its preset, so an empty one fails: {:?}",
        report.diagnostics
    );
}

#[test]
fn a_preset_whose_table_has_a_file_is_not_preset_empty() {
    let repo = repo_of(&["src/a.entity.ts"]);
    let config = in_a_preset(
        "team",
        &json!([table(
            "roles",
            &json!({ "entity": [], "policy": [] }),
            &json!({ "kinds": "**/*.{kind}.ts" }),
        )]),
    );
    let report = preset_report(&config, &repo);
    assert!(
        !codes(&report).contains(&"preset-empty"),
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn a_table_whose_only_files_are_not_source_files_is_empty() {
    let repo = repo_of(&["docs/guide/a.md", "src/a.ts"]);
    let config = json!({
        "rules": [table("t", &json!({ "guide": [] }), &json!({ "kinds": "docs/{kind}/**" }))],
    });
    let report = report_of(&config, &repo);
    assert!(
        codes(&report).contains(&"table-empty"),
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn a_row_with_only_files_that_are_not_source_files_is_a_row_with_no_file() {
    let repo = repo_of(&["src/domain/a.ts", "src/assets/s.css"]);
    let config = json!({
        "rules": [table(
            "t",
            &json!({ "domain": [], "assets": [] }),
            &json!({ "kinds": "src/{kind}/**" }),
        )],
    });
    let report = report_of(&config, &repo);
    assert_eq!(
        report.summary.rows_with_no_file.as_deref(),
        Some(&["t.assets".to_owned()][..]),
        "only a source file counts"
    );
}

#[test]
fn a_file_that_is_not_source_needs_no_row() {
    let repo = repo_of(&["src/domain/a.ts", "src/docs/r.md", "src/tsconfig.json"]);
    let config = json!({
        "rules": [table(
            "t",
            &json!({ "domain": [] }),
            &json!({ "kinds": "src/{kind}/**", "files": "src/**" }),
        )],
    });
    let report = report_of(&config, &repo);
    assert_eq!(
        violated(&report),
        [] as [&str; 0],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn an_instance_comes_from_the_captures_across_two_patterns_of_a_part() {
    let repo = FixtureRepo::new()
        .file("a/foo/x.ts", "import '../../b/foo/y';\n")
        .file("b/foo/y.ts", "import '../../a/bar/z';\n")
        .file("a/bar/z.ts", "");
    let config = json!({
        "parts": { "f": ["a/{x}/**", "b/{x}/**"] },
        "rules": [table("t", &json!({ "f": [] }), &json!({}))],
    });
    let report = report_of(&config, &repo);
    let found = of(&report, "t.f");
    assert_eq!(found.len(), 1, "{:?}", report.diagnostics);
    assert_eq!(
        found[0].file.as_deref(),
        Some("b/foo/y.ts"),
        "foo in a and foo in b are one instance, and foo and bar are two"
    );
}

#[test]
fn may_only_import_and_the_instance_option_agree_on_an_instance() {
    let repo = FixtureRepo::new()
        .file("a/foo/x.ts", "import '../../b/foo/y';\n")
        .file("b/foo/y.ts", "");
    let config = json!({
        "parts": { "f": ["a/{x}/**", "b/{x}/**"] },
        "rules": [
            { "name": "only", "files": "f", "mayOnlyImport": ["other"] },
            { "name": "different", "files": "f", "mayNotImport": "f", "instance": "different" },
        ],
        "groups": { "other": "a/**" },
    });
    let report = report_of(&config, &repo);
    assert_eq!(
        violated(&report),
        [] as [&str; 0],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn the_config_file_is_never_in_a_table() {
    let repo = FixtureRepo::new()
        .file("src/a.entity.ts", "")
        .file("archangel.config.ts", "import './src/a.entity';\n");
    let config = json!({
        "rules": [table("t", &json!({ "entity": [], "config": [] }), &json!({ "kinds": "**/*.{kind}.ts" }))],
    });
    let report = report_with_config_file(&config, &repo, "archangel.config.ts");
    assert_eq!(
        violated(&report),
        [] as [&str; 0],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn the_config_file_needs_no_row() {
    let repo = FixtureRepo::new()
        .file("src/a.entity.ts", "")
        .file("archangel.config.ts", "");
    let config = json!({
        "rules": [table(
            "t",
            &json!({ "entity": [] }),
            &json!({ "kinds": "**/*.{kind}.ts", "files": "**" }),
        )],
    });
    let report = report_with_config_file(&config, &repo, "archangel.config.ts");
    assert_eq!(
        violated(&report),
        [] as [&str; 0],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn a_file_with_no_row_has_the_code_every_file_has_a_kind() {
    let repo = repo_of(&["src/domain/a.ts", "src/other/b.ts"]);
    let config = json!({
        "rules": [table(
            "t",
            &json!({ "domain": [] }),
            &json!({ "kinds": "src/{kind}/**", "files": "src/**" }),
        )],
    });
    let report = report_of(&config, &repo);
    assert_eq!(
        codes(&report),
        ["every-file-has-a-kind"],
        "{:?}",
        report.diagnostics
    );
    assert!(
        report.diagnostics[0].code.is_about_a_path(),
        "only the baseline can permit it"
    );
}
