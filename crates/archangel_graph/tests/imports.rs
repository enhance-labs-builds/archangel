//! Scenarios from the import-graph spec: which imports archangel finds in a file, and how it
//! marks type imports.
#![cfg(test)]

use archangel_graph::{FileFacts, ImportForm, ImportKind, ParseFailure, extract_file};

fn facts(path: &str, text: &str) -> FileFacts {
    extract_file(path, text).expect("the file parses")
}

fn specifiers(facts: &FileFacts) -> Vec<(&str, ImportKind)> {
    facts
        .imports
        .iter()
        .map(|import| (import.specifier.as_str(), import.kind))
        .collect()
}

#[test]
fn finds_each_runtime_import_form_with_a_literal_specifier() {
    let text = "import a from './a';\nexport { b } from './b';\nconst c = import('./c');\nconst d = require('./d');\nimport e = require('./e');\n";
    let found = facts("src/x.ts", text);
    let mut seen: Vec<(&str, u32, ImportForm)> = found
        .imports
        .iter()
        .map(|i| (i.specifier.as_str(), i.line, i.form))
        .collect();
    seen.sort_by_key(|(_, line, _)| *line);
    assert_eq!(
        seen,
        [
            ("./a", 1, ImportForm::Static),
            ("./b", 2, ImportForm::ExportFrom),
            ("./c", 3, ImportForm::Dynamic),
            ("./d", 4, ImportForm::Require),
            ("./e", 5, ImportForm::ImportEquals),
        ],
        "five forms, five specifiers, five lines"
    );
}

#[test]
fn finds_import_types_as_type_imports() {
    let found = facts("src/x.ts", "type Store = import('./store').Store;\n");
    assert_eq!(
        specifiers(&found),
        [("./store", ImportKind::Type)],
        "an import type is a type import"
    );
    assert_eq!(
        found.imports[0].form,
        ImportForm::ImportType,
        "its form is an import type"
    );
}

#[test]
fn finds_triple_slash_references_as_type_imports() {
    let text = "/// <reference path=\"./globals.d.ts\" />\n/// <reference types=\"vite/client\" />\nexport {};\n";
    let found = facts("src/env.d.ts", text);
    let forms: Vec<(&str, ImportForm, ImportKind)> = found
        .imports
        .iter()
        .map(|i| (i.specifier.as_str(), i.form, i.kind))
        .collect();
    assert_eq!(
        forms,
        [
            (
                "./globals.d.ts",
                ImportForm::TripleSlashPath,
                ImportKind::Type
            ),
            (
                "vite/client",
                ImportForm::TripleSlashTypes,
                ImportKind::Type
            ),
        ],
        "both references are type imports"
    );
}

#[test]
fn computed_specifiers_are_counted_not_followed() {
    let text = "const page = import(`./pages/${name}`);\nconst lib = require(path);\n";
    let found = facts("src/x.ts", text);
    assert!(
        found.imports.is_empty(),
        "neither computed specifier is an import"
    );
    assert_eq!(
        found.unfollowed,
        [1, 2],
        "both lines are recorded as not followed"
    );
}

#[test]
fn marks_type_imports_by_their_modifiers() {
    let text = "import type { A } from './a';\nimport { type B } from './b';\nimport { type C, d } from './c';\n";
    let found = facts("src/x.ts", text);
    let mut kinds = specifiers(&found);
    kinds.sort_unstable();
    assert_eq!(
        kinds,
        [
            ("./a", ImportKind::Type),
            ("./b", ImportKind::Type),
            ("./c", ImportKind::Runtime)
        ],
        "statement and name modifiers decide the kind"
    );
}

#[test]
fn imports_in_declaration_files_are_type_imports() {
    let found = facts("src/env.d.ts", "import { Config } from './config';\n");
    assert_eq!(
        specifiers(&found),
        [("./config", ImportKind::Type)],
        "every import is a type import"
    );
}

#[test]
fn parses_jsx_in_javascript_files() {
    let text = "import Button from './button';\nexport const App = () => <Button label=\"x\" />;\n";
    let found = facts("src/app.js", text);
    assert_eq!(
        specifiers(&found),
        [("./button", ImportKind::Runtime)],
        "JSX in .js parses"
    );
}

#[test]
fn syntax_error_is_parse_failed_and_its_imports_are_not_used() {
    let text = "import a from './a';\nconst ok = 1;\nconst = ;\nconst also = ;\n";
    let failure: ParseFailure =
        extract_file("src/a.ts", text).expect_err("the file does not parse");
    assert!(!failure.errors.is_empty(), "the failure lists errors");
    assert_eq!(failure.errors[0].line, 3, "the first error is on line 3");
}

#[test]
fn flow_file_is_parse_failed_with_the_reason() {
    let text = "// @flow\nimport type { A } from './a';\nfunction f(x: ?string): void {}\n";
    let failure = extract_file("src/legacy.js", text).expect_err("Flow syntax does not parse");
    assert!(failure.flow, "the failure says the file is a Flow file");
}

#[test]
fn parses_code_nested_ten_thousand_levels_deep() {
    let depth = 10_000;
    let text = format!(
        "import './a';\nexport const x = {}1{};\n",
        "(".repeat(depth),
        ")".repeat(depth)
    );
    let result = std::thread::Builder::new()
        .stack_size(archangel_graph::PARSE_STACK_BYTES)
        .spawn(move || extract_file("src/deep.ts", &text).map(|facts| facts.imports.len()))
        .unwrap()
        .join()
        .expect("the parse thread does not overflow its stack");
    assert_eq!(result.ok(), Some(1), "the deeply nested file parses");
}

#[test]
fn error_the_parser_recovers_from_is_still_parse_failed() {
    // The parser recovers from a `const` without a value and goes on to find the import.
    let text = "import './a';\nconst a;\n";
    let failure = extract_file("src/a.ts", text).expect_err("the file does not parse");
    assert_eq!(
        failure.errors.iter().map(|e| e.line).collect::<Vec<_>>(),
        [2],
        "the one error is on line 2"
    );
}

#[test]
fn records_the_line_and_column_of_each_specifier_in_characters() {
    let text = "import a from './a';\nconst é = 1; const b = require('./b');\n";
    let found = facts("src/x.ts", text);
    let positions: Vec<(&str, u32, u32)> = found
        .imports
        .iter()
        .map(|i| (i.specifier.as_str(), i.line, i.column))
        .collect();
    assert_eq!(
        positions,
        [("./a", 1, 15), ("./b", 2, 32)],
        "columns count from 1, and é is one character"
    );
}

#[test]
fn records_the_utf16_start_and_end_of_each_specifier_on_its_line() {
    let text = "import a from './a';\n/* 🚀 */ const b = require('./bé');\n";
    let found = facts("src/x.ts", text);
    let positions: Vec<(&str, (u32, u32))> = found
        .imports
        .iter()
        .map(|i| (i.specifier.as_str(), i.utf16))
        .collect();
    assert_eq!(
        positions,
        [("./a", (14, 19)), ("./bé", (27, 33))],
        "from 0 on each line, quotes included; the rocket is two code units and é one"
    );
}

#[test]
fn records_the_names_each_import_statement_imports() {
    let text = "import d, { a as b, c } from './x';\nimport * as ns from './y';\nimport './z';\n";
    let found = facts("src/x.ts", text);
    let names: Vec<(&str, Vec<&str>)> = found
        .imports
        .iter()
        .map(|i| {
            (
                i.specifier.as_str(),
                i.names.iter().map(String::as_str).collect(),
            )
        })
        .collect();
    assert_eq!(
        names,
        [
            ("./x", vec!["default", "a", "c"]),
            ("./y", vec!["*"]),
            ("./z", vec![]),
        ],
        "an imported name is the exported one, not the local alias"
    );
}

#[test]
fn records_the_names_a_file_exports_and_its_star_exports() {
    let text = "export const a = 1;\nexport default function () {}\nexport { b as c } from './b';\nexport * from './d';\nexport { a as e };\n";
    let found = facts("src/x.ts", text);
    assert_eq!(
        found.exports.names,
        ["a", "c", "default", "e"],
        "every exported name, sorted"
    );
    assert_eq!(
        found.exports.star_from,
        ["./d"],
        "the star export's specifier"
    );
}

#[test]
fn marks_export_type_from_and_import_type_equals_as_type_imports() {
    let text = "export type { A } from './a';\nexport { b } from './b';\nimport type C = require('./c');\nimport d = require('./d');\n";
    let found = facts("src/x.ts", text);
    assert_eq!(
        specifiers(&found),
        [
            ("./a", ImportKind::Type),
            ("./b", ImportKind::Runtime),
            ("./c", ImportKind::Type),
            ("./d", ImportKind::Runtime),
        ],
        "each statement's own modifier decides its kind"
    );
}

#[test]
fn dynamic_import_of_a_template_without_substitutions_is_followed() {
    let found = facts("src/x.ts", "const page = import(`./pages/home`);\n");
    assert_eq!(
        specifiers(&found),
        [("./pages/home", ImportKind::Runtime)],
        "the template is a literal specifier"
    );
}

#[test]
fn dynamic_import_of_a_concatenation_is_counted_not_followed() {
    let found = facts("src/x.ts", "const page = import('./pages/' + name);\n");
    assert!(
        found.imports.is_empty(),
        "the computed specifier is not an import"
    );
    assert_eq!(
        found.unfollowed,
        [1],
        "its line is recorded as not followed"
    );
}

#[test]
fn finds_the_first_top_level_statement_that_is_the_files_own() {
    let own = |text: &str| {
        facts("src/a.ts", text)
            .own_statement
            .map(|own| (own.line, own.name))
    };
    let cases = [
        (
            "import { a } from 'x';\nexport * from 'y';\n;\nimport q = require('q');\n\
             export { a };\nexport { b } from 'z';\nexport const c = 1;\n",
            Some((7, Some("c".to_owned()))),
        ),
        (
            "import { a } from 'x';\nexport { a, b };\nfunction b() {}\n",
            Some((2, Some("b".to_owned()))),
        ),
        ("import x = Foo.Bar;\n", Some((1, Some("x".to_owned())))),
        ("export default 1;\n", Some((1, Some("default".to_owned())))),
        (
            "let first = 1, second = 2;\n",
            Some((1, Some("first".to_owned()))),
        ),
        ("function f() {}\n", Some((1, Some("f".to_owned())))),
        ("run();\n", Some((1, None))),
        ("import { a } from 'x';\nexport { a };\n", None),
    ];
    for (text, expected) in cases {
        assert_eq!(own(text), expected, "the own statement of:\n{text}");
    }
}
