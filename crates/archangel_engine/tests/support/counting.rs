//! Shared by the work-count tests of sessions: configs, repositories, fault switches, and the
//! changes that the structure and export counts measure.
#![cfg(test)]

use archangel_engine::{Change, PathEvent, Session};
use archangel_graph::Work;
use archangel_rules::Code;
use archangel_testkit::FixtureRepo;
use serde_json::{Value, json};

pub(crate) fn config(rules: &[Value]) -> Value {
    json!({
        "elements": { "app": "src/app/**", "feature": "src/features/{feature}/**", "shared": "src/**" },
        "rules": rules,
    })
}

pub(crate) fn layers() -> Value {
    json!({ "kind": "layers", "name": "layers", "order": ["app", "feature", "shared"] })
}

/// 2,000 shared files with up to four imports each, an app file, and `src/a.ts` with 6 imports.
pub(crate) fn big() -> FixtureRepo {
    FixtureRepo::new()
        .synthetic("src/lib", 1998, 4)
        .file("src/app/main.ts", "")
        .file("src/a.ts", &six_imports(""))
        .json_config(&config(&[layers()]))
}

pub(crate) fn six_imports(extra: &str) -> String {
    let imports = ["f0", "f1", "f2", "f3", "f4", "f5"].map(|f| format!("import './lib/{f}';\n"));
    imports.concat() + extra
}

pub(crate) fn open(repo: &FixtureRepo) -> Session {
    Session::open(repo.path(), None).expect("the session opens")
}

pub(crate) fn path(repo: &FixtureRepo, file: &str) -> std::path::PathBuf {
    repo.join(file)
}

/// Holds the fail points for one test, so that no other test's fault reaches it: the fail
/// crate's scenario, or nothing without fail points. Every test that counts work holds it,
/// because fail points are global to the process.
pub(crate) struct Faults {
    #[cfg(feature = "failpoints")]
    _scenario: fail::FailScenario<'static>,
}

pub(crate) fn faults() -> Faults {
    Faults {
        #[cfg(feature = "failpoints")]
        _scenario: fail::FailScenario::setup(),
    }
}

/// Runs `body` with the fail point on, or with none when `point` is `None`. The caller holds
/// [`faults`].
pub(crate) fn under(point: Option<&str>, body: impl FnOnce() -> Work) -> Work {
    #[cfg(feature = "failpoints")]
    if let Some(point) = point {
        fail::cfg(point, "return").unwrap();
    }
    let work = body();
    #[cfg(feature = "failpoints")]
    if let Some(point) = point {
        fail::remove(point);
    }
    #[cfg(not(feature = "failpoints"))]
    let _ = point;
    work
}

/// The rule violations of the session's report, as (code, file).
pub(crate) fn violating(session: &Session) -> Vec<(Code, String)> {
    let rules = session
        .report()
        .diagnostics
        .iter()
        .filter(|d| d.code.is_rule());
    rules
        .map(|d| (d.code, d.file.clone().unwrap_or_default()))
        .collect()
}

/// The number of imports in the repository.
pub(crate) fn imports(session: &Session) -> u64 {
    session.report().summary.imports
}

pub(crate) fn counts(work: &Work) -> (u64, u64) {
    (work.files_parsed, work.resolutions)
}

/// An edit to the text of one file, in a session with naming and mustContain rules.
pub(crate) fn structure_edit(point: Option<&str>) -> (Work, Session) {
    let repo = FixtureRepo::new()
        .synthetic("src/features/login", 20, 2)
        .file("src/features/login/index.ts", "export const login = 1;\n")
        .file(
            "src/features/billing/index.ts",
            "export const billing = 1;\n",
        )
        .json_config(&json!({
            "elements": { "feature": "src/features/{feature}/**" },
            "rules": [
                { "name": "kebab-folders", "folders": "src/**", "mustBeNamed": "kebab-case" },
                { "name": "camel-files", "files": "src/**", "mustBeNamed": "camelCase" },
                { "name": "public-api", "files": "feature", "mustContain": "index.ts" },
            ],
        }));
    let mut session = open(&repo);
    let work = under(point, || {
        session
            .apply(vec![Change::Text {
                path: "src/features/login/f3.ts".into(),
                text: Some("export const edited = 1;\n".into()),
            }])
            .work
    });
    (work, session)
}

/// A session with a folder naming rule on 500 folders, and the work of creating one file in one
/// of them.
pub(crate) fn new_file(point: Option<&str>) -> (Work, Session) {
    let repo = (0..500)
        .fold(FixtureRepo::new(), |repo, i| {
            repo.file(&format!("src/f{i}/a.ts"), "")
        })
        .json_config(&json!({
            "elements": { "src": "src/**" },
            "rules": [{ "name": "kebab-folders", "folders": "src/**", "mustBeNamed": "kebab-case" }],
        }));
    let mut session = open(&repo);
    let work = under(point, || {
        repo.write("src/f7/b.ts", "");
        session
            .apply(vec![Change::Paths(vec![PathEvent::Created(path(
                &repo,
                "src/f7/b.ts",
            ))])])
            .work
    });
    (work, session)
}

/// An edit to `src/a.ts`, which a barrel re-exports through `export *`, in a session with export
/// rules on 1,000 files.
pub(crate) fn export_edit(point: Option<&str>) -> (Work, Session) {
    let repo = FixtureRepo::new()
        .synthetic("src/lib", 998, 2)
        .file("src/a.ts", "export const a = 1;\n")
        .file("src/index.ts", "export * from './a';\n")
        .json_config(&json!({
            "elements": { "src": "src/**" },
            "rules": [{ "name": "no-default", "files": "src/**", "mayNotExport": "default" }],
        }));
    let mut session = open(&repo);
    let work = under(point, || {
        session
            .apply(vec![Change::Text {
                path: "src/a.ts".into(),
                text: Some("export const a = 2;\nexport default a;\n".into()),
            }])
            .work
    });
    (work, session)
}
