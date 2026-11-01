# import-graph

## MODIFIED Requirements

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
