# architecture-rules

## ADDED Requirements

### Requirement: A mayOnlyImport rule with an area judges only imports into that area

A `mayOnlyImport` rule MAY have an area, written with `within` and with the values of a rule side. The rule MUST then give a violation only for an import whose other end is in the area, and neither on the other side nor among its own files. The rule MUST NOT judge an import whose other end is outside the area.

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

- **GIVEN** random configs with `mayOnlyImport` rules that have an area and whose files have no capture, and the same configs with each rule written as `{ files, mayNotImport: { all: [area, { not: [side, files] }] } }`; the generator leaves captures out because the double negative permits every instance of `files`, where the rule permits only the same one, so the captured case is held by the scenario "One feature imports another" and by the reference evaluator
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

### Requirement: An area is refused on mayNotImport and on a condition that checks no import

Archangel MUST refuse `within` on a rule whose condition is not `mayOnlyImport` or `mayOnlyBeImportedBy`, with `config-invalid` that names the condition. For `mayNotImport`, the message MUST give the fix `mayNotImport: { all: [side, area] }` with the rule's own side and area, because that form already means what the area would mean.

#### Scenario: An area on mayNotImport

- **GIVEN** a rule `{ files: 'entity', mayNotImport: 'use-case', within: 'apps/api/src/**' }`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at `within`, whose message says to write `mayNotImport: { all: ['use-case', 'apps/api/src/**'] }`
- **TEST** `crates/archangel_config/tests/validate.rs` › "within_on_may_not_import_is_refused_with_the_narrowed_side_as_the_fix"

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

A config that uses none of the new keys, forms and options MUST load, check and test with the same diagnostics, summary, JSON output and exit status as with archangel 0.1.1. Two cases are exempt, and the test MUST name each fixture that they change: a `mayOnlyImport` rule whose files are a part with a capture, and a file whose tsconfig sets `verbatimModuleSyntax`. The JSON summary of such a config MUST have no key for rows with no file.

#### Scenario: The 0.1 test configs

- **GIVEN** the fixtures of `crates/archangel/tests/output.rs` and `crates/archangel/tests/test_command.rs`, with the text and JSON output of `archangel check` and `archangel test` on each, without `--stats`, recorded from main at 0.1.1 before any other commit of this change
- **WHEN** archangel 0.2.0 runs the same commands
- **THEN** the standard output and the exit status are the same as the recordings, byte for byte, except for the fixtures that the test names with one of the two exempt cases
- **AND** the JSON summary has no `rowsWithNoFile` key
- **TEST** `crates/archangel/tests/output.rs` › "configs_without_the_new_forms_give_the_recorded_output"

### Requirement: An area can name an externals set

An area MUST accept the name of an externals set. A `mayOnlyImport` rule whose area names an externals set MUST judge the imports of the packages in that set. The list of a `mayOnlyImport` rule MAY be empty only when the rule has an area: the rule then refuses every import into the area, except of its own files. Without an area, an empty list MUST stay `config-invalid`, and its message MUST name `within`.

#### Scenario: An area of drivers

- **GIVEN** the externals set `driver` as `['pg', 'drizzle-orm']`, and a rule `rule('drivers-in-infrastructure-only', { files: 'domain', mayOnlyImport: [], within: 'driver' })`
- **AND** a file in "domain" that imports `pg`, `zod` and a file in "domain"
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for the import of `pg`
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_import_within_an_externals_set_judges_its_packages"

#### Scenario: An empty list without an area

- **GIVEN** a rule `{ files: 'entity', mayOnlyImport: [] }`
- **AND** the same rule with `within: 'src/**'`
- **WHEN** archangel loads each config
- **THEN** the first gives `config-invalid` at `mayOnlyImport`, whose message names `within`, and the second loads
- **TEST** `crates/archangel_config/tests/validate.rs` › "an_empty_may_only_import_needs_an_area"

### Requirement: A package in an area must exist somewhere

A package that an area names MUST give the `element-empty` diagnostic that a package on a rule side gives, when no import uses it and no package manifest lists it, because a misspelt package in an area stops the rule from judging it.

#### Scenario: A misspelt package in an area

- **GIVEN** a rule with `within: [{ packages: ['zodd'] }]`, and a repository where no import uses `zodd` and no manifest lists it
- **WHEN** archangel makes a check
- **THEN** archangel gives one `element-empty` diagnostic that names the rule and `zodd`
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "an_area_package_that_no_import_uses_and_no_manifest_lists_is_an_error"

## MODIFIED Requirements

### Requirement: A mayOnlyImport rule refuses imports from its files into anything else

A `mayOnlyImport` rule MUST give a violation for each import from a file in its selector into a file or package that is neither on the other side nor its own files in the same instance. Without an area, the rule MUST judge every import, outside packages included. When the selector is a part with a capture, its own files MUST be the files of the same instance; otherwise they MUST be all the files of the selector. An import that does not resolve MUST NOT be a violation of the rule.

#### Scenario: A view imports a store

- **GIVEN** a rule `{ files: 'view', mayOnlyImport: ['shared', { packages: ['react'] }] }`, and a view file that imports a file in "model"
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation that names "shared" and the package `react` as what the file may import
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_import_rejects_an_import_outside_its_list"

#### Scenario: A view imports another view and React

- **GIVEN** the same rule, and a view file that imports another view file and `react`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of that rule
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_import_permits_its_list_and_its_own_files"

#### Scenario: A view imports a stylesheet

- **GIVEN** the same rule, and a view file that imports `./button.css`, which exists
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation for the import of `button.css`, because the stylesheet is not in the rule's list
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_import_counts_imports_of_non_source_files"

#### Scenario: A view imports a file that does not exist

- **GIVEN** the same rule, and a view file that imports `./missing`
- **WHEN** archangel makes a check
- **THEN** archangel gives an `import-unresolved` diagnostic for that import and no violation of the rule
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "an_unresolved_import_is_not_a_condition_violation"

#### Scenario: Without an area, an outside package is judged

- **GIVEN** a rule `{ files: 'view', mayOnlyImport: ['shared'] }` with no area, and a view file that imports `zod`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation for the import of `zod`
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_import_without_an_area_judges_outside_packages"

#### Scenario: One feature imports another

- **GIVEN** the part `feature` as `src/features/{feature}/**`, and a rule `{ files: 'feature', mayOnlyImport: ['shared'] }`
- **AND** the file `src/features/cart/a.ts`, which imports `src/features/cart/b.ts` and `src/features/login/c.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for the import of `src/features/login/c.ts`
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_import_permits_its_own_files_in_the_same_instance"

### Requirement: One import can break several rules

Archangel MUST check each import against every rule. When an import breaks more than one rule, archangel MUST give one violation for each rule that it breaks, and the summary MUST count each one. A table row is a rule in this count. An exception MUST permit only the violation of the rule that it names. The baseline MUST record one entry for each rule. The order of the rules in the config MUST NOT change the result.

#### Scenario: An import that breaks two rules

- **GIVEN** a layers rule and an entry rule, and one import that breaks both
- **WHEN** archangel makes a check
- **THEN** archangel gives two violations for that import, one for each rule
- **TEST** `crates/archangel_rules/tests/rules.rs` › "one_import_can_break_two_rules"

#### Scenario: An import that breaks a table row and a rule

- **GIVEN** the enhance3-slice config with one more rule, `rule('domain-not-to-application', { files: 'domain', mayNotImport: 'application' })`
- **AND** a domain entity that imports a use case
- **WHEN** archangel makes a check
- **THEN** archangel gives two violations for that import, one of `domain.entity` and one of `domain-not-to-application`, and the summary counts two problems
- **TEST** `crates/archangel_rules/tests/rules.rs` › "a_table_row_and_a_rule_each_count_for_one_import"

#### Scenario: An exception for one of two rules

- **GIVEN** the same config and import, under the comment `// archangel-allow domain.entity: the move to a port is in progress`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, of `domain-not-to-application`, and counts one exception for `domain.entity`
- **TEST** `crates/archangel_rules/tests/rules.rs` › "an_exception_permits_only_the_row_that_it_names"

#### Scenario: A baseline for one import that breaks two rules

- **GIVEN** the same config and import, with no exception
- **WHEN** the developer runs `archangel baseline add`
- **THEN** the baseline has one entry for `domain.entity` and one for `domain-not-to-application`, each with the count 1
- **TEST** `crates/archangel/tests/baseline_command.rs` › "a_baseline_records_one_entry_for_each_rule_that_an_import_breaks"

#### Scenario: A table that does not name a package

- **GIVEN** the enhance3-slice config, whose table `domain` names no externals set, and a domain entity that imports `pg`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, of `core-not-to-driver`, and none of the table
- **TEST** `crates/archangel_rules/tests/rules.rs` › "a_table_that_names_no_externals_set_leaves_a_package_to_other_rules"

#### Scenario: The order of the rules

- **GIVEN** a config and the same config with its rules in the opposite order
- **WHEN** archangel checks one repository with each config
- **THEN** the two checks give the same diagnostics
- **TEST** `crates/archangel_rules/tests/rules.rs` › "rule_order_does_not_change_the_result"

#### Scenario: Rules agree with a reference evaluator

- **GIVEN** random configs and random import graphs
- **WHEN** archangel evaluates the rules
- **THEN** the violations are the same as the violations that a slow reference evaluator gives
- **TEST** `crates/archangel_rules/tests/rule_properties.rs` › "violations_agree_with_the_reference_evaluator"

### Requirement: Each shorthand kind means the sentences that it stands for

Each of the four shorthand kinds, layers, isolate, entry and mayImport, MUST give exactly the violations of the sentences it stands for.

#### Scenario: Shorthands agree with their sentences

- **GIVEN** random configs written with shorthand kinds, tables included, and the same configs written as sentences
- **WHEN** archangel checks random repositories with both
- **THEN** each pair of checks gives the same violations
- **TEST** `crates/archangel_rules/tests/rule_properties.rs` › "shorthands_agree_with_their_sentences"

### Requirement: The documentation shows each shorthand as its sentences

The documentation page of each shorthand kind, and `archangel docs` for it, MUST show the sentences it stands for.

#### Scenario: The isolate page

- **GIVEN** the documentation page for isolate
- **WHEN** the test reads it
- **THEN** it shows `rule({ files: 'feature', mayNotImport: 'feature', instance: 'different' })`
- **TEST** `crates/archangel_rules/tests/diagnostics.rs` › "each_shorthand_page_shows_its_sentences"

#### Scenario: The mayImport page

- **GIVEN** the documentation page for mayImport
- **WHEN** the test reads it
- **THEN** it shows a row as one `mayOnlyImport` sentence with its cells and its own name in the same instance
- **AND** it shows a row that mayNotImportItself lists as that sentence and one `mayNotImport` sentence of its own name
- **TEST** `crates/archangel_rules/tests/diagnostics.rs` › "each_shorthand_page_shows_its_sentences"
