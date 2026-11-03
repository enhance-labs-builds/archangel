//! Scenarios about where a file or a package is: its place in each set of elements, which instance
//! it is in, and what a kind pattern reads from its path.
#![cfg(test)]

use archangel_config::{Config, ElementId, ElementMatcher, Term, from_value};
use serde_json::{Value, json};

fn valid(config: &Value) -> Config {
    from_value(config).unwrap_or_else(|problems| panic!("the config is valid: {problems:?}"))
}

/// A preset rooted at `app`.
fn preset(name: &str, parts: &Value) -> Value {
    json!({
        "name": name,
        "package": format!("archangel-plugin-{name}"),
        "root": "app",
        "elements": parts,
    })
}

const NO_RULE: [Value; 0] = [];

#[test]
fn a_file_in_two_parts_gives_an_overlap_that_names_each_part_once() {
    let config = valid(&json!({
        "parts": { "a": "src/**", "b": "src/x/**", "c": "src/x/y/**", "d": "other/**" },
    }));
    let found = ElementMatcher::new(&config).path_match("src/x/y/z.ts");
    assert_eq!(
        found.overlap,
        [ElementId(0), ElementId(1), ElementId(2)],
        "a file in a, b and c overlaps each once, and d, which does not hold it, stays out"
    );
    assert_eq!(
        found.classification.expect("a place").element,
        ElementId(0),
        "the file still has a place despite the overlap: the first part that matched"
    );
}

#[test]
fn a_file_that_two_patterns_of_one_part_match_gives_no_overlap() {
    let config = valid(&json!({ "parts": { "a": ["src/**", "src/x/**"] } }));
    let found = ElementMatcher::new(&config).path_match("src/x/z.ts");
    assert_eq!(
        found.overlap,
        [],
        "two patterns of one part name one part, so nothing overlaps"
    );
}

#[test]
fn a_file_that_two_elements_match_gives_no_overlap() {
    let config = valid(&json!({ "elements": { "a": "src/**", "b": "src/x/**" } }));
    let found = ElementMatcher::new(&config).path_match("src/x/z.ts");
    assert_eq!(
        found.overlap,
        [],
        "only parts can overlap, so a file in two elements is no overlap"
    );
    assert_eq!(
        found.classification.expect("a place").element,
        ElementId(0),
        "of two matching elements, the first declared holds the file"
    );
}

/// A config with parts of its own, and two presets that between them hold three elements.
fn own_parts_and_two_presets() -> Config {
    valid(&json!({
        "parts": { "own": "app/**" },
        "extends": [
            preset("one", &json!({ "first": "src/**", "second": "src/**" })),
            preset("two", &json!({ "third": "src/**" })),
        ],
    }))
}

#[test]
fn a_file_has_one_place_in_each_set_of_parts() {
    let config = own_parts_and_two_presets();
    let sets: Vec<(&str, u32)> = config
        .elements
        .iter()
        .map(|element| (element.name.as_str(), element.set))
        .collect();
    assert_eq!(
        sets,
        [
            ("one/first", 1),
            ("one/second", 1),
            ("two/third", 2),
            ("own", 0)
        ],
        "each preset has a set numbered by its place in extends; the config's own is 0"
    );
    let parts: Vec<bool> = config.elements.iter().map(|element| element.part).collect();
    assert_eq!(
        parts,
        [false, false, false, true],
        "only the config's own parts overlap"
    );

    let found = ElementMatcher::new(&config).path_match("app/src/a.ts");
    assert_eq!(
        found.overlap,
        [],
        "the second match in a preset is no overlap"
    );
    let place = found.classification.expect("a place");
    let elements: Vec<ElementId> = place.all().map(|place| place.element).collect();
    assert_eq!(
        elements,
        [ElementId(3), ElementId(0), ElementId(2)],
        "own set first"
    );
    assert_eq!(
        place.of(ElementId(2)).expect("in it").element,
        ElementId(2),
        "the file is in two/third, so asking for that element gives its place there"
    );
    assert!(
        place.of(ElementId(1)).is_none(),
        "the second match of a set is no place"
    );
}

#[test]
fn presets_share_the_one_set_when_the_config_writes_elements() {
    let config = valid(&json!({
        "elements": { "own": "src/**" },
        "extends": [preset("one", &json!({ "first": "src/**" }))],
    }));
    assert!(
        config.elements.iter().all(|element| element.set == 0),
        "`elements` puts each preset's elements in set 0 with the config's: {:?}",
        config.elements
    );
    assert!(
        config.elements.iter().all(|element| !element.part),
        "with `elements` written, no element is a part, so none can overlap: {:?}",
        config.elements
    );
}

#[test]
fn a_package_is_in_the_first_package_element_and_in_every_externals_set() {
    let config = valid(&json!({
        "elements": {
            "src": "src/**",
            "first": { "packages": ["react"] },
            "second": { "packages": ["react"] },
        },
        "externals": { "one": ["react"], "two": ["react"], "three": ["pg"] },
    }));
    let matcher = ElementMatcher::new(&config);
    let id = |name: &str| {
        let index = config
            .elements
            .iter()
            .position(|e| e.name == name)
            .expect("declared");
        ElementId(u32::try_from(index).expect("few"))
    };
    let mut found = matcher.package_elements("react");
    found.sort_by_key(|element| element.index());
    let mut expected = vec![id("first"), id("one"), id("two")];
    expected.sort_by_key(|element| element.index());
    assert_eq!(
        found, expected,
        "the second package element does not hold it"
    );

    let place = matcher.package_place("react").expect("a place");
    assert_eq!(place.root, "react");
    assert_eq!(
        place.all().count(),
        3,
        "react has one place in each of first, one and two"
    );
    assert!(
        place.of(id("two")).is_some(),
        "react is in the externals set two, so its place answers for that set: {place:?}"
    );
    assert!(
        place.of(id("three")).is_none(),
        "the externals set three holds only pg, so react has no place in it: {place:?}"
    );
    assert!(
        matcher.package_place("zod").is_none(),
        "no element or externals set names zod, so it has no place"
    );
}

#[test]
fn an_externals_set_does_not_stop_a_later_package_element() {
    let config = valid(&json!({
        "elements": { "src": "src/**" },
        "externals": { "one": ["react"] },
        "extends": [preset("p", &json!({ "packages": { "packages": ["react"] } }))],
    }));
    let found = ElementMatcher::new(&config).package_elements("react");
    assert_eq!(found.len(), 2, "{found:?}");
}

fn place(config: &Config, path: &str) -> archangel_config::Classification {
    ElementMatcher::new(config)
        .classify_file(path)
        .expect("in a part")
}

#[test]
fn two_places_are_one_instance_when_their_element_and_captures_agree() {
    let config = valid(&json!({
        "parts": {
            "feature": ["a/{name}/**", "b/{name}/**"],
            "other": "c/{name}/**",
        },
    }));
    let cart = place(&config, "a/cart/x.ts");
    assert!(
        cart.same_instance(&place(&config, "a/cart/y.ts")),
        "two files under one capture of one pattern are one instance: {cart:?}"
    );
    assert!(
        cart.same_instance(&place(&config, "b/cart/y.ts")),
        "two patterns, one capture value"
    );
    assert!(
        !cart.same_instance(&place(&config, "a/login/y.ts")),
        "another value"
    );
    assert!(
        !cart.same_instance(&place(&config, "c/cart/y.ts")),
        "another element"
    );
}

#[test]
fn two_places_share_a_root_only_in_one_folder_of_one_instance() {
    let config = valid(&json!({
        "parts": {
            "feature": ["a/{name}/**", "b/{name}/**"],
            "named": "n/{left}/**",
            "other": "o/{right}/**",
        },
    }));
    let cart = place(&config, "a/cart/x.ts");
    assert!(
        cart.same_root(&place(&config, "a/cart/y.ts")),
        "two files in the folder a/cart share that root: {cart:?}"
    );
    assert!(
        !cart.same_root(&place(&config, "b/cart/y.ts")),
        "another folder"
    );
    assert!(
        !cart.same_root(&place(&config, "a/login/y.ts")),
        "another instance"
    );

    let mut renamed = cart.clone();
    renamed.names = vec!["other".to_owned()];
    assert!(!cart.same_root(&renamed), "another capture name");
    let mut moved = cart.clone();
    moved.root = "elsewhere".to_owned();
    assert!(!cart.same_root(&moved), "another root");
    assert!(
        cart.same_root(&cart.clone()),
        "a copy shares its root, so the refusals above come from what changed: {cart:?}"
    );
}

fn kinds(pattern: &str) -> archangel_config::KindPattern {
    let config = valid(&json!({
        "rules": [archangel_testkit::table("t", &json!({ "entity": [] }), &json!({ "kinds": pattern }))],
    }));
    config.tables[0].kinds.clone().expect("a kind pattern")
}

#[test]
fn a_kind_pattern_reads_the_kind_and_the_other_captures() {
    let pattern = kinds("src/{context}/{kind}/**");
    assert!(pattern.has_instance(), "a capture other than the kind");
    let place = pattern
        .place("src/order/views/a.ts", true)
        .expect("a place");
    assert_eq!(place.kind, "views");
    assert_eq!(
        place.instance,
        ["order"],
        "order is the capture other than the kind, so it names the instance"
    );
    assert!(
        pattern.place("src/order/views/a.ts", false).is_none(),
        "the pass did not match"
    );
    assert!(
        pattern.place("other/a.ts", true).is_none(),
        "a path outside src/ does not fit the pattern, so it has no place"
    );

    let plain = kinds("**/*.{kind}.ts");
    assert!(!plain.has_instance(), "only the kind");
    assert_eq!(
        plain.place("a.entity.ts", true).expect("a place").instance,
        Vec::<String>::new(),
        "a pattern whose only capture is the kind has an empty instance"
    );
}

#[test]
fn a_declaration_file_counts_its_d_as_part_of_the_extension() {
    let plain = kinds("**/*.{kind}.ts");
    assert_eq!(
        plain.place("a.entity.d.ts", false).expect("a place").kind,
        "entity"
    );
    assert!(plain.place("env.d.ts", true).is_none(), "`d` is not a kind");

    let named = kinds("**/{kind}.d.ts");
    assert_eq!(named.place("x.d.ts", true).expect("a place").kind, "x");
    assert!(
        named.place("x.d.ts", false).is_none(),
        "the pass did not match"
    );
}

#[test]
fn a_repeated_kind_must_be_the_same_name_at_each_place() {
    let pattern = kinds("src/{kind}/*.{kind}.ts");
    assert_eq!(
        pattern.place("src/a/x.a.ts", true).expect("a place").kind,
        "a"
    );
    assert!(
        pattern.place("src/a/x.b.ts", true).is_none(),
        "the folder says kind a and the file name says kind b, so the file has no place"
    );
}

#[test]
fn each_row_refers_to_the_table_at_its_position() {
    let config = valid(&json!({
        "rules": [
            archangel_testkit::table("first", &json!({ "a": [] }), &json!({ "kinds": "x/{kind}/**" })),
            archangel_testkit::table("second", &json!({ "b": [] }), &json!({ "kinds": "y/{kind}/**" })),
        ],
    }));
    let of_second = config
        .rules
        .iter()
        .find_map(|rule| match &rule.kind {
            archangel_config::RuleKind::TableRow {
                table,
                row: Some(row),
            } if row == "b" => Some(*table),
            _ => None,
        })
        .expect("the row");
    assert_eq!(
        of_second.index(),
        1,
        "row b is in the second table, so it refers to index 1"
    );
    assert_eq!(config.tables[of_second.index()].name, "second");
}

#[test]
fn the_constructors_of_a_set_of_files_write_each_set_after_its_parts() {
    use archangel_config::FileSet;
    let a = FileSet::of(Term::AnyElement);
    let b = FileSet::of(Term::Directive("d".to_owned()));
    assert_eq!(
        FileSet::every_file().terms,
        [Term::Any(0), Term::Not],
        "every file is the negation of an empty union, written in postfix"
    );
    assert_eq!(
        FileSet::any(&[a.clone(), b.clone()]).terms,
        [
            Term::AnyElement,
            Term::Directive("d".to_owned()),
            Term::Any(2),
        ],
        "a union writes its members in order, then Any with their count"
    );
    assert_eq!(
        a.and(&b).terms,
        [
            Term::AnyElement,
            Term::Directive("d".to_owned()),
            Term::All(2),
        ],
        "an intersection writes both sides in order, then All with their count"
    );
    assert_eq!(
        a.not().terms,
        [Term::AnyElement, Term::Not],
        "a negation writes the set it negates, then Not"
    );
}

#[test]
fn a_config_calls_its_parts_what_it_writes() {
    let rule = json!({ "files": "x/**", "mustNotExist": true });
    let words = |config: Value| {
        let config = valid(&config);
        (config.part_word(), config.group_word())
    };
    let new = ("part", "group");
    let old = ("element", "selector");
    assert_eq!(
        words(json!({ "parts": { "a": "a/**" } })),
        new,
        "a config that writes `parts` uses the words part and group"
    );
    assert_eq!(
        words(json!({ "groups": { "a": "a/**" }, "rules": [rule] })),
        new,
        "a config that writes only `groups` also uses the new words"
    );
    assert_eq!(
        words(json!({ "rules": [rule] })),
        new,
        "a config that writes neither"
    );
    assert_eq!(
        words(json!({ "elements": { "a": "a/**" } })),
        old,
        "a config that writes `elements` keeps the words element and selector"
    );
    assert_eq!(
        words(json!({ "selectors": { "a": "a/**" }, "rules": [rule] })),
        old,
        "a config that writes only `selectors` also keeps the old words"
    );
    assert_eq!(
        words(json!({ "extends": [preset("p", &json!({ "a": "a/**" }))], "rules": NO_RULE })),
        old,
        "a preset's `elements` count as written when the config itself writes neither word"
    );
}

#[test]
fn a_config_that_writes_parts_or_groups_with_a_preset_calls_them_so() {
    let with_preset = |key: &str| {
        let mut config = json!({
            "extends": [preset("p", &json!({ "a": "a/**" }))],
            "rules": [{ "name": "r", "files": "x/**", "mustNotExist": true }],
        });
        config[key] = json!({ "own": "own/**" });
        valid(&config).part_word()
    };
    assert_eq!(with_preset("parts"), "part");
    assert_eq!(with_preset("groups"), "part");
}
