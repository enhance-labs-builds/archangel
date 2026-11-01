# architecture-rules

## ADDED Requirements

### Requirement: A mayOnlyImport rule with an area judges only imports into that area

A `mayOnlyImport` rule MAY have an area, written with `within` and with the values of a rule side. The rule MUST then give a violation only for an import whose other end is in the area, and neither on the other side nor in the rule's own selector. The rule MUST NOT judge an import whose other end is outside the area.

#### Scenario: An entity imports a use case

- **GIVEN** a rule `{ files: 'entity', mayOnlyImport: ['value-object'], within: 'apps/api/src/**' }`
- **AND** a file in "entity" that imports `apps/api/src/canvas/application/save.use-case.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation that names "value-object" as what the file may import and `apps/api/src/**` as the area
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_import_within_rejects_an_import_into_the_area_outside_its_list"

#### Scenario: An entity imports a package and a file of another app

- **GIVEN** the same rule
- **AND** a file in "entity" that imports `zod` and `apps/web/src/shared/lib/format.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of that rule
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_import_within_does_not_judge_an_import_outside_the_area"

#### Scenario: An area that includes packages

- **GIVEN** a rule `{ files: 'entity', mayOnlyImport: [{ packages: ['zod'] }], within: ['apps/api/src/**', { packages: ['*'] }] }`
- **AND** a file in "entity" that imports `zod` and `lodash`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for the import of `lodash`
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_import_within_judges_the_packages_in_its_area"

#### Scenario: The files import each other

- **GIVEN** the same rule as in the first scenario
- **AND** a file in "entity" that imports another file in "entity"
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of that rule
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_import_within_permits_its_own_files"

#### Scenario: An area with an import limit

- **GIVEN** a rule `{ files: 'entity', mayOnlyImport: ['value-object'], within: 'apps/api/src/**', specifier: 'parent' }`
- **AND** a file in "entity" that imports a use case through `../application/save.use-case`, and another through the alias `@api/application/load.use-case`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for the import that starts with `../`
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_import_within_combines_with_the_import_limits"

#### Scenario: An import that does not resolve

- **GIVEN** the rule `{ files: 'entity', mayOnlyImport: ['value-object'], within: 'apps/api/src/**' }`
- **AND** a file in "entity" that imports `./missing`
- **WHEN** archangel makes a check
- **THEN** archangel gives one `import-unresolved` diagnostic and no violation of the rule
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "an_unresolved_import_is_not_a_violation_of_a_rule_with_an_area"

#### Scenario: An area agrees with its double negative

- **GIVEN** random configs with `mayOnlyImport` rules that have an area, and the same configs with each rule written as `{ files, mayNotImport: { all: [area, { not: [side, files] }] } }`
- **WHEN** archangel checks random repositories with both
- **THEN** each pair of checks gives the same violations
- **TEST** `crates/archangel_rules/tests/rule_properties.rs` › "an_area_agrees_with_its_double_negative"

### Requirement: A mayOnlyBeImportedBy rule with an area judges only importers in that area

A `mayOnlyBeImportedBy` rule MAY have an area, written with `within`. The rule MUST then give a violation only for an import from a file in the area into its files, when that file is neither on the other side nor in the rule's own selector. An importer outside the area MUST NOT be judged.

#### Scenario: A script outside the area imports the database

- **GIVEN** a rule `{ files: 'db', mayOnlyBeImportedBy: 'api', within: 'apps/**' }`
- **AND** the file `scripts/seed.ts` and a file in "web", which both import a file in "db"
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for the import in "web"
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_be_imported_by_within_does_not_judge_an_importer_outside_the_area"

### Requirement: A mayNotImport rule with an area narrows its side to the area

A `mayNotImport` rule MAY have an area, written with `within`. The rule MUST then give the violations of the same rule whose side is `{ all: [side, area] }`.

#### Scenario: Use cases inside and outside the area

- **GIVEN** a rule `{ files: 'entity', mayNotImport: 'use-case', within: 'apps/api/src/**' }`
- **AND** a file in "entity" that imports `apps/api/src/canvas/application/save.use-case.ts` and `packages/shared/src/load.use-case.ts`, both in "use-case"
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for the import of `save.use-case.ts`
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_not_import_within_refuses_only_the_side_in_the_area"

#### Scenario: An area on mayNotImport agrees with the narrowed side

- **GIVEN** random configs with `mayNotImport` rules that have an area, and the same configs with each rule written as `{ files, mayNotImport: { all: [side, area] } }`
- **WHEN** archangel checks random repositories with both
- **THEN** each pair of checks gives the same violations
- **TEST** `crates/archangel_rules/tests/rule_properties.rs` › "an_area_on_may_not_import_agrees_with_the_narrowed_side"

### Requirement: An area is refused on a condition that checks no import

Archangel MUST refuse `within` on a rule whose condition is not an import condition, with `config-invalid` that names the condition.

#### Scenario: An area on a naming rule

- **GIVEN** a rule `{ files: 'entity', mustBeNamed: 'kebab-case', within: 'apps/api/src/**' }`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at `within`, saying that `mustBeNamed` checks no import
- **TEST** `crates/archangel_config/tests/validate.rs` › "within_on_a_condition_that_checks_no_import_is_refused"

### Requirement: An empty area is refused

An area with no value, such as an empty list or an `any` with no member, MUST be refused with `config-invalid` at `within`.

#### Scenario: An empty area

- **GIVEN** a rule `{ files: 'entity', mayOnlyImport: ['value-object'], within: [] }`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at `within`, saying that an area needs at least one value
- **TEST** `crates/archangel_config/tests/validate.rs` › "an_empty_area_is_refused"

### Requirement: An area that names an undeclared selector is refused

A name in an area that names no declared element or selector MUST give `rule-unknown-element` at `within`, with the closest declared name.

#### Scenario: A misspelt selector in an area

- **GIVEN** the declared selector `api-src` and a rule `{ files: 'entity', mayOnlyImport: ['value-object'], within: 'api-scr' }`
- **WHEN** archangel loads the config
- **THEN** archangel gives `rule-unknown-element` at `within`, and names `api-src` as the closest name
- **TEST** `crates/archangel_config/tests/validate.rs` › "within_naming_an_undeclared_selector_is_refused_with_the_closest_name"

### Requirement: An area that matches no file is an error

An inline pattern in an area that matches no file MUST give a `selector-empty` diagnostic that names the rule and the pattern, except a pattern that the area writes only inside a `not`.

#### Scenario: A misspelt area

- **GIVEN** a rule `{ name: 'entities', files: 'entity', mayOnlyImport: ['value-object'], within: 'apps/apii/src/**' }` and no file under `apps/apii`
- **WHEN** archangel makes a check
- **THEN** archangel gives a `selector-empty` diagnostic that names the rule "entities" and the pattern `apps/apii/src/**`
- **TEST** `crates/archangel_rules/tests/selectors.rs` › "an_area_pattern_that_matches_nothing_is_an_error"

### Requirement: An import outside an area costs no rule evaluation

An import whose other end is outside the area of a rule MUST NOT count a rule evaluation for that rule.

#### Scenario: Imports outside the area

- **GIVEN** a rule `{ files: 'entity', mayOnlyImport: ['value-object'], within: 'apps/api/src/**' }`, and two repositories: one where entity files have 10 imports of packages, and one where they have 1,000
- **WHEN** archangel makes a check of each repository
- **THEN** the two checks have the same count of rule evaluations
- **TEST** `crates/archangel_rules/tests/work.rs` › "imports_outside_an_area_cost_no_rule_evaluation"

#### Scenario: The fault control for areas

- **GIVEN** a build of archangel with a fault that tests the area of each rule for each import
- **WHEN** the area work test runs against that build
- **THEN** the test fails on the count of rule evaluations
- **TEST** `crates/archangel_rules/tests/work.rs` › "rule_evaluation_count_rejects_an_area_test_per_import"

### Requirement: A config without the new forms gives the results of 0.1

A config that uses no area and no matrix MUST load, check and test with the same diagnostics, summary, JSON output and exit status as with archangel 0.1.1. The JSON summary of such a config MUST have no key for rows with no file, and no key for files with no row.

#### Scenario: The 0.1 test configs

- **GIVEN** the fixtures of `crates/archangel/tests/output.rs` and `crates/archangel/tests/test_command.rs`, with the text and JSON output of `archangel check` and `archangel test` on each, without `--stats`, recorded from main at 0.1.1 before any other commit of this change
- **WHEN** archangel 0.2.0 runs the same commands
- **THEN** the standard output and the exit status are the same as the recordings, byte for byte
- **AND** the JSON summary has no `rowsWithNoFile` key and no `filesWithNoRow` key
- **TEST** `crates/archangel/tests/output.rs` › "configs_without_the_new_forms_give_the_recorded_output"

## MODIFIED Requirements

### Requirement: Each shorthand kind means the sentences that it stands for

Each of the four shorthand kinds, layers, isolate, entry and matrix, MUST give exactly the violations of the sentences it stands for.

#### Scenario: Shorthands agree with their sentences

- **GIVEN** random configs written with shorthand kinds, and the same configs written as sentences
- **WHEN** archangel checks random repositories with both
- **THEN** each pair of checks gives the same violations
- **TEST** `crates/archangel_rules/tests/rule_properties.rs` › "shorthands_agree_with_their_sentences"

#### Scenario: A matrix agrees with the rules that it replaces

- **GIVEN** random matrices with and without a role pattern, and for each row the rule `{ files: { all: [files, row] }, mayNotImport: { all: [area, { not: cells }] } }` under the row's name
- **WHEN** archangel checks random repositories with both
- **THEN** each pair of checks gives the same violations under the same rule names
- **TEST** `crates/archangel_rules/tests/rule_properties.rs` › "a_matrix_agrees_with_one_double_negative_rule_per_row"

### Requirement: The documentation shows each shorthand as its sentences

The documentation page of each shorthand kind, and `archangel docs` for it, MUST show the sentences it stands for.

#### Scenario: The isolate page

- **GIVEN** the documentation page for isolate
- **WHEN** the test reads it
- **THEN** it shows `rule({ files: 'feature', mayNotImport: 'feature', instance: 'different' })`
- **TEST** `crates/archangel_rules/tests/diagnostics.rs` › "each_shorthand_page_shows_its_sentences"

#### Scenario: The matrix page

- **GIVEN** the documentation page for matrix
- **WHEN** the test reads it
- **THEN** it shows a row that lists its own role as one sentence with `mayOnlyImport` and `within`
- **AND** it shows a row that does not list its own role as that sentence and one `mayNotImport` sentence of its own files
- **TEST** `crates/archangel_rules/tests/diagnostics.rs` › "each_shorthand_page_shows_its_sentences"
