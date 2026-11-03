//! Scenarios from the import-tables spec: the options of a table apply to each row, as on any rule.
#![cfg(test)]

#[path = "support/tables.rs"]
mod support;

use archangel_testkit::{FixtureRepo, table};
use serde_json::json;
use support::{of, report_of, slice, slice_repo, slice_table, violated, with_packages};

#[test]
fn the_import_kinds_of_a_table_apply_to_each_row() {
    let repo = slice_repo().file(
        "apps/api/src/board/domain/board.entity.ts",
        "import type { S } from '../application/save.use-case';\n",
    );
    let mut config = slice();
    config["rules"][0] = slice_table(&json!({ "imports": "runtime" }));
    let report = report_of(&config, &repo);
    assert_eq!(
        violated(&report),
        [] as [&str; 0],
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn the_reason_and_link_of_a_table_apply_to_each_row() {
    let repo = slice_repo().file(
        "apps/api/src/board/domain/board.entity.ts",
        "import '../application/save.use-case';\n",
    );
    let mut config = slice();
    config["rules"][0] = slice_table(&json!({
        "because": "Domain stays pure", "link": "https://example.test/adr/7",
    }));
    let report = report_of(&config, &repo);
    let found = of(&report, "domain.entity");
    assert_eq!(
        found[0].reason.as_deref(),
        Some("Domain stays pure"),
        "the reason and link of a table apply to each row"
    );
    assert_eq!(
        found[0].link.as_deref(),
        Some("https://example.test/adr/7"),
        "the reason and link of a table apply to each row"
    );
}

#[test]
fn the_import_limits_of_a_table_apply_to_each_row() {
    let repo = slice_repo()
        .file(
            "tsconfig.json",
            r#"{ "compilerOptions": { "baseUrl": ".", "paths": { "@api/*": ["apps/api/src/*"] } } }"#,
        )
        .file("apps/api/src/board/application/load.use-case.ts", "")
        .file(
            "apps/api/src/board/domain/board.entity.ts",
            "import '../application/save.use-case';\nimport '@api/board/application/load.use-case';\n",
        );
    let mut config = slice();
    config["rules"][0] = slice_table(&json!({ "specifier": "parent" }));
    let report = report_of(&config, &repo);
    let found = of(&report, "domain.entity");
    assert_eq!(found.len(), 1, "{:?}", report.diagnostics);
    assert_eq!(
        found[0].specifier.as_deref(),
        Some("../application/save.use-case"),
        "the import limits of a table apply to each row"
    );
}

#[test]
fn the_names_and_dependency_of_a_table_apply_to_each_row() {
    let repo = slice_repo().file(
        "apps/api/src/board/domain/board.entity.ts",
        "import { save } from '../application/save.use-case';\n",
    );
    let mut config = slice();
    config["rules"][0] = slice_table(&json!({ "names": ["default"] }));
    let report = report_of(&config, &repo);
    assert_eq!(
        of(&report, "domain.entity").len(),
        0,
        "{:?}",
        report.diagnostics
    );

    let repo = with_packages(
        FixtureRepo::new()
            .file(
                "package.json",
                r#"{ "name": "app", "dependencies": { "zod": "1.0.0" }, "devDependencies": { "vitest": "1.0.0" } }"#,
            )
            .file("src/entity/a.ts", "import 'zod';\nimport 'vitest';\n")
            .file("src/infrastructure/b.ts", ""),
        &["zod", "vitest"],
    );
    let config = json!({
        "parts": { "src": "src/**" },
        "groups": { "entity": "src/entity/**", "infrastructure": "src/infrastructure/**" },
        "externals": { "tools": ["zod", "vitest"] },
        "rules": [table(
            "t",
            &json!({ "infrastructure": ["tools"], "entity": [] }),
            &json!({ "dependency": "dev" }),
        )],
    });
    let report = report_of(&config, &repo);
    let found = of(&report, "t.entity");
    assert_eq!(found.len(), 1, "{:?}", report.diagnostics);
    assert_eq!(
        found[0].specifier.as_deref(),
        Some("vitest"),
        "the names and dependency of a table apply to each row"
    );
}

#[test]
fn the_instance_of_a_table_applies_to_each_row() {
    let repo = FixtureRepo::new()
        .file("src/shared/s.ts", "")
        .file("src/features/login/models/m.ts", "")
        .file("src/features/cart/models/cart.ts", "")
        .file(
            "src/features/cart/views/v.ts",
            "import '../models/cart';\nimport '../../login/models/m';\n",
        );
    let config = json!({
        "parts": { "feature": "src/features/{feature}/**", "shared": "src/shared/**" },
        "rules": [table(
            "t",
            &json!({ "feature": ["shared"] }),
            &json!({ "instance": "different" }),
        )],
    });
    let report = report_of(&config, &repo);
    let found = of(&report, "t.feature");
    assert_eq!(found.len(), 1, "{:?}", report.diagnostics);
    assert_eq!(
        found[0].specifier.as_deref(),
        Some("../../login/models/m"),
        "the instance of a table applies to each row"
    );
}
