//! Scenarios from the import-graph spec: the files that are not source, which the structure rules
//! read, follow file and folder events without a walk.
#![cfg(test)]

use archangel_engine::{Change, PathEvent, Session};
use archangel_testkit::FixtureRepo;
use serde_json::json;

/// A source file, an image, and a folder that holds one stylesheet and no source file.
fn repo() -> FixtureRepo {
    FixtureRepo::new()
        .file(".gitignore", "dist/\n")
        .file("src/a.ts", "")
        .file("src/logo.png", "")
        .file("src/styles/a.css", ".a {}\n")
        .json_config(&json!({ "elements": { "src": "src/**" }, "rules": [] }))
}

fn open(repo: &FixtureRepo) -> Session {
    Session::open(repo.path(), None).expect("the session opens")
}

fn apply(
    repo: &FixtureRepo,
    session: &mut Session,
    event: fn(std::path::PathBuf) -> PathEvent,
    path: &str,
) {
    session.apply(vec![Change::Paths(vec![event(repo.join(path))])]);
}

#[test]
fn a_folder_deletion_takes_its_other_files_out_of_the_graph() {
    let repo = repo();
    let mut session = open(&repo);
    repo.remove_folder("src/styles");
    apply(&repo, &mut session, PathEvent::Deleted, "src/styles");
    assert_eq!(
        session.graph().others(),
        [".gitignore", "archangel.config.json", "src/logo.png"],
        "the folder's one file is gone, and the files before it stay"
    );
}

#[test]
fn a_folder_that_appears_brings_the_other_files_the_walk_would_visit() {
    let repo = repo();
    let mut session = open(&repo);
    repo.write("src/assets/logo.png", "");
    repo.write("src/assets/dist/skipped.png", "");
    apply(&repo, &mut session, PathEvent::Created, "src/assets");
    let assets: Vec<&str> = session
        .graph()
        .others()
        .iter()
        .map(String::as_str)
        .filter(|other| other.starts_with("src/assets/"))
        .collect();
    assert_eq!(
        assets,
        ["src/assets/logo.png"],
        "the image, and not the one in a folder the walk ignores"
    );
}

#[test]
fn a_new_source_file_is_not_another_file() {
    let repo = repo();
    let mut session = open(&repo);
    repo.write("src/new.ts", "");
    apply(&repo, &mut session, PathEvent::Created, "src/new.ts");
    assert_eq!(
        (
            session.graph().file("src/new.ts").is_some(),
            session.graph().is_other("src/new.ts")
        ),
        (true, false),
        "the graph has it as a source file only"
    );
}
