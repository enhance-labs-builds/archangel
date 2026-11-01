# import-graph Specification

## Purpose
Every rule depends on a correct list of imports. This capability covers which files archangel reads,
which imports it finds in them, and how it resolves each import, so that no file or import is lost
without a diagnostic.

## Requirements

### Requirement: Archangel reads every source file that the repository does not ignore

Archangel MUST read every file below the config folder with the extension `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`, `.mjs` or `.cjs`, in hidden folders too. It MUST NOT read a file that a `.gitignore` file in the repository ignores, or that matches the config's `ignore` list. Declaration files MUST be source files.

#### Scenario: Ignored and included files

- **GIVEN** a repository with `src/a.ts`, `src/b.d.ts`, `dist/a.js`, and a `.gitignore` that ignores `dist/`
- **WHEN** archangel makes a check
- **THEN** the source files are `src/a.ts` and `src/b.d.ts`
- **TEST** `crates/archangel_graph/tests/files.rs` › "reads_source_files_that_git_does_not_ignore"

#### Scenario: A source file in a hidden folder

- **GIVEN** a repository with `.storybook/main.ts` and no ignore rule for it
- **WHEN** archangel makes a check
- **THEN** `.storybook/main.ts` is a source file
- **TEST** `crates/archangel_graph/tests/files.rs` › "reads_source_files_in_hidden_folders"

#### Scenario: The config ignores a folder

- **GIVEN** a config whose `ignore` list contains `fixtures/`, and a file `fixtures/broken.ts`
- **WHEN** archangel makes a check
- **THEN** `fixtures/broken.ts` is not a source file
- **TEST** `crates/archangel_graph/tests/files.rs` › "config_ignore_list_removes_files"

#### Scenario: A repository with no source files

- **GIVEN** a repository with a config and no source files
- **WHEN** the developer runs `archangel check`
- **THEN** the summary counts zero files
- **AND** archangel prints an `element-empty` diagnostic for each file element
- **TEST** `crates/archangel/tests/cli.rs` › "repository_with_no_source_files_reports_every_file_element_empty"

### Requirement: Archangel uses only the repository's own ignore rules

The repository MUST be the nearest folder at or above the config folder that contains `.git`, as a folder or as a file, or the config folder when no such folder exists. The summary MUST list each nested repository below the config folder, because the outer `.gitignore` rules also apply inside it. Archangel MUST use only the `.gitignore` files in the repository, so that the same files give the same source files on every machine.

#### Scenario: An ignore rule on one machine only

- **GIVEN** a repository with `src/a.ts`, and a global Git ignore file on the machine that ignores `src/`
- **WHEN** archangel makes a check
- **THEN** `src/a.ts` is a source file
- **TEST** `crates/archangel_graph/tests/files.rs` › "ignores_ignore_rules_from_outside_the_repository"

#### Scenario: Ignore files that differ between machines

- **GIVEN** a repository with `src/a.ts`, a `.ignore` file that ignores `src/`, and a `.git/info/exclude` file that ignores `src/`
- **WHEN** archangel makes a check
- **THEN** `src/a.ts` is a source file
- **TEST** `crates/archangel_graph/tests/files.rs` › "ignores_dot_ignore_and_git_info_exclude"

#### Scenario: A Git worktree

- **GIVEN** a Git worktree whose `.git` is a file, with a `.gitignore` at its top that ignores `dist/`, and a config in `packages/web`
- **WHEN** archangel makes a check
- **THEN** archangel uses the top `.gitignore`, and `packages/web/dist/a.js` is not a source file
- **TEST** `crates/archangel_graph/tests/files.rs` › "git_file_in_a_worktree_marks_the_repository"

#### Scenario: A .gitignore above a folder with no Git

- **GIVEN** a folder with a config and `src/a.ts` but no `.git`, inside a parent folder whose `.gitignore` ignores `src/`
- **WHEN** archangel makes a check
- **THEN** `src/a.ts` is a source file
- **TEST** `crates/archangel_graph/tests/files.rs` › "gitignore_above_the_repository_is_not_used"

### Requirement: Archangel tells the developer about each path it does not enter

Archangel MUST NOT follow a symbolic link, to a folder or to a file, when it looks for source files, because one file can then appear at two paths. The summary MUST list each symbolic link that archangel did not follow. A folder or file that archangel cannot open MUST give a `file-unreadable` diagnostic, and the check MUST fail.

#### Scenario: A symbolic link to a folder

- **GIVEN** a repository in which `src/legacy` is a symbolic link to `../old/src`
- **WHEN** archangel makes a check
- **THEN** archangel does not read files through `src/legacy`
- **AND** the summary lists `src/legacy` as a link that archangel did not follow
- **TEST** `crates/archangel_graph/tests/files.rs` › "symbolic_link_to_a_folder_is_listed_and_not_followed"

#### Scenario: A symbolic link to a file

- **GIVEN** a repository in which `src/b.ts` is a symbolic link to `src/a.ts`
- **WHEN** archangel makes a check
- **THEN** `src/a.ts` is a source file and `src/b.ts` is not
- **AND** the summary lists `src/b.ts` as a link that archangel did not follow
- **TEST** `crates/archangel_graph/tests/files.rs` › "symbolic_link_to_a_file_is_listed_and_not_followed"

#### Scenario: A folder that archangel cannot open

- **GIVEN** a repository with the folder `src/private`, which the user cannot read
- **WHEN** the developer runs `archangel check`
- **THEN** archangel prints a `file-unreadable` diagnostic for `src/private` that says permission was refused
- **AND** archangel exits with status 1
- **TEST** `crates/archangel_graph/tests/files.rs` › "folder_that_cannot_be_opened_is_file_unreadable"
- **TEST** `crates/archangel/tests/cli.rs` › "file_unreadable_exits_one"

### Requirement: Archangel reads source files as UTF-8 or UTF-16 text

Archangel MUST read a source file as UTF-8, or as UTF-16 when the file starts with a UTF-16 byte order mark. A file that is not valid text in one of these encodings MUST give a `file-unreadable` diagnostic that names the encoding problem.

#### Scenario: A file in UTF-16

- **GIVEN** the source file `src/a.ts` in UTF-16 with a byte order mark, which imports `./b`
- **WHEN** archangel makes a check
- **THEN** archangel finds the import of `./b`
- **TEST** `crates/archangel_graph/tests/files.rs` › "reads_utf16_files_with_a_byte_order_mark"

#### Scenario: A file that is not text

- **GIVEN** a source file `src/a.ts` whose bytes are not valid UTF-8 and that has no byte order mark
- **WHEN** archangel makes a check
- **THEN** archangel gives a `file-unreadable` diagnostic for `src/a.ts` that says the file is not UTF-8
- **TEST** `crates/archangel_graph/tests/files.rs` › "file_that_is_not_text_is_file_unreadable"

### Requirement: Archangel finds every import form that names a literal specifier

Archangel MUST find these imports: import statements, export-from statements, dynamic imports and require calls with a literal specifier, TypeScript import-equals declarations with a require, import types, and triple-slash path and types references. Archangel MUST record the line, the column in characters and the specifier of each import, the start and end of the specifier on its line in UTF-16 code units for editors, and the names that each import statement imports.

#### Scenario: Each runtime import form

- **GIVEN** a source file with one import statement, one export-from, one dynamic import, one require call and one import-equals declaration, each with a different specifier
- **WHEN** archangel reads the file
- **THEN** archangel finds five imports with the five specifiers and their lines
- **TEST** `crates/archangel_graph/tests/imports.rs` › "finds_each_runtime_import_form_with_a_literal_specifier"

#### Scenario: An import type

- **GIVEN** a source file with `type Store = import('./store').Store`
- **WHEN** archangel reads the file
- **THEN** archangel finds a type import of `./store`
- **TEST** `crates/archangel_graph/tests/imports.rs` › "finds_import_types_as_type_imports"

#### Scenario: The positions of a specifier

- **GIVEN** a source file with an `é` before a require call on one line, and on another a rocket emoji before a require call whose specifier has an `é`
- **WHEN** archangel reads the file
- **THEN** each import's column counts characters from 1, and its UTF-16 start and end count code units from 0, quotes included
- **TEST** `crates/archangel_graph/tests/imports.rs` › "records_the_line_and_column_of_each_specifier_in_characters"
- **TEST** `crates/archangel_graph/tests/imports.rs` › "records_the_utf16_start_and_end_of_each_specifier_on_its_line"

#### Scenario: Triple-slash references

- **GIVEN** a declaration file with `/// <reference path="./globals.d.ts" />` and `/// <reference types="vite/client" />`
- **WHEN** archangel reads the file
- **THEN** archangel finds a type import of the file `./globals.d.ts` and a type import of the package `vite`
- **TEST** `crates/archangel_graph/tests/imports.rs` › "finds_triple_slash_references_as_type_imports"

#### Scenario: Computed specifiers

- **GIVEN** a source file with `import(\`./pages/${name}\`)` and `require(path)`
- **WHEN** the developer runs `archangel explain` on the file
- **THEN** archangel makes no import from either
- **AND** the summary counts two imports that archangel cannot follow, and the explain output lists their lines
- **TEST** `crates/archangel_graph/tests/imports.rs` › "computed_specifiers_are_counted_not_followed"
- **TEST** `crates/archangel/tests/cli.rs` › "explain_lists_the_lines_of_computed_specifiers"

### Requirement: Archangel reads JSX in JavaScript files

Archangel MUST parse `.js`, `.mjs` and `.cjs` files with JSX syntax permitted, because many React projects write JSX in `.js` files.

#### Scenario: JSX in a JavaScript file

- **GIVEN** the file `src/app.js` with JSX and an import of `./button`
- **WHEN** archangel makes a check
- **THEN** archangel finds the import of `./button` and gives no `parse-failed` diagnostic
- **TEST** `crates/archangel_graph/tests/imports.rs` › "parses_jsx_in_javascript_files"

### Requirement: Archangel marks type imports

The runtime or type mark of an import MUST follow what TypeScript emits for the file's options, not the syntax alone. An import or export-from statement MUST be a type import when it has the `type` modifier before its names. It MUST also be a type import when every name that it imports or re-exports has the `type` modifier and the tsconfig of the file does not set `verbatimModuleSyntax`. With that option, TypeScript keeps such a statement and the module runs, so it MUST be a runtime import. Export-all statements with the `type` modifier, import types, triple-slash references and every import in a declaration file MUST be type imports. Every other import or export-from statement MUST be a runtime import. The mark MUST be decided when the import is resolved, from the tsconfig that governs the file, so a change to that tsconfig MUST change the mark on the next check with no new parse of the file.

#### Scenario: Three kinds of import statement

- **GIVEN** a source file with `import type { A } from './a'`, `import { type B } from './b'` and `import { type C, d } from './c'`, and a tsconfig without `verbatimModuleSyntax`
- **WHEN** archangel reads the file
- **THEN** the imports of `./a` and `./b` are type imports
- **AND** the import of `./c` is a runtime import
- **TEST** `crates/archangel_graph/tests/imports.rs` › "marks_type_imports_by_their_modifiers"

#### Scenario: Type names under verbatimModuleSyntax

- **GIVEN** a source file with `import type { A } from './a'` and `import { type B } from './b'`, and a tsconfig that sets `verbatimModuleSyntax: true`
- **WHEN** archangel reads the file
- **THEN** the import of `./a` is a type import, and the import of `./b` is a runtime import
- **TEST** `crates/archangel_graph/tests/imports.rs` › "inline_type_names_are_a_runtime_import_under_verbatim_module_syntax"

#### Scenario: A re-export of type names

- **GIVEN** a source file with `export { type A } from './a'` and `export { type B, c } from './c'`
- **AND** the same file read once with a tsconfig without `verbatimModuleSyntax` and once with `verbatimModuleSyntax: true`
- **WHEN** archangel reads the file each time
- **THEN** without the option, the re-export of `./a` is a type import and the re-export of `./c` is a runtime import
- **AND** with the option, both are runtime imports
- **TEST** `crates/archangel_graph/tests/imports.rs` › "a_re_export_of_type_names_follows_verbatim_module_syntax"

#### Scenario: A cycle through an inline type import

- **GIVEN** a tsconfig that sets `verbatimModuleSyntax: true`, a rule `{ files: 'src/**', noCycles: true }`, and the files `src/a.ts` with `import { type B } from './b'` and `src/b.ts` with `import { a } from './a'`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of the rule for the group `src/a.ts` and `src/b.ts`
- **TEST** `crates/archangel_rules/tests/cycles.rs` › "a_cycle_through_an_inline_type_import_runs_under_verbatim_module_syntax"

#### Scenario: A cycle through a barrel's type re-export

- **GIVEN** a tsconfig that sets `verbatimModuleSyntax: true`, a rule `{ files: 'src/**', noCycles: true }`, and the files `src/index.ts` with `export { type B } from './b'` and `src/b.ts` with `import { a } from './index'`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of the rule for the group `src/index.ts` and `src/b.ts`
- **AND** the same files with a tsconfig without the option give no violation
- **TEST** `crates/archangel_rules/tests/cycles.rs` › "a_cycle_through_a_type_re_export_runs_under_verbatim_module_syntax"

#### Scenario: A tsconfig gains verbatimModuleSyntax in a watched repository

- **GIVEN** a watched repository with a rule `{ files: 'src/**', noCycles: true }`, the files `src/a.ts` with `import { type B } from './b'` and `src/b.ts` with `import { a } from './a'`, and a tsconfig without `verbatimModuleSyntax`, whose first check gives no violation
- **WHEN** the tsconfig gains `verbatimModuleSyntax: true` and neither file is edited
- **THEN** the next check gives one violation of the rule for the group `src/a.ts` and `src/b.ts`, with zero files parsed
- **TEST** `crates/archangel_engine/tests/work.rs` › "a_tsconfig_that_gains_verbatim_module_syntax_re_marks_every_import_without_a_parse"

#### Scenario: A type export of all names

- **GIVEN** a source file with `export type * from './a'` and `export type * as b from './b'`
- **WHEN** archangel reads the file
- **THEN** both imports are type imports
- **TEST** `crates/archangel_graph/tests/imports.rs` › "export_type_star_is_a_type_import"

#### Scenario: An import in a declaration file

- **GIVEN** the declaration file `src/env.d.ts` with `import { Config } from './config'`
- **WHEN** archangel reads the file
- **THEN** the import of `./config` is a type import
- **TEST** `crates/archangel_graph/tests/imports.rs` › "imports_in_declaration_files_are_type_imports"

### Requirement: A file that archangel cannot parse is a diagnostic

When a source file has a syntax error, archangel MUST give one `parse-failed` diagnostic that lists every error the parser found, each with its line and column. Archangel MUST NOT use any import from that file, because a parser that recovers from an error can find imports that are not there. The check MUST fail.

#### Scenario: A syntax error

- **GIVEN** a source file `src/a.ts` with a syntax error on line 3 and a valid import on line 1
- **WHEN** the developer runs `archangel check`
- **THEN** archangel prints a `parse-failed` diagnostic for `src/a.ts` that lists the error on line 3
- **AND** no rule checks the import on line 1
- **AND** archangel exits with status 1
- **TEST** `crates/archangel_graph/tests/imports.rs` › "syntax_error_is_parse_failed_and_its_imports_are_not_used"
- **TEST** `crates/archangel/tests/cli.rs` › "parse_failed_exits_one"

#### Scenario: A file that is too large for the parser

- **GIVEN** a sparse source file larger than 4 GiB
- **WHEN** archangel makes a check
- **THEN** archangel gives a `parse-failed` diagnostic that says the file is larger than the parser accepts, without a read of the file
- **TEST** `crates/archangel_graph/tests/files.rs` › "file_larger_than_the_parser_limit_is_parse_failed"

#### Scenario: Deeply nested code

- **GIVEN** a generated source file with expressions nested 10,000 levels deep
- **WHEN** archangel makes a check
- **THEN** archangel parses the file and does not stop with a stack overflow
- **TEST** `crates/archangel_graph/tests/imports.rs` › "parses_code_nested_ten_thousand_levels_deep"

#### Scenario: A Flow file

- **GIVEN** the file `src/legacy.js` with a `@flow` comment and Flow type syntax
- **WHEN** archangel makes a check
- **THEN** archangel gives a `parse-failed` diagnostic that says Flow syntax is not supported and that the config's ignore list can leave the file out
- **TEST** `crates/archangel_graph/tests/imports.rs` › "flow_file_is_parse_failed_with_the_reason"

### Requirement: Archangel resolves an import as TypeScript does

Archangel MUST resolve each import to a repository file or a package. It MUST use the nearest tsconfig of the importing file, with its `extends`, `baseUrl` and `paths`. It MUST use the `exports` and `imports` fields of package manifests. It MUST find the file behind a specifier that ends in `.js`, `.jsx`, `.mjs` or `.cjs` in TypeScript's order: the TypeScript file, then the declaration file, then the JavaScript file. It MUST resolve a symbolic link to the real path of its file.

#### Scenario: A JavaScript extension in front of a declaration or JSX file

- **GIVEN** the file `src/a.ts` that imports `./b.js`, and a repository whose only `b` is `src/b.tsx`, `src/b.d.ts` or `src/b.jsx`
- **WHEN** archangel resolves the import
- **THEN** the resolution is that file, as TypeScript's is
- **TEST** `crates/archangel_graph/tests/resolve.rs` › "js_extension_resolves_to_a_declaration_or_jsx_file"

#### Scenario: A JSX extension

- **GIVEN** the file `src/a.ts` that imports `./b.jsx`, and a repository whose only `b` is `src/b.tsx`, `src/b.ts`, `src/b.d.ts`, `src/b.jsx` or `src/b.js`
- **WHEN** archangel resolves the import
- **THEN** the resolution is that file
- **TEST** `crates/archangel_graph/tests/resolve.rs` › "jsx_extension_resolves_to_the_typescript_file"

#### Scenario: Module and CommonJS declaration files

- **GIVEN** the file `src/a.ts` that imports `./b.mjs` with only `src/b.d.mts`, or `./b.cjs` with only `src/b.d.cts`
- **WHEN** archangel resolves the import
- **THEN** the resolution is the declaration file
- **TEST** `crates/archangel_graph/tests/resolve.rs` › "mjs_and_cjs_extensions_resolve_to_their_declaration_files"

#### Scenario: Several files behind one JavaScript specifier

- **GIVEN** `src/b.ts`, `src/b.tsx`, `src/b.d.ts`, `src/b.js` and `src/b.jsx`, and `src/c.tsx` and `src/c.ts`, with `src/a.ts` that imports `./b.js` and `./c.jsx`
- **WHEN** archangel resolves the imports
- **THEN** `./b.js` resolves to `src/b.ts` and `./c.jsx` to `src/c.tsx`, the first file in TypeScript's order
- **TEST** `crates/archangel_graph/tests/resolve.rs` › "a_js_specifier_prefers_the_typescript_file_in_typescripts_order"

#### Scenario: A path alias from the nearest tsconfig

- **GIVEN** `apps/web/tsconfig.json` with the path `@/*` mapped to `src/*`
- **AND** the file `apps/web/src/a.ts` that imports `@/shared/button`
- **WHEN** archangel resolves the import
- **THEN** the resolution is `apps/web/src/shared/button.tsx`
- **TEST** `crates/archangel_graph/tests/resolve.rs` › "resolves_a_path_alias_through_the_nearest_tsconfig"

#### Scenario: A JavaScript extension that points to a TypeScript file

- **GIVEN** the file `src/a.ts` that imports `./b.js`, and the file `src/b.ts`
- **WHEN** archangel resolves the import
- **THEN** the resolution is `src/b.ts`
- **TEST** `crates/archangel_graph/tests/resolve.rs` › "js_extension_resolves_to_the_typescript_file"

#### Scenario: A workspace package linked into node_modules

- **GIVEN** a workspace package at `packages/contract`, linked as `node_modules/@acme/contract`
- **AND** a source file that imports `@acme/contract`
- **WHEN** archangel resolves the import
- **THEN** the resolution is the repository file `packages/contract/src/index.ts`, not a package
- **TEST** `crates/archangel_graph/tests/resolve.rs` › "linked_workspace_package_resolves_to_its_repository_file"

#### Scenario: An installed package

- **GIVEN** the package `react` installed in `node_modules`
- **WHEN** a source file imports `react/jsx-runtime`
- **THEN** the resolution is the package `react`
- **TEST** `crates/archangel_graph/tests/resolve.rs` › "import_of_an_installed_package_resolves_to_the_package"

#### Scenario: A file that is not a source file

- **GIVEN** the file `src/a.tsx` that imports `./a.module.css`, and that CSS file
- **WHEN** archangel resolves the import
- **THEN** the resolution is the repository file `src/a.module.css`
- **TEST** `crates/archangel_graph/tests/resolve.rs` › "import_of_a_css_file_resolves_to_the_repository_file"

#### Scenario: Environment variables do not change a resolution

- **GIVEN** a repository without Plug'n'Play that has `react` in `node_modules`, and a folder `/opt/lib` with a package `left-pad`
- **AND** the environment variables `NODE_PATH=/opt/lib` and `OXC_RESOLVER_YARN_PNP=1`
- **WHEN** archangel resolves an import of `react` and an import of `left-pad`
- **THEN** the resolution of `react` is the package `react`, and the import of `left-pad` is unresolved
- **TEST** `crates/archangel_graph/tests/resolve.rs` › "environment_variables_do_not_change_resolutions"

#### Scenario: Archangel agrees with the TypeScript compiler

- **GIVEN** the fixture repositories in `crates/archangel_graph/tests/fixtures/resolution`
- **WHEN** archangel and the TypeScript compiler each resolve every import in them
- **THEN** the two resolutions are the same for every import
- **TEST** `crates/archangel_graph/tests/resolve_differential.rs` › "resolutions_agree_with_the_typescript_compiler"

### Requirement: An unresolved import is a diagnostic unless the config permits it

When archangel cannot resolve an import, it MUST give an `import-unresolved` diagnostic with the specifier and the reason in plain words. The config's `allowUnresolved` field MUST map specifiers, which may end in `*`, to a reason. Archangel MUST NOT give the diagnostic for a specifier that matches one of them, and the summary MUST count those imports.

#### Scenario: A relative import with no file

- **GIVEN** the file `src/a.ts` that imports `./missing`
- **WHEN** the developer runs `archangel check`
- **THEN** archangel prints an `import-unresolved` diagnostic for `./missing` in `src/a.ts`
- **AND** archangel exits with status 1
- **TEST** `crates/archangel_graph/tests/resolve.rs` › "relative_import_with_no_file_is_import_unresolved"
- **TEST** `crates/archangel/tests/cli.rs` › "import_unresolved_exits_one"

#### Scenario: A JavaScript extension with no file

- **GIVEN** the file `src/a.ts` that imports `./b.js`, and no `src/b.ts`, `src/b.tsx`, `src/b.d.ts`, `src/b.js` or `src/b.jsx`
- **WHEN** archangel makes a check
- **THEN** archangel gives an `import-unresolved` diagnostic that says there is no `./b.js` and no TypeScript or declaration file beside it
- **TEST** `crates/archangel_graph/tests/resolve.rs` › "js_extension_with_no_file_says_so_in_plain_words"

#### Scenario: A specifier that the config permits

- **GIVEN** a config whose `allowUnresolved` maps `astro:*` to "Astro makes these at build time"
- **AND** a source file that imports `astro:content`
- **WHEN** archangel makes a check
- **THEN** archangel gives no `import-unresolved` diagnostic
- **AND** the summary counts one permitted unresolved import
- **TEST** `crates/archangel_rules/tests/rules.rs` › "permitted_unresolved_specifier_is_counted_not_reported"

#### Scenario: A Yarn Plug'n'Play repository

- **GIVEN** a repository that uses Yarn Plug'n'Play, with no `node_modules` folder, and a source file that imports `react`
- **WHEN** archangel resolves the import
- **THEN** the resolution is the package `react`
- **TEST** `crates/archangel_graph/tests/resolve.rs` › "resolves_packages_in_a_plug_n_play_repository"

#### Scenario: The resolver reports a loop

- **GIVEN** a resolver failure that reports a loop, which the resolver gives after 64 steps through alias fields
- **WHEN** archangel turns the failure into a diagnostic
- **THEN** the `import-unresolved` diagnostic says that the chain loops or is too long, in plain words
- **TEST** `crates/archangel_graph/tests/resolve.rs` › "resolver_loop_is_import_unresolved_with_the_reason"

#### Scenario: A tsconfig with a missing comma

- **GIVEN** `apps/web/tsconfig.json` with a missing comma on line 4, and 30 source files that it applies to
- **WHEN** the developer runs `archangel check`
- **THEN** archangel prints one `resolution-file-invalid` diagnostic for `apps/web/tsconfig.json` on line 4
- **AND** archangel exits with status 1
- **TEST** `crates/archangel_graph/tests/resolve.rs` › "invalid_tsconfig_gives_one_diagnostic_on_that_file"
- **TEST** `crates/archangel/tests/cli.rs` › "resolution_file_invalid_exits_one"

#### Scenario: A tsconfig that archangel cannot read

- **GIVEN** `apps/web/tsconfig.json`, which the user cannot read, and a source file below it
- **WHEN** archangel makes a check
- **THEN** archangel gives a `resolution-file-invalid` diagnostic for `apps/web/tsconfig.json` that says permission was refused
- **AND** archangel does not resolve the imports below it with another tsconfig
- **TEST** `crates/archangel_graph/tests/resolve.rs` › "unreadable_tsconfig_is_resolution_file_invalid"

#### Scenario: A tsconfig that extends a package that is not installed

- **GIVEN** `tsconfig.json` with `"extends": "@tsconfig/node20/tsconfig.json"`, and no such package installed
- **WHEN** archangel makes a check
- **THEN** archangel gives one `resolution-file-invalid` diagnostic for `tsconfig.json` that names `@tsconfig/node20` and says it is not installed
- **TEST** `crates/archangel_graph/tests/resolve.rs` › "tsconfig_extending_a_missing_package_gives_one_diagnostic"

### Requirement: A check reads and parses each source file once

In one check, archangel MUST read each source file one time, parse it one time, and resolve each of its imports one time.

#### Scenario: Work for a full check

- **GIVEN** a repository with 40 source files, in which each file has 3 imports
- **WHEN** archangel makes a check
- **THEN** the work counts are 40 files read, 40 files parsed and 120 resolutions
- **TEST** `crates/archangel_graph/tests/work.rs` › "full_check_reads_and_parses_each_file_once"

#### Scenario: The fault control for the work count

- **GIVEN** a build of archangel with a fault that parses each file two times
- **WHEN** the full check work test runs against that build
- **THEN** the test fails on the count of files parsed
- **TEST** `crates/archangel_graph/tests/work.rs` › "work_count_rejects_a_second_parse"

### Requirement: A new tsconfig takes effect in watch and the editor

A tsconfig that appears at a path that the lookup of a file passed MUST make archangel resolve and mark that file's imports again, so that a session gives the answer of a full check.

#### Scenario: A first tsconfig that sets verbatimModuleSyntax

- **GIVEN** a session on a repository with no tsconfig, whose cycle exists only as an inline type import
- **WHEN** `tsconfig.json` appears with `verbatimModuleSyntax: true`
- **THEN** the session reports the cycle
- **TEST** `crates/archangel_engine/tests/work.rs` › "a_new_tsconfig_with_verbatim_module_syntax_re_marks_imports_in_the_editor"

#### Scenario: A nearer tsconfig

- **GIVEN** a session on a repository whose root tsconfig sets `verbatimModuleSyntax: true`, with that cycle reported
- **WHEN** `src/tsconfig.json` appears with `verbatimModuleSyntax: false`
- **THEN** the session reports no cycle
- **TEST** `crates/archangel_engine/tests/work.rs` › "a_nearer_new_tsconfig_takes_over_from_the_one_above_in_the_editor"

#### Scenario: A tsconfig that gives an alias its target

- **GIVEN** a session in which `@/a` does not resolve
- **WHEN** `tsconfig.json` appears with a `paths` entry for `@/*`
- **THEN** the import resolves to its file
- **TEST** `crates/archangel_engine/tests/work.rs` › "a_new_tsconfig_with_paths_resolves_an_alias_in_the_editor"
