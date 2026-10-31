//! Scenarios from the diagnostics spec: archangel offers a fix for an entry violation only after it
//! checks the fix, and checking it parses no file.
#![cfg(test)]

use archangel_engine::Session;
use archangel_rules::{Code, Diagnostic};
use archangel_testkit::FixtureRepo;
use serde_json::{Value, json};

const TSCONFIG: &str =
    r#"{ "compilerOptions": { "baseUrl": ".", "paths": { "@/*": ["src/*"] } } }"#;

fn config(extra_elements: &Value, extra_rules: &[Value]) -> Value {
    let mut elements = extra_elements.as_object().cloned().unwrap_or_default();
    elements.insert("entity".into(), json!("src/entities/{entity}/**"));
    elements.insert("feature".into(), json!("src/features/{feature}/**"));
    let mut rules = vec![
        json!({ "kind": "entry", "name": "entry(entity)", "element": "entity", "files": ["index.ts"] }),
    ];
    rules.extend(extra_rules.iter().cloned());
    json!({ "elements": elements, "rules": rules })
}

fn repo(index: &str, import: &str) -> FixtureRepo {
    FixtureRepo::new()
        .file("tsconfig.json", TSCONFIG)
        .file("src/entities/canvas/index.ts", index)
        .file(
            "src/entities/canvas/model/store.ts",
            "export const store = 1;\n",
        )
        .file(
            "src/entities/canvas/model/index.ts",
            "export const store = 1;\n",
        )
        .file("src/features/login/a.ts", import)
        .json_config(&config(&json!({}), &[]))
}

fn entry(session: &Session) -> &Diagnostic {
    session
        .report()
        .diagnostics
        .iter()
        .find(|d| d.code == Code::Entry && d.file.as_deref() == Some("src/features/login/a.ts"))
        .expect("an entry violation in login/a.ts")
}

fn fix_of(repo: &FixtureRepo) -> (Option<String>, String) {
    let session = Session::open(repo.path(), None).expect("the session opens");
    let found = entry(&session);
    (
        found.fix.as_ref().map(|fix| fix.specifier.clone()),
        found.help.clone().unwrap_or_default(),
    )
}

const DEEP: &str = "import { store } from '@/entities/canvas/model/store';\n";

#[test]
fn entry_fix_points_the_specifier_at_the_entry_file() {
    let repo = repo("export { store } from './model/store';\n", DEEP);
    let session = Session::open(repo.path(), None).expect("the session opens");
    let found = entry(&session);
    let fix = found.fix.as_ref().expect("a fix");
    assert_eq!(fix.specifier, "@/entities/canvas", "the shorter specifier");
    let (start, end) = found.span.expect("the span");
    assert_eq!(
        fix.span,
        (start + 1, end - 1),
        "the fix replaces the text between the quotes, so the quotes stay as they are"
    );
}

#[test]
fn entry_fix_is_not_offered_when_the_entry_does_not_export_the_name() {
    let (fix, help) = fix_of(&repo("export const other = 1;\n", DEEP));
    assert_eq!(fix, None, "no fix");
    assert!(
        help.contains("does not export") && help.contains("store"),
        "the help says why: {help}"
    );
}

#[test]
fn entry_fix_follows_export_star_chains() {
    let (fix, help) = fix_of(&repo(
        "export * from './model';\n",
        "import { store } from '@/entities/canvas/model';\n",
    ));
    assert_eq!(
        fix.as_deref(),
        Some("@/entities/canvas"),
        "index.ts re-exports store through model/index.ts: {help}"
    );
}

#[test]
fn entry_fix_is_not_offered_when_it_adds_a_violation() {
    let repo = repo("export { store } from './model/store';\n", DEEP).json_config(&config(
        &json!({ "facade": "src/entities/*/index.ts" }),
        &[json!({ "name": "no-facades", "files": "feature", "mayNotImport": "facade" })],
    ));
    let (fix, help) = fix_of(&repo);
    assert_eq!(fix, None, "the entry file would break no-facades: {help}");
    assert!(
        help.contains("no-facades"),
        "the help names the rule the fix would break: {help}"
    );
}

#[test]
fn entry_fix_is_not_offered_when_no_specifier_resolves() {
    let repo = repo(
        "export { store } from './model/store';\n",
        "import { store } from '../../entities/canvas/model/store';\n",
    )
    .file("tsconfig.json", "{}");
    let shortened = fix_of(&repo);
    assert_eq!(
        shortened.0.as_deref(),
        Some("../../entities/canvas"),
        "a relative specifier shortens too"
    );
    let repo = FixtureRepo::new()
        .file("src/entities/canvas/index.ts", "export { store } from './model/store';\n")
        .file("src/entities/canvas/model/store.ts", "export const store = 1;\n")
        .file("src/features/login/a.ts", "import { store } from '#canvas-store';\n")
        .file("package.json", r##"{ "name": "app", "imports": { "#canvas-store": "./src/entities/canvas/model/store.ts" } }"##)
        .json_config(&config(&json!({}), &[]));
    let (fix, help) = fix_of(&repo);
    assert_eq!(fix, None, "no shorter specifier resolves to the entry file");
    assert!(
        help.contains("src/entities/canvas/index.ts"),
        "the help names the entry file: {help}"
    );
}

/// 30 source files, 10 of which reach past the entry file.
fn ten_violations() -> FixtureRepo {
    let repo = repo("export { store } from './model/store';\n", "");
    (0..26).fold(repo, |repo, i| {
        let text = if i < 10 { DEEP } else { "" };
        repo.file(&format!("src/features/f{i}/a.ts"), text)
    })
}

fn files_parsed() -> (u64, usize) {
    let repo = ten_violations();
    let session = Session::open(repo.path(), None).expect("the session opens");
    let fixes = session
        .report()
        .diagnostics
        .iter()
        .filter(|d| d.fix.is_some())
        .count();
    (session.work().files_parsed, fixes)
}

#[test]
fn fixes_parse_no_file_again() {
    #[cfg(feature = "failpoints")]
    let _quiet = fail::FailScenario::setup();
    assert_eq!(
        files_parsed(),
        (30, 10),
        "(files parsed, fixes) for 30 files and 10 fixes"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn fix_work_count_rejects_a_parse_per_fix() {
    let scenario = fail::FailScenario::setup();
    fail::cfg("engine::reparse_per_fix", "return").unwrap();
    let (parsed, _) = files_parsed();
    scenario.teardown();
    assert_eq!(
        parsed, 40,
        "the count catches a parse for each of the 10 entry violations"
    );
}

#[test]
fn entry_fix_is_not_offered_through_a_named_reexport_of_other_names() {
    let (fix, help) = fix_of(
        &repo("export { other } from './model/store';\n", DEEP).file(
            "src/entities/canvas/model/store.ts",
            "export const store = 1;\nexport const other = 2;\n",
        ),
    );
    assert_eq!(fix, None, "index.ts passes on `other` only");
    assert!(
        help.contains("does not export") && help.contains("store"),
        "the help says why: {help}"
    );
}

#[test]
fn entry_fix_is_not_offered_when_the_entry_imports_the_name_without_exporting_it() {
    let (fix, help) = fix_of(&repo(
        "import { store } from './model/store';\nexport const size = store;\n",
        DEEP,
    ));
    assert_eq!(fix, None, "index.ts uses `store` but does not export it");
    assert!(
        help.contains("does not export") && help.contains("store"),
        "the help says why: {help}"
    );
}

#[test]
fn entry_fix_can_be_a_specifier_of_one_segment() {
    let repo = FixtureRepo::new()
        .file(
            "src/entities/canvas/index.ts",
            "export { store } from './model/store';\n",
        )
        .file(
            "src/entities/canvas/model/store.ts",
            "export const store = 1;\n",
        )
        .file(
            "src/features/login/a.ts",
            "import { store } from '#canvas/model/store';\n",
        )
        .file(
            "package.json",
            r##"{ "name": "app", "imports": {
                "#canvas": "./src/entities/canvas/index.ts",
                "#canvas/*": "./src/entities/canvas/*.ts"
            } }"##,
        )
        .json_config(&config(&json!({}), &[]));
    let (fix, help) = fix_of(&repo);
    assert_eq!(
        fix.as_deref(),
        Some("#canvas"),
        "the subpath import of the entry: {help}"
    );
}

#[test]
fn entry_fix_never_imports_a_folder_that_holds_the_importing_file() {
    let repo = repo("export { store } from './model/store';\n", "")
        .file(
            "src/entities/canvas/ui/a.ts",
            "import { store } from '../model/store';\n",
        )
        .json_config(&config(
            &json!({ "canvas-ui": "src/entities/*/ui/**" }),
            &[],
        ));
    let session = Session::open(repo.path(), None).expect("the session opens");
    let found = session
        .report()
        .diagnostics
        .iter()
        .find(|d| d.code == Code::Entry && d.file.as_deref() == Some("src/entities/canvas/ui/a.ts"))
        .expect("an entry violation in ui/a.ts, which is in another element");
    assert_eq!(
        found.fix.as_ref().map(|fix| fix.specifier.as_str()),
        None,
        "`..` reaches the entry file, but importing the barrel of the importer's own folder is the \
         classic way into a cycle"
    );
}
