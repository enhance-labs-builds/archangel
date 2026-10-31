//! Scenarios from the configuration spec: how patterns match and which element a file is in.
#![cfg(test)]

use archangel_config::{ElementMatcher, Pattern, PatternId, parse_json};
use serde_json::json;

fn matcher(config: &serde_json::Value) -> (archangel_config::Config, ElementMatcher) {
    let config = parse_json(&config.to_string()).expect("a valid config");
    let matcher = ElementMatcher::new(&config);
    (config, matcher)
}

#[test]
fn capture_matches_one_folder_and_sets_the_instance_root() {
    let pattern = Pattern::parse("src/features/{feature}/**").unwrap();
    let found = pattern.matches("src/features/login/ui/form.tsx").unwrap();
    assert_eq!(found.captures, ["login"], "the capture is the folder name");
    assert_eq!(
        found.root, "src/features/login",
        "the root stops before the first wildcard"
    );
}

#[test]
fn capture_never_matches_an_empty_part() {
    let pattern = Pattern::parse("src/{name}.ts").unwrap();
    assert!(
        pattern.matches("src/.ts").is_none(),
        "a capture needs at least one character"
    );
    assert!(
        pattern.matches("src/a.ts").is_some(),
        "a one-character capture matches"
    );
}

#[test]
fn braces_with_a_comma_list_alternatives() {
    let (config, matcher) = matcher(&json!({
        "elements": { "ui": "src/{components,widgets}/**" },
    }));
    let ui = config.element("ui").unwrap();
    for path in ["src/components/a.ts", "src/widgets/b.ts"] {
        let found = matcher.classify_file(path).unwrap();
        assert_eq!(found.element, ui, "{path} is in ui");
        assert!(found.captures.is_empty(), "alternatives capture nothing");
    }
    assert!(
        matcher.classify_file("src/other/c.ts").is_none(),
        "other folders are not in ui"
    );
}

#[test]
fn windows_paths_match_with_forward_slashes() {
    let (config, matcher) = matcher(&json!({ "elements": { "shared": "src/shared/**" } }));
    let found = matcher.classify_file("src\\shared\\ui\\button.ts").unwrap();
    assert_eq!(
        found.element,
        config.element("shared").unwrap(),
        "backslashes act as slashes"
    );
}

#[test]
fn first_declared_element_wins() {
    let (config, matcher) = matcher(&json!({
        "elements": {
            "model": "src/**/*.model.ts",
            "feature": "src/features/{feature}/**",
        },
    }));
    let found = matcher
        .classify_file("src/features/login/session.model.ts")
        .unwrap();
    assert_eq!(
        found.element,
        config.element("model").unwrap(),
        "the earlier element wins"
    );
}

#[test]
fn package_scope_matches_every_package_in_it() {
    let (config, matcher) =
        matcher(&json!({ "elements": { "contracts": { "packages": ["@acme/*"] } } }));
    assert_eq!(
        matcher.classify_package("@acme/contract"),
        config.element("contracts"),
        "a scope pattern matches its packages"
    );
    assert_eq!(
        matcher.classify_package("react"),
        None,
        "other packages are in no element"
    );
}

#[test]
fn first_declared_package_element_wins() {
    let (config, matcher) = matcher(&json!({
        "elements": {
            "ui": { "packages": ["react"] },
            "vendor": { "packages": ["@*/*", "react"] },
        },
    }));
    assert_eq!(
        matcher.classify_package("react"),
        config.element("ui"),
        "the earlier element wins"
    );
}

/// The element a path is in, by name.
fn element_of(
    config: &archangel_config::Config,
    matcher: &ElementMatcher,
    path: &str,
) -> Option<String> {
    matcher
        .classify_file(path)
        .map(|class| config.elements[class.element.index()].name.clone())
}

#[test]
fn ten_thousand_exact_paths_each_classify_their_file() {
    let paths: Vec<String> = (0..10_000)
        .map(|i| format!("src/f{}/m{i}.ts", i % 97))
        .collect();
    let (config, matcher) = matcher(&json!({
        "elements": {
            "first": paths[..5_000],
            "second": paths[5_000..],
            "rest": "src/**",
        },
        "rules": [{ "name": "no-cycles", "files": "**", "noCycles": true }],
    }));
    let found: Vec<Option<String>> = ["src/f0/m0.ts", "src/f53/m5000.ts", "src/f1/other.ts"]
        .iter()
        .map(|path| element_of(&config, &matcher, path))
        .collect();
    assert_eq!(
        found,
        [
            Some("first".into()),
            Some("second".into()),
            Some("rest".into())
        ],
        "every exact path is in its element, and other files fall to the glob after them; a matcher \
         that drops its patterns would put all three in no element or in rest"
    );
}

#[test]
fn many_glob_patterns_classify_like_one_and_keep_the_declared_order() {
    let mut elements = serde_json::Map::new();
    for i in 0..3_000 {
        elements.insert(
            format!("e{i}"),
            json!(format!("src/{{layer}}/x{i}/**/*.{{ts,tsx}}")),
        );
    }
    elements.insert("rest".into(), json!("src/**"));
    let (config, matcher) = matcher(
        &json!({ "elements": elements, "rules": [{ "name": "no-cycles", "files": "**", "noCycles": true }] }),
    );
    assert_eq!(
        (
            element_of(&config, &matcher, "src/app/x2999/a/b.tsx"),
            element_of(&config, &matcher, "src/app/y/b.ts"),
        ),
        (Some("e2999".into()), Some("rest".into())),
        "the last glob still matches its file, so no pattern was dropped, and the catch-all comes after"
    );
    assert_eq!(
        matcher.path_match("src/app/x2999/a/b.tsx").passes,
        1,
        "three thousand globs compile into one set, so a path takes one pass, not one per glob"
    );
}

#[test]
fn a_path_that_only_a_rule_pattern_matches_is_in_no_element() {
    let (config, matcher) = matcher(&json!({
        "elements": { "shared": "src/shared/**" },
        "rules": [
            { "files": "lib/**", "mayNotImport": "src/shared/**" },
            { "files": "**/*.test.ts", "noCycles": true },
        ],
    }));
    let sources: Vec<&str> = config.patterns.iter().map(Pattern::source).collect();
    assert_eq!(
        sources,
        ["lib/**", "src/shared/**", "**/*.test.ts"],
        "each pattern the rules name, once, in the order they appear"
    );
    let named: Vec<Vec<PatternId>> = config
        .rules
        .iter()
        .map(|rule| rule.sentences[0].files.included_patterns())
        .collect();
    assert_eq!(
        named,
        [vec![PatternId(0)], vec![PatternId(2)]],
        "each rule's files refer to their pattern by its index"
    );
    let lib = matcher.path_match("lib/a.ts");
    assert_eq!(
        (lib.classification, lib.patterns),
        (None, vec![true, false, false]),
        "a rule's pattern, even the first, puts no file in an element"
    );
    let shared = matcher.path_match("src/shared/a.test.ts");
    assert_eq!(
        (
            shared.classification.map(|class| class.element.index()),
            shared.patterns
        ),
        (Some(0), vec![false, true, true]),
        "the element's pattern classifies the file and the rules' patterns still report it"
    );
}

#[test]
fn an_exact_path_declared_after_a_glob_that_matches_it_loses_to_the_glob() {
    let (config, matcher) = matcher(&json!({
        "elements": { "glob": "src/**", "exact": ["src/a.ts"] },
        "rules": [{ "name": "no-cycles", "files": "**", "noCycles": true }],
    }));
    assert_eq!(
        element_of(&config, &matcher, "src/a.ts"),
        Some("glob".into()),
        "the first declared element wins, whichever kind of pattern it has"
    );
}

#[test]
fn a_pattern_without_a_wildcard_capture_or_alternative_is_exact() {
    let exact = [
        "src/index.ts",
        "src/*.ts",
        "src/**",
        "src/{name}.ts",
        "src/{a,b}.ts",
    ]
    .map(|source| Pattern::parse(source).unwrap().is_exact());
    assert_eq!(
        exact,
        [true, false, false, false, false],
        "only a pattern that names one path is exact"
    );
}

#[test]
fn a_pattern_lists_its_capture_names_in_the_order_they_appear() {
    let pattern = Pattern::parse("src/{area}/{feature_2}/**/*.{ts,tsx}").unwrap();
    assert_eq!(
        pattern.captures(),
        ["area", "feature_2"],
        "the captures, and no name for the alternatives"
    );
}
