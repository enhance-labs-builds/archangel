//! Scenarios from the import-graph spec: a type import that falls back to `@types`, an `import()`
//! of a template literal, which stands for one import of each file it can load, and the files that
//! the config's ignore list leaves out.
#![cfg(test)]

#[path = "support/resolutions.rs"]
mod resolutions;

use resolutions::{build, resolution};

use archangel_graph::{GraphOptions, ImportGraph, Resolution, UnresolvedReason, Work};
use archangel_testkit::FixtureRepo;

/// Each import of a file and its resolution, as (specifier, resolution).
fn imports_of(graph: &ImportGraph, file: &str) -> Vec<(String, Resolution)> {
    let facts = graph
        .file(file)
        .and_then(|f| f.facts.as_ref())
        .expect("facts");
    facts
        .imports
        .iter()
        .zip(graph.resolutions(file).expect("resolutions"))
        .map(|(import, resolution)| (import.specifier.clone(), resolution.clone()))
        .collect()
}

#[test]
fn a_type_import_of_a_types_only_package_resolves_to_the_package() {
    let repo = FixtureRepo::new()
        .file(
            "node_modules/@types/geo/package.json",
            r#"{ "name": "@types/geo", "version": "1.0.0", "types": "index.d.ts" }"#,
        )
        .file("node_modules/@types/geo/index.d.ts", "export interface Point { x: number }\n")
        .file(
            "node_modules/@types/acme__maps/package.json",
            r#"{ "name": "@types/acme__maps", "version": "1.0.0", "types": "index.d.ts" }"#,
        )
        .file("node_modules/@types/acme__maps/index.d.ts", "export interface Map { z: number }\n")
        .file(".gitignore", "node_modules/\n")
        .file(
            "src/a.ts",
            "import type { Point } from 'geo';\nimport type { Map } from '@acme/maps';\nimport { Point as P } from 'geo';\n",
        );
    let graph = build(&repo);
    let resolutions = graph.resolutions("src/a.ts").expect("resolutions");
    assert_eq!(
        resolutions[..2],
        [
            Resolution::Package("geo".to_owned()),
            Resolution::Package("@acme/maps".to_owned())
        ],
        "a type import falls back to @types, as TypeScript does, under the name it imports"
    );
    assert_eq!(
        resolutions[2],
        Resolution::Unresolved(UnresolvedReason::NotFound),
        "a runtime import of a package that is not installed stays unresolved"
    );
}

#[test]
fn a_template_import_resolves_to_each_file_that_its_static_parts_match() {
    let repo = FixtureRepo::new()
        .file(
            "src/app/load.ts",
            "export const load = (name: string) => import(`../locales/${name}.json`);\nexport const page = (p: string) => import(`./pages/${p}/index.ts`);\n",
        )
        .file("src/locales/en.json", "{}")
        .file("src/locales/fr.json", "{}")
        .file("src/locales/nested/de.json", "{}")
        .file("src/app/pages/home/index.ts", "")
        .file("src/app/pages/about/index.ts", "");
    let file = |path: &str| Resolution::File(path.to_owned());
    let locales = "../locales/${name}.json".to_owned();
    let pages = "./pages/${p}/index.ts".to_owned();
    assert_eq!(
        imports_of(&build(&repo), "src/app/load.ts"),
        [
            (locales.clone(), file("src/locales/en.json")),
            (locales, file("src/locales/fr.json")),
            (pages.clone(), file("src/app/pages/about/index.ts")),
            (pages, file("src/app/pages/home/index.ts")),
        ],
        "each `${{…}}` is one file or folder name, so nested/de.json is out"
    );
}

#[test]
fn a_template_that_matches_no_file_is_unresolved_and_a_computed_specifier_unfollowed() {
    let repo = FixtureRepo::new().file(
        "src/app/load.ts",
        "export const none = (x: string) => import(`./missing/${x}.ts`);\nexport const any = (url: string) => import(url);\n",
    );
    let graph = build(&repo);
    assert_eq!(
        imports_of(&graph, "src/app/load.ts"),
        [(
            "./missing/${x}.ts".to_owned(),
            Resolution::Unresolved(UnresolvedReason::TemplateMatchesNothing)
        )],
        "a template that matches no file is an unresolved import"
    );
    let facts = graph
        .file("src/app/load.ts")
        .and_then(|f| f.facts.as_ref())
        .expect("facts");
    assert_eq!(
        facts.unfollowed,
        [2],
        "import(url) has no static part, so it is unfollowed"
    );
}

#[test]
fn a_template_import_reaches_the_same_files_after_resolving_again() {
    let repo = FixtureRepo::new()
        .file(
            "src/app/load.ts",
            "export const load = (name: string) => import(`../locales/${name}.json`);\n",
        )
        .file("src/locales/en.json", "{}")
        .file("src/locales/fr.json", "{}");
    let mut graph = build(&repo);
    assert!(
        graph.has_template_imports(),
        "the file has a template import"
    );
    let before = imports_of(&graph, "src/app/load.ts");
    graph.resolve_all_again(&mut Work::default());
    assert_eq!(
        imports_of(&graph, "src/app/load.ts"),
        before,
        "the template stands for one import of each file, however often it resolves"
    );
    assert_eq!(before.len(), 2, "one import of each locale");
}

#[test]
fn a_graph_without_a_template_import_says_so() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "import './b';\n")
        .file("src/b.ts", "");
    assert!(!build(&repo).has_template_imports(), "no template import");
}

#[test]
fn a_template_with_a_backslash_is_not_followed() {
    let repo = FixtureRepo::new().file(
        "src/load.ts",
        "export const load = (n: string) => import(`./locales/${n}\\u0041.json`);\n",
    );
    let graph = build(&repo);
    let facts = graph
        .file("src/load.ts")
        .and_then(|f| f.facts.as_ref())
        .expect("facts");
    assert_eq!(
        (facts.imports.len(), facts.unfollowed.clone()),
        (0, vec![1]),
        "an escape can stand for any text, so archangel does not read the template as a pattern"
    );
}

#[test]
fn verbatim_module_syntax_keeps_a_typed_name_from_falling_back_to_types() {
    let types_only = |tsconfig: &str| {
        FixtureRepo::new()
            .file("tsconfig.json", tsconfig)
            .file(
                "node_modules/@types/geo/package.json",
                r#"{ "name": "@types/geo", "version": "1.0.0", "types": "index.d.ts" }"#,
            )
            .file(
                "node_modules/@types/geo/index.d.ts",
                "export interface Point { x: number }\n",
            )
            .file(".gitignore", "node_modules/\n")
            .file("src/a.ts", "import { type Point } from 'geo';\n")
    };
    let first = |repo: &FixtureRepo| resolution(&build(repo), "src/a.ts", "geo");
    assert_eq!(
        first(&types_only("{}")),
        Resolution::Package("geo".to_owned()),
        "the compiler removes an import of typed names, so it falls back to @types"
    );
    assert_eq!(
        first(&types_only(
            r#"{ "compilerOptions": { "verbatimModuleSyntax": true } }"#
        )),
        Resolution::Unresolved(UnresolvedReason::NotFound),
        "under verbatimModuleSyntax the import stays, and runs, so it does not fall back"
    );
}

#[test]
fn the_ignore_list_names_the_files_it_leaves_out() {
    let repo = FixtureRepo::new()
        .file("src/feature/public/index.ts", "")
        .file("src/generated/a.ts", "")
        .file("src/b.ts", "");
    let options = GraphOptions {
        ignore: vec!["public/".to_owned(), "src/generated/".to_owned()],
    };
    let graph = ImportGraph::build(repo.path(), &options, &mut Work::default());
    assert_eq!(
        graph.left_out_by_ignore(),
        [
            (
                "src/feature/public/index.ts".to_owned(),
                "public/".to_owned()
            ),
            ("src/generated/a.ts".to_owned(), "src/generated/".to_owned()),
        ],
        "each file the list leaves out, with the first line that does"
    );
    assert_eq!(
        build(&repo).left_out_by_ignore(),
        [],
        "without an ignore list, nothing is left out"
    );
}
