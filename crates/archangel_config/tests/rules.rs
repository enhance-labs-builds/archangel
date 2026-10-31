//! Scenarios from the architecture-rules and structure-rules specs: what each rule compiles to,
//! the values its condition and options read, and the default name it gets.
#![cfg(test)]

#[path = "support/elements.rs"]
mod elements;
#[path = "support/validating.rs"]
mod support;

use archangel_config::{
    Condition, DependencyKind, ExportKind, InstanceMatch, NameRule, SpecifierForm, Term, parse_json,
};
use elements::only_problem;
use serde_json::{Value, json};
use support::{every_kind, sentence_elements};

#[test]
fn isolate_compiles_to_a_sentence_against_other_instances_of_the_element() {
    let config = every_kind();
    let entity = config.element("entity").unwrap();
    let sentence = &config.rules[1].sentences[0];
    let side = sentence.condition.side().expect("an import condition");
    assert_eq!(
        (
            &sentence.files.terms,
            side.files.iter().map(|set| &set.terms).collect::<Vec<_>>(),
            side.text.as_str(),
            sentence.options.instance,
        ),
        (
            &vec![Term::Element(entity)],
            vec![&vec![Term::Element(entity)]],
            "entity",
            Some(InstanceMatch::Different),
        ),
        "entity may not import entity in another instance"
    );
}

/// A layer's sentence in words: its files, the side's text, and the elements the side names as
/// files and as packages.
fn layer_sentence(
    config: &archangel_config::Config,
    sentence: &archangel_config::Sentence,
) -> String {
    let name = |term: &Term| match term {
        Term::Element(id) => config.elements[id.index()].name.clone(),
        other => format!("{other:?}"),
    };
    let side = sentence.condition.side().expect("an import condition");
    let files: Vec<String> = side
        .files
        .iter()
        .flat_map(|set| set.terms.iter().map(name))
        .collect();
    let packages: Vec<&str> = side
        .elements
        .iter()
        .map(|id| config.elements[id.index()].name.as_str())
        .collect();
    format!(
        "{} may not import {}: files {files:?}, packages {packages:?}",
        sentence.files.terms.iter().map(name).collect::<String>(),
        side.text
    )
}

#[test]
fn layers_compile_to_one_sentence_per_later_file_layer() {
    let config = parse_json(
        &json!({
            "elements": {
                "app": "src/app/**",
                "react": { "packages": ["react"] },
                "feature": "src/features/{feature}/**",
                "shared": "src/shared/**",
            },
            "rules": [{ "kind": "layers", "order": ["react", "app", "feature", "shared"] }],
        })
        .to_string(),
    )
    .expect("the layers are valid");
    let sentences: Vec<String> = config.rules[0]
        .sentences
        .iter()
        .map(|sentence| layer_sentence(&config, sentence))
        .collect();
    assert_eq!(
        sentences,
        [
            r#"app may not import react: files [], packages ["react"]"#,
            r#"feature may not import react, app: files ["app"], packages ["react"]"#,
            r#"shared may not import react, app, feature: files ["app", "feature"], packages ["react"]"#,
        ],
        "each file layer may not import an earlier layer; the package layer imports nothing, so \
         it has no sentence of its own"
    );
}

/// The one rule of a config of sentence elements.
fn one_rule(rule: &Value) -> archangel_config::Rule {
    let mut config =
        parse_json(&json!({ "elements": sentence_elements(), "rules": [rule] }).to_string())
            .unwrap_or_else(|problems| panic!("the rule {rule} is valid: {problems:?}"));
    config.rules.remove(0)
}

/// A rule of each condition that has a value besides its files.
fn valued_rules() -> [archangel_config::Rule; 7] {
    [
        json!({ "files": "src/**", "mustBeNamed": "kebab-case" }),
        json!({ "files": "src/**", "mustBeNamed": "{FileName}.test" }),
        json!({ "files": "feature", "mustContain": ["index.ts", "README.md"] }),
        json!({ "files": "view", "mustExport": "{FileName}" }),
        json!({ "files": "view", "mayNotExport": "default" }),
        json!({ "files": "view", "mayNotExport": "star" }),
        json!({ "files": "view", "mayNotExport": "internal*" }),
    ]
    .map(|rule| one_rule(&rule))
}

#[test]
fn a_condition_with_a_value_writes_it_in_the_default_name() {
    let rules = valued_rules();
    let names: Vec<&str> = rules.iter().map(|rule| rule.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "mustBeNamed(src/**, kebab-case)",
            "mustBeNamed(src/**, {FileName}.test)",
            "mustContain(feature, index.ts, README.md)",
            "mustExport(view, {FileName})",
            "mayNotExport(view, default)",
            "mayNotExport(view, star)",
            "mayNotExport(view, internal*)",
        ],
        "each default name holds the condition's value as written"
    );
}

#[test]
fn a_condition_reads_a_style_a_kind_of_export_or_a_name_pattern() {
    let conditions: Vec<String> = valued_rules()
        .iter()
        .map(|rule| match &rule.sentences[0].condition {
            Condition::MustBeNamed(NameRule::Style(style)) => format!("style {style}"),
            Condition::MustBeNamed(NameRule::Pattern(p)) => format!("name pattern {}", p.source()),
            Condition::MustContain(patterns) => format!("contains {}", patterns.len()),
            Condition::MustExport(p) => format!("exports {}", p.source()),
            Condition::MayNotExport(ExportKind::Default) => "no default export".to_owned(),
            Condition::MayNotExport(ExportKind::Star) => "no star export".to_owned(),
            Condition::MayNotExport(ExportKind::Names(p)) => format!("no export {}", p.source()),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(
        conditions,
        [
            "style kebab-case",
            "name pattern {FileName}.test",
            "contains 2",
            "exports {FileName}",
            "no default export",
            "no star export",
            "no export internal*",
        ],
        "a style name is a style, `default` and `star` are kinds of export, and anything else is \
         a name pattern"
    );
}

#[test]
fn conditions_are_about_exports_structure_or_neither() {
    let rules = [
        json!({ "files": "view", "mustExport": "{FileName}" }),
        json!({ "files": "view", "mayNotExport": "default" }),
        json!({ "files": "view", "mustOnlyReExport": true }),
        json!({ "files": "src/**", "mustBeNamed": "kebab-case" }),
        json!({ "files": "feature", "mustContain": "index.ts" }),
        json!({ "files": "src/legacy/**", "mustNotExist": true }),
        json!({ "files": "view", "mayNotImport": "db" }),
        json!({ "files": "src/**", "noCycles": true }),
    ]
    .map(|rule| one_rule(&rule));
    let kinds: Vec<(bool, bool)> = rules
        .iter()
        .map(|rule| {
            let condition = &rule.sentences[0].condition;
            (condition.is_export(), condition.is_structure())
        })
        .collect();
    assert_eq!(
        kinds,
        [
            (true, false),
            (true, false),
            (true, false),
            (false, true),
            (false, true),
            (false, true),
            (false, false),
            (false, false),
        ],
        "the export conditions, the structure conditions, and the import conditions that are neither"
    );
}

#[test]
fn a_rule_about_folders_covers_its_patterns_except_those_after_a_bang() {
    let rule = one_rule(&json!({
        "folders": ["src/*", "lib/*", "!src/legacy"],
        "mustBeNamed": "kebab-case",
    }));
    let folders = rule.sentences[0]
        .folders
        .as_ref()
        .expect("a rule about folders");
    let sources = |patterns: &[archangel_config::Pattern]| -> Vec<String> {
        patterns.iter().map(|p| p.source().to_owned()).collect()
    };
    assert_eq!(
        (sources(&folders.patterns), sources(&folders.except)),
        (
            vec!["src/*".to_owned(), "lib/*".to_owned()],
            vec!["src/legacy".to_owned()]
        ),
        "the leading `!` is gone from the folders left out"
    );
    let covered =
        ["src/ui", "lib/ui", "src/legacy", "docs/ui"].map(|folder| folders.covers(folder));
    assert_eq!(
        covered,
        [true, true, false, false],
        "a folder one pattern matches, unless one left out matches it too"
    );
}

#[test]
fn must_contain_with_no_pattern_is_refused() {
    let problem = only_problem(&json!({
        "elements": sentence_elements(),
        "rules": [{ "files": "feature", "mustContain": [] }],
    }));
    assert_eq!(
        (problem.field.as_deref(), problem.message.as_str()),
        (
            Some("rules[0].mustContain"),
            "must be a pattern or a list of patterns"
        ),
        "{problem:?}"
    );
}

#[test]
fn import_options_read_their_values() {
    let rule = one_rule(&json!({
        "files": "feature",
        "mayNotImport": "feature",
        "specifier": "alias",
        "instance": "same",
        "dependency": "undeclared",
        "names": ["useStore"],
    }));
    let options = &rule.sentences[0].options;
    assert_eq!(
        (
            options.specifier,
            options.instance,
            options.dependency,
            options.names.as_deref()
        ),
        (
            Some(SpecifierForm::Alias),
            Some(InstanceMatch::Same),
            Some(DependencyKind::Undeclared),
            Some(&["useStore".to_owned()][..])
        ),
        "each option as written"
    );
}

#[test]
fn a_wrong_import_option_gets_the_values_there_are() {
    let wrong = |option: &str| {
        only_problem(&json!({
            "elements": sentence_elements(),
            "rules": [{ "files": "feature", "mayNotImport": "db", option: "elsewhere" }],
        }))
        .message
    };
    assert_eq!(
        [wrong("specifier"), wrong("instance"), wrong("dependency")],
        [
            "must be 'relative', 'parent', 'alias' or 'package'",
            "must be 'same' or 'different'",
            "must be 'dev' or 'undeclared'",
        ],
        "a wrong value gets the values there are"
    );
}

#[test]
fn a_transitive_rule_takes_no_option_that_limits_one_import() {
    let limits = [
        ("names", json!(["useStore"])),
        ("specifier", json!("relative")),
        ("instance", json!("different")),
        ("dependency", json!("dev")),
    ];
    let fields: Vec<Option<String>> = limits
        .iter()
        .map(|(option, value)| {
            only_problem(&json!({
                "elements": sentence_elements(),
                "rules": [{ "files": "feature", "mayNotImport": "db", "transitively": true,
                            *option: value }],
            }))
            .field
        })
        .collect();
    assert_eq!(
        fields,
        vec![Some("rules[0].transitively".to_owned()); 4],
        "each of the four limits one import, which a chain of imports does not have"
    );
}
