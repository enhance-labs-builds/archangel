# export-rules Specification

## Purpose
Rules about what a file exports: names it must export, kinds of export it may not have, and files that
may only pass on what other files export, so that a module's public face is part of the architecture.

## Requirements

### Requirement: A mustExport rule requires an export whose name matches

A `mustExport` rule MUST take a name pattern, which may use the spellings of the file's and folder's own names, and MUST give one violation for each file in its selector that exports no name matching it. Re-exports, and the names an `export *` chain brings in, MUST count as exports of the file.

#### Scenario: A view that does not export its component

- **GIVEN** a rule `{ files: '**/*.view.tsx', mustExport: '{FileName}' }`, and the file `src/button.view.tsx` that exports only `ButtonView`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation that says the file must export `Button` and lists the names it exports
- **TEST** `crates/archangel_rules/tests/exports.rs` › "must_export_rejects_a_file_without_the_name"

#### Scenario: A name passed on from another file

- **GIVEN** the same rule, and the file `src/card.view.tsx` with `export { Card } from './card.impl'`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation for that file
- **TEST** `crates/archangel_rules/tests/exports.rs` › "a_re_export_counts_as_an_export"

#### Scenario: A name passed on through export star

- **GIVEN** the same rule, the file `src/card.view.tsx` with `export * from './card.impl'`, and `src/card.impl.ts` that exports `Card`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation for `src/card.view.tsx`
- **TEST** `crates/archangel_rules/tests/exports.rs` › "export_star_names_count_as_exports"

#### Scenario: An export star that archangel cannot follow

- **GIVEN** the same rule, and the file `src/chip.view.tsx` whose only export is `export * from '@acme/chips'`, a package
- **WHEN** archangel makes a check
- **THEN** archangel gives an `exports-unknown` diagnostic for the file that names `@acme/chips`, and no violation of the rule
- **TEST** `crates/archangel_rules/tests/exports.rs` › "an_export_star_into_a_package_is_exports_unknown"

#### Scenario: The source of an export star changes in the editor

- **GIVEN** a session with the same rule, `src/card.view.tsx` with `export * from './card.impl'`, and no violation
- **WHEN** the developer removes the export of `Card` from `src/card.impl.ts`
- **THEN** the language server reports the violation on `src/card.view.tsx`
- **AND** the result is the same as a full check
- **TEST** `crates/archangel_engine/tests/sessions.rs` › "an_edit_to_an_export_star_source_re_checks_the_barrel"

#### Scenario: A file that does not parse

- **GIVEN** the same rule, and a view file with a syntax error
- **WHEN** archangel makes a check
- **THEN** archangel gives `parse-failed` for the file and no violation of the rule
- **TEST** `crates/archangel_rules/tests/exports.rs` › "an_unparsed_file_gets_no_export_violation"

### Requirement: A mayNotExport rule refuses default exports, star exports or named exports

A `mayNotExport` rule MUST accept `default`, `star` (an `export *` or `export * as`) or a name pattern, and MUST give one violation for each such export in a file of its selector, at the export's line.

#### Scenario: A default export

- **GIVEN** a rule `{ files: 'src/**', mayNotExport: 'default' }`, and a file with `export default function App() {}`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation at the line of that export
- **TEST** `crates/archangel_rules/tests/exports.rs` › "may_not_export_rejects_a_default_export"

#### Scenario: A wildcard in a public API

- **GIVEN** a rule `{ files: '**/index.ts', mayNotExport: 'star' }`, and an index file with `export * from './model'`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation at the line of that export
- **TEST** `crates/archangel_rules/tests/exports.rs` › "may_not_export_rejects_a_star_export"

#### Scenario: The other forms of export

- **GIVEN** a rule `{ files: 'src/**', mayNotExport: 'star' }` and a file with `export * as model from './model'`, and a rule `{ files: 'src/**', mayNotExport: 'default' }` and a file with `export { default } from './app'`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation for each file
- **TEST** `crates/archangel_rules/tests/exports.rs` › "namespace_star_and_re_exported_default_count"

#### Scenario: A name pattern

- **GIVEN** a rule `{ files: 'src/**', mayNotExport: 'internal*' }`, and a file that exports `internalState` and `state`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for `internalState`
- **TEST** `crates/archangel_rules/tests/exports.rs` › "may_not_export_rejects_names_that_match"

### Requirement: A mustOnlyReExport rule keeps a file to re-exports

A `mustOnlyReExport` rule MUST give one violation for each file in its selector that has a top-level statement other than an import, an export from another file, a directive, or an `export { … }` of names the file imported. A type declaration MUST count as the file's own.

#### Scenario: A barrel that declares a function

- **GIVEN** a rule `{ files: '**/index.ts', mustOnlyReExport: true }`, and an index file with `export { a } from './a'` and `export function b() {}`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation that points at the declaration of `b`
- **TEST** `crates/archangel_rules/tests/exports.rs` › "must_only_re_export_rejects_a_declaration"

#### Scenario: A barrel that imports and re-exports

- **GIVEN** the same rule, and an index file with `import { a } from './a'` and `export { a }`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation, because the file declares nothing of its own
- **TEST** `crates/archangel_rules/tests/exports.rs` › "an_import_then_export_is_a_re_export"

#### Scenario: A barrel that declares a type

- **GIVEN** the same rule, and an index file with `export type Id = string`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation that points at the declaration of `Id`
- **TEST** `crates/archangel_rules/tests/exports.rs` › "a_type_declaration_is_the_files_own"

### Requirement: Export rules need no second parse

Archangel MUST evaluate the export rules from the facts that the one parse of each file kept. An edit to one file MUST evaluate the export rules for that file and for the files that re-export from it through `export *` chains, and no others.

#### Scenario: An edit to one file

- **GIVEN** a session with export rules on a repository of 1,000 files, in which one barrel re-exports `a.ts` through `export *`
- **WHEN** the developer edits the text of `a.ts`
- **THEN** the count of files parsed is one and the count of export evaluations is two
- **TEST** `crates/archangel_engine/tests/work.rs` › "an_edit_evaluates_the_export_rules_of_one_file"

#### Scenario: The fault control for export evaluations

- **GIVEN** a build of archangel with a fault that evaluates the export rules of every file after an edit
- **WHEN** the edit work test runs against that build
- **THEN** the test fails on the count of export evaluations
- **TEST** `crates/archangel_engine/tests/work.rs` › "export_evaluation_count_rejects_every_file_per_edit"
