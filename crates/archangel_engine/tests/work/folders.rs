//! The cost of folder events: each costs only the work it needs, and a fault control proves each
//! count would catch the costly way.

use archangel_engine::{Change, PathEvent, Session};
use archangel_graph::Work;
use archangel_rules::Code;
use archangel_testkit::FixtureRepo;
use serde_json::json;

use super::support::{config, counts, faults, imports, open, path, under};

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
