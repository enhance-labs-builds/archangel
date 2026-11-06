//! Scenarios from the architecture-rules and import-graph specs: package patterns that name a
//! subpath, and `import()` of a template literal, which reaches each file that it can load.
#![cfg(test)]

#[path = "support/sites.rs"]
mod sites;
#[path = "support/check.rs"]
mod support;
#[path = "support/violations.rs"]
mod violations;

use archangel_rules::Report;
use archangel_testkit::FixtureRepo;
use serde_json::{Value, json};
use sites::{site, sites};
use violations::rule_violations;

fn report_with(config: &Value, repo: &FixtureRepo) -> Report {
    support::check(config, repo, None).0
}

fn src_only(rules: &Value) -> Value {
    json!({ "elements": { "src": "src/**" }, "rules": rules })
}

/// A package `utils` with a root entry and a subpath, and a scoped package of tokens.
fn subpath_packages(repo: FixtureRepo) -> FixtureRepo {
    repo.file(
        "node_modules/utils/package.json",
        r#"{ "name": "utils", "main": "index.js" }"#,
    )
    .file("node_modules/utils/index.js", "export const get = 1;\n")
    .file("node_modules/utils/get.js", "export default 1;\n")
    .file(
        "node_modules/@scope/tokens/package.json",
        r#"{ "name": "@scope/tokens", "main": "index.js" }"#,
    )
    .file("node_modules/@scope/tokens/index.js", "")
    .file("node_modules/@scope/tokens/core/a.js", "")
    .file("node_modules/@scope/tokens/base/a.js", "")
    .file(".gitignore", "node_modules/\n")
}

#[test]
fn a_package_pattern_with_a_subpath_bans_deep_imports_only() {
    let repo = subpath_packages(
        FixtureRepo::new()
            .file("src/root.ts", "import { get } from 'utils';\n")
            .file("src/deep.ts", "import get from 'utils/get';\n")
            .file("src/core.ts", "import '@scope/tokens/core/a';\n")
            .file("src/base.ts", "import '@scope/tokens/base/a';\n"),
    );
    let rules = json!([
        { "files": "src/**", "mayNotImport": { "packages": ["utils/*"] } },
        { "files": "src/**", "mayNotImport": { "packages": ["@scope/tokens/core/*"] } },
    ]);
    assert_eq!(
        sites(&report_with(&src_only(&rules), &repo)),
        [
            site("src/core.ts", "@scope/tokens/core/a"),
            site("src/deep.ts", "utils/get"),
        ],
        "a subpath pattern names the subpaths it matches, and not the root or another subtree"
    );
}

#[test]
fn a_package_pattern_without_a_subpath_still_names_every_subpath() {
    let repo = subpath_packages(
        FixtureRepo::new()
            .file("src/root.ts", "import { get } from 'utils';\n")
            .file("src/deep.ts", "import get from 'utils/get';\n"),
    );
    let rules = json!([{ "files": "src/**", "mayNotImport": { "packages": ["utils"] } }]);
    assert_eq!(
        sites(&report_with(&src_only(&rules), &repo)),
        [
            site("src/deep.ts", "utils/get"),
            site("src/root.ts", "utils")
        ],
        "`utils` names the package, its root and each subpath"
    );
}

#[test]
fn a_template_import_is_judged_against_each_file_it_reaches() {
    let repo = FixtureRepo::new()
        .file("src/app/static.ts", "import { secret } from '../features/billing/internal/a';\n")
        .file(
            "src/app/load.ts",
            "export const load = async (name: string) =>\n  import(`../features/billing/internal/${name}.ts`);\n",
        )
        .file("src/features/billing/internal/a.ts", "export const secret = 1;\n")
        .file("src/features/billing/internal/b.ts", "export const other = 1;\n")
        .file("src/features/billing/public/index.ts", "export {};\n");
    let config = json!({
        "parts": { "app": "src/app/**", "feature": "src/features/{feature}/**" },
        "rules": [{ "kind": "entry", "element": "feature", "files": ["public/**"] }],
    });
    let found = report_with(&config, &repo);
    let lines: Vec<(String, u32)> = rule_violations(&found)
        .iter()
        .map(|d| (d.file.clone().unwrap_or_default(), d.line))
        .collect();
    assert_eq!(
        lines,
        [
            ("src/app/load.ts".to_owned(), 2),
            ("src/app/load.ts".to_owned(), 2),
            ("src/app/static.ts".to_owned(), 1),
        ],
        "the template import reaches both internal files, and each is an entry violation: {:?}",
        found.diagnostics
    );
    assert_eq!(
        found.summary.unfollowed,
        [],
        "the template import is followed"
    );
}
