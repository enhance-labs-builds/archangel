//! Scenarios from the command-line spec: `archangel watch` checks again after each change and
//! prints what is new and what is gone. Each test waits for the line that answers its change,
//! never for a duration; the timeout only stops a test that would hang.
#![cfg(test)]

use std::io::{BufRead, BufReader};
use std::process::{Child, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::{Duration, Instant};

use archangel_testkit::{FixtureRepo, app_and_shared};
use serde_json::{Value, json};

/// How long a test waits for a line before it fails as hung.
const HUNG: Duration = Duration::from_secs(60);

/// How many lines a test reads, at most, for the one it waits for.
const MOST_LINES: usize = 1000;

/// `archangel watch` running in a repository, with its output lines.
struct Watching {
    child: Child,
    lines: Receiver<String>,
}

impl Drop for Watching {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

impl Watching {
    fn start(repo: &FixtureRepo, args: &[&str], failpoints: Option<&str>) -> Self {
        let mut command = std::process::Command::new(assert_cmd::cargo::cargo_bin("archangel"));
        command
            .arg("watch")
            .args(args)
            .current_dir(repo.path())
            .env_remove("NO_COLOR")
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        match failpoints {
            Some(points) => command.env("FAILPOINTS", points),
            None => command.env_remove("FAILPOINTS"),
        };
        let mut child = command.spawn().expect("archangel watch starts");
        let stdout = child.stdout.take().expect("its output");
        let (sender, lines) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        Self { child, lines }
    }

    /// The next line for which `wanted` is true, after the lines before it.
    fn until(&self, what: &str, wanted: impl Fn(&str) -> bool) -> String {
        let deadline = Instant::now() + HUNG;
        let mut seen = Vec::new();
        for _ in 0..MOST_LINES {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.lines.recv_timeout(left) {
                Ok(line) if wanted(&line) => return line,
                Ok(line) => seen.push(line),
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => break,
            }
        }
        panic!("no line {what} came; the lines were:\n{}", seen.join("\n"))
    }

    /// The next summary line of a check.
    fn summary(&self) -> String {
        self.until("with a summary", |line| line.starts_with("checked "))
    }

    /// The lines of the next check whose summary line `wanted` accepts, and that summary: the
    /// check that answers a change, past any check for an event the system reported late.
    fn check(&self, what: &str, wanted: impl Fn(&str) -> bool) -> (Vec<String>, String) {
        let deadline = Instant::now() + HUNG;
        let mut seen = Vec::new();
        let mut lines = Vec::new();
        for _ in 0..MOST_LINES {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.lines.recv_timeout(left) {
                Ok(line) if line.starts_with("checked ") && wanted(&line) => return (lines, line),
                Ok(line) if line.starts_with("checked ") => {
                    seen.append(&mut lines);
                    seen.push(line);
                }
                Ok(line) => lines.push(line),
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => break,
            }
        }
        seen.append(&mut lines);
        panic!("no check {what} came; the lines were:\n{}", seen.join("\n"))
    }

    /// The next JSON line, parsed.
    fn json(&self) -> Value {
        self.json_until("of JSON", |_| true)
    }

    /// The next JSON line for which `wanted` is true: the line that answers a change, past any
    /// line for an event the system reported late, such as the fixture's own writes.
    fn json_until(&self, what: &str, wanted: impl Fn(&Value) -> bool) -> Value {
        let line = self.until(what, |line| {
            serde_json::from_str::<Value>(line).is_ok_and(|value| wanted(&value))
        });
        serde_json::from_str(&line).expect("one JSON document per line")
    }
}

fn layers() -> Value {
    json!({ "kind": "layers", "name": "layers", "order": ["app", "shared"] })
}

fn clean() -> FixtureRepo {
    FixtureRepo::new()
        .file("src/app/main.ts", "import '../s';\n")
        .file("src/s.ts", "export const s = 1;\n")
        .json_config(&app_and_shared(&[layers()]))
}

const VIOLATION: &str = "import './app/main';\nexport const s = 1;\n";

fn codes(list: &Value) -> Vec<String> {
    list.as_array()
        .map(|items| {
            items
                .iter()
                .map(|d| {
                    format!(
                        "{}:{}",
                        d["code"].as_str().unwrap_or_default(),
                        d["file"].as_str().unwrap_or_default()
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn watch_prints_a_new_violation_after_a_save() {
    let repo = clean();
    let watch = Watching::start(&repo, &[], None);
    assert!(
        watch.summary().contains("no problems"),
        "the first check is clean"
    );
    repo.write("src/s.ts", VIOLATION);
    let (lines, summary) = watch.check("with the problem", |line| line.contains("1 problem"));
    assert_eq!(
        lines.join("\n"),
        "error[layers]: \"shared\" may not import \"app\", an earlier layer, through \"./app/main\"\n \
         --> src/s.ts:1:8\n  \
         |\n\
         1 | import './app/main';\n  \
         |        ^^^^^^^^^^^^ \"shared\" may import only \"shared\"\n",
        "the check that counts the problem prints it as new: {summary}"
    );
}

#[test]
fn watch_prints_a_violation_as_gone_after_a_fix() {
    let repo = clean().file("src/s.ts", VIOLATION);
    let watch = Watching::start(&repo, &[], None);
    assert!(
        watch.summary().contains("1 problem"),
        "the first check has the violation"
    );
    repo.write("src/s.ts", "export const s = 1;\n");
    let (lines, summary) = watch.check("with no problems", |line| line.contains("no problems"));
    assert_eq!(
        lines,
        [
            "gone: [layers] src/s.ts:1: \"shared\" may not import \"app\", an earlier layer, through \"./app/main\""
        ],
        "the check that has no problems says the violation is gone, with its place: {summary}"
    );
}

#[test]
fn watch_json_prints_one_line_per_check() {
    let repo = clean();
    let watch = Watching::start(&repo, &["--format", "json"], None);
    assert_eq!(watch.json()["schemaVersion"], 1, "the first line");
    repo.write("src/s.ts", VIOLATION);
    let line = watch.json_until("with the problem", |line| line["summary"]["problems"] == 1);
    assert_eq!(
        codes(&line["new"]),
        ["layers:src/s.ts"],
        "exactly the new violation: {line}"
    );
    assert!(line["summary"].is_object(), "and the summary: {line}");
}

#[test]
fn watch_checks_again_after_a_config_change() {
    let repo = clean().file("src/t.ts", "import './app/main';\n");
    let watch = Watching::start(&repo, &["--format", "json"], None);
    let first = watch.json();
    assert_eq!(
        codes(&first["new"]),
        ["layers:src/t.ts"],
        "the layers violation: {first}"
    );
    repo.write(
        "archangel.config.json",
        &app_and_shared(&[
            layers(),
            json!({ "name": "f", "files": "shared", "mayNotImport": "app" }),
        ])
        .to_string(),
    );
    let line = watch.json_until("with both problems", |line| {
        line["summary"]["problems"] == 2
    });
    assert_eq!(
        codes(&line["new"]),
        ["may-not-import:src/t.ts"],
        "the new rule's violation: {line}"
    );
}

/// 200 source files, and `src/a.ts` with 4 imports. A save that adds the app import in place of
/// one library import keeps four imports and adds a violation, so its line is the one to wait for.
fn two_hundred() -> FixtureRepo {
    FixtureRepo::new()
        .synthetic("src/lib", 199, 2)
        .file("src/a.ts", &four_imports(""))
        .file("src/app/main.ts", "")
        .json_config(&app_and_shared(&[layers()]))
}

fn four_imports(extra: &str) -> String {
    ["f0", "f1", "f2", "f3"]
        .map(|f| format!("import './lib/{f}';\n"))
        .concat()
        + extra
}

const SAVED: &str =
    "import './lib/f0';\nimport './lib/f1';\nimport './lib/f2';\nimport './app/main';\n";

/// The work of the line that shows the saved violation.
fn answer(watch: &Watching) -> (u64, u64) {
    let line = watch.json_until("showing the save", |line| {
        codes(&line["new"]) == ["layers:src/a.ts"]
    });
    let work = &line["summary"]["work"];
    (
        work["filesParsed"].as_u64().unwrap_or(0),
        work["resolutions"].as_u64().unwrap_or(0),
    )
}

fn save_counts(failpoints: Option<&str>) -> (u64, u64) {
    let repo = two_hundred();
    let watch = Watching::start(&repo, &["--format", "json"], failpoints);
    watch.json();
    repo.write("src/a.ts", SAVED);
    answer(&watch)
}

#[test]
fn one_save_parses_one_file_in_watch_mode() {
    assert_eq!(
        save_counts(None),
        (1, 4),
        "(parsed, resolutions) for one save of a file with four imports"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn watch_work_count_rejects_a_full_check_per_save() {
    let (parsed, _) = save_counts(Some("engine::full_check_per_event=return"));
    assert_ne!(parsed, 1, "the count catches a full check per file event");
}

#[test]
fn save_by_rename_counts_as_a_text_change() {
    let repo = two_hundred();
    let watch = Watching::start(&repo, &["--format", "json"], None);
    watch.json();
    repo.write("src/.a.ts.tmp", SAVED);
    repo.rename("src/.a.ts.tmp", "src/a.ts");
    assert_eq!(
        answer(&watch),
        (1, 4),
        "(parsed, resolutions) for a save by rename"
    );
}

#[test]
fn gitignore_change_updates_the_source_files() {
    let repo = clean().file("src/gen/a.ts", "import '../app/main';\n");
    let watch = Watching::start(&repo, &["--format", "json"], None);
    assert_eq!(
        codes(&watch.json()["new"]),
        ["layers:src/gen/a.ts"],
        "the violation first"
    );
    repo.write(".gitignore", "src/gen/\n");
    let line = watch.json_until("with no problems", |line| line["summary"]["problems"] == 0);
    assert_eq!(
        codes(&line["gone"]),
        ["layers:src/gen/a.ts"],
        "the violation is gone: {line}"
    );
}

#[test]
fn config_ignore_change_updates_the_source_files() {
    let repo = clean()
        .file("fixtures/a.ts", "import '../src/app/main';\n")
        .json_config(&json!({
            "elements": { "app": "src/app/**", "shared": "**" },
            "rules": [layers()],
        }));
    let watch = Watching::start(&repo, &["--format", "json"], None);
    assert_eq!(
        codes(&watch.json()["new"]),
        ["layers:fixtures/a.ts"],
        "the violation first"
    );
    repo.write(
        "archangel.config.json",
        &json!({ "elements": { "app": "src/app/**", "shared": "**" }, "ignore": ["fixtures/"], "rules": [layers()] }).to_string(),
    );
    let line = watch.json_until("with no problems", |line| line["summary"]["problems"] == 0);
    assert_eq!(
        codes(&line["gone"]),
        ["layers:fixtures/a.ts"],
        "the violation is gone: {line}"
    );
}

#[test]
fn watch_applies_a_new_baseline() {
    let repo = clean()
        .file("src/x.ts", "import './app/main';\n")
        .file("src/y.ts", "import './app/main';\n")
        .file("src/z.ts", "import './app/main';\n");
    let watch = Watching::start(&repo, &["--format", "json"], None);
    assert_eq!(
        codes(&watch.json()["new"]).len(),
        3,
        "three violations first"
    );
    let added = assert_cmd::Command::cargo_bin("archangel")
        .unwrap()
        .args(["baseline", "add"])
        .current_dir(repo.path())
        .env_remove("FAILPOINTS")
        .output()
        .unwrap();
    assert!(added.status.success(), "the baseline is written");
    let line = watch.json_until("with no problems", |line| line["summary"]["problems"] == 0);
    assert_eq!(codes(&line["gone"]).len(), 3, "the three are gone: {line}");
}

#[test]
fn last_watch_line_answers_the_last_save() {
    let repo = clean();
    let watch = Watching::start(&repo, &["--format", "json"], None);
    watch.json();
    repo.write("src/s.ts", "import './app/main';\n");
    repo.write("src/s.ts", "\n\n\nimport './app/main';\n");
    let mut present: Vec<(String, u64)> = Vec::new();
    for _ in 0..MOST_LINES {
        let line = watch.json();
        for item in line["gone"].as_array().into_iter().flatten() {
            present.retain(|(code, at)| {
                !(code == item["code"].as_str().unwrap_or_default()
                    && *at == item["line"].as_u64().unwrap_or(0))
            });
        }
        for item in line["new"].as_array().into_iter().flatten() {
            present.push((
                item["code"].as_str().unwrap_or_default().to_owned(),
                item["line"].as_u64().unwrap_or(0),
            ));
        }
        if present.iter().any(|(_, at)| *at == 4) {
            break;
        }
    }
    assert_eq!(
        present,
        [("layers".to_owned(), 4)],
        "the shown state answers the second save"
    );
}

#[test]
fn watch_prints_nothing_new_for_an_unchanged_save() {
    let repo = clean().file("src/s.ts", VIOLATION);
    let watch = Watching::start(&repo, &["--format", "json"], None);
    watch.json();
    repo.write("src/s.ts", VIOLATION);
    let line = watch.json();
    assert_eq!(
        (codes(&line["new"]), codes(&line["gone"])),
        (vec![], vec![]),
        "nothing new or gone: {line}"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn lost_events_cause_a_full_check() {
    let repo = FixtureRepo::new()
        .synthetic("src/lib", 30, 1)
        .json_config(&app_and_shared(&[layers()]));
    let watch = Watching::start(
        &repo,
        &["--format", "json"],
        Some("engine::watcher_reports_lost_events=return"),
    );
    watch.json();
    let line = watch.json();
    assert_eq!(
        line["summary"]["work"]["filesParsed"], 30,
        "a full check from the start: {line}"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn watch_without_watches_polls_and_says_why() {
    let repo = clean();
    let watch = Watching::start(&repo, &[], Some("engine::watcher_has_no_watches=return"));
    let first = watch.summary();
    repo.write("src/s.ts", VIOLATION);
    let second = watch.until("after the poll", |line| {
        line.starts_with("checked ") && line.contains("1 problem")
    });
    for line in [first, second] {
        assert!(
            line.contains("polls") && line.contains("watch limit") && line.contains("raise"),
            "each summary line says it polls, the limit, and how to raise it: {line}"
        );
    }
}
