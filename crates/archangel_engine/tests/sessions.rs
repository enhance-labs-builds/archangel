//! Simulated sessions: random sequences of edits, saves, deletions, renames, new files the editor
//! opens, folders that appear or go, tsconfig, manifest, `.gitignore` and config changes, applied
//! through `Session::apply`. After every step the session's report must equal a full check of the
//! same files from scratch. A failing session shrinks to the shortest sequence that fails.
#![cfg(test)]

use std::collections::BTreeMap;

use archangel_engine::{Change, PathEvent, Session};
use archangel_testkit::FixtureRepo;
use proptest::prelude::*;
use proptest_state_machine::{ReferenceStateMachine, StateMachineTest, prop_state_machine};
use serde_json::json;

/// The source files a session can touch.
const SOURCES: [&str; 8] = [
    "src/app/a.ts",
    "src/app/b.ts",
    "src/features/x/index.ts",
    "src/features/x/c.ts",
    "src/features/y/index.ts",
    "src/shared/e.ts",
    "src/shared/f.ts",
    "src/gen/g.ts",
];

/// A file that is not source, which folder rules look for.
const README: &str = "src/features/x/README.md";

/// The folders that can go or appear at once.
const FOLDERS: [&str; 3] = ["src/features/x", "src/shared", "src/gen"];

/// The source files in a folder.
fn inside(folder: &str) -> impl Iterator<Item = usize> {
    (0..SOURCES.len()).filter(move |&file| SOURCES[file].starts_with(&format!("{folder}/")))
}

/// When the editor opens a new file, against when the file and its event reach the session.
#[derive(Debug, Clone, Copy)]
enum Opening {
    /// The file is written, and its text and its event arrive in one batch.
    WithItsEvent,
    /// The file is written, and its text arrives a batch before its event.
    BeforeItsEvent,
    /// Its text arrives before the file is written, and its event after.
    BeforeTheFile,
    /// The file is written before the session hears of it, and no event ever comes, as for a
    /// file made between the walk and the start of the watch. The walk must find the file.
    WithoutAnEvent,
}

/// What an import points at: a source file by relative path or by alias, a CSS file, or a package.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Relative(usize),
    Alias(usize),
    Css,
    React,
}

#[derive(Debug, Clone)]
enum Step {
    /// Save a source file with these imports, in place.
    Save(usize, Vec<Target>),
    /// Save a source file by a write to a temporary file and a rename over it, reported as the
    /// system reports it or, with `true`, as the engine's watcher does: the debouncer gives the
    /// creation of the file, and the watcher adds that the temporary file is gone.
    SaveByRename(usize, Vec<Target>, bool),
    /// Delete a source file that is not open.
    Delete(usize),
    /// An editor's unsaved text for an open document.
    Edit(usize, Vec<Target>),
    /// Close a document without saving.
    Close(usize),
    /// Write a new file with the first imports, and open it in the editor with the second.
    OpenNew(usize, Vec<Target>, Vec<Target>, Opening),
    /// Delete a folder, which a watcher reports as the folder alone.
    DeleteFolder(usize),
    /// Fill a folder with files that a watcher reports as the folder alone, as for a move into
    /// the repository.
    AddFolder(usize, Vec<Vec<Target>>),
    /// Point the alias at `src/` or at `src/shared/`, through an `extends` chain or not.
    Tsconfig { shared: bool, extends: bool },
    /// List `react` as a dependency, or not.
    Manifest(bool),
    /// Ignore `src/gen/` in `.gitignore`, or not.
    Gitignore(bool),
    /// One of four configs: more rules or fewer, and `src/gen/` in the ignore list or not.
    Config { strict: bool, ignore_gen: bool },
    /// Write `src/features/x/README.md`, a file that is not source, or delete it.
    Readme(bool),
}

/// The files on disk, and the open documents' texts, which win while they are open.
#[derive(Debug, Clone, Default)]
struct Model {
    disk: BTreeMap<String, String>,
    open: BTreeMap<String, String>,
}

impl Model {
    /// Whether the walk skips a source file, by the `.gitignore` file or the config's ignore list.
    /// Nothing but an event tells the session about such a file.
    fn ignored(&self, file: usize) -> bool {
        let lists = [".gitignore", "archangel.config.json"].map(|path| self.disk.get(path));
        SOURCES[file].starts_with("src/gen/")
            && lists
                .iter()
                .any(|list| list.is_some_and(|text| text.contains("src/gen/")))
    }

    /// What a full check sees: the disk with each open document's text over it.
    fn effective(&self) -> BTreeMap<String, String> {
        let mut files = self.disk.clone();
        files.extend(self.open.clone());
        files
    }
}

fn relative(from: &str, to: &str) -> String {
    let from: Vec<&str> = from.split('/').collect();
    let to: Vec<&str> = to.split('/').collect();
    let shared = from
        .iter()
        .zip(&to)
        .take_while(|(a, b)| a == b)
        .count()
        .min(from.len() - 1);
    let ups = from.len() - 1 - shared;
    let mut parts: Vec<String> = if ups == 0 {
        vec![".".into()]
    } else {
        vec!["..".into(); ups]
    };
    parts.extend(to[shared..].iter().map(|s| (*s).to_owned()));
    parts.join("/").trim_end_matches(".ts").to_owned()
}

fn text(file: usize, imports: &[Target]) -> String {
    let from = SOURCES[file];
    let lines: Vec<String> = imports
        .iter()
        .map(|target| {
            let specifier = match *target {
                Target::Relative(to) => relative(from, SOURCES[to]),
                Target::Alias(to) => format!(
                    "@/{}",
                    SOURCES[to]
                        .trim_start_matches("src/")
                        .trim_end_matches(".ts")
                ),
                Target::Css => relative(from, "src/shared/style.module.css"),
                Target::React => "react".to_owned(),
            };
            format!("import '{specifier}';\n")
        })
        .collect();
    // A file that imports React is a client component, so an edit can change a file's class.
    let directive = if imports.contains(&Target::React) {
        "'use client';\n"
    } else {
        ""
    };
    format!("{directive}{}export const value = 1;\n", lines.concat())
}

fn tsconfig(shared: bool, extends: bool) -> (String, Option<String>) {
    let target = if shared { "src/shared/*" } else { "src/*" };
    let options =
        json!({ "compilerOptions": { "baseUrl": ".", "paths": { "@/*": [target] } } }).to_string();
    if extends {
        (
            json!({ "extends": "./tsconfig.base.json" }).to_string(),
            Some(options),
        )
    } else {
        (options, None)
    }
}

fn config(strict: bool, ignore_gen: bool) -> String {
    let mut rules = vec![
        json!({ "kind": "layers", "name": "layers", "order": ["app", "feature", "shared"] }),
        json!({ "name": "no-cycles", "files": "src/**", "noCycles": true }),
        json!({ "name": "server-stays-off-the-client", "files": "src/app/**", "mayNotImport": "client" }),
    ];
    if strict {
        rules.push(json!({ "kind": "isolate", "name": "isolate", "element": "feature" }));
        rules.push(json!({ "kind": "entry", "name": "entry", "element": "feature", "files": ["index.ts"] }));
        rules.push(json!({ "name": "no-react", "files": "shared", "mayNotImport": "react" }));
        rules.push(
            json!({ "name": "feature-entry-file", "files": "feature", "mustContain": "index.ts" }),
        );
        rules.push(json!({ "name": "feature-readme", "folders": "src/features/*", "mustContain": "README.md" }));
        rules.push(
            json!({ "name": "kebab-folders", "folders": "src/**", "mustBeNamed": "kebab-case" }),
        );
        rules.push(json!({ "name": "gen-keeps-value-private", "files": "src/gen/**", "mayNotExport": "value" }));
        rules.push(json!({ "name": "barrels", "files": "src/features/*/index.ts", "mustOnlyReExport": true }));
        rules.push(json!({ "name": "shared-exports-value", "files": "src/shared/**", "mustExport": "value" }));
        rules.push(json!({
            "name": "app-never-reaches-react",
            "files": "src/app/**",
            "mayNotImport": { "packages": ["react"] },
            "transitively": true,
        }));
    }
    let ignore: Vec<&str> = if ignore_gen {
        vec!["src/gen/"]
    } else {
        Vec::new()
    };
    json!({
        "elements": {
            "app": "src/app/**",
            "feature": "src/features/{feature}/**",
            "shared": "src/**",
            "react": { "packages": ["react"] },
        },
        "selectors": { "client": { "directive": "use client" } },
        "ignore": ignore,
        "rules": rules,
    })
    .to_string()
}

fn manifest(react: bool) -> String {
    let deps = if react {
        json!({ "react": "19.0.0" })
    } else {
        json!({})
    };
    json!({ "name": "app", "dependencies": deps }).to_string()
}

fn imports() -> impl Strategy<Value = Vec<Target>> {
    let target = prop_oneof![
        4 => (0..SOURCES.len()).prop_map(Target::Relative),
        2 => (0..SOURCES.len()).prop_map(Target::Alias),
        1 => Just(Target::Css),
        1 => Just(Target::React),
    ];
    prop::collection::vec(target, 0..4)
}

struct Reference;

impl ReferenceStateMachine for Reference {
    type State = Model;
    type Transition = Step;

    fn init_state() -> BoxedStrategy<Model> {
        prop::collection::vec(imports(), SOURCES.len())
            .prop_map(|all| {
                let mut disk: BTreeMap<String, String> = all
                    .iter()
                    .enumerate()
                    .map(|(file, imports)| (SOURCES[file].to_owned(), text(file, imports)))
                    .collect();
                let (ts, base) = tsconfig(false, false);
                disk.insert("tsconfig.json".into(), ts);
                if let Some(base) = base {
                    disk.insert("tsconfig.base.json".into(), base);
                }
                disk.insert("package.json".into(), manifest(false));
                disk.insert("src/shared/style.module.css".into(), ".a {}\n".into());
                disk.insert(
                    "node_modules/react/package.json".into(),
                    r#"{ "name": "react", "main": "index.js" }"#.into(),
                );
                disk.insert("node_modules/react/index.js".into(), String::new());
                disk.insert(".gitignore".into(), "node_modules/\n".into());
                disk.insert("archangel.config.json".into(), config(false, false));
                Model {
                    disk,
                    open: BTreeMap::new(),
                }
            })
            .boxed()
    }

    fn transitions(_state: &Model) -> BoxedStrategy<Step> {
        let file = 0..SOURCES.len();
        prop_oneof![
            4 => (file.clone(), imports()).prop_map(|(f, i)| Step::Save(f, i)),
            2 => (file.clone(), imports(), any::<bool>())
                .prop_map(|(f, i, debounced)| Step::SaveByRename(f, i, debounced)),
            2 => file.clone().prop_map(Step::Delete),
            3 => (file.clone(), imports()).prop_map(|(f, i)| Step::Edit(f, i)),
            2 => file.clone().prop_map(Step::Close),
            2 => (file, imports(), imports(), prop_oneof![
                Just(Opening::WithItsEvent),
                Just(Opening::BeforeItsEvent),
                Just(Opening::BeforeTheFile),
                Just(Opening::WithoutAnEvent),
            ])
                .prop_map(|(f, disk, open, opening)| Step::OpenNew(f, disk, open, opening)),
            1 => (0..FOLDERS.len()).prop_map(Step::DeleteFolder),
            1 => (0..FOLDERS.len(), prop::collection::vec(imports(), SOURCES.len()))
                .prop_map(|(folder, all)| Step::AddFolder(folder, all)),
            1 => (any::<bool>(), any::<bool>()).prop_map(|(shared, extends)| Step::Tsconfig { shared, extends }),
            1 => any::<bool>().prop_map(Step::Manifest),
            1 => any::<bool>().prop_map(Step::Gitignore),
            1 => (any::<bool>(), any::<bool>()).prop_map(|(strict, ignore_gen)| Step::Config { strict, ignore_gen }),
            1 => any::<bool>().prop_map(Step::Readme),
        ]
        .boxed()
    }

    fn preconditions(state: &Model, step: &Step) -> bool {
        match step {
            Step::Delete(file) => !state.open.contains_key(SOURCES[*file]),
            Step::Edit(file, _) => state.disk.contains_key(SOURCES[*file]),
            Step::Close(file) => state.open.contains_key(SOURCES[*file]),
            Step::OpenNew(file, _, _, opening) => {
                let found = match opening {
                    Opening::WithoutAnEvent => !state.ignored(*file),
                    _ => true,
                };
                found
                    && !state.disk.contains_key(SOURCES[*file])
                    && !state.open.contains_key(SOURCES[*file])
            }
            Step::DeleteFolder(folder) => {
                inside(FOLDERS[*folder]).all(|file| !state.open.contains_key(SOURCES[file]))
            }
            Step::AddFolder(folder, _) => {
                inside(FOLDERS[*folder]).all(|file| !state.disk.contains_key(SOURCES[file]))
            }
            _ => true,
        }
    }

    fn apply(mut state: Model, step: &Step) -> Model {
        match step {
            Step::Tsconfig { .. }
            | Step::Readme(_)
            | Step::Manifest(_)
            | Step::Gitignore(_)
            | Step::Config { .. } => {
                state.change_settings(step);
            }
            _ => state.change_files(step),
        }
        state
    }
}

impl Model {
    /// Applies a step to the source files or the open documents.
    fn change_files(&mut self, step: &Step) {
        let state = self;
        match step {
            Step::Save(file, imports) | Step::SaveByRename(file, imports, _) => {
                state
                    .disk
                    .insert(SOURCES[*file].to_owned(), text(*file, imports));
            }
            Step::Delete(file) => {
                state.disk.remove(SOURCES[*file]);
            }
            Step::Edit(file, imports) => {
                state
                    .open
                    .insert(SOURCES[*file].to_owned(), text(*file, imports));
            }
            Step::Close(file) => {
                state.open.remove(SOURCES[*file]);
            }
            Step::OpenNew(file, disk, open, _) => {
                let path = SOURCES[*file].to_owned();
                state.disk.insert(path.clone(), text(*file, disk));
                state.open.insert(path, text(*file, open));
            }
            Step::DeleteFolder(folder) => {
                let below = format!("{}/", FOLDERS[*folder]);
                state.disk.retain(|path, _| !path.starts_with(&below));
            }
            Step::AddFolder(folder, all) => {
                for file in inside(FOLDERS[*folder]) {
                    state
                        .disk
                        .insert(SOURCES[file].to_owned(), text(file, &all[file]));
                }
            }
            _ => {}
        }
    }

    /// Applies a step to the tsconfig, the manifest, the `.gitignore` file or the config.
    fn change_settings(&mut self, step: &Step) {
        let state = self;
        match step {
            Step::Tsconfig { shared, extends } => {
                let (ts, base) = tsconfig(*shared, *extends);
                state.disk.insert("tsconfig.json".into(), ts);
                match base {
                    Some(base) => state.disk.insert("tsconfig.base.json".into(), base),
                    None => state.disk.remove("tsconfig.base.json"),
                };
            }
            Step::Manifest(react) => {
                state.disk.insert("package.json".into(), manifest(*react));
            }
            Step::Gitignore(ignored) => {
                let text = if *ignored {
                    "node_modules/\nsrc/gen/\n"
                } else {
                    "node_modules/\n"
                };
                state.disk.insert(".gitignore".into(), text.into());
            }
            Step::Config { strict, ignore_gen } => {
                state
                    .disk
                    .insert("archangel.config.json".into(), config(*strict, *ignore_gen));
            }
            Step::Readme(present) => {
                if *present {
                    state.disk.insert(README.into(), "# x\n".into());
                } else {
                    state.disk.remove(README);
                }
            }
            _ => {}
        }
    }
}

/// The repository on disk and the session over it.
struct Live {
    repo: FixtureRepo,
    session: Session,
}

impl Live {
    /// Writes a new file and opens it, in the order given; returns the changes still to apply.
    fn open_new(&mut self, model: &Model, file: usize, opening: Opening) -> Vec<Change> {
        let path = SOURCES[file];
        let open = Change::Text {
            path: path.to_owned(),
            text: model.open.get(path).cloned(),
        };
        let created = Change::Paths(vec![PathEvent::Created(self.repo.join(path))]);
        match opening {
            Opening::WithItsEvent => {
                self.repo.write(path, &model.disk[path]);
                vec![open, created]
            }
            Opening::BeforeItsEvent => {
                self.repo.write(path, &model.disk[path]);
                self.session.apply(vec![open]);
                vec![created]
            }
            Opening::BeforeTheFile => {
                self.session.apply(vec![open]);
                self.repo.write(path, &model.disk[path]);
                vec![created]
            }
            Opening::WithoutAnEvent => {
                self.repo.write(path, &model.disk[path]);
                vec![open]
            }
        }
    }

    /// Deletes a folder, reported as the folder alone.
    fn delete_folder(&self, folder: usize) -> Vec<Change> {
        let folder = self.repo.join(FOLDERS[folder]);
        if folder.exists() {
            std::fs::remove_dir_all(&folder).expect("the folder is deleted");
        }
        vec![Change::Paths(vec![PathEvent::Deleted(folder)])]
    }

    /// Fills a folder with the model's files, reported as the folder alone.
    fn add_folder(&self, model: &Model, folder: usize) -> Vec<Change> {
        for file in inside(FOLDERS[folder]) {
            self.repo.write(SOURCES[file], &model.disk[SOURCES[file]]);
        }
        vec![Change::Paths(vec![PathEvent::Created(
            self.repo.join(FOLDERS[folder]),
        )])]
    }

    /// Saves a file through a temporary file and a rename over it.
    fn save_by_rename(&self, model: &Model, file: usize, debounced: bool) -> Vec<Change> {
        let path = SOURCES[file];
        let temp = format!("{path}.tmp");
        self.repo.write(&temp, &model.disk[path]);
        self.repo.rename(&temp, path);
        let (temp, path) = (self.repo.join(&temp), self.repo.join(path));
        let events = if debounced {
            vec![PathEvent::Created(path), PathEvent::Deleted(temp)]
        } else {
            vec![
                PathEvent::Created(temp.clone()),
                PathEvent::Renamed {
                    from: temp,
                    to: path,
                },
            ]
        };
        vec![Change::Paths(events)]
    }

    /// Writes the model's disk, and the event for each file that changed, appeared or went.
    fn sync(&self, model: &Model) -> Vec<PathEvent> {
        let mut events = Vec::new();
        let mut existing = BTreeMap::new();
        for path in SOURCES.iter().copied().chain([
            "tsconfig.json",
            "tsconfig.base.json",
            "package.json",
            ".gitignore",
            "archangel.config.json",
            README,
        ]) {
            existing.insert(path.to_owned(), self.repo.join(path).is_file());
        }
        for (path, was) in existing {
            let full = self.repo.join(&path);
            match (was, model.disk.get(&path)) {
                (true, Some(text)) if self.repo.read(&path) != *text => {
                    self.repo.write(&path, text);
                    events.push(PathEvent::Changed(full));
                }
                (false, Some(text)) => {
                    self.repo.write(&path, text);
                    events.push(PathEvent::Created(full));
                }
                (true, None) => {
                    self.repo.remove(&path);
                    events.push(PathEvent::Deleted(full));
                }
                _ => {}
            }
        }
        events
    }
}

/// A full check of the model's files from scratch: the oracle.
fn full_check(model: &Model) -> archangel_rules::Report {
    let repo = model
        .effective()
        .iter()
        .fold(FixtureRepo::new(), |repo, (path, text)| {
            repo.file(path, text)
        });
    Session::open(repo.path(), None)
        .expect("the oracle opens")
        .report()
        .clone()
}

struct Sessions;

impl StateMachineTest for Sessions {
    type SystemUnderTest = Live;
    type Reference = Reference;

    fn init_test(model: &Model) -> Live {
        let repo = model
            .disk
            .iter()
            .fold(FixtureRepo::new(), |repo, (path, text)| {
                repo.file(path, text)
            });
        let session = Session::open(repo.path(), None).expect("the session opens");
        Live { repo, session }
    }

    fn apply(mut live: Live, model: &Model, step: Step) -> Live {
        let changes = match &step {
            Step::Edit(file, _) => vec![Change::Text {
                path: SOURCES[*file].to_owned(),
                text: model.open.get(SOURCES[*file]).cloned(),
            }],
            Step::Close(file) => vec![Change::Text {
                path: SOURCES[*file].to_owned(),
                text: None,
            }],
            Step::OpenNew(file, _, _, opening) => live.open_new(model, *file, *opening),
            Step::DeleteFolder(folder) => live.delete_folder(*folder),
            Step::AddFolder(folder, _) => live.add_folder(model, *folder),
            Step::SaveByRename(file, _, debounced) => live.save_by_rename(model, *file, *debounced),
            _ => vec![Change::Paths(live.sync(model))],
        };
        live.session.apply(changes);
        live
    }

    fn check_invariants(live: &Live, model: &Model) {
        let expected = full_check(model);
        let found = live.session.report();
        assert_eq!(
            found.diagnostics, expected.diagnostics,
            "the session's diagnostics match a full check"
        );
        assert_eq!(
            (found.summary.files, found.summary.imports),
            (expected.summary.files, expected.summary.imports),
            "(files, imports) match a full check"
        );
    }
}

prop_state_machine! {
    #![proptest_config(ProptestConfig {
        cases: 1000,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn random_sessions_agree_with_a_full_check(sequential 1..12 => Sessions);
}

/// A repository opened as a session, and the report of a full check after the same change.
fn client_repo(files: &[(&str, &str)]) -> FixtureRepo {
    let config = json!({
        "elements": { "app": "app/**", "server": "src/server/**" },
        "selectors": { "client": { "directive": "use client" } },
        "rules": [
            { "name": "client-stays-client", "files": "client", "mayNotImport": { "packages": ["server-only"] } },
            { "name": "server-stays-off-the-client", "files": "src/server/**", "mayNotImport": "client" },
        ],
    });
    files
        .iter()
        .fold(
            FixtureRepo::new()
                .file(".gitignore", "node_modules/\n")
                .file(
                    "node_modules/server-only/package.json",
                    r#"{ "name": "server-only", "main": "index.js" }"#,
                )
                .file("node_modules/server-only/index.js", ""),
            |repo, (path, text)| repo.file(path, text),
        )
        .json_config(&config)
}

/// Opens a session, applies an editor's text for one file, and returns the session's report, the
/// work of the change, and a full check of the repository with that text saved.
fn edited(
    files: &[(&str, &str)],
    path: &str,
    text: &str,
) -> (
    archangel_rules::Report,
    archangel_graph::Work,
    archangel_rules::Report,
) {
    let repo = client_repo(files);
    let mut session = Session::open(repo.path(), None).expect("the session opens");
    let work = session
        .apply(vec![Change::Text {
            path: path.to_owned(),
            text: Some(text.to_owned()),
        }])
        .work;
    repo.write(path, text);
    let full = Session::open(repo.path(), None)
        .expect("the full check opens")
        .report()
        .clone();
    (session.report().clone(), work, full)
}

fn violated(report: &archangel_rules::Report) -> Vec<(String, String)> {
    report
        .diagnostics
        .iter()
        .filter(|d| d.code.is_rule())
        .map(|d| {
            (
                d.rule.clone().unwrap_or_default(),
                d.file.clone().unwrap_or_default(),
            )
        })
        .collect()
}

#[test]
fn a_directive_edit_changes_the_files_class() {
    let (found, _, full) = edited(
        &[
            ("app/a.tsx", "'use client';\n"),
            ("app/b.tsx", "import 'server-only';\n"),
        ],
        "app/b.tsx",
        "'use client';\nimport 'server-only';\n",
    );
    assert_eq!(
        violated(&found),
        [("client-stays-client".to_owned(), "app/b.tsx".to_owned())],
        "the client file may not import server-only"
    );
    assert_eq!(
        found.diagnostics, full.diagnostics,
        "the same as a full check"
    );
}

#[test]
fn a_class_change_re_checks_the_files_importers() {
    let (found, work, full) = edited(
        &[
            ("app/a.tsx", "'use client';\n"),
            ("app/b.tsx", "export const b = 1;\n"),
            ("src/server/c.ts", "import '../../app/b';\n"),
            ("src/server/d.ts", "import '../../app/a';\nimport './c';\n"),
        ],
        "app/b.tsx",
        "'use client';\nexport const b = 1;\n",
    );
    assert_eq!(
        violated(&found),
        [
            (
                "server-stays-off-the-client".to_owned(),
                "src/server/c.ts".to_owned()
            ),
            (
                "server-stays-off-the-client".to_owned(),
                "src/server/d.ts".to_owned()
            ),
        ],
        "c.ts now imports a client file"
    );
    assert_eq!(
        found.diagnostics, full.diagnostics,
        "the same as a full check"
    );
    assert_eq!(
        work.rule_evaluations, 1,
        "the evaluations are those of the import into b.tsx, which has no imports of its own"
    );
}

#[test]
fn a_selector_that_empties_in_the_editor_is_reported() {
    let (found, work, full) = edited(
        &[("app/a.tsx", "'use client';\n"), ("app/b.tsx", "")],
        "app/a.tsx",
        "export const a = 1;\n",
    );
    assert!(
        found
            .diagnostics
            .iter()
            .any(|d| d.code == archangel_rules::Code::SelectorEmpty
                && d.message.contains("\"client\"")),
        "client is empty: {:?}",
        found.diagnostics
    );
    assert_eq!(
        found.diagnostics, full.diagnostics,
        "the same as a full check"
    );
    assert_eq!(work.files_classified, 1, "no other file is classified");
}

/// The session's report after a batch of events, and a full check of the repository as it is.
fn after_events(
    repo: &FixtureRepo,
    session: &mut Session,
    events: Vec<PathEvent>,
) -> (archangel_rules::Report, archangel_rules::Report) {
    session.apply(vec![Change::Paths(events)]);
    let full = Session::open(repo.path(), None)
        .expect("the full check opens")
        .report()
        .clone();
    (session.report().clone(), full)
}

#[test]
fn deleting_a_required_file_reports_its_instance() {
    let repo = FixtureRepo::new()
        .file("src/features/login/index.ts", "export const login = 1;\n")
        .file("src/features/login/a.ts", "")
        .json_config(&json!({
            "elements": { "slice": "src/features/{slice}/**" },
            "rules": [{ "name": "public-api", "files": "slice", "mustContain": "index.ts" }],
        }));
    let mut session = Session::open(repo.path(), None).expect("the session opens");
    assert_eq!(violated(session.report()), [], "no violation at first");
    repo.remove("src/features/login/index.ts");
    let (found, full) = after_events(
        &repo,
        &mut session,
        vec![PathEvent::Deleted(repo.join("src/features/login/index.ts"))],
    );
    assert_eq!(
        violated(&found),
        [("public-api".to_owned(), "src/features/login".to_owned())],
        "the instance login has no index.ts now"
    );
    assert_eq!(
        found.diagnostics, full.diagnostics,
        "the same as a full check"
    );
}

#[test]
fn a_rename_into_a_badly_named_folder_is_reported() {
    let repo = FixtureRepo::new()
        .file("src/a/x.ts", "")
        .file("src/b/y.ts", "")
        .json_config(&json!({
            "elements": { "src": "src/**" },
            "rules": [{ "name": "kebab-folders", "folders": "src/**", "mustBeNamed": "kebab-case" }],
        }));
    let mut session = Session::open(repo.path(), None).expect("the session opens");
    repo.write("src/newThing/x.ts", "");
    repo.remove("src/a/x.ts");
    std::fs::remove_dir(repo.join("src/a")).expect("the old folder is gone");
    let (found, full) = after_events(
        &repo,
        &mut session,
        vec![PathEvent::Renamed {
            from: repo.join("src/a/x.ts"),
            to: repo.join("src/newThing/x.ts"),
        }],
    );
    assert_eq!(
        violated(&found),
        [("kebab-folders".to_owned(), "src/newThing".to_owned())],
        "the new folder is not kebab-case"
    );
    assert_eq!(
        found.diagnostics, full.diagnostics,
        "the same as a full check"
    );
}

#[test]
fn an_edit_to_an_export_star_source_re_checks_the_barrel() {
    let repo = FixtureRepo::new()
        .file("src/card.view.tsx", "export * from './card.impl';\n")
        .file("src/card.impl.ts", "export const Card = 1;\n")
        .json_config(&json!({
            "elements": { "src": "src/**" },
            "rules": [{ "name": "views", "files": "**/*.view.tsx", "mustExport": "{FileName}" }],
        }));
    let mut session = Session::open(repo.path(), None).expect("the session opens");
    assert_eq!(violated(session.report()), [], "no violation at first");
    let text = "export const Other = 1;\n";
    session.apply(vec![Change::Text {
        path: "src/card.impl.ts".to_owned(),
        text: Some(text.to_owned()),
    }]);
    repo.write("src/card.impl.ts", text);
    let full = Session::open(repo.path(), None)
        .expect("the full check opens")
        .report()
        .clone();
    let found = session.report();
    assert_eq!(
        violated(found),
        [("views".to_owned(), "src/card.view.tsx".to_owned())],
        "the barrel no longer passes Card on"
    );
    assert_eq!(
        found.diagnostics, full.diagnostics,
        "the same as a full check"
    );
}

#[test]
fn a_class_change_re_checks_the_files_structure_and_export_rules() {
    let repo = FixtureRepo::new()
        .file("app/A.tsx", "'use client';\nexport const a = 1;\n")
        .file("app/b.tsx", "export default function B() {}\n")
        .json_config(&json!({
            "elements": { "app": "app/**" },
            "selectors": { "client": { "directive": "use client" } },
            "rules": [
                { "name": "client-files-are-pascal", "files": { "all": ["client", "app/**/*.tsx"] }, "mustBeNamed": "PascalCase" },
                { "name": "client-has-no-default", "files": "client", "mayNotExport": "default" },
            ],
        }));
    let mut session = Session::open(repo.path(), None).expect("the session opens");
    let text = "'use client';\nexport default function B() {}\n";
    let work = session
        .apply(vec![Change::Text {
            path: "app/b.tsx".to_owned(),
            text: Some(text.to_owned()),
        }])
        .work;
    repo.write("app/b.tsx", text);
    let full = Session::open(repo.path(), None)
        .expect("the full check opens")
        .report()
        .clone();
    let found = session.report();
    assert_eq!(
        violated(found),
        [
            ("client-files-are-pascal".to_owned(), "app/b.tsx".to_owned()),
            ("client-has-no-default".to_owned(), "app/b.tsx".to_owned()),
        ],
        "b.tsx is a client file now, so its name and its default export are refused"
    );
    assert_eq!(
        found.diagnostics, full.diagnostics,
        "the same as a full check"
    );
    assert_eq!(
        work.structure_evaluations, 1,
        "the naming rule runs for b.tsx alone"
    );
}

#[test]
fn a_part_overlap_appears_and_goes_with_the_file_in_the_editor() {
    let repo = FixtureRepo::new()
        .file("src/features/login/a.ts", "")
        .file("src/x.model.ts", "")
        .json_config(&json!({
            "parts": { "model": "src/**/*.model.ts", "feature": "src/features/{feature}/**" },
        }));
    let overlaps = |report: &archangel_rules::Report| {
        report
            .diagnostics
            .iter()
            .filter(|d| d.code == archangel_rules::Code::PartOverlap)
            .map(|d| d.file.clone().unwrap_or_default())
            .collect::<Vec<_>>()
    };
    let mut session = Session::open(repo.path(), None).expect("the session opens");
    assert_eq!(
        overlaps(session.report()),
        [] as [String; 0],
        "none at first"
    );
    let path = "src/features/login/session.model.ts";
    repo.write(path, "");
    let (found, full) = after_events(
        &repo,
        &mut session,
        vec![PathEvent::Created(repo.join(path))],
    );
    assert_eq!(overlaps(&found), [path], "the file is in two parts");
    assert_eq!(
        found.diagnostics, full.diagnostics,
        "the same as a full check"
    );
    repo.remove(path);
    let (found, full) = after_events(
        &repo,
        &mut session,
        vec![PathEvent::Deleted(repo.join(path))],
    );
    assert_eq!(
        overlaps(&found),
        [] as [String; 0],
        "the diagnostic is gone"
    );
    assert_eq!(
        found.diagnostics, full.diagnostics,
        "the same as a full check"
    );
}

/// A table with an entity row and an aggregate-root row, on a repository with no aggregate root.
fn table_repo() -> FixtureRepo {
    FixtureRepo::new()
        .file("apps/api/src/board/application/save.use-case.ts", "")
        .file("apps/api/src/board/domain/board.entity.ts", "")
        .json_config(&json!({
            "parts": { "api": "apps/api/**" },
            "rules": [{
                "kind": "mayImport",
                "name": "domain",
                "rows": { "entity": [], "aggregate-root": ["entity"], "use-case": [] },
                "kinds": "**/*.{kind}.ts",
            }],
        }))
}

fn rows_with_no_file(report: &archangel_rules::Report) -> Vec<String> {
    report.summary.rows_with_no_file.clone().unwrap_or_default()
}

/// The files of the violations of one rule.
fn files_breaking(report: &archangel_rules::Report, rule: &str) -> Vec<String> {
    report
        .diagnostics
        .iter()
        .filter(|d| d.code.is_rule() && d.rule.as_deref() == Some(rule))
        .filter_map(|d| d.file.clone())
        .collect()
}

#[test]
fn a_row_becomes_active_when_its_first_file_is_created() {
    let repo = table_repo();
    let mut session = Session::open(repo.path(), None).expect("the session opens");
    assert_eq!(
        rows_with_no_file(session.report()),
        ["domain.aggregate-root"],
        "a row becomes active when its first file is created"
    );
    let path = "apps/api/src/board/domain/board.aggregate-root.ts";
    repo.write(path, "import '../application/save.use-case';\n");
    let (found, full) = after_events(
        &repo,
        &mut session,
        vec![PathEvent::Created(repo.join(path))],
    );
    assert_eq!(
        files_breaking(&found, "domain.aggregate-root"),
        [path],
        "the import in the new file breaks its row"
    );
    assert_eq!(
        rows_with_no_file(&found),
        [] as [String; 0],
        "the row is active"
    );
    assert_eq!(
        found.diagnostics, full.diagnostics,
        "the same as a full check"
    );
    assert_eq!(
        found.summary, full.summary,
        "the same summary as a full check"
    );
}

#[test]
fn a_row_is_listed_again_when_its_last_file_is_deleted() {
    let repo = table_repo().file("apps/api/src/board/domain/board.aggregate-root.ts", "");
    let mut session = Session::open(repo.path(), None).expect("the session opens");
    assert_eq!(
        rows_with_no_file(session.report()),
        [] as [String; 0],
        "a row is listed again when its last file is deleted"
    );
    let path = "apps/api/src/board/domain/board.aggregate-root.ts";
    repo.remove(path);
    let (found, full) = after_events(
        &repo,
        &mut session,
        vec![PathEvent::Deleted(repo.join(path))],
    );
    assert_eq!(
        rows_with_no_file(&found),
        ["domain.aggregate-root"],
        "a row is listed again when its last file is deleted"
    );
    assert_eq!(
        found.diagnostics, full.diagnostics,
        "the same as a full check"
    );
    assert_eq!(
        found.summary, full.summary,
        "the same summary as a full check"
    );
}
