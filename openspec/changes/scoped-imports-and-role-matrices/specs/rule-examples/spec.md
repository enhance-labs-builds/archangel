# rule-examples

## ADDED Requirements

### Requirement: Archangel proves each table row from two repository files

`archangel test` MUST make one generated proof for each table row that has files. The proof MUST take the first file of the row, in path order, and the first repository file, in path order, that the row refuses. It MUST check that the row rejects an import from the first file to the second, with the in-memory check that every example uses. It MUST print the row, the word "generated" and the two files. A proof that the row does not reject MUST fail, and `archangel test` MUST exit with status 1.

#### Scenario: The rows of the small app

- **GIVEN** the small-app fixture, which has no written example
- **WHEN** the developer runs `archangel test`
- **THEN** archangel prints one generated proof for each of `app.feature`, `feature-code.views`, `feature-code.models` and `feature-code.api`, each with its two files
- **AND** archangel gives no `rule-unproven` diagnostic, and exits with status 0
- **TEST** `crates/archangel/tests/test_command.rs` › "each_table_row_is_proven_from_two_repository_files"

#### Scenario: The fault control for generated proof

- **GIVEN** a build of archangel with a fault that drops the sentence of every table row
- **WHEN** the developer runs `archangel test` on the small-app fixture
- **THEN** each generated proof fails, and archangel exits with status 1
- **TEST** `crates/archangel/tests/test_command.rs` › "a_generated_proof_fails_when_its_row_cannot_reject"

### Requirement: A row that cannot be proven yet is listed

A table row with no file, or with no repository file that it refuses, MUST NOT fail `archangel test`. `archangel test` MUST list it as a row that proves nothing yet, with the reason.

#### Scenario: A row with no file

- **GIVEN** the enhance3-slice fixture, with no aggregate-root file
- **WHEN** the developer runs `archangel test`
- **THEN** archangel lists `domain.aggregate-root` as a row that proves nothing yet, because no file is in it
- **AND** this does not change the exit status
- **TEST** `crates/archangel/tests/test_command.rs` › "a_row_with_no_file_proves_nothing_yet"

#### Scenario: A row that permits every file

- **GIVEN** a table with the part rows `a: ['b']` and `b: ['a']`, on a repository whose files are all in `a` or `b`
- **WHEN** the developer runs `archangel test`
- **THEN** archangel lists `a` and `b` as rows that prove nothing yet, because no file is refused
- **TEST** `crates/archangel/tests/test_command.rs` › "a_row_that_refuses_no_file_proves_nothing_yet"

### Requirement: A written example of a table runs against all its rows

A table MAY have written examples. They MUST run once against all its rules. A rejected example MUST pass when one of these rules rejects it, and archangel MUST name that rule. It MUST fail when none rejects it, and the message MUST say that no row of the table rejected it.

#### Scenario: A written example that one row rejects

- **GIVEN** the enhance3-slice table `domain` with a rejected example in which an entity imports a use case
- **WHEN** the developer runs `archangel test`
- **THEN** the example passes, and archangel names `domain.entity` as the row that rejected it
- **TEST** `crates/archangel_rules/tests/examples.rs` › "a_written_table_example_passes_when_one_row_rejects_it"

#### Scenario: A written example that no row rejects

- **GIVEN** the same table with a rejected example in which an entity imports a value object
- **WHEN** the developer runs `archangel test`
- **THEN** the example fails, and the message says that no row of `domain` rejected it
- **TEST** `crates/archangel_rules/tests/examples.rs` › "a_written_table_example_fails_when_no_row_rejects_it"

### Requirement: A written accepted example of a table passes only when a row judged it

An accepted example of a table MUST pass when no row rejects it and a row judged one of its imports. It MUST fail when a row rejects it, and archangel MUST name that row. When no row judged an import, it MUST fail with `example-proves-nothing`, and the message MUST name the table and say what import to add.

#### Scenario: A written accepted example

- **GIVEN** the same table with an accepted example in which an entity imports a value object
- **AND** the same table with an accepted example in which an entity imports a use case
- **WHEN** the developer runs `archangel test` with each
- **THEN** the first example passes, and the second fails and names `domain.entity`
- **TEST** `crates/archangel_rules/tests/examples.rs` › "a_written_table_accepted_example_passes_when_no_row_rejects_it"

#### Scenario: An accepted example that no row judges

- **GIVEN** the small-app table `feature-code`, and an accepted example in which a view imports only `react`
- **WHEN** the developer runs `archangel test`
- **THEN** the example fails with `example-proves-nothing`, and the message names `feature-code` and says to give the example an import from a file of a row into the repository
- **TEST** `crates/archangel_rules/tests/examples.rs` › "a_table_accepted_example_that_no_row_judges_proves_nothing"

### Requirement: An accepted example outside a rule's area proves nothing

An accepted example of a rule with an area MUST fail with `example-proves-nothing` when the rule judged none of its imports, and the message MUST name the area.

#### Scenario: An accepted example that imports only outside the area

- **GIVEN** a rule `{ files: 'entity', mayOnlyImport: ['value-object'], within: 'apps/api/src/**' }`, and an accepted example in which an entity imports only `zod`
- **WHEN** the developer runs `archangel test`
- **THEN** the example fails with `example-proves-nothing`, and the message says to give it an import from the rule's files into `apps/api/src/**`
- **TEST** `crates/archangel_rules/tests/examples.rs` › "an_accepted_example_outside_the_area_proves_nothing"

## MODIFIED Requirements

### Requirement: Every rule proves that it can reject

`archangel test` MUST fail when a rule has no rejected example, with a `rule-unproven` diagnostic that names the rule. A rule without a rejected example has no proof that it can find a violation. A table row, and the rule of a table's `everyFileHasAKind`, MUST NOT need a written rejected example, because the generated proofs prove the rows.

#### Scenario: A rule without a rejected example

- **GIVEN** a config with a `mayNotImport` rule that has only accepted examples
- **WHEN** the developer runs `archangel test`
- **THEN** archangel prints a `rule-unproven` diagnostic that names the `mayNotImport` rule
- **AND** archangel exits with status 1
- **TEST** `crates/archangel/tests/test_command.rs` › "rule_without_a_rejected_example_is_rule_unproven"

#### Scenario: A table without a written example

- **GIVEN** the enhance3-slice fixture, whose tables have no written example, and whose other rules each have one
- **WHEN** the developer runs `archangel test`
- **THEN** archangel gives no `rule-unproven` diagnostic for a row of `workspace` or `domain`, or for `domain.every-file-has-a-kind`
- **TEST** `crates/archangel/tests/test_command.rs` › "a_table_row_needs_no_written_example"
