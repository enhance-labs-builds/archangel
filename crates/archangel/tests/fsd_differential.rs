//! The differential test from the presets spec: on the Feature-Sliced Design fixtures in
//! `tests/fixtures/fsd/`, which together break each of Steiger's 21 rules, the `fsd` preset reports
//! each rule for exactly the files and folders that Steiger reports, with the pinned Steiger and
//! its plugin from the root `package.json`. The differences the presets spec names are listed
//! here, each with the path it shows on, so a new difference fails and a mended one does too.
#![cfg(test)]

#[path = "support/cli.rs"]
mod support;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use archangel_testkit::FixtureRepo;
use serde_json::Value;
use support::run;

/// Steiger's 21 rules and the preset rule that matches each, and whether Steiger's recommended
/// config turns it on.
const RULES: [(&str, &str, bool); 21] = [
    (
        "fsd/ambiguous-slice-names",
        "fsd/ambiguous-slice-names",
        true,
    ),
    ("fsd/excessive-slicing", "fsd/excessive-slicing", true),
    ("fsd/forbidden-imports", "fsd/forbidden-imports", true),
    ("fsd/inconsistent-naming", "fsd/inconsistent-naming", true),
    ("fsd/insignificant-slice", "fsd/insignificant-slice", true),
    ("fsd/no-layer-public-api", "fsd/no-layer-public-api", true),
    (
        "fsd/no-public-api-sidestep",
        "fsd/no-public-api-sidestep",
        true,
    ),
    (
        "fsd/no-reserved-folder-names",
        "fsd/no-reserved-folder-names",
        true,
    ),
    (
        "fsd/no-segmentless-slices",
        "fsd/no-segmentless-slices",
        true,
    ),
    (
        "fsd/no-segments-on-sliced-layers",
        "fsd/no-segments-on-sliced-layers",
        true,
    ),
    ("fsd/no-ui-in-app", "fsd/no-ui-in-app", true),
    ("fsd/public-api", "fsd/public-api", true),
    ("fsd/repetitive-naming", "fsd/repetitive-naming", true),
    ("fsd/segments-by-purpose", "fsd/segments-by-purpose", true),
    ("fsd/shared-lib-grouping", "fsd/shared-lib-grouping", true),
    ("fsd/typo-in-layer-name", "fsd/typo-in-layer-name", true),
    ("fsd/no-processes", "fsd/no-processes", true),
    ("fsd/no-cross-imports", "fsd/no-cross-imports", false),
    (
        "fsd/no-higher-level-imports",
        "fsd/no-higher-level-imports",
        false,
    ),
    ("fsd/import-locality", "fsd/import-locality", false),
    ("fsd/no-wildcard-exports", "fsd/no-wildcard-exports", false),
];

/// Who reports a path that the other does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Only {
    Archangel,
}

/// The intended differences of the presets spec, as (fixture, Steiger rule, path, who reports it).
const INTENDED: [(&str, &str, &str, Only); 3] = [
    // A folder one typo from a layer is reported even when that layer exists.
    (
        "imports",
        "fsd/typo-in-layer-name",
        "src/featurez",
        Only::Archangel,
    ),
    // Every folder in shared/ui and shared/lib needs its own index, even when the segment has one.
    (
        "structure",
        "fsd/public-api",
        "src/shared/ui/modal",
        Only::Archangel,
    ),
    // Every folder in a sliced layer is a slice, so `stuff`, which has no segment, is one, and
    // its file `helpers.ts` is a segment named for what it is.
    (
        "structure",
        "fsd/segments-by-purpose",
        "src/features/stuff/helpers.ts",
        Only::Archangel,
    ),
];

fn root() -> PathBuf {
    dunce::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

/// A path as a `file:` URL, which an ES module imports on every platform.
fn file_url(path: &Path) -> String {
    let plain = path.display().to_string().replace('\\', "/");
    if plain.starts_with('/') {
        format!("file://{plain}")
    } else {
        format!("file:///{plain}")
    }
}

fn fixtures() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(root().join("tests/fixtures/fsd"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect();
    found.sort();
    found
}

/// The fixture's files, copied into a repository of their own.
fn copy(fixture: &Path) -> FixtureRepo {
    let mut stack = vec![fixture.to_path_buf()];
    let mut repo = FixtureRepo::new();
    while let Some(folder) = stack.pop() {
        for entry in std::fs::read_dir(&folder).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let relative = path.strip_prefix(fixture).unwrap().display().to_string();
                repo = repo.bytes(&relative.replace('\\', "/"), &std::fs::read(&path).unwrap());
            }
        }
    }
    repo
}

/// What one tool reported, as (rule, path relative to the repository).
type Reported = BTreeSet<(String, String)>;

fn steiger(repo: &FixtureRepo, strict: bool) -> Reported {
    let plugin = root().join("node_modules/@feature-sliced/steiger-plugin/dist/index.js");
    let config = if strict {
        "[fsd.plugin, { rules: Object.fromEntries(fsd.plugin.ruleDefinitions.map((r) => [r.name, 'error'])) }]"
    } else {
        "fsd.configs.recommended"
    };
    repo.write(
        "steiger.config.mjs",
        &format!(
            "import fsd from '{}';\nexport default {config};\n",
            file_url(&plugin)
        ),
    );
    let output = Command::new("node")
        .arg(root().join("node_modules/steiger/dist/cli.mjs"))
        .args(["src", "--reporter", "json"])
        .current_dir(repo.path())
        .output()
        .expect("node runs Steiger");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let found: Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "Steiger prints JSON: {e}\n{stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    let base = dunce::canonicalize(repo.path()).unwrap();
    found
        .as_array()
        .into_iter()
        .flatten()
        .map(|d| {
            let path = PathBuf::from(d["location"]["path"].as_str().unwrap_or_default());
            let path = dunce::canonicalize(&path).unwrap_or(path);
            let relative = path
                .strip_prefix(&base)
                .unwrap_or(&path)
                .display()
                .to_string();
            (
                d["ruleName"].as_str().unwrap_or_default().to_owned(),
                relative.replace('\\', "/"),
            )
        })
        .collect()
}

fn archangel(repo: &FixtureRepo, strict: bool) -> Reported {
    let package = root().join("npm/archangel/index.js");
    repo.write(
        "archangel.config.mjs",
        &format!(
            "import {{ fsd }} from '{}';\nexport default {{ extends: [fsd({{ root: 'src', strict: {strict} }})] }};\n",
            file_url(&package)
        ),
    );
    let found = run(repo, &["check", "--format", "json"]);
    assert!(
        found.status <= 1,
        "the check ran: {}{}",
        found.stdout,
        found.stderr
    );
    let checked: Value = serde_json::from_str(&found.stdout).unwrap_or_else(|e| {
        panic!(
            "archangel prints JSON: {e}\n{}{}",
            found.stdout, found.stderr
        )
    });
    let mut reported = Reported::new();
    for d in checked["diagnostics"].as_array().into_iter().flatten() {
        let rule = d["rule"].as_str().unwrap_or_default();
        let file = d["file"].as_str().unwrap_or_default();
        assert!(
            rule.starts_with("fsd/"),
            "archangel reports only preset rules: {d}"
        );
        reported.insert((rule.to_owned(), file.to_owned()));
    }
    reported
}

/// For each Steiger rule, the paths only one tool reports, beyond the intended differences.
fn compare(strict: bool) -> Vec<String> {
    let mut disagreements = Vec::new();
    let mut broken: BTreeSet<String> = BTreeSet::new();
    for fixture in fixtures() {
        let name = fixture.file_name().unwrap().to_string_lossy().into_owned();
        let repo = copy(&fixture);
        let theirs = steiger(&repo, strict);
        let ours = archangel(&repo, strict);
        for (steiger_rule, preset_rule, recommended) in RULES {
            let on = strict || recommended;
            let paths = |found: &Reported, rule: &str| -> BTreeSet<String> {
                found
                    .iter()
                    .filter(|(r, _)| r == rule)
                    .map(|(_, p)| p.clone())
                    .collect()
            };
            let theirs = paths(&theirs, steiger_rule);
            let ours = paths(&ours, preset_rule);
            if !on {
                assert!(
                    ours.is_empty() && theirs.is_empty(),
                    "{name}: an opt-in rule {preset_rule} is off by default: {ours:?} {theirs:?}"
                );
                continue;
            }
            if !theirs.is_empty() {
                broken.insert(steiger_rule.to_owned());
            }
            let intended: BTreeSet<String> = INTENDED
                .iter()
                .filter(|(f, r, _, who)| {
                    *f == name && *r == steiger_rule && *who == Only::Archangel
                })
                .map(|(_, _, p, _)| (*p).to_owned())
                .collect();
            let only_ours: BTreeSet<String> = ours.difference(&theirs).cloned().collect();
            if only_ours != intended {
                disagreements.push(format!(
                    "{name} {steiger_rule}: only archangel {only_ours:?}, intended {intended:?}"
                ));
            }
            let only_theirs: Vec<&String> = theirs.difference(&ours).collect();
            if !only_theirs.is_empty() {
                disagreements.push(format!(
                    "{name} {steiger_rule}: only Steiger {only_theirs:?}"
                ));
            }
        }
    }
    let expected = RULES.iter().filter(|(_, _, r)| strict || *r).count();
    assert_eq!(
        broken.len(),
        expected,
        "the fixtures together break each rule that is on: {broken:?}"
    );
    disagreements
}

#[test]
fn the_fsd_preset_reports_what_steiger_reports() {
    let disagreements = compare(true);
    assert!(disagreements.is_empty(), "{}", disagreements.join("\n"));
}

#[test]
fn the_fsd_default_matches_steigers_recommended_rules() {
    let disagreements = compare(false);
    assert!(disagreements.is_empty(), "{}", disagreements.join("\n"));
}
