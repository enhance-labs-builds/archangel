# import-graph

## MODIFIED Requirements

### Requirement: Archangel marks type imports

An import MUST be a type import when its statement has the `type` modifier before its names. An import MUST also be a type import when every name that it imports has the `type` modifier, unless the tsconfig of the file sets `verbatimModuleSyntax`. With that option, TypeScript keeps the statement and the module runs, so the import MUST be a runtime import. Export-all statements with the `type` modifier, import types, triple-slash references and every import in a declaration file MUST be type imports. Every other import MUST be a runtime import.

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

#### Scenario: A cycle through an inline type import

- **GIVEN** a tsconfig that sets `verbatimModuleSyntax: true`, a rule `{ files: 'src/**', noCycles: true }`, and the files `src/a.ts` with `import { type B } from './b'` and `src/b.ts` with `import { a } from './a'`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of the rule for the group `src/a.ts` and `src/b.ts`
- **TEST** `crates/archangel_rules/tests/cycles.rs` › "a_cycle_through_an_inline_type_import_runs_under_verbatim_module_syntax"

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
