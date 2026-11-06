//! Scenarios from the selectors spec: the forms a set of files takes, in a selector, in a rule's
//! files and on a rule's other side, and how a default name writes them.
#![cfg(test)]

#[path = "support/elements.rs"]
mod elements;

use archangel_config::{Config, ElementId, ProblemCode, SelectorId, Term, parse_json};
use elements::{declared_elements, only_problem};
use serde_json::{Value, json};

fn valid(config: &Value) -> Config {
    parse_json(&config.to_string())
        .unwrap_or_else(|problems| panic!("the config is valid: {problems:?}"))
}

/// The terms of each selector of a config of the declared elements.
fn selector_terms(selectors: &Value) -> Vec<Vec<Term>> {
    valid(&json!({ "elements": declared_elements(), "selectors": selectors }))
        .selectors
        .into_iter()
        .map(|selector| selector.definition.terms)
        .collect()
}

#[test]
fn each_form_of_a_set_of_files_reads_into_its_terms_each_combination_after_its_parts() {
    let entity = Term::Element(ElementId(0));
    let shared = Term::Element(ElementId(1));
    let terms = selector_terms(&json!({
        "list": ["entity", "shared"],
        "all": { "all": ["entity", { "not": "shared" }] },
        "any": { "any": ["entity", "shared"] },
        "client": { "directive": "use client" },
        "element": { "element": "shared" },
        "everywhere": { "element": "*" },
        "user": { "element": "entity", "capture": { "entity": "user" } },
    }));
    assert_eq!(
        terms,
        [
            vec![entity.clone(), shared.clone(), Term::Any(2)],
            vec![entity.clone(), shared.clone(), Term::Not, Term::All(2)],
            vec![entity.clone(), shared.clone(), Term::Any(2)],
            vec![Term::Directive("use client".into())],
            vec![shared],
            vec![Term::AnyElement],
            vec![Term::Instance {
                element: ElementId(0),
                captures: vec![("entity".into(), "user".into())],
            }],
        ],
        "a list is any of its parts, and each element form names its files"
    );
    let config = valid(&json!({
        "elements": declared_elements(),
        "selectors": { "a": "**/*.test.ts", "b": { "not": "a" } },
    }));
    assert_eq!(
        (config.selector("b"), config.selector("c")),
        (Some(SelectorId(1)), None),
        "a selector by its name"
    );
}

#[test]
fn a_directive_needs_its_text() {
    let problem = only_problem(&json!({
        "elements": declared_elements(),
        "selectors": { "client": { "directive": "" } },
    }));
    assert_eq!(
        problem.field.as_deref(),
        Some("selectors.client.directive"),
        "{problem:?}"
    );
}

#[test]
fn an_element_term_names_a_file_element_and_any_element_takes_no_capture() {
    let field_of = |term: Value| {
        only_problem(&json!({ "elements": declared_elements(), "selectors": { "s": term } }))
    };
    let package = field_of(json!({ "element": "react" }));
    let star = field_of(json!({ "element": "*", "capture": { "entity": "user" } }));
    assert_eq!(
        [
            (package.code, package.field.as_deref()),
            (star.code, star.field.as_deref())
        ],
        [
            (ProblemCode::RuleUnknownElement, Some("selectors.s.element")),
            (ProblemCode::RuleUnknownElement, Some("selectors.s.element")),
        ],
        "a package element selects no files, and no instance belongs to any element: \
         {package:?} {star:?}"
    );
    assert!(
        package
            .message
            .contains("the file elements are entity, shared"),
        "{}",
        package.message
    );
}

#[test]
fn an_instance_names_only_the_captures_its_element_has() {
    let problem_of = |capture: Value| {
        only_problem(&json!({
            "elements": declared_elements(),
            "selectors": { "s": { "element": "entity", "capture": capture } },
        }))
    };
    let unknown = problem_of(json!({ "feature": "login" }));
    assert_eq!(
        (
            unknown.code,
            unknown.field.as_deref(),
            unknown.message.as_str()
        ),
        (
            ProblemCode::RuleNeedsCapture,
            Some("selectors.s.capture.feature"),
            "the element \"entity\" has no capture `{feature}`; its captures are entity"
        ),
        "{unknown:?}"
    );
    let not_text = problem_of(json!({ "entity": 3 }));
    assert_eq!(
        (not_text.code, not_text.field.as_deref()),
        (
            ProblemCode::ConfigInvalid,
            Some("selectors.s.capture.entity")
        ),
        "{not_text:?}"
    );
    let empty = problem_of(json!({}));
    assert_eq!(
        empty.field.as_deref(),
        Some("selectors.s.capture"),
        "an instance needs at least one capture: {empty:?}"
    );
}

#[test]
fn the_other_side_takes_combinations_of_files_and_packages_by_name() {
    let config = valid(&json!({
        "elements": declared_elements(),
        "rules": [{
            "files": "shared",
            "mayNotImport": [{ "any": ["entity", "src/app/**"] }, { "packages": ["lodash", "@acme/*"] }],
        }],
    }));
    let side = config.rules[0].sentences[0]
        .condition
        .side()
        .expect("an import condition");
    let packages: Vec<&str> = side
        .packages
        .iter()
        .map(archangel_config::Pattern::source)
        .collect();
    assert_eq!(
        (
            side.files
                .iter()
                .map(|set| set.terms.clone())
                .collect::<Vec<_>>(),
            packages
        ),
        (
            vec![vec![
                Term::Element(ElementId(0)),
                Term::Pattern(archangel_config::PatternId(0)),
                Term::Any(2)
            ]],
            vec!["lodash", "@acme/*"]
        ),
        "a combination of files, and inline packages"
    );
}

#[test]
fn a_default_name_writes_each_set_of_files_as_the_config_wrote_it() {
    let config = valid(&json!({
        "elements": declared_elements(),
        "rules": [
            { "files": [{ "element": "entity", "capture": { "entity": "user" } }, { "not": "shared" }],
              "noCycles": true },
            { "files": { "all": ["entity", { "element": "*" }, { "not": "shared" }] },
              "noCycles": true },
            { "files": { "any": [{ "element": "shared" }, "src/app/**"] }, "noCycles": true },
            { "files": { "directive": "use client" },
              "mayNotImport": ["shared", { "packages": ["react", "react-dom"] }] },
        ],
    }));
    let names: Vec<&str> = config.rules.iter().map(|rule| rule.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "noCycles(entity entity user | not shared)",
            "noCycles(all of (entity, any element, not shared))",
            "noCycles(any of (shared, src/app/**))",
            "mayNotImport('use client' files, shared | react | react-dom)",
        ],
        "each set in a few words, one level deep"
    );
}

#[test]
fn each_selector_comes_once_in_the_order_after_the_selectors_it_refers_to() {
    let config = valid(&json!({
        "elements": declared_elements(),
        "selectors": {
            "everything": { "any": ["tests", "stories"] },
            "tests": "**/*.test.ts",
            "stories": { "any": ["tests", "**/*.stories.tsx"] },
        },
    }));
    let order: Vec<&str> = config
        .selector_order
        .iter()
        .map(|id| config.selectors[id.0 as usize].name.as_str())
        .collect();
    assert_eq!(
        order,
        ["tests", "stories", "everything"],
        "tests before the two that refer to it, and once, though two refer to it"
    );
}

#[test]
fn a_tag_without_at_is_config_invalid() {
    let problem = only_problem(&json!({
        "elements": declared_elements(),
        "selectors": { "pure": { "tag": "side-effect-free" } },
    }));
    assert_eq!(problem.code, ProblemCode::ConfigInvalid, "{problem:?}");
    assert!(
        problem.message.contains("'@side-effect-free'"),
        "the message gives the tag with its @: {}",
        problem.message
    );
}

#[test]
fn isolate_still_refuses_a_group() {
    let problem = only_problem(&json!({
        "elements": declared_elements(),
        "selectors": { "layer": "src/shared/**" },
        "rules": [{ "kind": "isolate", "element": "layer" }],
    }));
    assert_eq!(
        problem.code,
        ProblemCode::RuleUnknownElement,
        "isolate needs a part with a capture, so a group is refused: {problem:?}"
    );
}

#[test]
fn a_root_file_is_named_with_dot_slash() {
    let config = valid(&json!({
        "elements": declared_elements(),
        "selectors": { "configs": ["./app.config.ts", "./src/**"] },
    }));
    let sources: Vec<&str> = config
        .patterns
        .iter()
        .map(archangel_config::Pattern::source)
        .collect();
    assert_eq!(
        sources,
        ["app.config.ts", "src/**"],
        "`./` names the config folder, so the pattern is the root file"
    );
}

#[test]
fn a_bare_file_name_suggests_dot_slash() {
    let problem = only_problem(&json!({
        "elements": declared_elements(),
        "selectors": { "configs": ["app.config.ts"] },
    }));
    assert!(
        problem
            .message
            .ends_with(", so a file at the root is written './app.config.ts'"),
        "the message says how to name the file: {}",
        problem.message
    );
}

#[test]
fn a_transitive_rule_refuses_a_package_subpath() {
    let problem = only_problem(&json!({
        "elements": declared_elements(),
        "rules": [{ "files": "shared", "mayNotImport": { "packages": ["lodash-es/*"] }, "transitively": true }],
    }));
    assert!(
        problem.message.contains("`lodash-es/*` cannot end a chain"),
        "a chain ends at a package, so the subpath is refused: {}",
        problem.message
    );
}

#[test]
fn a_rule_warns_only_with_severity_warn() {
    let warns = |severity: Value| {
        let mut rule = json!({ "files": "shared", "mayNotImport": "entity" });
        if !severity.is_null() {
            rule["severity"] = severity;
        }
        valid(&json!({ "elements": declared_elements(), "rules": [rule] })).rules[0].warns
    };
    assert_eq!(
        [
            warns(json!("warn")),
            warns(json!("error")),
            warns(Value::Null)
        ],
        [true, false, false],
        "'warn' warns; 'error' and no severity fail the check"
    );
    let problem = only_problem(&json!({
        "elements": declared_elements(),
        "rules": [{ "files": "shared", "mayNotImport": "entity", "severity": "info" }],
    }));
    assert_eq!(
        problem.field.as_deref(),
        Some("rules[0].severity"),
        "another value is refused at the field: {problem:?}"
    );
}

#[test]
fn a_name_suggests_dot_slash_only_when_it_ends_in_an_extension() {
    for name in ["configs", ".eslintrc", "notes.", "a.b-c"] {
        let problem = only_problem(&json!({
            "elements": declared_elements(),
            "selectors": { "s": [name] },
        }));
        assert!(
            !problem.message.contains("is written './"),
            "{name} has no extension after a name, so the message suggests no path: {}",
            problem.message
        );
    }
}

#[test]
fn a_module_is_its_package_and_the_subpath_after_it() {
    use archangel_config::{names_a_subpath, package_of_module};
    assert_eq!(
        [
            package_of_module("lodash-es"),
            package_of_module("lodash-es/fp/get"),
            package_of_module("@scope/tokens/core/a"),
            package_of_module("@scope/tokens"),
            package_of_module("node:fs/promises"),
        ],
        [
            "lodash-es",
            "lodash-es",
            "@scope/tokens",
            "@scope/tokens",
            "node:fs/promises"
        ],
        "the package is one name, or two for a scope; a built-in module is its own"
    );
    assert_eq!(
        [
            names_a_subpath("lodash-es/*"),
            names_a_subpath("lodash-es"),
            names_a_subpath("@scope/*"),
            names_a_subpath("@scope/tokens/core/*"),
            names_a_subpath("node:fs/promises"),
        ],
        [true, false, false, true, false],
        "a pattern names a subpath when it has more names than a package"
    );
}
