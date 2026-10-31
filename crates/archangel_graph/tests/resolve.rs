//! Scenarios from the import-graph and configuration specs: where each import resolves, and what
//! archangel says when it cannot.
#![cfg(test)]

use archangel_graph::{
    GraphOptions, GraphProblem, ImportGraph, Resolution, UnresolvedReason, Work,
    describe_resolve_error,
};
use archangel_testkit::FixtureRepo;

fn build(repo: &FixtureRepo) -> ImportGraph {
    ImportGraph::build(repo.path(), &GraphOptions::default(), &mut Work::default())
}

fn resolution(graph: &ImportGraph, file: &str, specifier: &str) -> Resolution {
    let facts = graph
        .file(file)
        .and_then(|f| f.facts.as_ref())
        .expect("the file has facts");
    let index = facts
        .imports
        .iter()
        .position(|i| i.specifier == specifier)
        .expect("the import");
    graph.resolutions(file).expect("the file has resolutions")[index].clone()
}

fn package(name: &str, main: &str) -> String {
    format!(r#"{{ "name": "{name}", "version": "1.0.0", "main": "{main}" }}"#)
}

#[test]
fn resolves_a_path_alias_through_the_nearest_tsconfig() {
    let repo = FixtureRepo::new()
        .file(
            "apps/web/tsconfig.json",
            r#"{ "compilerOptions": { "paths": { "@/*": ["./src/*"] } } }"#,
        )
        .file(
            "apps/web/src/a.ts",
            "import { Button } from '@/shared/button';\n",
        )
        .file(
            "apps/web/src/shared/button.tsx",
            "export const Button = 1;\n",
        );
    let graph = build(&repo);
    assert_eq!(
        resolution(&graph, "apps/web/src/a.ts", "@/shared/button"),
        Resolution::File("apps/web/src/shared/button.tsx".to_owned()),
        "the alias resolves through the nearest tsconfig"
    );
}

#[test]
fn js_extension_resolves_to_the_typescript_file() {
    let repo = FixtureRepo::new()
        .file("src/a.ts", "import './b.js';\n")
        .file("src/b.ts", "");
    let graph = build(&repo);
    assert_eq!(
        resolution(&graph, "src/a.ts", "./b.js"),
        Resolution::File("src/b.ts".to_owned()),
        "b.ts is found"
    );
}

/// The files a specifier with a JavaScript extension finds, as TypeScript tries them: each
/// `(specifier, file)` is a repository holding only `file`, imported from `src/a.ts`.
fn finds_alone(cases: &[(&str, &str)]) {
    for (specifier, file) in cases {
        let repo = FixtureRepo::new()
            .file("src/a.ts", &format!("import '{specifier}';\n"))
            .file(file, "export {};\n");
        let graph = build(&repo);
        assert_eq!(
            resolution(&graph, "src/a.ts", specifier),
            Resolution::File((*file).to_owned()),
            "{specifier} finds {file}, as TypeScript does"
        );
    }
}

#[test]
fn js_extension_resolves_to_a_declaration_or_jsx_file() {
    finds_alone(&[
        ("./b.js", "src/b.tsx"),
        ("./b.js", "src/b.d.ts"),
        ("./b.js", "src/b.jsx"),
    ]);
}

#[test]
fn jsx_extension_resolves_to_the_typescript_file() {
    finds_alone(&[
        ("./b.jsx", "src/b.tsx"),
        ("./b.jsx", "src/b.ts"),
        ("./b.jsx", "src/b.d.ts"),
        ("./b.jsx", "src/b.jsx"),
        ("./b.jsx", "src/b.js"),
    ]);
}

#[test]
fn mjs_and_cjs_extensions_resolve_to_their_declaration_files() {
    finds_alone(&[("./b.mjs", "src/b.d.mts"), ("./b.cjs", "src/b.d.cts")]);
}

#[test]
fn a_js_specifier_prefers_the_typescript_file_in_typescripts_order() {
    let repo = [
        "src/b.ts",
        "src/b.tsx",
        "src/b.d.ts",
        "src/b.js",
        "src/b.jsx",
    ]
    .iter()
    .fold(
        FixtureRepo::new().file("src/a.ts", "import './b.js';\nimport './c.jsx';\n"),
        |repo, file| repo.file(file, "export {};\n"),
    )
    .file("src/c.tsx", "export {};\n")
    .file("src/c.ts", "export {};\n");
    let graph = build(&repo);
    assert_eq!(
        resolution(&graph, "src/a.ts", "./b.js"),
        Resolution::File("src/b.ts".to_owned()),
        "./b.js finds b.ts before b.tsx, b.d.ts, b.js and b.jsx"
    );
    assert_eq!(
        resolution(&graph, "src/a.ts", "./c.jsx"),
        Resolution::File("src/c.tsx".to_owned()),
        "./c.jsx finds c.tsx before c.ts"
    );
}

#[test]
fn js_extension_with_no_file_says_so_in_plain_words() {
    let repo = FixtureRepo::new().file("src/a.ts", "import './b.js';\n");
    let graph = build(&repo);
    let Resolution::Unresolved(reason) = resolution(&graph, "src/a.ts", "./b.js") else {
        panic!("the import is unresolved");
    };
    let message = reason.message("./b.js");
    assert!(
        message.contains("./b.js") && message.contains("TypeScript"),
        "plain words: {message}"
    );
}

#[cfg(unix)]
#[test]
fn linked_workspace_package_resolves_to_its_repository_file() {
    let repo = FixtureRepo::new()
        .file(
            "packages/contract/package.json",
            &package("@acme/contract", "src/index.ts"),
        )
        .file("packages/contract/src/index.ts", "export {};\n")
        .file("apps/web/a.ts", "import '@acme/contract';\n")
        .symlink("node_modules/@acme/contract", "../../packages/contract");
    let graph = build(&repo);
    assert_eq!(
        resolution(&graph, "apps/web/a.ts", "@acme/contract"),
        Resolution::File("packages/contract/src/index.ts".to_owned()),
        "the link resolves to the repository file"
    );
}

#[test]
fn import_of_an_installed_package_resolves_to_the_package() {
    let repo = FixtureRepo::new()
        .file(
            "node_modules/react/package.json",
            &package("react", "index.js"),
        )
        .file("node_modules/react/index.js", "")
        .file("node_modules/react/jsx-runtime.js", "")
        .file(".gitignore", "node_modules/\n")
        .file("src/a.ts", "import 'react/jsx-runtime';\n");
    let graph = build(&repo);
    assert_eq!(
        resolution(&graph, "src/a.ts", "react/jsx-runtime"),
        Resolution::Package("react".to_owned()),
        "it is the package"
    );
}

#[test]
fn import_of_a_css_file_resolves_to_the_repository_file() {
    let repo = FixtureRepo::new()
        .file("src/a.tsx", "import './a.module.css';\n")
        .file("src/a.module.css", "");
    let graph = build(&repo);
    assert_eq!(
        resolution(&graph, "src/a.tsx", "./a.module.css"),
        Resolution::File("src/a.module.css".to_owned()),
        "the CSS file is the resolution"
    );
}

#[test]
fn builtin_without_prefix_resolves_to_its_node_name() {
    let repo = FixtureRepo::new().file("src/a.ts", "import 'fs';\n");
    let graph = build(&repo);
    assert_eq!(
        resolution(&graph, "src/a.ts", "fs"),
        Resolution::Package("node:fs".to_owned()),
        "fs is node:fs"
    );
}

#[test]
fn relative_import_with_no_file_is_import_unresolved() {
    let repo = FixtureRepo::new().file("src/a.ts", "import './missing';\n");
    let graph = build(&repo);
    assert_eq!(
        resolution(&graph, "src/a.ts", "./missing"),
        Resolution::Unresolved(UnresolvedReason::NotFound),
        "the import is unresolved because nothing is there"
    );
}

#[test]
fn package_whose_name_ends_in_js_that_is_not_installed_is_not_found() {
    let repo = FixtureRepo::new().file("src/a.ts", "import 'chart.js';\n");
    let graph = build(&repo);
    assert_eq!(
        resolution(&graph, "src/a.ts", "chart.js"),
        Resolution::Unresolved(UnresolvedReason::NotFound),
        "a package name is not a file, so there is no TypeScript file to look for beside it"
    );
}

#[test]
fn resolver_loop_is_import_unresolved_with_the_reason() {
    let message = describe_resolve_error(&oxc_resolver::ResolveError::Recursion).message("a/x");
    assert!(
        message.contains("loops"),
        "the reason says the chain loops: {message}"
    );
}

#[test]
fn invalid_tsconfig_gives_one_diagnostic_on_that_file() {
    let mut repo = FixtureRepo::new().file(
        "apps/web/tsconfig.json",
        "{\n  \"compilerOptions\": {\n    \"strict\": true\n    \"paths\": {}\n  }\n}\n",
    );
    for index in 0..30 {
        repo = repo.file(&format!("apps/web/src/f{index}.ts"), "import './other';\n");
    }
    let graph = build(&repo);
    let found: Vec<&GraphProblem> = graph
        .problems()
        .iter()
        .filter(|p| matches!(p, GraphProblem::ResolutionFileInvalid { .. }))
        .collect();
    assert_eq!(
        found.len(),
        1,
        "one diagnostic for the tsconfig, found {found:?}"
    );
    let GraphProblem::ResolutionFileInvalid { path, reason } = found[0] else {
        panic!("the problem is a resolution-file problem");
    };
    assert_eq!(
        path, "apps/web/tsconfig.json",
        "the diagnostic is on the tsconfig"
    );
    assert!(
        reason.contains("line 4"),
        "the reason gives the line: {reason}"
    );
}

#[test]
fn tsconfig_extending_a_missing_package_gives_one_diagnostic() {
    let repo = FixtureRepo::new()
        .file(
            "tsconfig.json",
            r#"{ "extends": "@tsconfig/node20/tsconfig.json" }"#,
        )
        .file("src/a.ts", "import './b';\n")
        .file("src/b.ts", "")
        .file("src/c.ts", "import './b';\n");
    let graph = build(&repo);
    let found: Vec<&GraphProblem> = graph
        .problems()
        .iter()
        .filter(|p| matches!(p, GraphProblem::ResolutionFileInvalid { .. }))
        .collect();
    assert_eq!(found.len(), 1, "one diagnostic, found {found:?}");
    let GraphProblem::ResolutionFileInvalid { path, reason } = found[0] else {
        panic!("the problem is a resolution-file problem");
    };
    assert_eq!(path, "tsconfig.json", "the diagnostic is on the tsconfig");
    assert_eq!(
        reason, "its `extends` names `@tsconfig/node20`, which is not installed or does not exist",
        "the reason names the package, not the path the resolver looked for"
    );
}

#[cfg(unix)]
#[test]
fn unreadable_tsconfig_is_resolution_file_invalid() {
    let repo = FixtureRepo::new()
        .file(
            "tsconfig.json",
            r#"{ "compilerOptions": { "paths": { "@/*": ["./root/*"] } } }"#,
        )
        .file(
            "apps/web/tsconfig.json",
            r#"{ "compilerOptions": { "paths": { "@/*": ["./src/*"] } } }"#,
        )
        .file("apps/web/src/a.ts", "import '@/b';\n")
        .file("apps/web/src/b.ts", "")
        .file("root/b.ts", "")
        .unreadable("apps/web/tsconfig.json");
    let graph = build(&repo);
    let found = graph.problems().iter().find_map(|problem| match problem {
        GraphProblem::ResolutionFileInvalid { path, reason }
            if path == "apps/web/tsconfig.json" =>
        {
            Some(reason.clone())
        }
        _ => None,
    });
    let reason = found.expect("a diagnostic for the unreadable tsconfig");
    assert!(
        reason.contains("permission"),
        "the reason says permission was refused: {reason}"
    );
    assert_ne!(
        resolution(&graph, "apps/web/src/a.ts", "@/b"),
        Resolution::File("root/b.ts".to_owned()),
        "the parent's alias is not used instead"
    );
}

#[test]
fn environment_variables_do_not_change_resolutions() {
    let extra = FixtureRepo::new()
        .file("left-pad/package.json", &package("left-pad", "index.js"))
        .file("left-pad/index.js", "");
    let env = [
        ("NODE_PATH", extra.path().as_os_str()),
        ("OXC_RESOLVER_YARN_PNP", std::ffi::OsStr::new("1")),
    ];
    if !archangel_testkit::in_child_with_env(
        "environment_variables_do_not_change_resolutions",
        &env,
    ) {
        return;
    }
    let repo = FixtureRepo::new()
        .file(
            "node_modules/react/package.json",
            &package("react", "index.js"),
        )
        .file("node_modules/react/index.js", "")
        .file(".gitignore", "node_modules/\n")
        .file("src/a.ts", "import 'react';\nimport 'left-pad';\n");
    let graph = build(&repo);
    assert_eq!(
        resolution(&graph, "src/a.ts", "react"),
        Resolution::Package("react".to_owned()),
        "react resolves"
    );
    assert!(
        matches!(
            resolution(&graph, "src/a.ts", "left-pad"),
            Resolution::Unresolved(_)
        ),
        "NODE_PATH does not add folders"
    );
}

#[test]
fn resolves_packages_in_a_plug_n_play_repository() {
    let repo = FixtureRepo::new()
        .file(
            "package.json",
            r#"{ "name": "root", "dependencies": { "react": "18.0.0" } }"#,
        )
        .file(
            ".pnp.cjs",
            &archangel_testkit::pnp_manifest(&[("react", "./.yarn/cache/react/")]),
        )
        .file(
            ".yarn/cache/react/package.json",
            &package("react", "index.js"),
        )
        .file(".yarn/cache/react/index.js", "")
        .file(".gitignore", ".yarn/\n")
        .file("src/a.ts", "import 'react';\n");
    let graph = build(&repo);
    assert_eq!(
        resolution(&graph, "src/a.ts", "react"),
        Resolution::Package("react".to_owned()),
        "react resolves through Plug'n'Play"
    );
}

#[test]
fn describes_each_lookup_error_in_archangel_terms() {
    use oxc_resolver::{ResolveError, SpecifierError};
    let path = std::path::PathBuf::from("/repo/package.json");
    let cases = [
        (
            ResolveError::NotFound("x".to_owned()),
            UnresolvedReason::NotFound,
        ),
        (
            ResolveError::MatchedAliasNotFound("x".to_owned(), "@/*".to_owned()),
            UnresolvedReason::NotFound,
        ),
        (
            ResolveError::ExtensionAlias("x.js".to_owned(), "x.ts".to_owned(), path.clone()),
            UnresolvedReason::NoFileBehindJs,
        ),
        (
            ResolveError::PackagePathNotExported {
                subpath: "./x".to_owned(),
                package_path: path.clone(),
                package_json_path: path.clone(),
                conditions: vec!["import".to_owned()].into(),
            },
            UnresolvedReason::NotExported,
        ),
        (
            ResolveError::PackageImportNotDefined("#x".to_owned(), path.clone()),
            UnresolvedReason::ImportNotDefined,
        ),
        (
            ResolveError::InvalidPackageTarget("x".to_owned(), ".".to_owned(), path.clone()),
            UnresolvedReason::InvalidTarget,
        ),
        (
            ResolveError::InvalidModuleSpecifier("x".to_owned(), path.clone()),
            UnresolvedReason::InvalidSpecifier,
        ),
        (
            ResolveError::Specifier(SpecifierError::Empty(String::new())),
            UnresolvedReason::InvalidSpecifier,
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(describe_resolve_error(&error), expected, "{error:?}");
    }
}

#[test]
fn describes_unsupported_paths_loops_and_disk_errors_in_archangel_terms() {
    use oxc_resolver::ResolveError;
    let path = std::path::PathBuf::from("/repo/package.json");
    let cases = [
        (
            ResolveError::PathNotSupported(path.clone()),
            UnresolvedReason::InvalidSpecifier,
        ),
        (ResolveError::Recursion, UnresolvedReason::Loop),
        (
            ResolveError::from(std::io::Error::other("disk on fire")),
            UnresolvedReason::Io("disk on fire".to_owned()),
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(describe_resolve_error(&error), expected, "{error:?}");
    }
}

#[test]
fn describes_each_resolution_file_error_as_a_problem_with_that_file() {
    use oxc_resolver::{JSONError, ResolveError};
    let path = std::path::PathBuf::from("/repo/package.json");
    let cases = [
        (
            ResolveError::Json(JSONError {
                path: path.clone(),
                message: String::new(),
                line: 1,
                column: 1,
            }),
            UnresolvedReason::ResolutionFile,
        ),
        (
            ResolveError::TsconfigNotFound(path.clone()),
            UnresolvedReason::ResolutionFile,
        ),
        (
            ResolveError::TsconfigCircularExtend(vec![path.clone(), path.clone()].into()),
            UnresolvedReason::ResolutionFile,
        ),
        (
            ResolveError::TsconfigSelfReference(path.clone()),
            UnresolvedReason::ResolutionFile,
        ),
        (
            ResolveError::InvalidPackageConfig(path.clone()),
            UnresolvedReason::ResolutionFile,
        ),
        (
            ResolveError::InvalidPackageConfigDefault(path.clone()),
            UnresolvedReason::ResolutionFile,
        ),
        (
            ResolveError::InvalidPackageConfigDirectory(path.clone()),
            UnresolvedReason::ResolutionFile,
        ),
        (
            ResolveError::FailedToFindYarnPnpManifest(path.clone()),
            UnresolvedReason::ResolutionFile,
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(describe_resolve_error(&error), expected, "{error:?}");
    }
}

#[test]
fn describes_each_plug_n_play_error_in_archangel_terms() {
    use oxc_resolver::ResolveError;
    let path = std::path::PathBuf::from("/repo/package.json");
    let undeclared = pnp::UndeclaredDependency {
        message: String::new(),
        request: "react".to_owned(),
        dependency_name: "react".to_owned(),
        issuer_locator: pnp::PackageLocator {
            name: "root".to_owned(),
            reference: "workspace:.".to_owned(),
        },
        issuer_path: path.clone(),
    };
    let bad_specifier = pnp::BadSpecifier {
        message: String::new(),
        specifier: String::new(),
    };
    let cases = [
        (
            ResolveError::YarnPnpError(pnp::Error::BadSpecifier(Box::new(bad_specifier))),
            UnresolvedReason::InvalidSpecifier,
        ),
        (
            ResolveError::YarnPnpError(pnp::Error::UndeclaredDependency(Box::new(undeclared))),
            UnresolvedReason::NotDeclared,
        ),
        (ResolveError::Unimplemented("x"), UnresolvedReason::Unknown),
    ];
    for (error, expected) in cases {
        assert_eq!(describe_resolve_error(&error), expected, "{error:?}");
    }
}

#[test]
fn undeclared_package_in_a_plug_n_play_repository_is_not_declared() {
    let repo = FixtureRepo::new()
        .file(
            "package.json",
            r#"{ "name": "root", "dependencies": { "react": "18.0.0" } }"#,
        )
        .file(
            ".pnp.cjs",
            &archangel_testkit::pnp_manifest(&[("react", "./.yarn/cache/react/")]),
        )
        .file("src/a.ts", "import 'lodash';\n");
    let graph = build(&repo);
    assert_eq!(
        resolution(&graph, "src/a.ts", "lodash"),
        Resolution::Unresolved(UnresolvedReason::NotDeclared),
        "the map does not list lodash"
    );
}

/// The resolution-file problems of a graph, as (path, reason).
fn resolution_file_problems(graph: &ImportGraph) -> Vec<(&str, &str)> {
    graph
        .problems()
        .iter()
        .filter_map(|problem| match problem {
            GraphProblem::ResolutionFileInvalid { path, reason } => {
                Some((path.as_str(), reason.as_str()))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn tsconfig_extends_chain_that_loops_gives_a_diagnostic_on_the_tsconfig() {
    let repo = FixtureRepo::new()
        .file("a/tsconfig.json", r#"{ "extends": "../b/tsconfig.json" }"#)
        .file("b/tsconfig.json", r#"{ "extends": "../a/tsconfig.json" }"#)
        .file("a/x.ts", "import './y';\n");
    let graph = build(&repo);
    let found = resolution_file_problems(&graph);
    assert_eq!(found.len(), 1, "one diagnostic: {found:?}");
    assert_eq!(
        found[0].0, "a/tsconfig.json",
        "on the importing file's tsconfig"
    );
    assert_eq!(
        found[0].1,
        "its `extends` chain loops: `a/tsconfig.json` -> `b/tsconfig.json` -> `a/tsconfig.json`",
        "the reason names the chain as repository paths, the same on every machine"
    );
}

#[test]
fn tsconfig_that_references_itself_gives_a_diagnostic_on_the_tsconfig() {
    let repo = FixtureRepo::new()
        .file(
            "tsconfig.json",
            r#"{ "references": [{ "path": "./tsconfig.json" }] }"#,
        )
        .file("src/a.ts", "import './b';\n");
    let graph = build(&repo);
    assert_eq!(
        resolution_file_problems(&graph),
        [("tsconfig.json", "it extends or references itself")],
        "one diagnostic on the tsconfig"
    );
}

#[cfg(unix)]
#[test]
fn package_with_an_exports_field_that_is_not_valid_gives_a_diagnostic_on_its_manifest() {
    let repo = FixtureRepo::new()
        .file(
            "packages/p/package.json",
            r#"{ "name": "p", "exports": { ".": "./i.js", "a": "./a.js" } }"#,
        )
        .file("packages/p/i.js", "")
        .file("src/a.ts", "import 'p';\n")
        .symlink("node_modules/p", "../packages/p");
    let graph = build(&repo);
    assert_eq!(
        resolution(&graph, "src/a.ts", "p"),
        Resolution::Unresolved(UnresolvedReason::ResolutionFile),
        "the import points at the manifest's diagnostic"
    );
    assert_eq!(
        resolution_file_problems(&graph),
        [(
            "node_modules/p/package.json",
            "its `exports` or `imports` field is not valid"
        )],
        "one diagnostic on the manifest"
    );
}

#[test]
fn plug_n_play_file_deleted_under_a_built_graph_gives_a_diagnostic() {
    let repo = FixtureRepo::new()
        .file(
            "package.json",
            r#"{ "name": "root", "dependencies": { "react": "18.0.0" } }"#,
        )
        .file(
            ".pnp.cjs",
            &archangel_testkit::pnp_manifest(&[("react", "./.yarn/cache/react/")]),
        )
        .file("src/a.ts", "");
    let mut graph = build(&repo);
    repo.remove(".pnp.cjs");
    assert!(
        graph.replace_file("src/a.ts", Some("import 'react';\n"), &mut Work::default()),
        "the edit is parsed"
    );
    assert_eq!(
        resolution_file_problems(&graph),
        [(
            ".pnp.cjs",
            "the Plug'n'Play file is gone, so no package can be resolved through it"
        )],
        "the diagnostic names the file that is gone"
    );
}

#[cfg(unix)]
#[test]
fn unreadable_tsconfig_above_the_config_folder_is_resolution_file_invalid() {
    // The resolver uses the nearest tsconfig even above the config folder, so an unreadable one
    // there must be reported, not passed over.
    let repo = FixtureRepo::new()
        .file(
            "tsconfig.json",
            r#"{ "compilerOptions": { "paths": { "@/*": ["./inner/src/*"] } } }"#,
        )
        .file("inner/src/a.ts", "import '@/b';\n")
        .file("inner/src/b.ts", "")
        .unreadable("tsconfig.json");
    let graph = ImportGraph::build(
        &repo.join("inner"),
        &GraphOptions::default(),
        &mut Work::default(),
    );
    let reasons: Vec<&str> = resolution_file_problems(&graph)
        .into_iter()
        .map(|(_, reason)| reason)
        .collect();
    assert_eq!(
        reasons,
        ["archangel cannot read it: permission was refused"],
        "the unreadable tsconfig is reported"
    );
}

#[test]
fn package_without_a_name_in_its_nearest_manifest_is_named_by_its_folder() {
    let repo = FixtureRepo::new()
        .file(
            "node_modules/@scope/kit/package.json",
            r#"{ "main": "dist/index.js" }"#,
        )
        .file(
            "node_modules/@scope/kit/dist/package.json",
            r#"{ "type": "module" }"#,
        )
        .file("node_modules/@scope/kit/dist/index.js", "")
        .file(
            "node_modules/plain/package.json",
            r#"{ "main": "lib/index.js" }"#,
        )
        .file(
            "node_modules/plain/lib/package.json",
            r#"{ "type": "commonjs" }"#,
        )
        .file("node_modules/plain/lib/index.js", "")
        .file(".gitignore", "node_modules/\n")
        .file("src/a.ts", "import '@scope/kit';\nimport 'plain';\n");
    let graph = build(&repo);
    assert_eq!(
        graph.resolutions("src/a.ts"),
        Some(
            [
                Resolution::Package("@scope/kit".to_owned()),
                Resolution::Package("plain".to_owned()),
            ]
            .as_slice()
        ),
        "the folder after node_modules names the package, with its scope"
    );
}

#[test]
fn resolution_inputs_hold_the_files_read_and_the_files_found() {
    let repo = FixtureRepo::new()
        .file("tsconfig.json", r#"{ "extends": "./configs/base.json" }"#)
        .file(
            "configs/base.json",
            r#"{ "compilerOptions": { "strict": true } }"#,
        )
        .file(
            "node_modules/react/package.json",
            &package("react", "index.js"),
        )
        .file("node_modules/react/index.js", "")
        .file(".gitignore", "node_modules/\n")
        .file("src/data.json", "{}")
        .file("src/a.ts", "import 'react';\nimport './data.json';\n");
    let graph = build(&repo);
    let inputs = graph.resolution_inputs();
    let root = graph.root();
    for read in [
        "tsconfig.json",
        "configs/base.json",
        "node_modules/react/package.json",
    ] {
        assert!(
            inputs.contents.contains(&root.join(read)),
            "the contents of {read} decide resolutions"
        );
    }
    for found in ["node_modules/react/index.js", "src/data.json"] {
        assert!(
            inputs.existence.contains(&root.join(found)),
            "the existence of {found} decides a resolution"
        );
        assert!(
            !inputs.contents.contains(&root.join(found)),
            "the contents of {found} decide nothing"
        );
    }
}

#[cfg(unix)]
#[test]
fn resolution_inputs_hold_the_real_path_of_a_linked_package_manifest() {
    let repo = FixtureRepo::new()
        .file(
            "packages/contract/package.json",
            &package("@acme/contract", "src/index.ts"),
        )
        .file("packages/contract/src/index.ts", "export {};\n")
        .file("apps/web/a.ts", "import '@acme/contract';\n")
        .symlink("node_modules/@acme/contract", "../../packages/contract");
    let graph = build(&repo);
    assert!(
        graph
            .resolution_inputs()
            .contents
            .contains(&graph.root().join("packages/contract/package.json")),
        "an edit to the package's manifest in the repository changes the resolution"
    );
}

#[cfg(unix)]
#[test]
fn resolution_inputs_hold_the_real_path_of_a_linked_tsconfig() {
    let repo = FixtureRepo::new()
        .file(
            "configs/tsconfig.shared.json",
            r#"{ "compilerOptions": { "paths": { "@/*": ["./src/*"] } } }"#,
        )
        .symlink("tsconfig.json", "configs/tsconfig.shared.json")
        .file("src/b.ts", "")
        .file("src/a.ts", "import '@/b';\n");
    let graph = build(&repo);
    assert!(
        graph
            .resolution_inputs()
            .contents
            .contains(&graph.root().join("configs/tsconfig.shared.json")),
        "an edit to the file the tsconfig links to changes the resolution"
    );
}

#[cfg(unix)]
#[test]
fn resolution_inputs_hold_the_real_path_of_a_tsconfig_in_a_linked_folder() {
    let repo = FixtureRepo::new()
        .file(
            "configs/tsconfig.base.json",
            r#"{ "compilerOptions": { "paths": { "@/*": ["./src/*"] } } }"#,
        )
        .symlink("shared", "configs")
        .file(
            "tsconfig.json",
            r#"{ "extends": "./shared/tsconfig.base.json" }"#,
        )
        .file("src/b.ts", "")
        .file("src/a.ts", "import '@/b';\n");
    let graph = build(&repo);
    assert!(
        graph
            .resolution_inputs()
            .contents
            .contains(&graph.root().join("configs/tsconfig.base.json")),
        "an edit to the extended tsconfig, at its real path, changes the resolution"
    );
}
