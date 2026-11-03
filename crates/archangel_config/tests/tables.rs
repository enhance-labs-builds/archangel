//! Scenarios from the import-tables spec about loading: what a table may name, and which names it
//! refuses.
#![cfg(test)]

use archangel_config::{
    Condition, Config, ConfigProblem, ElementId, FileSet, ImportKinds, ProblemCode, Rule,
    SelectorId, Side, TableId, Term, parse_json,
};
use archangel_testkit::table;
use serde_json::{Value, json};

fn problems(config: &Value) -> Vec<ConfigProblem> {
    parse_json(&config.to_string()).expect_err("the config has a problem")
}

fn with_rules(rules: &[Value]) -> Value {
    json!({ "parts": { "api": "apps/api/**", "shared": "src/shared/**" }, "rules": rules })
}

const KINDS: &str = "**/*.{kind}.ts";

/// The problem of one code, which must be the only problem of that code.
fn one_of(config: &Value, code: ProblemCode) -> (String, Option<String>) {
    let found = problems(config);
    let of_code: Vec<_> = found.iter().filter(|p| p.code == code).collect();
    assert_eq!(of_code.len(), 1, "one problem of {code:?}: {found:?}");
    (of_code[0].message.clone(), of_code[0].field.clone())
}

#[test]
fn a_row_that_names_itself_and_may_not_import_itself_is_refused() {
    let config = with_rules(&[table(
        "t",
        &json!({ "port": ["port", "entity"], "entity": [] }),
        &json!({ "kinds": KINDS, "mayNotImportItself": ["port"] }),
    )]);
    let (message, field) = one_of(&config, ProblemCode::ConfigInvalid);
    assert!(message.contains("names itself"), "{message}");
    assert_eq!(
        field.as_deref(),
        Some("rules[0].rows.port"),
        "a row that names itself and may not import itself is refused"
    );
}

#[test]
fn a_kind_pattern_without_the_kind_capture_is_refused() {
    let config = with_rules(&[table(
        "t",
        &json!({ "entity": [] }),
        &json!({ "kinds": "**/*.entity.ts" }),
    )]);
    let (message, field) = one_of(&config, ProblemCode::ConfigInvalid);
    assert!(message.contains("{kind}"), "{message}");
    assert_eq!(
        field.as_deref(),
        Some("rules[0].kinds"),
        "a kind pattern without the kind capture is refused"
    );
}

#[test]
fn a_misspelt_cell_is_refused_with_the_closest_name() {
    let config = with_rules(&[table(
        "t",
        &json!({ "entity": ["value-objects"], "value-object": [] }),
        &json!({ "kinds": KINDS }),
    )]);
    let (message, field) = one_of(&config, ProblemCode::RuleUnknownElement);
    assert!(message.contains("value-object\""), "{message}");
    assert!(
        field.is_some_and(|f| f.starts_with("rules[0].rows.entity")),
        "a misspelt cell is refused with the closest name"
    );
}

#[test]
fn a_row_key_that_names_nothing_is_refused_with_the_closest_name() {
    let config = json!({
        "parts": { "canvas-document": "packages/canvas-document/**" },
        "rules": [table("t", &json!({ "canvas-documents": [] }), &json!({}))],
    });
    let (message, field) = one_of(&config, ProblemCode::RuleUnknownElement);
    assert!(message.contains("\"canvas-document\""), "{message}");
    assert_eq!(
        field.as_deref(),
        Some("rules[0].rows.canvas-documents"),
        "a row key that names nothing is refused with the closest name"
    );
}

#[test]
fn a_row_key_that_names_an_externals_set_is_refused() {
    let config = json!({
        "parts": { "api": "apps/api/**" },
        "externals": { "driver": ["pg"] },
        "rules": [table("t", &json!({ "driver": [] }), &json!({}))],
    });
    let (message, field) = one_of(&config, ProblemCode::ConfigInvalid);
    assert!(message.contains("no files that import"), "{message}");
    assert_eq!(
        field.as_deref(),
        Some("rules[0].rows.driver"),
        "a row key that names an externals set is refused"
    );
}

#[test]
fn a_row_name_that_another_rule_has_is_a_duplicate() {
    let config = with_rules(&[
        table(
            "domain",
            &json!({ "entity": [] }),
            &json!({ "kinds": KINDS }),
        ),
        json!({ "name": "domain.entity", "files": "api", "mayNotImport": "shared" }),
    ]);
    let (message, _) = one_of(&config, ProblemCode::RuleNameDuplicate);
    assert!(message.contains("domain.entity"), "{message}");
}

#[test]
fn two_tables_with_one_name_are_refused() {
    let config = with_rules(&[
        table(
            "domain",
            &json!({ "entity": [] }),
            &json!({ "kinds": KINDS }),
        ),
        table(
            "domain",
            &json!({ "policy": [] }),
            &json!({ "kinds": KINDS }),
        ),
    ]);
    let (message, _) = one_of(&config, ProblemCode::RuleNameDuplicate);
    assert!(
        message.contains("domain") && message.contains("its own name"),
        "{message}"
    );
}

#[test]
fn a_table_without_a_name_is_refused() {
    let config = json!({
        "parts": { "api": "apps/api/**" },
        "rules": [{ "kind": "mayImport", "rows": { "api": [] } }],
    });
    let (message, field) = one_of(&config, ProblemCode::ConfigInvalid);
    assert!(message.contains("needs a name"), "{message}");
    assert_eq!(
        field.as_deref(),
        Some("rules[0].name"),
        "a table without a name is refused"
    );
}

#[test]
fn a_table_name_with_a_colon_or_slash_is_refused() {
    let colon = with_rules(&[table("api:domain", &json!({ "api": [] }), &json!({}))]);
    let (message, field) = one_of(&colon, ProblemCode::ConfigInvalid);
    assert!(
        message.contains("colon") && message.contains("exception comment"),
        "{message}"
    );
    assert_eq!(
        field.as_deref(),
        Some("rules[0].name"),
        "a table name with a colon or slash is refused"
    );
    let slash = with_rules(&[table("api/domain", &json!({ "api": [] }), &json!({}))]);
    let (message, field) = one_of(&slash, ProblemCode::ConfigInvalid);
    assert!(message.contains("slash"), "{message}");
    assert_eq!(
        field.as_deref(),
        Some("rules[0].name"),
        "a table name with a colon or slash is refused"
    );
}

#[test]
fn a_row_key_with_a_colon_or_slash_is_refused() {
    for (key, word) in [("aggregate:root", "colon"), ("aggregate/root", "slash")] {
        let config = with_rules(&[table("t", &json!({ key: [] }), &json!({ "kinds": KINDS }))]);
        let (message, field) = one_of(&config, ProblemCode::ConfigInvalid);
        assert!(message.contains(word), "{message}");
        assert_eq!(
            field,
            Some(format!("rules[0].rows.{key}")),
            "a row key with a colon or slash is refused"
        );
    }
}

#[test]
fn a_dot_or_a_space_in_a_name_is_accepted_and_a_clash_is_a_duplicate() {
    let loads = with_rules(&[table(
        "feature-code",
        &json!({ "api.v2": ["shared"], "my views": ["shared"] }),
        &json!({ "kinds": "src/features/*/{kind}/**" }),
    )]);
    let config = parse_json(&loads.to_string()).expect("the names load");
    assert!(
        config.rules.iter().any(|r| r.name == "feature-code.api.v2"),
        "a dot or a space in a name is accepted and a clash is a duplicate"
    );
    assert!(
        config
            .rules
            .iter()
            .any(|r| r.name == "feature-code.my views"),
        "a dot or a space in a name is accepted and a clash is a duplicate"
    );
    let clash = with_rules(&[
        table("a.b", &json!({ "c": [] }), &json!({ "kinds": KINDS })),
        table("a", &json!({ "b.c": [] }), &json!({ "kinds": KINDS })),
    ]);
    let (message, _) = one_of(&clash, ProblemCode::RuleNameDuplicate);
    assert!(message.contains("a.b.c"), "{message}");
}

#[test]
fn instance_on_a_row_without_a_capture_is_refused() {
    let config = with_rules(&[table(
        "t",
        &json!({ "shared": [] }),
        &json!({ "instance": "different" }),
    )]);
    let (_, field) = one_of(&config, ProblemCode::RuleNeedsCapture);
    assert_eq!(
        field.as_deref(),
        Some("rules[0].rows.shared"),
        "instance on a row without a capture is refused"
    );
}

#[test]
fn only_the_kind_capture_may_repeat_in_a_kind_pattern() {
    let config = with_rules(&[
        table(
            "domain",
            &json!({ "entity": [] }),
            &json!({ "kinds": "**/{kind}s/*.{kind}.ts" }),
        ),
        table(
            "apps",
            &json!({ "entity": [] }),
            &json!({ "kinds": "apps/{app}/{app}/*.{kind}.ts" }),
        ),
    ]);
    let found = problems(&config);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        found[0].code,
        ProblemCode::PatternInvalid,
        "only the kind capture may repeat in a kind pattern"
    );
    for word in ["apps", "apps/{app}/{app}/*.{kind}.ts", "app"] {
        assert!(
            found[0].message.contains(word),
            "{word}: {}",
            found[0].message
        );
    }
}

#[test]
fn a_kind_pattern_that_cannot_mean_what_it_says_is_pattern_invalid() {
    let config = with_rules(&[table(
        "domain",
        &json!({ "entity": [] }),
        &json!({ "kinds": "**//*.{kind}.ts" }),
    )]);
    let (message, _) = one_of(&config, ProblemCode::PatternInvalid);
    assert!(
        message.contains("domain") && message.contains("**//*.{kind}.ts"),
        "{message}"
    );
}

#[test]
fn a_row_that_every_row_may_import_lists_and_may_not_import_itself_is_refused() {
    let config = with_rules(&[table(
        "t",
        &json!({ "port": [], "entity": [] }),
        &json!({ "kinds": KINDS, "everyRowMayImport": ["port"], "mayNotImportItself": ["port"] }),
    )]);
    let (message, _) = one_of(&config, ProblemCode::ConfigInvalid);
    assert!(message.contains("everyRowMayImport"), "{message}");
}

#[test]
fn a_rule_and_a_table_with_one_name_are_refused() {
    let config = with_rules(&[
        json!({ "name": "t", "files": "api", "mayNotImport": "shared" }),
        table("t", &json!({ "port": [] }), &json!({ "kinds": KINDS })),
    ]);
    let (message, _) = one_of(&config, ProblemCode::RuleNameDuplicate);
    assert!(
        message.contains("\"t\"") || message.contains("`t`"),
        "{message}"
    );
}

fn table_config(rows: &Value, options: &Value) -> Config {
    parse_json(&with_rules(&[table("t", rows, options)]).to_string())
        .unwrap_or_else(|problems| panic!("the config is valid: {problems:?}"))
}

fn row_rule<'c>(config: &'c Config, row: &str) -> &'c Rule {
    let name = format!("t.{row}");
    config
        .rules
        .iter()
        .find(|rule| rule.name == name)
        .expect("the row's rule")
}

fn permitted(rule: &Rule) -> &Side {
    match &rule.sentences[0].condition {
        Condition::MayOnlyImport(side) => side,
        other => panic!("a row permits a list: {other:?}"),
    }
}

#[test]
fn only_a_row_that_may_not_import_itself_has_the_sentence_that_refuses_it() {
    let config = table_config(
        &json!({ "port": ["entity"], "entity": [] }),
        &json!({ "kinds": KINDS, "mayNotImportItself": ["port"] }),
    );
    assert_eq!(
        row_rule(&config, "port").sentences.len(),
        2,
        "port is in mayNotImportItself, so it adds the sentence that refuses itself"
    );
    assert_eq!(
        row_rule(&config, "entity").sentences.len(),
        1,
        "entity is not in mayNotImportItself, so it has only the sentence of its row"
    );
}

#[test]
fn may_not_import_itself_refuses_only_a_name_that_is_no_row() {
    let found = problems(&with_rules(&[table(
        "t",
        &json!({ "port": [] }),
        &json!({ "kinds": KINDS, "mayNotImportItself": ["prot"] }),
    )]));
    assert!(
        found.iter().any(|p| p.message.contains("\"prot\"")),
        "{found:?}"
    );
    table_config(
        &json!({ "port": [] }),
        &json!({ "kinds": KINDS, "mayNotImportItself": ["port"] }),
    );
}

#[test]
fn every_row_may_import_goes_with_may_not_import_itself_unless_it_names_that_row() {
    let config = table_config(
        &json!({ "port": [], "entity": [] }),
        &json!({
            "kinds": KINDS,
            "everyRowMayImport": ["src/shared/**", "entity"],
            "mayNotImportItself": ["port"],
        }),
    );
    let side = permitted(row_rule(&config, "port"));
    assert_eq!(side.files.len(), 2, "the row has every cell: {side:?}");
    assert!(side.text.contains("src/shared/**"), "{}", side.text);
}

#[test]
fn a_because_is_what_the_rule_remembers_it_wrote() {
    let config = table_config(
        &json!({ "a": [] }),
        &json!({ "kinds": KINDS, "because": "Why." }),
    );
    let rule = row_rule(&config, "a");
    assert!(
        rule.because,
        "a table written with `because` passes that word on to each row: {rule:?}"
    );
    assert_eq!(
        rule.reason.as_deref(),
        Some("Why."),
        "the text of `because` becomes the reason of the row"
    );
    let config = table_config(
        &json!({ "a": [] }),
        &json!({ "kinds": KINDS, "reason": "Why." }),
    );
    assert!(
        !row_rule(&config, "a").because,
        "a table written with `reason` does not claim `because` for its row: {:?}",
        row_rule(&config, "a")
    );
}

#[test]
fn a_table_without_files_judges_its_kinds_and_its_named_rows() {
    let kinds_only = table_config(&json!({ "a": [], "b": [] }), &json!({ "kinds": KINDS }));
    let of_table = &kinds_only.tables[0];
    assert!(
        of_table.default_files,
        "a table with no `files` marks them as its default: {:?}",
        of_table.files
    );
    assert_eq!(
        of_table.files.terms,
        [Term::AnyKind(TableId(0)), Term::Any(1)],
        "with no `files`, a table holds each file with a kind, and here names no part"
    );

    let with_part = table_config(&json!({ "a": [], "api": [] }), &json!({ "kinds": KINDS }));
    assert_eq!(
        with_part.tables[0].files.terms,
        [
            Term::AnyKind(TableId(0)),
            Term::Element(ElementId(0)),
            Term::Any(2),
        ],
        "row api names a part, so the default files hold that part and each kind"
    );
}

#[test]
fn a_table_with_files_judges_those_files() {
    let config = table_config(
        &json!({ "a": [] }),
        &json!({ "kinds": KINDS, "files": "api" }),
    );
    let of_table = &config.tables[0];
    assert!(
        !of_table.default_files,
        "a table that writes `files` does not use the default: {:?}",
        of_table.files
    );
    assert_eq!(
        of_table.files.terms,
        [Term::Element(ElementId(0))],
        "`files` replaces the default, so the table holds api and not each kind"
    );
}

#[test]
fn a_cell_that_names_a_row_permits_the_files_of_that_row() {
    let config = table_config(&json!({ "a": ["b"], "b": [] }), &json!({ "kinds": KINDS }));
    let side = permitted(row_rule(&config, "a"));
    let of_b = &config.tables[0].rows[1].1;
    assert_eq!(
        side.files.as_slice(),
        std::slice::from_ref(of_b),
        "the cell b permits exactly the files of row b, no more and no fewer"
    );
}

#[test]
fn a_row_key_or_a_cell_that_names_a_group_means_that_group() {
    let config: Config = parse_json(
        &json!({
            "parts": { "api": "apps/api/**" },
            "groups": { "first": "a/**", "second": "b/**" },
            "rules": [table("t", &json!({ "second": ["first"], "first": [] }), &json!({}))],
        })
        .to_string(),
    )
    .unwrap_or_else(|problems| panic!("the config is valid: {problems:?}"));
    let rows = &config.tables[0].rows;
    assert_eq!(
        rows[0].1.terms,
        [Term::Selector(SelectorId(1))],
        "row key second means the group second, at index 1, though it is written first"
    );
    assert_eq!(
        rows[1].1.terms,
        [Term::Selector(SelectorId(0))],
        "row key first means the group first, at index 0, though it is written second"
    );
}

#[test]
fn a_row_that_may_not_import_itself_refuses_the_files_of_its_name() {
    let config = table_config(
        &json!({ "port": [] }),
        &json!({ "kinds": KINDS, "mayNotImportItself": ["port"] }),
    );
    let rule = row_rule(&config, "port");
    let Condition::MayNotImport(side) = &rule.sentences[1].condition else {
        panic!("a refusal: {rule:?}");
    };
    assert_eq!(side.text, "port");
    assert_eq!(
        side.files,
        [config.tables[0].rows[0].1.clone()],
        "the refusing sentence refuses exactly the files of the row's own name"
    );
    let own = rule.sentences[0]
        .options
        .own
        .as_ref()
        .expect("its own files");
    assert!(
        !own.permitted,
        "a row that may not import itself does not permit its own files: {own:?}"
    );
    assert_eq!(
        own.files, config.tables[0].rows[0].1,
        "the own files of the row are the files that the table holds for its name"
    );
}

#[test]
fn a_row_permits_its_own_name_and_looks_at_the_whole_repository_by_default() {
    let config = table_config(&json!({ "a": [] }), &json!({ "kinds": KINDS }));
    let options = &row_rule(&config, "a").sentences[0].options;
    assert!(
        options.own.as_ref().expect("its own files").permitted,
        "by default a row may import its own files: {options:?}"
    );
    let area = options.within.as_ref().expect("an area");
    assert_eq!(area.text, "the repository");
    assert_eq!(
        area.files,
        [FileSet::every_file()],
        "the default area of a row holds every file in the repository"
    );
    assert_eq!(
        area.elements,
        [],
        "a table that names no externals set has no package in its default area"
    );
}

#[test]
fn the_default_area_of_a_row_holds_each_externals_set_that_the_table_names_once() {
    let config = parse_json(
        &json!({
            "parts": { "api": "apps/api/**" },
            "externals": { "driver": ["pg"], "cloud": ["@aws-sdk/*"] },
            "rules": [table(
                "t",
                &json!({ "a": ["driver"], "b": ["driver"] }),
                &json!({ "kinds": KINDS, "everyRowMayImport": ["cloud"] }),
            )],
        })
        .to_string(),
    )
    .expect("a valid config");
    let area = row_rule(&config, "a").sentences[0]
        .options
        .within
        .clone()
        .expect("an area");
    assert_eq!(area.elements.len(), 2, "{area:?}");
}

#[test]
fn a_within_of_the_table_replaces_the_default_area() {
    let config = table_config(
        &json!({ "a": [] }),
        &json!({ "kinds": KINDS, "within": "src/**" }),
    );
    let area = row_rule(&config, "a").sentences[0]
        .options
        .within
        .clone()
        .expect("an area");
    assert_eq!(area.text, "src/**");
}

#[test]
fn the_table_own_rule_counts_every_file_whatever_imports_the_table_counts() {
    let config = table_config(
        &json!({ "a": [] }),
        &json!({ "kinds": KINDS, "imports": "runtime" }),
    );
    assert_eq!(
        row_rule(&config, "a").imports,
        ImportKinds::Runtime,
        "a row counts the kinds of import that its table names"
    );
    assert_eq!(
        row_rule(&config, "every-file-has-a-kind").imports,
        ImportKinds::All,
        "the table's own rule checks files, so `imports` of the table does not narrow it"
    );
}

#[test]
fn the_text_of_a_row_joins_its_cells_and_those_of_every_row() {
    let text = |rows: &Value, options: &Value| {
        let mut options = options.clone();
        options["kinds"] = json!(KINDS);
        let config = table_config(rows, &options);
        permitted(row_rule(&config, "a")).text.clone()
    };
    assert_eq!(text(&json!({ "a": [] }), &json!({})), "");
    assert_eq!(text(&json!({ "a": ["api"] }), &json!({})), "api");
    assert_eq!(
        text(&json!({ "a": ["api", "shared"] }), &json!({})),
        "api, shared"
    );
    assert_eq!(
        text(
            &json!({ "a": [] }),
            &json!({ "everyRowMayImport": ["api"] })
        ),
        "api"
    );
    assert_eq!(
        text(
            &json!({ "a": ["shared"] }),
            &json!({ "everyRowMayImport": ["api"] })
        ),
        "shared, api"
    );
}
