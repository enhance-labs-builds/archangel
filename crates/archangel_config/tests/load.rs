//! Scenarios from the configuration spec that need no Node: finding the config, judging the Node
//! that would load a TypeScript config, and reading the loader's answer.
#![cfg(test)]

use archangel_config::{ProblemCode, discover, from_loader, node_version_problem};
use archangel_testkit::FixtureRepo;
use serde_json::json;

#[test]
fn the_nearest_config_above_a_folder_is_found_and_typescript_comes_before_json() {
    let repo = FixtureRepo::new()
        .file("archangel.config.json", "{}")
        .file("archangel.config.ts", "export default {};")
        .file("apps/web/src/index.ts", "");
    let found = discover(&repo.join("apps/web/src"));
    assert_eq!(
        found,
        Some(repo.join("archangel.config.ts")),
        "the search climbs to the repository root and takes the first name archangel looks for"
    );
    let inner = FixtureRepo::new()
        .file("archangel.config.ts", "export default {};")
        .file("apps/web/archangel.config.json", "{}");
    assert_eq!(
        discover(&inner.join("apps/web")),
        Some(inner.join("apps/web/archangel.config.json")),
        "a config in the folder itself is nearer than one above it"
    );
}

#[test]
fn node_older_than_22_18_needs_a_newer_node_and_22_18_or_later_runs_the_config() {
    let judged: Vec<(&str, Option<ProblemCode>)> = [
        "v20.11.0",
        "v22.17.9",
        "v22.18.0",
        "v22.19.1",
        "v23.0.0",
        "v24.1.0\n",
    ]
    .into_iter()
    .map(|printed| (printed, node_version_problem(printed).map(|p| p.code)))
    .collect();
    let needs = Some(ProblemCode::ConfigNeedsNode);
    assert_eq!(
        judged,
        [
            ("v20.11.0", needs),
            ("v22.17.9", needs),
            ("v22.18.0", None),
            ("v22.19.1", None),
            ("v23.0.0", None),
            ("v24.1.0\n", None),
        ],
        "22.18 is the oldest Node that strips types"
    );
    let message = node_version_problem("v20.11.0")
        .map(|p| p.message)
        .unwrap_or_default();
    assert!(
        message.contains("20.11.0") && message.contains("22.18") && message.contains("JSON config"),
        "the message names both versions and the config that needs no Node: {message}"
    );
}

#[test]
fn a_node_version_that_is_not_a_version_needs_node() {
    let problem = node_version_problem("command not found").expect("not a version");
    assert_eq!(problem.code, ProblemCode::ConfigNeedsNode, "{problem:?}");
    assert!(
        problem.message.contains("`command not found`"),
        "the message shows what Node printed: {}",
        problem.message
    );
}

#[test]
fn the_loader_answers_with_the_config_or_the_error_that_stopped_it() {
    let config = from_loader(&json!({
        "type": "config",
        "config": { "elements": { "shared": "src/shared/**" }, "rules": [] },
        "files": [],
    }))
    .expect("the config is valid");
    assert_eq!(
        config
            .element("shared")
            .map(archangel_config::ElementId::index),
        Some(0),
        "the answer's config is validated"
    );
    let stopped = from_loader(&json!({
        "type": "error",
        "error": { "kind": "no-default-export" },
        "files": [],
    }))
    .expect_err("the config did not load");
    assert_eq!(
        (stopped.len(), stopped[0].code),
        (1, ProblemCode::ConfigLoadFailed),
        "{stopped:?}"
    );
    assert!(
        stopped[0].message.contains("no default export"),
        "{}",
        stopped[0].message
    );
    let strange = from_loader(&json!({ "type": "result" })).expect_err("not an answer");
    assert!(
        strange[0].code == ProblemCode::ConfigLoadFailed
            && strange[0].message.contains("instead of the config"),
        "{strange:?}"
    );
}

/// The message for the error the loader sends, as the developer reads it.
fn load_failure(error: &serde_json::Value) -> String {
    let problems = from_loader(&json!({ "type": "error", "error": error, "files": [] }))
        .expect_err("the config did not load");
    assert_eq!(
        problems[0].code,
        ProblemCode::ConfigLoadFailed,
        "{problems:?}"
    );
    problems[0].message.clone()
}

#[test]
fn each_load_failure_says_what_to_write_instead() {
    let unsupported = "ERR_UNSUPPORTED_TYPESCRIPT_SYNTAX";
    let cases = [
        (
            json!({ "kind": "thrown", "code": unsupported, "message": "TypeScript enum is not supported" }),
            "as const",
        ),
        (
            json!({ "kind": "thrown", "code": unsupported, "message": "TypeScript namespace is not supported" }),
            "plain JavaScript with types",
        ),
        (
            json!({ "kind": "thrown", "code": "ERR_MODULE_NOT_FOUND", "message": "Cannot find './rules'" }),
            "`.ts` extension",
        ),
        (
            json!({ "kind": "thrown", "code": null, "message": "the enum of colours is empty" }),
            "threw an error while it loaded: the enum of colours is empty",
        ),
    ];
    let found: Vec<bool> = cases
        .iter()
        .map(|(error, advice)| load_failure(error).contains(advice))
        .collect();
    assert_eq!(
        found,
        [true, true, true, true],
        "an enum, other syntax Node cannot strip, a missing import, and any other error each get \
         their own advice; an error that only mentions an enum is not about one"
    );
    assert!(
        !load_failure(&json!({ "kind": "thrown", "code": unsupported, "message": "enum" }))
            .contains("no default export"),
        "only the loader's own kind says the default export is missing"
    );
}

#[test]
fn only_a_config_ending_in_json_is_read_without_node() {
    let judged = [
        "archangel.config.json",
        "archangel.config.ts",
        "archangel.config.mjs",
        "json/archangel.config.js",
    ]
    .map(|name| archangel_config::is_json(std::path::Path::new(name)));
    assert_eq!(
        judged,
        [true, false, false, false],
        "the extension decides, not a folder's name"
    );
}
