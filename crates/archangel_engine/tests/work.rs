//! Scenarios from the language-server and import-graph specs: each kind of change costs only the
//! work it needs, and a fault control proves each count would catch the costly way.
#![cfg(test)]

#[path = "support/counting.rs"]
mod support;

use archangel_engine::{Applied, Change, PathEvent, Session};
use archangel_graph::Work;
use archangel_rules::Code;
use archangel_testkit::FixtureRepo;
use serde_json::json;
use support::{
    big, config, counts, export_edit, faults, imports, layers, new_file, open, path, six_imports,
    structure_edit, under, violating,
};

// One edit.

/// The work of the edit, and of the save after it.
fn edit_and_save(point: Option<&str>) -> (Work, Work) {
    let repo = big();
    let mut session = open(&repo);
    let mut saved = Work::default();
    let edited = under(point, || {
        let text = six_imports("export const edited = 1;\n");
        let edited = session.apply(vec![Change::Text {
            path: "src/a.ts".into(),
            text: Some(text.clone()),
        }]);
        repo.write("src/a.ts", &text);
        saved = session
            .apply(vec![Change::Paths(vec![PathEvent::Changed(path(
                &repo, "src/a.ts",
            ))])])
            .work;
        edited.work
    });
    (edited, saved)
}

#[test]
fn one_edit_parses_one_file_and_resolves_its_imports() {
    let _faults = faults();
    let (mut work, saved) = edit_and_save(None);
    work.add(&saved);
    assert_eq!(
        (work.files_parsed, work.resolutions, work.rule_evaluations),
        (1, 6, 0),
        "one parse, six resolutions, and the evaluations of six shared-to-shared imports, which no rule cell holds"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn edit_work_count_rejects_a_full_check_per_edit() {
    let _faults = faults();
    let (edited, _) = edit_and_save(Some("engine::full_check_per_edit"));
    assert_ne!(
        edited.files_parsed, 1,
        "the edit's count catches a full check"
    );
}

/// An edit to a.ts, in a repository where `src/x.ts` has an import that a rule cell holds, so an
/// evaluation of every file would cost evaluations that the edit does not.
fn evaluated_edit(point: Option<&str>) -> Work {
    let repo = big()
        .file("src/x.ts", "import './app/main';\n")
        .json_config(&config(&[
            layers(),
            json!({ "name": "f", "files": "shared", "mayNotImport": "app" }),
        ]));
    let mut session = open(&repo);
    under(point, || {
        session
            .apply(vec![Change::Text {
                path: "src/a.ts".into(),
                text: Some(six_imports("")),
            }])
            .work
    })
}

#[test]
fn edit_evaluates_only_the_edited_files_imports() {
    let _faults = faults();
    let work = evaluated_edit(None);
    assert_eq!(
        work.rule_evaluations, 0,
        "a.ts's six shared imports are in no cell, and main.ts is not evaluated again"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn edit_work_count_rejects_an_evaluation_of_every_file() {
    let _faults = faults();
    let work = evaluated_edit(Some("engine::evaluate_every_file_per_edit"));
    assert_ne!(
        work.rule_evaluations, 0,
        "the count catches an evaluation of every file"
    );
}

// File events.

fn deletion(point: Option<&str>) -> (Work, bool) {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "import './b';\n")
        .file("src/b.ts", "")
        .json_config(&config(&[json!({ "files": "src/**", "noCycles": true })]));
    let mut session = open(&repo);
    let work = under(point, || {
        repo.remove("src/b.ts");
        session
            .apply(vec![Change::Paths(vec![PathEvent::Deleted(path(
                &repo, "src/b.ts",
            ))])])
            .work
    });
    let unresolved = session
        .report()
        .diagnostics
        .iter()
        .any(|d| d.code == Code::ImportUnresolved && d.file.as_deref() == Some("src/a.ts"));
    (work, unresolved)
}

#[test]
fn file_deletion_parses_no_file() {
    let _faults = faults();
    let (work, unresolved) = deletion(None);
    assert!(unresolved, "a.ts's import of ./b is unresolved now");
    assert_eq!(work.files_parsed, 0, "no file is parsed");
}

#[cfg(feature = "failpoints")]
#[test]
fn file_event_work_count_rejects_a_full_check_per_event() {
    let _faults = faults();
    let (work, _) = deletion(Some("engine::full_check_per_event"));
    assert_ne!(work.files_parsed, 0, "the count catches a full check");
}

fn rename_save(point: Option<&str>) -> (Work, u64) {
    let repo = big();
    let mut session = open(&repo);
    let work = under(point, || {
        repo.write("src/.a.ts.swp", &six_imports("export const x = 1;\n"));
        repo.rename("src/.a.ts.swp", "src/a.ts");
        let temp = path(&repo, "src/.a.ts.swp");
        session
            .apply(vec![Change::Paths(vec![
                PathEvent::Created(temp.clone()),
                PathEvent::Renamed {
                    from: temp,
                    to: path(&repo, "src/a.ts"),
                },
            ])])
            .work
    });
    (work, imports(&session))
}

#[test]
fn save_by_rename_is_a_text_change() {
    let _faults = faults();
    assert_eq!(
        counts(&rename_save(None).0),
        (1, 6),
        "(parsed, resolutions) for one file with six imports"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn rename_save_work_count_rejects_events_that_are_not_netted() {
    let _faults = faults();
    let (work, all) = rename_save(Some("engine::skip_netting"));
    assert_eq!(
        counts(&work),
        (0, all),
        "the count catches events that are not netted: the rename alone looks like a new file, \
         so every import is resolved again"
    );
}

fn css_save(point: Option<&str>) -> Work {
    let repo = big()
        .file("src/b.tsx", "import './a.module.css';\n")
        .file("src/a.module.css", ".a {}\n");
    let mut session = open(&repo);
    under(point, || {
        repo.write("src/a.module.css", ".a { color: red }\n");
        session
            .apply(vec![Change::Paths(vec![PathEvent::Changed(path(
                &repo,
                "src/a.module.css",
            ))])])
            .work
    })
}

#[test]
fn save_to_an_imported_css_file_costs_nothing() {
    let _faults = faults();
    assert_eq!(
        counts(&css_save(None)),
        (0, 0),
        "(parsed, resolutions) for a change to a found file's text"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn css_save_work_count_rejects_a_resolution_per_found_file() {
    let _faults = faults();
    assert_ne!(
        css_save(Some("engine::resolve_per_found_file")).resolutions,
        0,
        "the count catches a resolution"
    );
}

fn many_creations(point: Option<&str>) -> (Work, u64) {
    let repo = big();
    let mut session = open(&repo);
    let work = under(point, || {
        let batches: Vec<Change> = (0..5)
            .map(|batch| {
                let events = (0..100)
                    .map(|i| {
                        let file = format!("src/new/n{batch}_{i}.ts");
                        repo.write(&file, "import '../lib/f1';\n");
                        PathEvent::Created(path(&repo, &file))
                    })
                    .collect();
                Change::Paths(events)
            })
            .collect();
        session.apply(batches).work
    });
    (work, imports(&session))
}

#[test]
fn many_file_events_resolve_all_imports_one_time() {
    let _faults = faults();
    let (work, all) = many_creations(None);
    assert_eq!(
        counts(&work),
        (500, all),
        "500 parsed, and every import, the new ones included, resolved once"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn file_event_work_count_rejects_a_resolution_per_event() {
    let _faults = faults();
    let (work, all) = many_creations(Some("engine::resolve_per_event"));
    assert_ne!(
        work.resolutions, all,
        "the count catches a resolution per event"
    );
}

// Resolution inputs.

fn alias_repo() -> FixtureRepo {
    FixtureRepo::new()
        .file(
            "tsconfig.json",
            r#"{ "compilerOptions": { "baseUrl": ".", "paths": { "@x/*": ["src/one/*"] } } }"#,
        )
        .file("src/one/a.ts", "")
        .file("src/two/a.ts", "")
        .file("src/b.ts", "import '@x/a';\n")
        .json_config(&config(&[json!({ "files": "src/**", "noCycles": true })]))
}

#[test]
fn tsconfig_change_parses_nothing_and_resolves_every_import_once() {
    let _faults = faults();
    let repo = alias_repo();
    let mut session = open(&repo);
    repo.write(
        "tsconfig.json",
        r#"{ "compilerOptions": { "baseUrl": ".", "paths": { "@x/*": ["src/two/*"] } } }"#,
    );
    let applied = session.apply(vec![Change::Paths(vec![PathEvent::Changed(path(
        &repo,
        "tsconfig.json",
    ))])]);
    assert_eq!(
        counts(&applied.work),
        (0, imports(&session)),
        "(parsed, resolutions)"
    );
    let to = session.graph().resolutions("src/b.ts").map(<[_]>::to_vec);
    assert_eq!(
        to,
        Some(vec![archangel_graph::Resolution::File(
            "src/two/a.ts".into()
        )]),
        "the alias points at the new folder"
    );
}

/// A workspace of 40 packages, each with a manifest the resolver reads for an import of it.
#[cfg(unix)]
fn workspace() -> FixtureRepo {
    (0..40)
        .fold(
            FixtureRepo::new().file(".gitignore", "node_modules/\n"),
            |repo, i| {
                repo.file(
                    &format!("packages/p{i}/package.json"),
                    &format!(r#"{{ "name": "p{i}", "main": "index.js" }}"#),
                )
                .file(&format!("packages/p{i}/index.js"), "")
                .symlink(&format!("node_modules/p{i}"), &format!("../packages/p{i}"))
                .file(
                    &format!("src/u{i}.ts"),
                    &format!("import 'p{i}';\nimport './u{}';\n", (i + 1) % 40),
                )
            },
        )
        .json_config(&config(&[
            json!({ "kind": "layers", "order": ["app", "shared"] }),
        ]))
}

#[cfg(unix)]
fn manifest_changes(point: Option<&str>) -> (Work, u64) {
    let repo = workspace();
    let mut session = open(&repo);
    let work = under(point, || {
        let batches: Vec<Change> = (0..4)
            .map(|batch| {
                Change::Paths(
                    (0..10)
                        .map(|i| {
                            let file = format!("packages/p{}/package.json", batch * 10 + i);
                            let text = format!(
                                r#"{{ "name": "p{}", "main": "index.js", "version": "2.0.0" }}"#,
                                batch * 10 + i
                            );
                            repo.write(&file, &text);
                            PathEvent::Changed(path(&repo, &file))
                        })
                        .collect(),
                )
            })
            .collect();
        session.apply(batches).work
    });
    (work, imports(&session))
}

#[cfg(unix)]
#[test]
fn many_manifest_changes_resolve_all_imports_one_time() {
    let _faults = faults();
    let (work, all) = manifest_changes(None);
    assert_eq!(
        counts(&work),
        (0, all),
        "nothing parsed, every import resolved once"
    );
}

#[cfg(all(unix, feature = "failpoints"))]
#[test]
fn manifest_change_work_count_rejects_a_resolution_per_event() {
    let _faults = faults();
    let (work, all) = manifest_changes(Some("engine::resolve_per_event"));
    assert_ne!(
        work.resolutions, all,
        "the count catches a resolution per event"
    );
}

fn unread_manifest(point: Option<&str>) -> Work {
    let repo = FixtureRepo::new()
        .file("packages/web/package.json", r#"{ "name": "web" }"#)
        .file("packages/web/src/a.ts", "")
        .file("src/c.ts", "")
        .file("src/d.ts", "import './c';\n")
        .json_config(&json!({
            "elements": { "web": "packages/web/**", "react": { "packages": ["react"] } },
            "rules": [{ "files": "web", "mayNotImport": "react" }],
        }));
    let mut session = open(&repo);
    let empty = |s: &Session| {
        s.report()
            .diagnostics
            .iter()
            .any(|d| d.code == Code::ElementEmpty && d.message.contains("react"))
    };
    assert!(empty(&session), "react is element-empty before the change");
    let work = under(point, || {
        repo.write(
            "packages/web/package.json",
            r#"{ "name": "web", "dependencies": { "react": "19.0.0" } }"#,
        );
        session
            .apply(vec![Change::Paths(vec![PathEvent::Changed(path(
                &repo,
                "packages/web/package.json",
            ))])])
            .work
    });
    assert!(
        !empty(&session),
        "react is listed now, so it is not element-empty"
    );
    work
}

#[test]
fn unread_manifest_change_parses_and_resolves_nothing() {
    let _faults = faults();
    assert_eq!(
        counts(&unread_manifest(None)),
        (0, 0),
        "(parsed, resolutions)"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn unread_manifest_work_count_rejects_a_resolution_per_event() {
    let _faults = faults();
    let work = unread_manifest(Some("engine::resolve_per_event"));
    assert_ne!(
        work.resolutions, 0,
        "the count catches a resolution per event"
    );
}

#[cfg(unix)]
#[test]
fn manifest_read_before_an_edit_still_triggers_a_resolution() {
    let _faults = faults();
    let repo = workspace().file("src/v.ts", "");
    let mut session = open(&repo);
    session.apply(vec![Change::Text {
        path: "src/v.ts".into(),
        text: Some("import 'p3';\n".into()),
    }]);
    repo.write(
        "packages/p3/package.json",
        r#"{ "name": "p3", "exports": "./index.js" }"#,
    );
    let applied = session.apply(vec![Change::Paths(vec![PathEvent::Changed(path(
        &repo,
        "packages/p3/package.json",
    ))])]);
    assert_eq!(
        applied.work.resolutions,
        imports(&session),
        "every import, the edited file's included"
    );
}

#[test]
fn gitignore_change_parses_nothing_and_resolves_once() {
    let _faults = faults();
    let repo = FixtureRepo::new()
        .file("src/app/main.ts", "")
        .file("src/gen/a.ts", "import '../app/main';\n")
        .file("src/b.ts", "import './gen/a';\n")
        .json_config(&config(&[layers()]));
    let mut session = open(&repo);
    let in_gen = |s: &Session| {
        s.report()
            .diagnostics
            .iter()
            .any(|d| d.file.as_deref() == Some("src/gen/a.ts"))
    };
    assert!(in_gen(&session), "the violation is there first");
    let repo = repo.file(".gitignore", "src/gen/\n");
    let applied = session.apply(vec![Change::Paths(vec![PathEvent::Created(path(
        &repo,
        ".gitignore",
    ))])]);
    assert!(!in_gen(&session), "src/gen/a.ts has no diagnostics now");
    assert_eq!(
        counts(&applied.work),
        (0, imports(&session)),
        "(parsed, resolutions)"
    );
}

// The baseline and the config.

fn three_violations() -> FixtureRepo {
    FixtureRepo::new()
        .file("src/app/main.ts", "")
        .file("src/x.ts", "import './app/main';\n")
        .file("src/y.ts", "import './app/main';\n")
        .file("src/z.ts", "import './app/main';\n")
        .json_config(&config(&[layers()]))
}

fn baseline_change(point: Option<&str>) -> (Work, Applied, usize) {
    let repo = three_violations();
    let mut session = open(&repo);
    let (work, applied) = {
        let mut kept = None;
        let work = under(point, || {
            session.baseline_add().unwrap();
            let applied = session.apply(vec![Change::Paths(vec![PathEvent::Created(path(
                &repo,
                "archangel.baseline.json",
            ))])]);
            let work = applied.work;
            kept = Some(applied);
            work
        });
        (work, kept.unwrap())
    };
    (work, applied, violating(&session).len())
}

#[test]
fn baseline_change_applies_without_a_check() {
    let _faults = faults();
    let (work, applied, left) = baseline_change(None);
    assert_eq!(left, 0, "the three violations are gone");
    assert_eq!(
        applied.changed.len(),
        3,
        "three files changed: {:?}",
        applied.changed
    );
    assert_eq!(
        (work.files_parsed, work.resolutions, work.rule_evaluations),
        (0, 0, 0),
        "no work but the baseline"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn baseline_change_work_count_rejects_a_full_check() {
    let _faults = faults();
    let (work, _, _) = baseline_change(Some("engine::full_check_per_baseline"));
    assert_ne!(work.files_parsed, 0, "the count catches a full check");
}

fn config_change(point: Option<&str>) -> (Work, usize) {
    let repo =
        three_violations().json_config(&config(&[json!({ "files": "src/**", "noCycles": true })]));
    let mut session = open(&repo);
    let work = under(point, || {
        let forbid = config(&[json!({ "name": "f", "files": "shared", "mayNotImport": "app" })]);
        let config = session.config_path().to_path_buf();
        std::fs::write(&config, forbid.to_string()).expect("the config is saved");
        session
            .apply(vec![Change::Paths(vec![PathEvent::Changed(config)])])
            .work
    });
    let found = violating(&session);
    let forbid = found.iter().filter(|(code, _)| *code == Code::MayNotImport);
    (work, forbid.count())
}

#[test]
fn config_change_parses_no_file() {
    let _faults = faults();
    let (work, forbid) = config_change(None);
    assert_eq!(forbid, 3, "the new rule's violations");
    assert_eq!(counts(&work), (0, 0), "(parsed, resolutions)");
}

#[cfg(feature = "failpoints")]
#[test]
fn config_change_work_count_rejects_a_full_check() {
    let _faults = faults();
    let (work, _) = config_change(Some("engine::full_check_per_config"));
    assert_ne!(work.files_parsed, 0, "the count catches a full check");
}

// Documents the graph does not know yet.

/// `src/n.ts`, which imports the app from the shared layer, so a check of it has a diagnostic.
const NEW: &str = "import './app/main';\n";

fn layers_in_new_file(session: &Session) -> bool {
    violating(session).contains(&(Code::Layers, "src/n.ts".to_owned()))
}

/// The work of opening `src/n.ts` in the editor and of its creation on disk, in the order given:
/// `true` writes the file before the editor opens it.
fn open_new(point: Option<&str>, written_first: bool, one_batch: bool) -> (Work, Session) {
    let repo = big();
    let mut session = open(&repo);
    let text = Change::Text {
        path: "src/n.ts".into(),
        text: Some(NEW.into()),
    };
    let created = Change::Paths(vec![PathEvent::Created(path(&repo, "src/n.ts"))]);
    let work = under(point, || {
        let mut work = Work::default();
        if written_first {
            repo.write("src/n.ts", NEW);
        }
        if one_batch {
            work.add(&session.apply(vec![text, created]).work);
        } else {
            work.add(&session.apply(vec![text]).work);
            if !written_first {
                repo.write("src/n.ts", NEW);
            }
            work.add(&session.apply(vec![created]).work);
        }
        work
    });
    (work, session)
}

#[test]
fn a_new_file_opened_with_its_event_is_a_new_source_file() {
    let _faults = faults();
    let (work, session) = open_new(None, true, true);
    assert!(layers_in_new_file(&session), "src/n.ts is checked");
    assert_eq!(
        counts(&work),
        (1, imports(&session)),
        "(parsed, resolutions): the new file parsed once, every import resolved once"
    );
}

#[test]
fn a_new_file_opened_before_its_event_is_a_new_source_file() {
    let _faults = faults();
    let (work, session) = open_new(None, true, false);
    assert!(layers_in_new_file(&session), "src/n.ts is checked");
    assert_eq!(
        counts(&work),
        (1, imports(&session)),
        "(parsed, resolutions): the opening finds the file, and its event later costs nothing"
    );
}

#[test]
fn a_document_opened_before_its_file_is_written_is_checked_once_written() {
    let _faults = faults();
    let (work, session) = open_new(None, false, false);
    assert!(layers_in_new_file(&session), "src/n.ts is checked");
    assert_eq!(
        counts(&work),
        (1, imports(&session)),
        "(parsed, resolutions): nothing for the opening, a new file for the event"
    );
}

#[test]
fn a_file_no_event_reported_is_a_new_source_file_when_the_editor_opens_it() {
    let _faults = faults();
    let repo = big();
    let mut session = open(&repo);
    repo.write("src/n.ts", NEW);
    let applied = session.apply(vec![Change::Text {
        path: "src/n.ts".into(),
        text: Some(NEW.into()),
    }]);
    assert!(layers_in_new_file(&session), "src/n.ts is checked");
    assert_eq!(
        counts(&applied.work),
        (1, imports(&session)),
        "(parsed, resolutions) for a new file"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn new_document_work_count_rejects_a_full_check_per_event() {
    let _faults = faults();
    let (work, _) = open_new(Some("engine::full_check_per_event"), true, true);
    assert_ne!(work.files_parsed, 1, "the count catches a full check");
}

#[test]
fn closing_a_document_whose_file_is_gone_removes_the_file() {
    let _faults = faults();
    let repo = FixtureRepo::new()
        .file("src/a.ts", "import './b';\n")
        .file("src/b.ts", "")
        .json_config(&config(&[json!({ "files": "src/**", "noCycles": true })]));
    let mut session = open(&repo);
    session.apply(vec![Change::Text {
        path: "src/b.ts".into(),
        text: Some("export const b = 1;\n".into()),
    }]);
    repo.remove("src/b.ts");
    session.apply(vec![Change::Paths(vec![PathEvent::Deleted(path(
        &repo, "src/b.ts",
    ))])]);
    let closed = session.apply(vec![Change::Text {
        path: "src/b.ts".into(),
        text: None,
    }]);
    let report = session.report();
    assert_eq!(
        report
            .diagnostics
            .iter()
            .filter(|d| d.code != Code::ElementEmpty)
            .map(|d| (d.code, d.file.clone()))
            .collect::<Vec<_>>(),
        [(Code::ImportUnresolved, Some("src/a.ts".to_owned()))],
        "b.ts is gone, so only a.ts's import of it is a problem"
    );
    assert_eq!(report.summary.files, 1, "one source file is left");
    assert_eq!(
        counts(&closed.work),
        (0, imports(&session)),
        "(parsed, resolutions) for a deletion"
    );
}

// Paths a watcher reports as hints.

fn short_lived(point: Option<&str>) -> Work {
    let repo = big();
    let mut session = open(&repo);
    under(point, || {
        session
            .apply(vec![Change::Paths(vec![PathEvent::Deleted(path(
                &repo,
                "src/.temp.ts",
            ))])])
            .work
    })
}

#[test]
fn a_source_file_the_graph_never_had_costs_nothing_when_it_is_gone() {
    let _faults = faults();
    let work = short_lived(None);
    assert_eq!(
        (work.files_found, work.files_parsed, work.resolutions),
        (0, 0, 0),
        "(found, parsed, resolutions) for a file the graph never had and the disk no longer has"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn short_lived_file_work_count_rejects_a_full_check_per_event() {
    let _faults = faults();
    assert_ne!(
        short_lived(Some("engine::full_check_per_event")).files_parsed,
        0,
        "the count catches a full check"
    );
}

fn folder_repo() -> FixtureRepo {
    FixtureRepo::new()
        .file("src/a.ts", "import './x/b';\n")
        .file("src/x/b.ts", "import './c';\n")
        .file("src/x/c.ts", "")
        .file(".gitignore", "dist/\n")
        .json_config(&config(&[json!({ "files": "src/**", "noCycles": true })]))
}

fn folder_deletion(point: Option<&str>) -> (Work, Session) {
    let repo = folder_repo();
    let mut session = open(&repo);
    let work = under(point, || {
        let folder = repo.join("src/x");
        std::fs::remove_dir_all(&folder).expect("the folder is deleted");
        session
            .apply(vec![Change::Paths(vec![PathEvent::Deleted(path(
                &repo, "src/x",
            ))])])
            .work
    });
    (work, session)
}

#[test]
fn a_folder_deletion_reported_as_the_folder_deletes_its_files() {
    let _faults = faults();
    let (work, session) = folder_deletion(None);
    assert_eq!(session.report().summary.files, 1, "only src/a.ts is left");
    assert!(
        session
            .report()
            .diagnostics
            .iter()
            .any(|d| d.code == Code::ImportUnresolved && d.file.as_deref() == Some("src/a.ts")),
        "a.ts's import of the folder's file is unresolved"
    );
    assert_eq!(
        counts(&work),
        (0, imports(&session)),
        "(parsed, resolutions) for a deletion"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn folder_deletion_work_count_rejects_a_full_check_per_event() {
    let _faults = faults();
    let (work, _) = folder_deletion(Some("engine::full_check_per_event"));
    assert_ne!(work.files_parsed, 0, "the count catches a full check");
}

#[test]
fn a_folder_that_appears_alone_brings_its_files() {
    let _faults = faults();
    let repo = folder_repo();
    let mut session = open(&repo);
    repo.write("src/y/deep/d.ts", "import '../../a';\n");
    repo.write("src/y/deep/e.ts", "");
    let applied = session.apply(vec![Change::Paths(vec![PathEvent::Created(path(
        &repo, "src/y",
    ))])]);
    assert_eq!(
        session.report().summary.files,
        5,
        "the two files, in a folder inside the new one, are new"
    );
    assert_eq!(
        counts(&applied.work),
        (2, imports(&session)),
        "(parsed, resolutions): the two new files, and every import once"
    );
}

#[test]
fn a_folder_that_the_walk_skips_costs_nothing_when_it_appears() {
    let _faults = faults();
    let repo = folder_repo();
    let mut session = open(&repo);
    repo.write("dist/a.js", "import '../src/a';\n");
    repo.write("node_modules/p/index.js", "");
    let applied = session.apply(vec![Change::Paths(vec![
        PathEvent::Created(path(&repo, "dist")),
        PathEvent::Created(path(&repo, "node_modules/p")),
    ])]);
    assert_eq!(
        (
            applied.work.files_found,
            applied.work.files_parsed,
            applied.work.resolutions
        ),
        (0, 0, 0),
        "(found, parsed, resolutions) for an ignored folder and a package"
    );
}

fn delete_folders(repo: &FixtureRepo, folders: &[&str]) {
    for folder in folders {
        std::fs::remove_dir_all(repo.join(folder)).expect("the folder is deleted");
    }
}

/// `src/a.ts` imports a CSS file in `src/styles/`, a folder that holds no source file.
fn styles_repo() -> FixtureRepo {
    FixtureRepo::new()
        .file("src/a.ts", "import './styles/a.css';\n")
        .file("src/b.ts", "import './a';\n")
        .file("src/styles/a.css", ".a {}\n")
        .file("src/stylesheets/b.css", ".b {}\n")
        .file("docs/notes/n.md", "")
        .json_config(&config(&[json!({ "files": "src/**", "noCycles": true })]))
}

#[test]
fn a_folder_deletion_resolves_again_when_a_resolution_found_a_file_in_it() {
    let _faults = faults();
    let repo = styles_repo();
    let mut session = open(&repo);
    delete_folders(&repo, &["src/styles", "docs/notes"]);
    let applied = session.apply(vec![Change::Paths(vec![
        PathEvent::Deleted(path(&repo, "docs/notes")),
        PathEvent::Deleted(path(&repo, "src/styles")),
    ])]);
    assert!(
        session
            .report()
            .diagnostics
            .iter()
            .any(|d| d.code == Code::ImportUnresolved && d.file.as_deref() == Some("src/a.ts")),
        "a.ts's import of the CSS file is unresolved"
    );
    assert_eq!(
        counts(&applied.work),
        (0, imports(&session)),
        "(parsed, resolutions): every import resolved once, nothing parsed"
    );
}

#[test]
fn a_folder_deletion_costs_nothing_when_nothing_depends_on_it() {
    let _faults = faults();
    let repo = styles_repo();
    let mut session = open(&repo);
    delete_folders(&repo, &["src/stylesheets", "docs/notes"]);
    let applied = session.apply(vec![Change::Paths(vec![
        PathEvent::Deleted(path(&repo, "src/stylesheets")),
        PathEvent::Deleted(path(&repo, "docs/notes")),
        PathEvent::Deleted(path(&repo, "docs/notes/n.md")),
    ])]);
    assert_eq!(
        (
            applied.work.files_found,
            applied.work.files_parsed,
            applied.work.resolutions
        ),
        (0, 0, 0),
        "(found, parsed, resolutions) for folders no source file or resolution depends on"
    );
}

#[test]
fn a_late_report_of_a_known_folder_costs_nothing() {
    let _faults = faults();
    let repo = folder_repo();
    let mut session = open(&repo);
    let applied = session.apply(vec![Change::Paths(vec![
        PathEvent::Created(repo.path().to_path_buf()),
        PathEvent::Created(path(&repo, "src")),
        PathEvent::Changed(path(&repo, "src/x")),
    ])]);
    assert_eq!(
        (
            applied.work.files_found,
            applied.work.files_parsed,
            applied.work.resolutions
        ),
        (0, 0, 0),
        "(found, parsed, resolutions) for the config folder and folders the graph already has \
         files below, as macOS reports the creation of a repository's folders after the watch \
         starts"
    );
}

/// An edit to one file of 1,000 that adds an import which breaks the layers rule.
fn classified_edit(point: Option<&str>) -> (Work, bool) {
    let repo = FixtureRepo::new()
        .synthetic("src/lib", 998, 2)
        .file("src/app/main.ts", "")
        .file("src/a.ts", "")
        .json_config(&config(&[layers()]));
    let mut session = open(&repo);
    let work = under(point, || {
        session
            .apply(vec![Change::Text {
                path: "src/a.ts".into(),
                text: Some("import './app/main';\n".into()),
            }])
            .work
    });
    let reported = violating(&session).contains(&(Code::Layers, "src/a.ts".to_owned()));
    (work, reported)
}

#[test]
fn an_edit_classifies_one_file() {
    let _faults = faults();
    let (work, reported) = classified_edit(None);
    assert!(reported, "the edit's new import breaks the layers rule");
    assert_eq!(
        work.files_classified, 1,
        "the edit classifies the edited file only"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn classification_count_rejects_every_file_per_edit() {
    let _faults = faults();
    let (work, _) = classified_edit(Some("engine::classify_every_file_per_edit"));
    assert_ne!(
        work.files_classified, 1,
        "the count catches a classification of every file per edit"
    );
}

#[test]
fn a_manifest_change_checks_only_its_packages_imports() {
    let _faults = faults();
    let repo = FixtureRepo::new()
        .file(".gitignore", "node_modules/\n")
        .file("node_modules/lodash/package.json", r#"{ "name": "lodash", "main": "index.js" }"#)
        .file("node_modules/lodash/index.js", "")
        .file("package.json", r#"{ "name": "root", "dependencies": { "lodash": "1" } }"#)
        .file("packages/a/package.json", r#"{ "name": "a" }"#)
        .file("packages/a/src/x.ts", "import 'lodash';\n")
        .file("packages/b/package.json", r#"{ "name": "b" }"#)
        .file("packages/b/src/y.ts", "import 'lodash';\nimport '../../a/src/x';\n")
        .json_config(&json!({
            "elements": { "packages": "packages/**" },
            "rules": [{ "name": "declared", "files": "packages/**", "mayNotImport": { "packages": ["*"] }, "dependency": "undeclared" }],
        }));
    let mut session = open(&repo);
    let files = |s: &Session| -> Vec<String> { violating(s).into_iter().map(|(_, f)| f).collect() };
    assert_eq!(
        files(&session),
        ["packages/a/src/x.ts", "packages/b/src/y.ts"],
        "neither package declares lodash"
    );
    let manifest = "packages/a/package.json";
    repo.write(manifest, r#"{ "dependencies": { "lodash": "1" } }"#);
    let changed = PathEvent::Changed(path(&repo, manifest));
    let work = session.apply(vec![Change::Paths(vec![changed])]).work;
    assert_eq!(
        files(&session),
        ["packages/b/src/y.ts"],
        "packages/a now declares lodash"
    );
    assert_eq!(
        work.rule_evaluations, 1,
        "the one import under packages/a is checked again, and neither import of packages/b is"
    );
}

// Structure and export rules.

#[test]
fn a_text_edit_evaluates_no_structure_rule() {
    let _faults = faults();
    let (work, session) = structure_edit(None);
    assert_eq!(violating(&session), [], "every name and instance is right");
    assert_eq!(work.files_parsed, 1, "the edit parses its file");
    assert_eq!(
        (work.structure_evaluations, work.folder_evaluations),
        (0, 0),
        "the file's class did not change, so no naming or structure rule runs"
    );
}

#[test]
fn a_new_file_evaluates_only_its_folder() {
    let _faults = faults();
    let (work, session) = new_file(None);
    assert_eq!(violating(&session), [], "every folder is kebab-case");
    assert_eq!(
        session.report().summary.files,
        501,
        "the session has the new file"
    );
    assert_eq!(
        work.folder_evaluations, 1,
        "only src/f7, the folder that changed, is evaluated"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn folder_evaluation_count_rejects_every_folder_per_event() {
    let _faults = faults();
    let (work, _) = new_file(Some("engine::evaluate_every_folder_per_event"));
    assert_ne!(
        work.folder_evaluations, 1,
        "the count catches an evaluation of every folder"
    );
}

#[test]
fn an_edit_evaluates_the_export_rules_of_one_file() {
    let _faults = faults();
    let (work, session) = export_edit(None);
    assert_eq!(
        violating(&session),
        [(Code::MayNotExport, "src/a.ts".to_owned())],
        "the new default export is refused"
    );
    assert_eq!(
        (work.files_parsed, work.export_evaluations),
        (1, 2),
        "one parse, and the export rules of a.ts and of the barrel that re-exports it"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn export_evaluation_count_rejects_every_file_per_edit() {
    let _faults = faults();
    let (work, _) = export_edit(Some("engine::evaluate_exports_of_every_file"));
    assert_ne!(
        work.export_evaluations, 2,
        "the count catches an evaluation of every file's exports"
    );
}
