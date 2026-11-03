//! Scenarios from the import-graph spec: the mark of an import follows the tsconfig, so a cycle
//! that runs is reported, and a cycle of imports that the compiler removes is not.
#![cfg(test)]

#[path = "support/check.rs"]
mod support;
#[path = "support/violations.rs"]
mod violations;

use archangel_testkit::FixtureRepo;
use serde_json::json;
use violations::rule_violations;

const WITH_OPTION: &str = r#"{ "compilerOptions": { "verbatimModuleSyntax": true } }"#;
const WITHOUT_OPTION: &str = r#"{ "compilerOptions": { "strict": true } }"#;

fn cycles(repo: &FixtureRepo) -> usize {
    let config = json!({
        "elements": { "src": "src/**" },
        "rules": [{ "name": "no-cycles", "files": "src/**", "noCycles": true }],
    });
    rule_violations(&support::check(&config, repo, None).0).len()
}

#[test]
fn a_cycle_through_an_inline_type_import_runs_under_verbatim_module_syntax() {
    let repo = FixtureRepo::new()
        .file("tsconfig.json", WITH_OPTION)
        .file("src/a.ts", "import { type B } from './b';\n")
        .file("src/b.ts", "import { a } from './a';\n");
    assert_eq!(cycles(&repo), 1, "TypeScript keeps both imports");
}

#[test]
fn a_cycle_through_a_type_re_export_runs_under_verbatim_module_syntax() {
    let files = |tsconfig: &str| {
        FixtureRepo::new()
            .file("tsconfig.json", tsconfig)
            .file("src/index.ts", "export { type B } from './b';\n")
            .file("src/b.ts", "import { a } from './index';\n")
    };
    assert_eq!(cycles(&files(WITH_OPTION)), 1, "the barrel runs");
    assert_eq!(cycles(&files(WITHOUT_OPTION)), 0, "the compiler removes it");
}
