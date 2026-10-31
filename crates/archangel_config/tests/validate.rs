//! Scenarios from the configuration spec: a config that checks less than it says is refused, and
//! every problem is reported at once.
#![cfg(test)]

#[path = "support/elements.rs"]
mod elements;
#[path = "support/loading.rs"]
mod loading;
#[path = "support/validating.rs"]
mod support;

use archangel_config::{
    AllowUnresolved, ConfigProblem, ExampleFiles, ImportKinds, ProblemCode, RuleKind, parse_json,
};
use elements::{declared_elements, only_problem, problems};
use loading::loader_problem;
use serde_json::{Value, json};
use support::{every_kind, sentence_elements};

/// A problem with a field, where the config's text puts that field.
fn at(code: ProblemCode, field: &str, position: (usize, usize), message: &str) -> ConfigProblem {
    ConfigProblem {
        code,
        message: message.to_owned(),
        field: Some(field.to_owned()),
        position: Some(position),
    }
}

/// A config as the loader sends it: the plugin "team", described by its conditions, and `rules`.
fn with_team_plugin(schema_version: u32, rules: &Value) -> Value {
    let max = json!({ "type": "object", "properties": { "max": { "type": "number" } } });
    json!({
        "elements": declared_elements(),
        "plugins": { "team": { "schemaVersion": schema_version, "conditions": {
            "one-store-per-entity": { "description": "", "reads": ["imports"], "optionsSchema": max },
            "no-barrels": { "description": "", "reads": ["exports"], "optionsSchema": null },
        } } },
        "rules": rules,
    })
}

#[test]
fn unknown_field_is_config_invalid_with_its_location() {
    let problem = only_problem(&json!({
        "elements": declared_elements(),
        "rules": [{ "files": "src/**", "noCycles": true, "reasn": "typo" }],
    }));
    assert_eq!(
        problem.code,
        ProblemCode::ConfigInvalid,
        "an unknown field is config-invalid"
    );
    assert_eq!(
        problem.field.as_deref(),
        Some("rules[0].reasn"),
        "the problem names its field"
    );
}

#[test]
fn wrong_type_is_config_invalid_with_its_location() {
    let problem = only_problem(&json!({
        "elements": declared_elements(),
        "rules": [{ "files": "src/**", "noCycles": true, "imports": 5 }],
    }));
    assert_eq!(
        problem.code,
        ProblemCode::ConfigInvalid,
        "a wrong type is config-invalid"
    );
    assert_eq!(
        problem.field.as_deref(),
        Some("rules[0].imports"),
        "the problem names its field"
    );
    for value in ["'all'", "'runtime'", "'types'"] {
        assert!(problem.message.contains(value), "the message lists {value}");
    }
}

#[test]
fn every_problem_in_a_config_is_reported() {
    let found = problems(&json!({
        "elements": declared_elements(),
        "rules": [
            { "files": "src/**", "noCycles": true, "reasn": "typo" },
            { "files": "entity", "mayNotImport": "reactt" },
        ],
    }));
    let codes: Vec<ProblemCode> = found.iter().map(|p| p.code).collect();
    assert_eq!(
        codes,
        [ProblemCode::ConfigInvalid, ProblemCode::RuleUnknownElement],
        "both problems are reported"
    );
}

fn pattern_problem(pattern: &str) -> ConfigProblem {
    let problem = only_problem(&json!({ "elements": { "x": pattern } }));
    assert_eq!(
        problem.code,
        ProblemCode::PatternInvalid,
        "{pattern} is pattern-invalid"
    );
    assert!(
        problem.message.contains("\"x\""),
        "the message names the element"
    );
    assert!(
        problem.message.contains(pattern),
        "the message names the pattern"
    );
    problem
}

#[test]
fn double_star_inside_a_name_is_pattern_invalid() {
    pattern_problem("src/ui**/x");
}

#[test]
fn empty_folder_name_is_pattern_invalid() {
    pattern_problem("src//x");
}

#[test]
fn capture_without_a_name_is_pattern_invalid() {
    pattern_problem("src/{}/x");
}

#[test]
fn repeated_capture_name_is_pattern_invalid() {
    let problem = pattern_problem("src/{a}/{a}");
    assert!(
        problem.message.contains("`a`"),
        "the message names the capture"
    );
}

#[test]
fn rule_naming_an_undeclared_element_is_rule_unknown_element() {
    let problem = only_problem(&json!({
        "elements": declared_elements(),
        "rules": [{ "files": "entity", "mayNotImport": "reactt" }],
    }));
    assert_eq!(
        problem.code,
        ProblemCode::RuleUnknownElement,
        "an undeclared element is refused"
    );
    assert!(
        problem.message.contains("reactt"),
        "the message names the element"
    );
    for declared in ["entity", "shared", "react"] {
        assert!(
            problem.message.contains(declared),
            "the message lists {declared}"
        );
    }
}

#[test]
fn isolate_without_a_capture_is_rule_needs_capture() {
    let problem = only_problem(&json!({
        "elements": declared_elements(),
        "rules": [{ "kind": "isolate", "element": "shared" }],
    }));
    assert_eq!(
        problem.code,
        ProblemCode::RuleNeedsCapture,
        "isolate needs a capture"
    );
    assert!(
        problem.message.contains("\"shared\""),
        "the message names the element"
    );
}

#[test]
fn rule_without_a_name_gets_one_from_its_kind_and_elements() {
    let config = parse_json(
        &json!({
            "elements": declared_elements(),
            "rules": [{ "kind": "entry", "element": "entity", "files": ["index.ts"] }],
        })
        .to_string(),
    )
    .unwrap();
    assert_eq!(
        config.rules[0].name, "entry(entity)",
        "the name comes from the kind and element"
    );
}

#[test]
fn two_rules_with_one_name_is_rule_name_duplicate() {
    let problem = only_problem(&json!({
        "elements": declared_elements(),
        "rules": [
            { "files": "entity", "mayNotImport": "react" },
            { "files": "entity", "mayNotImport": "react", "imports": "runtime" },
        ],
    }));
    assert_eq!(
        problem.code,
        ProblemCode::RuleNameDuplicate,
        "a shared name is refused"
    );
    assert!(
        problem.message.contains("mayNotImport(entity, react)"),
        "the message names the rule"
    );
    assert!(
        problem.message.contains("`name`"),
        "the message says to give each rule a name"
    );
}

#[test]
fn misspelled_elements_field_is_reported_with_every_rule_that_needs_it() {
    let found = problems(&json!({
        "elemen1s": { "app": "src/app/**", "feature": "src/features/{feature}/**" },
        "rules": [
            { "kind": "layers", "order": ["app", "feature"] },
            { "kind": "isolate", "element": "feature" },
        ],
    }));
    let codes: Vec<ProblemCode> = found.iter().map(|p| p.code).collect();
    assert!(
        codes.contains(&ProblemCode::ConfigInvalid)
            && codes.contains(&ProblemCode::RuleUnknownElement),
        "the unknown field and the undeclared elements, found by the config fuzz target: {found:?}"
    );
}

#[test]
fn a_problem_in_a_json_config_has_the_line_and_column_of_its_field() {
    let text = "{\n  \"elements\": { \"shared\": \"src/shared/**\" },\n  \"rules\": [\n    { \"files\": \"src/**\", \"noCycles\": true,\n      \"reasn\": \"typo\" }\n  ]\n}\n";
    let found = parse_json(text).expect_err("the config has a problem");
    assert_eq!(
        found[0].position,
        Some((5, 7)),
        "the key's opening quote, from 1"
    );
}

#[test]
fn a_missing_field_points_at_the_object_that_lacks_it() {
    let text = "{\n  \"elements\": { \"shared\": \"src/shared/**\" },\n  \"rules\": [\n    { \"mayNotImport\": \"shared\" }\n  ]\n}\n";
    let found = parse_json(text).expect_err("the rule has no files");
    assert_eq!(
        found[0].field.as_deref(),
        Some("rules[0].files"),
        "the missing field"
    );
    assert_eq!(found[0].position, Some((4, 5)), "the rule that lacks it");
}

#[test]
fn a_misspelt_field_kind_or_element_gets_the_nearest_name() {
    let cases = [
        (
            json!({ "elements": declared_elements(), "rules": [{ "files": "src/**", "noCycles": true, "reasn": "x" }] }),
            "did you mean `reason`?",
        ),
        (
            json!({ "elements": declared_elements(), "rules": [{ "kind": "isolat", "element": "entity" }] }),
            "did you mean `isolate`?",
        ),
        (
            json!({ "elements": declared_elements(), "rules": [{ "kind": "isolate", "element": "entiy" }] }),
            "did you mean \"entity\"?",
        ),
        (
            json!({ "elemnts": declared_elements(), "rules": [] }),
            "did you mean `elements`?",
        ),
    ];
    for (config, suggestion) in cases {
        let found = problems(&config);
        assert!(
            found
                .iter()
                .any(|problem| problem.message.contains(suggestion)),
            "{suggestion} in {found:?}"
        );
    }
    let far = only_problem(
        &json!({ "elements": declared_elements(), "rules": [{ "files": "src/**", "noCycles": true, "colour": "x" }] }),
    );
    assert!(
        !far.message.contains("did you mean"),
        "no suggestion when nothing is near: {}",
        far.message
    );
}

#[test]
fn a_json_config_may_name_its_schema() {
    let config = json!({ "$schema": "./node_modules/@enhance-eng/archangel/archangel.schema.json", "elements": declared_elements(), "rules": [] });
    assert!(
        parse_json(&config.to_string()).is_ok(),
        "`$schema` is for editors and checks nothing"
    );
    let wrong =
        only_problem(&json!({ "$schema": 1, "elements": declared_elements(), "rules": [] }));
    assert_eq!(
        wrong.field.as_deref(),
        Some("$schema"),
        "but it must be a string"
    );
}

#[test]
fn a_parse_error_at_the_start_of_a_line_has_column_one() {
    let found = parse_json("{\n").expect_err("the text ends early");
    assert_eq!(
        found[0].position,
        Some((2, 1)),
        "serde_json's column 0 is the first column"
    );
}

#[test]
fn a_field_after_a_number_or_a_boolean_has_its_own_line_and_column() {
    let text = r#"{
  "elements": { "shared": "src/shared/**" },
  "rules": [
    12,
    { "files": "src/**", "noCycles": true, "imports": true},
    { "files": "src/**", "noCycles": true, "reasn": "typo" }
  ]
}"#;
    assert_eq!(
        parse_json(text).expect_err("every rule has a problem"),
        [
            at(
                ProblemCode::ConfigInvalid,
                "rules[0]",
                (4, 5),
                "must be a rule object"
            ),
            at(
                ProblemCode::ConfigInvalid,
                "rules[1].imports",
                (5, 44),
                "must be 'all', 'runtime' or 'types'"
            ),
            at(
                ProblemCode::ConfigInvalid,
                "rules[2].reasn",
                (6, 44),
                "archangel does not know this field; did you mean `reason`? The known fields are \
                 files, folders, name, reason, link, imports, examples, mayNotImport, \
                 mayOnlyImport, mayOnlyBeImportedBy, mustBeNamed, mustContain, mustNotExist, \
                 mustExport, mayNotExport, mustOnlyReExport, noCycles, names, specifier, \
                 instance, dependency, transitively, stopAt"
            ),
        ],
        "a value of several characters is one array item, and a value right before `}}` does not \
         take the brace with it"
    );
}

#[test]
fn a_long_element_name_tolerates_more_typos_than_a_short_one() {
    let text = r#"{
  "elements": { "notifications": "src/n/**", "permissions": "src/p/**" },
  "rules": [{ "files": "ntfcations", "mayNotImport": "prmssons" }]
}"#;
    assert_eq!(
        parse_json(text).expect_err("neither element is declared"),
        [
            at(
                ProblemCode::RuleUnknownElement,
                "rules[0].files",
                (3, 15),
                "the config declares no element or selector \"ntfcations\"; did you mean \
                 \"notifications\"? the declared names are notifications, permissions; a package \
                 is written { packages: ['ntfcations'] }, and a path pattern has a `/` or a `*`"
            ),
            at(
                ProblemCode::RuleUnknownElement,
                "rules[0].mayNotImport",
                (3, 38),
                "the config declares no element or selector \"prmssons\"; the declared names are \
                 notifications, permissions; a package is written { packages: ['prmssons'] }, and \
                 a path pattern has a `/` or a `*`"
            ),
        ],
        "three typos are near a name of thirteen letters, a quarter of its length, but not a name \
         of eleven, which allows two"
    );
}

#[test]
fn a_capture_name_is_letters_digits_and_underscores_and_does_not_start_with_a_digit() {
    let text = r#"{
  "elements": { "x": "src/{1a}/x", "y": "src/{a-b}/y", "z": "src/{a_b}/z" }
}"#;
    let needs = "needs a name of letters, digits and underscores that does not start with a digit";
    assert_eq!(
        parse_json(text).expect_err("two captures have bad names"),
        [
            at(
                ProblemCode::PatternInvalid,
                "elements.x",
                (2, 17),
                &format!(
                    "the pattern `src/{{1a}}/x` of element \"x\" is invalid: the capture `{{1a}}` \
                     {needs}"
                )
            ),
            at(
                ProblemCode::PatternInvalid,
                "elements.y",
                (2, 36),
                &format!(
                    "the pattern `src/{{a-b}}/y` of element \"y\" is invalid: the capture \
                     `{{a-b}}` {needs}"
                )
            ),
        ],
        "`{{a_b}}` is a valid name"
    );
}

#[test]
fn allow_unresolved_maps_each_specifier_to_its_reason() {
    let config = parse_json(
        &json!({
            "elements": declared_elements(),
            "allowUnresolved": { "virtual:*": "made by the bundler", "fs": "a Node built-in" },
        })
        .to_string(),
    )
    .expect("the config is valid");
    let entries: Vec<(&str, &str)> = config
        .allow_unresolved
        .iter()
        .map(|entry| (entry.specifier.as_str(), entry.reason.as_str()))
        .collect();
    assert_eq!(
        entries,
        [
            ("virtual:*", "made by the bundler"),
            ("fs", "a Node built-in")
        ],
        "each entry, in the order the config gives them"
    );
}

#[test]
fn allow_unresolved_without_a_reason_is_config_invalid() {
    let text = r#"{
  "elements": { "shared": "src/shared/**" },
  "allowUnresolved": {
    "virtual:*": "made by the bundler",
    "fs": "  ",
    "path": 1
  }
}"#;
    let message = "must give the reason the specifier cannot be resolved";
    assert_eq!(
        parse_json(text).expect_err("two entries give no reason"),
        [
            at(
                ProblemCode::ConfigInvalid,
                "allowUnresolved.fs",
                (5, 5),
                message
            ),
            at(
                ProblemCode::ConfigInvalid,
                "allowUnresolved.path",
                (6, 5),
                message
            ),
        ],
        "a blank reason is no reason, and neither is a number"
    );
}

#[test]
fn an_allowed_specifier_ending_in_a_star_permits_every_specifier_it_starts() {
    let allowed = |specifier: &str| AllowUnresolved {
        specifier: specifier.to_owned(),
        reason: "a reason".to_owned(),
    };
    let cases = [
        (allowed("virtual:*").permits("virtual:icons"), true),
        (allowed("virtual:*").permits("virtual:"), true),
        (allowed("virtual:*").permits("virtual"), false),
        (allowed("fs").permits("fs"), true),
        (allowed("fs").permits("fs/promises"), false),
        (allowed("fs").permits("path"), false),
    ];
    assert_eq!(
        cases.map(|(permits, _)| permits),
        cases.map(|(_, expected)| expected),
        "a star is a prefix; anything else names one specifier exactly"
    );
}

#[test]
fn each_rule_kind_parses_to_its_kind_and_default_name() {
    let config = every_kind();
    let id = |name: &str| config.element(name).unwrap();
    let names: Vec<&str> = config.rules.iter().map(|rule| rule.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "layers(entity, shared)",
            "isolate(entity)",
            "entry(entity)",
            "mayOnlyBeImportedBy(shared, entity)",
            "mayNotImport(entity, react)",
            "mayOnlyImport(not test, shared | zod)",
            "noCycles(src/**)",
        ],
        "each name comes from the kind or the condition and its sides"
    );
    let kinds = &config.rules;
    assert!(
        matches!(&kinds[0].kind, RuleKind::Layers { order } if *order == [id("entity"), id("shared")])
            && matches!(&kinds[1].kind, RuleKind::Isolate { element } if *element == id("entity"))
            && matches!(&kinds[2].kind, RuleKind::Entry { element, files }
                if *element == id("entity") && files.len() == 1 && files[0].source() == "index.ts")
            && kinds[3..]
                .iter()
                .all(|rule| matches!(rule.kind, RuleKind::Sentence)),
        "each rule has its kind: {kinds:?}"
    );
    let conditions: Vec<&str> = kinds
        .iter()
        .flat_map(|rule| rule.sentences.iter().map(|s| s.condition.name()))
        .collect();
    assert_eq!(
        conditions,
        [
            "mayNotImport",
            "mayNotImport",
            "entry",
            "mayOnlyBeImportedBy",
            "mayNotImport",
            "mayOnlyImport",
            "noCycles",
        ],
        "each shorthand compiles to its sentences, and each sentence has its condition"
    );
}

#[test]
fn isolate_accepts_an_element_whose_pattern_captures_a_name() {
    let config = parse_json(
        &json!({
            "elements": declared_elements(),
            "rules": [{ "kind": "isolate", "element": "entity" }],
        })
        .to_string(),
    )
    .expect("entity captures `{entity}`");
    let captures: Vec<bool> = config
        .elements
        .iter()
        .map(archangel_config::Element::has_capture)
        .collect();
    assert_eq!(
        captures,
        [true, false, false],
        "entity has a capture; shared has none, and a package element never has one"
    );
}

#[test]
fn imports_is_all_runtime_or_types_and_defaults_by_kind() {
    let config = parse_json(
        &json!({
            "elements": declared_elements(),
            "rules": [
                { "name": "a", "files": "entity", "mayNotImport": "react", "imports": "all" },
                { "name": "r", "files": "entity", "mayNotImport": "react", "imports": "runtime" },
                { "name": "t", "files": "entity", "mayNotImport": "react", "imports": "types" },
                { "name": "d", "files": "entity", "mayNotImport": "react" },
                { "files": "src/**", "noCycles": true },
            ],
        })
        .to_string(),
    )
    .expect("every rule is valid");
    let imports: Vec<ImportKinds> = config.rules.iter().map(|rule| rule.imports).collect();
    assert_eq!(
        imports,
        [
            ImportKinds::All,
            ImportKinds::Runtime,
            ImportKinds::Types,
            ImportKinds::All,
            ImportKinds::Runtime,
        ],
        "each value as given; without one, a cycle counts runtime imports and every other rule \
         counts all"
    );
}

#[test]
fn a_rule_keeps_its_name_reason_and_link() {
    let config = parse_json(
        &json!({
            "elements": declared_elements(),
            "rules": [{
                "files": "src/**",
                "noCycles": true,
                "name": "acyclic",
                "reason": "cycles break hot reload",
                "link": "https://example.com/cycles",
            }],
        })
        .to_string(),
    )
    .expect("the rule is valid");
    let rule = &config.rules[0];
    assert_eq!(
        (
            rule.name.as_str(),
            rule.reason.as_deref(),
            rule.link.as_deref()
        ),
        (
            "acyclic",
            Some("cycles break hot reload"),
            Some("https://example.com/cycles")
        ),
        "the given fields replace the default name and reach the diagnostics"
    );
}

#[test]
fn a_rule_keeps_its_accepted_and_rejected_examples() {
    let config = parse_json(
        &json!({
            "elements": declared_elements(),
            "rules": [{
                "files": "src/**",
                "noCycles": true,
                "examples": {
                    "accepted": [{ "src/a.ts": "export {};" }],
                    "rejected": [{ "src/a.ts": "import './b';", "src/b.ts": "import './a';" }],
                },
            }],
        })
        .to_string(),
    )
    .expect("the examples are valid");
    let files = |pairs: &[(&str, &str)]| -> ExampleFiles {
        pairs
            .iter()
            .map(|(path, text)| ((*path).to_owned(), (*text).to_owned()))
            .collect()
    };
    let examples = &config.rules[0].examples;
    assert_eq!(
        (&examples.accepted, &examples.rejected),
        (
            &vec![files(&[("src/a.ts", "export {};")])],
            &vec![files(&[
                ("src/a.ts", "import './b';"),
                ("src/b.ts", "import './a';")
            ])],
        ),
        "every file set with every file and its text"
    );
}

#[test]
fn an_example_path_outside_the_repository_is_config_invalid() {
    let text = r#"{
  "elements": { "shared": "src/shared/**" },
  "rules": [{ "noCycles": true, "files": "src/**", "examples": { "accepted": [
    { "": "", "/abs.ts": "", "src\\a.ts": "", "../a.ts": "", "src/../a.ts": "", "src//a.ts": "", "src/": "", "src/ok.ts": "" }
  ] } }]
}"#;
    let refused = |path: &str| {
        at(
            ProblemCode::ConfigInvalid,
            "rules[0].examples.accepted[0]",
            (4, 5),
            &format!(
                "`{path}` must be a path inside the repository, relative to the config folder"
            ),
        )
    };
    assert_eq!(
        parse_json(text).expect_err("seven paths are refused"),
        [
            refused(""),
            refused("/abs.ts"),
            refused("src\\a.ts"),
            refused("../a.ts"),
            refused("src/../a.ts"),
            refused("src//a.ts"),
            refused("src/"),
        ],
        "an empty, absolute, backslashed or escaping path, or one with an empty folder name, is \
         refused, and `src/ok.ts` is not"
    );
}

#[test]
fn json_that_does_not_parse_gives_serde_json_s_message_without_its_position() {
    let found = parse_json("{\n").expect_err("the text ends early");
    assert_eq!(
        found,
        [ConfigProblem {
            code: ProblemCode::ConfigInvalid,
            message: "the config is not valid JSON: EOF while parsing an object".to_owned(),
            field: None,
            position: Some((2, 1)),
        }],
        "the line and column are in the position, not the message"
    );
}

#[test]
fn each_problem_code_is_the_diagnostic_code_the_developer_sees() {
    let codes = [
        ProblemCode::ConfigMissing,
        ProblemCode::ConfigNeedsNode,
        ProblemCode::ConfigLoadFailed,
        ProblemCode::ConfigInvalid,
        ProblemCode::PatternInvalid,
        ProblemCode::RuleUnknownElement,
        ProblemCode::RuleNeedsCapture,
        ProblemCode::RuleNameDuplicate,
    ];
    assert_eq!(
        codes.map(ProblemCode::as_str),
        [
            "config-missing",
            "config-needs-node",
            "config-load-failed",
            "config-invalid",
            "pattern-invalid",
            "rule-unknown-element",
            "rule-needs-capture",
            "rule-name-duplicate",
        ],
        "the codes are part of the output people and editors read"
    );
}

#[test]
fn a_problem_displays_as_its_code_its_field_and_its_message() {
    let with_field = only_problem(&json!({
        "elements": declared_elements(),
        "rules": [{ "files": "src/**", "noCycles": true, "colour": "x" }],
    }));
    let without_field = archangel_config::missing(std::path::Path::new("/repo"));
    assert_eq!(
        [with_field.to_string(), without_field.to_string()],
        [
            "config-invalid: rules[0].colour: archangel does not know this field; the known fields \
             are files, folders, name, reason, link, imports, examples, mayNotImport, \
             mayOnlyImport, mayOnlyBeImportedBy, mustBeNamed, mustContain, mustNotExist, \
             mustExport, mayNotExport, mustOnlyReExport, noCycles, names, specifier, instance, \
             dependency, transitively, stopAt"
                .to_owned(),
            "config-missing: no config in /repo or any folder above it; archangel looks for \
             archangel.config.ts, archangel.config.mts, archangel.config.js, archangel.config.mjs, \
             archangel.config.json"
                .to_owned(),
        ],
        "the field comes between the code and the message when the problem has one"
    );
}

#[test]
fn a_rule_with_two_conditions_is_refused() {
    let problem = only_problem(&json!({
        "elements": sentence_elements(),
        "rules": [{ "files": "view", "mayOnlyImport": ["shared"], "mustExport": "{FileName}" }],
    }));
    assert_eq!(
        problem.code,
        ProblemCode::ConfigInvalid,
        "two conditions are refused"
    );
    assert_eq!(problem.field.as_deref(), Some("rules[0]"), "at the rule");
    assert!(
        problem.message.contains("`mayOnlyImport`")
            && problem.message.contains("`mustExport`")
            && problem.message.contains("two rules"),
        "the message names both conditions and says to write two rules: {}",
        problem.message
    );
}

#[test]
fn a_rule_with_no_condition_is_refused() {
    let problem = only_problem(&json!({
        "elements": sentence_elements(),
        "rules": [{ "name": "views", "files": "view" }],
    }));
    assert_eq!(
        problem.code,
        ProblemCode::ConfigInvalid,
        "a rule needs a condition"
    );
    assert_eq!(problem.field.as_deref(), Some("rules[0]"), "at the rule");
    assert!(
        problem.message.contains("no condition")
            && problem.message.contains("mayNotImport")
            && problem.message.contains("noCycles"),
        "the message says it has no condition and lists the conditions: {}",
        problem.message
    );
}

#[test]
fn a_rule_with_no_files_is_refused() {
    let problem = only_problem(&json!({
        "elements": sentence_elements(),
        "selectors": { "test": "**/*.test.ts" },
        "rules": [{ "name": "views", "mayNotImport": "test" }],
    }));
    assert_eq!(
        problem.code,
        ProblemCode::ConfigInvalid,
        "a rule needs files"
    );
    assert_eq!(
        problem.field.as_deref(),
        Some("rules[0].files"),
        "at the rule's files"
    );
    assert!(
        problem.message.contains("`files`") && problem.message.contains("`folders`"),
        "the message says it needs files or folders: {}",
        problem.message
    );
}

#[test]
fn a_removed_kind_is_refused_with_its_sentence() {
    let cases = [
        (
            json!({ "kind": "only", "element": "db", "importedBy": ["api"] }),
            r#"{ "files": "db", "mayOnlyBeImportedBy": ["api"] }"#,
        ),
        (
            json!({ "kind": "forbid", "from": ["view"], "to": ["db"] }),
            r#"{ "files": ["view"], "mayNotImport": ["db"] }"#,
        ),
        (
            json!({ "kind": "noCycles" }),
            r#"{ "files": "**", "noCycles": true }"#,
        ),
    ];
    for (rule, sentence) in cases {
        let problem = only_problem(&json!({ "elements": sentence_elements(), "rules": [rule] }));
        assert_eq!(
            problem.code,
            ProblemCode::ConfigInvalid,
            "a removed kind is refused"
        );
        assert_eq!(
            problem.field.as_deref(),
            Some("rules[0].kind"),
            "at the rule's kind"
        );
        assert!(
            problem.message.contains(sentence),
            "the message gives the sentence {sentence}: {}",
            problem.message
        );
    }
}

#[test]
fn instance_without_a_captured_element_is_rule_needs_capture() {
    let problem = only_problem(&json!({
        "elements": sentence_elements(),
        "rules": [{ "files": "src/**", "mayNotImport": "src/**", "instance": "different" }],
    }));
    assert_eq!(
        problem.code,
        ProblemCode::RuleNeedsCapture,
        "instance needs files to name an element with a capture"
    );
    assert_eq!(
        problem.field.as_deref(),
        Some("rules[0].instance"),
        "at the option"
    );
}

#[test]
fn a_transitive_rule_on_type_imports_is_refused() {
    let problem = only_problem(&json!({
        "elements": sentence_elements(),
        "rules": [{ "files": "src/**", "mayNotImport": "db", "transitively": true, "imports": "types" }],
    }));
    assert_eq!(
        problem.code,
        ProblemCode::ConfigInvalid,
        "a chain of type imports is refused"
    );
    assert!(
        problem.message.contains("never runs"),
        "the message says a chain of type imports never runs: {}",
        problem.message
    );
}

#[test]
fn a_selector_of_packages_is_refused() {
    let problem = only_problem(&json!({
        "elements": sentence_elements(),
        "selectors": { "react": { "packages": ["react"] } },
    }));
    assert_eq!(
        problem.code,
        ProblemCode::ConfigInvalid,
        "a selector selects files only"
    );
    assert_eq!(
        problem.field.as_deref(),
        Some("selectors.react"),
        "at the selector"
    );
    assert!(
        problem.message.contains("package element"),
        "the message says packages belong in a package element: {}",
        problem.message
    );
}

#[test]
fn selectors_that_refer_to_each_other_are_refused() {
    let problem = only_problem(&json!({
        "elements": sentence_elements(),
        "selectors": { "a": { "not": "b" }, "b": { "any": ["a"] } },
    }));
    assert_eq!(
        problem.code,
        ProblemCode::ConfigInvalid,
        "a loop of selectors is refused"
    );
    assert!(
        problem.message.contains("\"a\" -> \"b\" -> \"a\""),
        "the message shows the chain: {}",
        problem.message
    );
}

#[test]
fn a_bare_string_that_names_nothing_is_refused() {
    let problem = only_problem(&json!({
        "elements": sentence_elements(),
        "rules": [{ "files": "src/**", "mayNotImport": "lodash" }],
    }));
    assert_eq!(
        problem.code,
        ProblemCode::RuleUnknownElement,
        "a string without `/` or `*` must be a declared name"
    );
    assert!(
        problem.message.contains("\"lodash\"")
            && problem.message.contains("{ packages: ['lodash'] }"),
        "the message says how to write the package: {}",
        problem.message
    );
}

// Plugins.

#[test]
fn a_rule_that_names_a_missing_condition_is_refused() {
    let rule = json!({ "files": "entity", "plugin": "team/one-store-per-entities" });
    let found = loader_problem(&with_team_plugin(1, &json!([rule])));
    assert_eq!(
        found.field.as_deref(),
        Some("rules[0].plugin"),
        "at the rule's plugin"
    );
    assert!(
        found
            .message
            .contains("did you mean `team/one-store-per-entity`?")
            && found.message.contains("team/no-barrels"),
        "the message suggests the near name and lists the loaded conditions: {}",
        found.message
    );
}

#[test]
fn plugin_options_are_checked_against_the_schema() {
    let rule = json!({ "files": "entity", "plugin": "team/one-store-per-entity",
                       "options": { "max": "twenty" } });
    let found = loader_problem(&with_team_plugin(1, &json!([rule])));
    assert_eq!(
        found.field.as_deref(),
        Some("rules[0].options"),
        "at the rule's options"
    );
    assert!(
        found.message.contains("`max`") && found.message.contains("number"),
        "the message says `max` must be a number: {}",
        found.message
    );
}

#[test]
fn a_plugin_for_another_schema_version_is_refused() {
    let rule = json!({ "files": "entity", "plugin": "team/no-barrels" });
    let message = loader_problem(&with_team_plugin(2, &json!([rule]))).message;
    assert!(
        ["\"team\"", "version 2", "version 1"]
            .iter()
            .all(|part| message.contains(part)),
        "the message names the plugin and both versions: {message}"
    );
}

#[test]
fn a_json_config_cannot_extend_a_preset() {
    let found = problems(&json!({
        "elements": declared_elements(),
        "extends": [{ "name": "fsd" }],
        "rules": [],
    }));
    let problem = found
        .iter()
        .find(|p| p.field.as_deref() == Some("extends"))
        .unwrap_or_else(|| panic!("a problem at extends: {found:?}"));
    assert_eq!(
        problem.code,
        ProblemCode::ConfigInvalid,
        "extends in JSON is config-invalid"
    );
    assert!(
        problem.message.contains("TypeScript or JavaScript"),
        "the message says presets need a TypeScript or JavaScript config: {}",
        problem.message
    );
}
