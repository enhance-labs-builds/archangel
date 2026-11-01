//! Scenarios from the command line spec: the formats for CI, the formats each command takes, and
//! the JSON of `explain` and `why`.
#![cfg(test)]

#[path = "support/cli.rs"]
mod support;

#[path = "support/fixtures.rs"]
mod fixtures;

use archangel_testkit::FixtureRepo;
use serde_json::{Value, json};
use support::run;

fn layered(rule_name: &str) -> FixtureRepo {
    FixtureRepo::new()
        .file("src/app/main.ts", "import { s } from '../shared/s';\n")
        .file("src/shared/s.ts", "export const s = 1;\n")
        .file("src/shared/bad.ts", "import { main } from '../app/main';\n")
        .json_config(&json!({
            "elements": { "app": "src/app/**", "shared": "src/shared/**" },
            "rules": [{ "kind": "layers", "name": rule_name, "order": ["app", "shared"],
                        "reason": "Shared code knows nothing of the app" }],
        }))
}

fn document(stdout: &str) -> Value {
    serde_json::from_str(stdout).unwrap_or_else(|e| panic!("one JSON document: {e}\n{stdout}"))
}

#[test]
fn github_format_prints_one_annotation_per_diagnostic_then_the_summary() {
    let found = run(&layered("layers"), &["check", "--format", "github"]);
    assert_eq!(found.status, 1, "a violation exits 1: {}", found.stderr);
    let lines: Vec<&str> = found.stdout.lines().collect();
    assert_eq!(
        lines[0],
        "::error file=src/shared/bad.ts,line=1,col=22,endColumn=35,title=archangel layers::\
         \"shared\" may not import \"app\", an earlier layer, through \"../app/main\"%0A\
         help: \"shared\" may import only \"shared\"%0A\
         reason: Shared code knows nothing of the app",
        "the annotation, with its place and every line of the diagnostic"
    );
    assert!(
        lines[1].starts_with("checked 3 files and 2 imports: 1 problem"),
        "the summary follows as a plain line, which GitHub prints as log: {:?}",
        lines[1]
    );
}

#[test]
fn github_format_escapes_what_would_end_a_property_or_the_message() {
    let found = run(&layered("a, b: 100%"), &["check", "--format", "github"]);
    assert!(
        found.stdout.contains("title=archangel a%2C b%3A 100%25::"),
        "commas, colons and percent signs in a property are escaped: {}",
        found.stdout
    );
}

#[test]
fn github_format_annotates_the_config_that_stops_a_command() {
    let repo =
        FixtureRepo::new().json_config(&json!({ "elements": { "app": "src/**" }, "rulez": [] }));
    let found = run(&repo, &["check", "--format", "github"]);
    assert_eq!(found.status, 2, "a bad config exits 2");
    assert!(
        found.stderr.starts_with(
            "::error file=archangel.config.json,line=5,col=3,title=archangel config-invalid::rulez:"
        ),
        "the annotation names the config: {}",
        found.stderr
    );
}

#[test]
fn sarif_format_is_a_sarif_log_with_every_code_as_a_rule() {
    let found = run(&layered("layers"), &["check", "--format", "sarif"]);
    assert_eq!(found.status, 1, "a violation exits 1: {}", found.stderr);
    let log = document(&found.stdout);
    assert_eq!(log["version"], "2.1.0", "SARIF 2.1.0");
    let driver = &log["runs"][0]["tool"]["driver"];
    assert_eq!(driver["name"], "archangel", "the tool");
    assert_eq!(driver["version"], env!("CARGO_PKG_VERSION"), "its version");
    assert_eq!(
        log["runs"][0]["columnKind"], "unicodeCodePoints",
        "columns count characters, as the other formats do, not SARIF's default UTF-16 units"
    );
    let rules = driver["rules"].as_array().unwrap();
    assert_eq!(rules.len(), 44, "every code is a rule");
    let layers = rules.iter().position(|r| r["id"] == "layers").unwrap();
    assert!(
        rules[layers]["help"]["markdown"]
            .as_str()
            .unwrap()
            .starts_with("# layers"),
        "each rule's help is its documentation page"
    );
}

#[test]
fn a_sarif_result_has_the_code_place_rule_and_identity_of_its_violation() {
    let log = document(&run(&layered("layers"), &["check", "--format", "sarif"]).stdout);
    let rules = log["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .unwrap();
    let layers = rules.iter().position(|r| r["id"] == "layers").unwrap();
    let result = &log["runs"][0]["results"][0];
    assert_eq!(result["ruleId"], "layers", "the code");
    assert_eq!(result["ruleIndex"], layers, "the index of its rule");
    assert_eq!(result["level"], "error", "every diagnostic is an error");
    let location = &result["locations"][0]["physicalLocation"];
    assert_eq!(
        location["artifactLocation"]["uri"], "src/shared/bad.ts",
        "the file"
    );
    assert_eq!(
        location["artifactLocation"]["uriBaseId"], "SRCROOT",
        "relative to the config folder"
    );
    assert_eq!(
        location["region"],
        json!({ "startLine": 1, "startColumn": 22, "endColumn": 35 }),
        "the specifier, quotes and all"
    );
    assert!(
        result["partialFingerprints"]["archangelViolation/v1"].is_string(),
        "the violation's identity, which survives moved lines, lets code scanning follow it"
    );
    assert_eq!(
        result["properties"]["rule"], "layers",
        "the rule's name from the config"
    );
}

#[test]
fn a_command_refuses_a_format_it_cannot_print() {
    let found = run(
        &layered("layers"),
        &["explain", "src/app/main.ts", "--format", "sarif"],
    );
    assert_eq!(found.status, 2, "the command line stops it");
    assert_eq!(
        found.stderr, "error: `archangel explain` prints --format text or json, not sarif\n",
        "it names the formats the command takes"
    );
}

#[test]
fn explain_as_json_gives_the_element_rules_imports_and_importers() {
    let found = run(
        &layered("layers"),
        &["explain", "src/shared/bad.ts", "--format", "json"],
    );
    assert_eq!(found.status, 0, "explain answers: {}", found.stderr);
    assert_eq!(
        document(&found.stdout),
        json!({
            "schemaVersion": 1,
            "file": "src/shared/bad.ts",
            "element": { "name": "shared", "instance": [] },
            "selectors": [],
            "rules": ["layers"],
            "sentences": [{ "rule": "layers", "sentence": "files in shared may not import app" }],
            "imports": [{
                "line": 1,
                "specifier": "../app/main",
                "resolution": { "file": "src/app/main.ts" },
                "element": { "name": "app", "instance": [] },
                "breaks": ["layers"],
            }],
            "unfollowedLines": [],
            "importedBy": [],
        }),
        "the same answer as the text, as data"
    );
}

#[test]
fn why_as_json_gives_the_chain_or_null() {
    let found = run(
        &layered("layers"),
        &[
            "why",
            "src/shared/bad.ts",
            "src/shared/s.ts",
            "--format",
            "json",
        ],
    );
    assert_eq!(found.status, 0, "a chain exists: {}", found.stderr);
    assert_eq!(
        document(&found.stdout),
        json!({
            "schemaVersion": 1,
            "from": "src/shared/bad.ts",
            "to": "src/shared/s.ts",
            "chain": [
                { "file": "src/shared/bad.ts", "line": 1, "specifier": "../app/main" },
                { "file": "src/app/main.ts", "line": 1, "specifier": "../shared/s" },
            ],
        }),
        "each step names the importing file, the line and the specifier"
    );
    let none = run(
        &layered("layers"),
        &["why", "src/shared/s.ts", "react", "--format", "json"],
    );
    assert_eq!(none.status, 1, "no chain exits 1");
    assert_eq!(
        document(&none.stdout)["chain"],
        Value::Null,
        "no chain is null"
    );
}

#[test]
fn docs_prints_a_codes_page() {
    let found = run(&FixtureRepo::new(), &["docs", "entry"]);
    assert_eq!(found.status, 0, "a known code: {}", found.stderr);
    assert!(
        found.stdout.starts_with("# entry\n"),
        "its page: {}",
        found.stdout
    );
}

#[test]
fn docs_of_an_unknown_code_suggests_the_nearest_and_lists_every_code() {
    let found = run(&FixtureRepo::new(), &["docs", "lyers"]);
    assert_eq!(found.status, 2, "an unknown code stops it");
    assert!(
        found
            .stderr
            .starts_with("error: archangel has no code \"lyers\"; did you mean \"layers\"?\n"),
        "the nearest code: {}",
        found.stderr
    );
    assert!(
        found.stderr.contains("example-failed"),
        "and every code: {}",
        found.stderr
    );
}

#[test]
fn text_output_ends_with_where_to_read_about_the_codes_it_printed() {
    let found = run(&layered("layers"), &["check"]);
    assert!(
        found
            .stdout
            .ends_with("For what a code means, run `archangel docs layers`.\n"),
        "the hint names the codes printed: {}",
        found.stdout
    );
    let clean = FixtureRepo::new()
        .file("src/app/main.ts", "")
        .json_config(&json!({ "elements": { "app": "src/app/**" }, "rules": [] }));
    assert!(
        !run(&clean, &["check"]).stdout.contains("archangel docs"),
        "no hint with no diagnostics"
    );
}

#[test]
fn a_command_that_prints_one_format_names_it_alone() {
    let found = run(&layered("layers"), &["lsp", "--format", "json"]);
    assert_eq!(found.status, 2, "the command line stops it");
    assert_eq!(
        found.stderr, "error: `archangel lsp` prints --format text, not json\n",
        "one format is named on its own"
    );
}

fn misspelled_config() -> FixtureRepo {
    FixtureRepo::new().json_config(&json!({ "elements": { "app": "src/**" }, "rulez": [] }))
}

const RULEZ: &str = "rulez: archangel does not know this field; did you mean `rules`? The known fields are $schema, parts, elements, groups, selectors, externals, rules, ignore, allowUnresolved, plugins, extends, off";

#[test]
fn json_format_prints_the_config_that_stops_a_command_as_one_document() {
    let found = run(&misspelled_config(), &["check", "--format", "json"]);
    assert_eq!(found.status, 2, "a bad config exits 2");
    let expected = json!({
        "schemaVersion": 1,
        "diagnostics": [{
            "code": "config-invalid", "rule": null, "file": "archangel.config.json", "line": 5,
            "column": 3, "span": null, "specifier": null, "message": RULEZ, "help": null,
            "reason": null, "link": null, "id": null, "cycle": [], "group": [], "chain": [], "fix": null,
        }],
    });
    assert_eq!(
        (found.stdout.as_str(), found.stderr),
        (
            "",
            format!("{}\n", serde_json::to_string_pretty(&expected).unwrap())
        ),
        "the document of the diagnostics that stopped it, as check's own document has them"
    );
}

/// A repository in a folder whose name a URL must escape.
fn in_a_folder_with_a_space(config: &Value) -> FixtureRepo {
    FixtureRepo::new()
        .file("my repo#1/src/a.ts", "import './b';\n")
        .file("my repo#1/src/b.ts", "import './a';\n")
        .file("my repo#1/sub/notes.txt", "")
        .file("my repo#1/archangel.config.json", &config.to_string())
}

/// The `file:` URL of the repository's folder, ending in a slash, as SARIF's base URIs must.
fn folder_url(repo: &FixtureRepo) -> String {
    let folder = dunce::canonicalize(repo.join("my repo#1")).unwrap();
    let path = folder
        .to_string_lossy()
        .replace('\\', "/")
        .replace(' ', "%20")
        .replace('#', "%23");
    let rooted = if path.starts_with('/') {
        path
    } else {
        format!("/{path}")
    };
    format!("file://{rooted}/")
}

#[test]
fn sarif_base_uri_is_the_config_folder_as_a_file_url() {
    let repo = in_a_folder_with_a_space(
        &json!({ "elements": { "src": "src/**" }, "rules": [{ "name": "no-cycles", "files": "**", "noCycles": true }] }),
    );
    let found: support::Finished = support::command(&repo)
        .current_dir(repo.join("my repo#1/sub"))
        .args(["check", "--format", "sarif"])
        .output()
        .unwrap()
        .into();
    assert_eq!(found.status, 1, "the cycle exits 1: {}", found.stderr);
    let log = document(&found.stdout);
    assert_eq!(
        log["runs"][0]["originalUriBaseIds"],
        json!({ "SRCROOT": { "uri": folder_url(&repo) } }),
        "the folder the results' paths are relative to"
    );
}

#[test]
fn sarif_base_uri_of_a_stopped_command_is_the_config_folder() {
    let repo = in_a_folder_with_a_space(&json!({ "elements": { "src": "src/**" }, "rulez": [] }));
    let found: support::Finished = support::command(&repo)
        .current_dir(repo.join("my repo#1/sub"))
        .args(["check", "--format", "sarif"])
        .output()
        .unwrap()
        .into();
    assert_eq!(found.status, 2, "a bad config exits 2");
    let log = document(&found.stderr);
    let run = &log["runs"][0];
    assert_eq!(
        (
            &run["originalUriBaseIds"],
            &run["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"]
        ),
        (
            &json!({ "SRCROOT": { "uri": folder_url(&repo) } }),
            &json!({ "uri": "archangel.config.json", "uriBaseId": "SRCROOT" })
        ),
        "the config's path is relative to its own folder, not to the folder the command ran in"
    );
}

/// A repository whose tsconfig does not parse: a diagnostic about a whole file, at no line.
fn bad_tsconfig() -> FixtureRepo {
    FixtureRepo::new()
        .file("src/a.ts", "import './b';\n")
        .file("src/b.ts", "import './c';\n")
        .file("src/c.ts", "")
        .file(
            "tsconfig.json",
            "{\n  \"compilerOptions\": {\n    \"strict\": true\n    \"baseUrl\": \".\"\n  }\n}\n",
        )
        .json_config(&json!({ "elements": { "src": "src/**" }, "rules": [] }))
}

const TSCONFIG: &str =
    "it is not valid JSON at line 4, column 5: expected `,` or `}` at line 4 column 5";

#[test]
fn a_diagnostic_about_a_whole_file_names_the_file_and_no_line() {
    let repo = bad_tsconfig();
    let text = run(&repo, &["check"]);
    assert_eq!(text.status, 1, "the tsconfig is a problem: {}", text.stderr);
    assert_eq!(
        text.stdout,
        format!(
            "error[resolution-file-invalid]: {TSCONFIG}\n\
             --> tsconfig.json\n\n\
             checked 3 files and 2 imports: 1 problem\n\
             For what a code means, run `archangel docs resolution-file-invalid`.\n"
        ),
        "text names the file without a line or column"
    );
    let github = run(&repo, &["check", "--format", "github"]);
    assert_eq!(
        github.stdout.lines().next(),
        Some(
            format!(
                "::error file=tsconfig.json,title=archangel resolution-file-invalid::{TSCONFIG}"
            )
            .as_str()
        ),
        "the annotation has a file and no line: {}",
        github.stdout
    );
    let sarif = document(&run(&repo, &["check", "--format", "sarif"]).stdout);
    assert_eq!(
        sarif["runs"][0]["results"][0]["locations"],
        json!([{ "physicalLocation": {
            "artifactLocation": { "uri": "tsconfig.json", "uriBaseId": "SRCROOT" },
        } }]),
        "the location has a file and no region"
    );
}

#[test]
fn rules_as_json_gives_each_rule_with_its_reach() {
    let found = run(
        &fixtures::fixture("small-app"),
        &["rules", "--format", "json"],
    );
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    let doc = document(&found.stdout);
    let views = doc["rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|rule| rule["name"] == "feature-code.views")
        .expect("an entry for the row");
    assert_eq!(
        views["sentences"][0],
        "views may import only models, shared, and itself"
    );
    assert_eq!(views["table"], "feature-code");
    assert_eq!(views["row"], "views");
    assert_eq!(views["file"], "archangel.config.ts");
    assert!(views["line"].as_u64().unwrap() > 1, "{views}");
    assert!(
        views["because"]
            .as_str()
            .unwrap()
            .contains("A view shows a model")
    );
    assert_eq!(
        views["reach"],
        json!({ "files": 2, "importsJudged": 3, "problems": 1, "exceptions": 0, "baselineMatches": 0 })
    );
    assert_eq!(doc["notJudged"], json!({ "files": [], "imports": [] }));
    assert_eq!(
        doc["outsidePackages"],
        json!([{ "name": "react", "imports": 2 }])
    );
}

#[test]
fn explain_of_one_import_as_json() {
    let found = run(
        &fixtures::fixture("small-app"),
        &[
            "explain",
            "src/features/cart/views/cart-page.tsx:2",
            "--format",
            "json",
        ],
    );
    assert_eq!(found.status, 0, "{}{}", found.stdout, found.stderr);
    let doc = document(&found.stdout);
    let import = &doc["imports"][0];
    assert_eq!(
        (&import["specifier"], &import["target"]),
        (
            &json!("../api/fetch-cart"),
            &json!({ "file": "src/features/cart/api/fetch-cart.ts" })
        ),
        "the import and its target"
    );
    assert_eq!(
        import["from"],
        json!({ "part": "feature", "instance": ["cart"], "kinds": [{ "table": "feature-code", "kind": "views" }] }),
        "the importing end"
    );
    assert_eq!(
        import["to"]["kinds"],
        json!([{ "table": "feature-code", "kind": "api" }]),
        "the kind of the target"
    );
    let of = |name: &str| {
        let rules = import["rules"].as_array().unwrap();
        rules
            .iter()
            .find(|rule| rule["rule"] == name)
            .unwrap()
            .clone()
    };
    assert_eq!(of("feature-code.views")["verdict"], "refuses");
    assert_eq!(of("app.feature")["verdict"], "permits");
    assert_eq!(of("app.shared")["verdict"], "skips");
    assert!(
        of("app.shared")["reason"]
            .as_str()
            .unwrap()
            .contains("not in shared")
    );
}
