# rule-examples

## ADDED Requirements

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

`archangel test` MUST fail when a rule has no rejected example, with a `rule-unproven` diagnostic that names the rule. A rule without a rejected example has no proof that it can find a violation. A table row, and a table's `every-file-has-a-kind` rule, MUST NOT need a written rejected example: what shows that a row can fire is the summary's list of rows with no file, `table-empty` and the reach that `archangel rules` prints.

#### Scenario: A rule without a rejected example

- **GIVEN** a config with a `mayNotImport` rule that has only accepted examples
- **WHEN** the developer runs `archangel test`
- **THEN** archangel prints a `rule-unproven` diagnostic that names the `mayNotImport` rule
- **AND** archangel exits with status 1
- **TEST** `crates/archangel/tests/test_command.rs` › "rule_without_a_rejected_example_is_rule_unproven"

#### Scenario: A table without a written example

- **GIVEN** the enhance3-slice fixture, whose tables have no written example, and whose other rules each have one
- **WHEN** the developer runs `archangel test`
- **THEN** archangel gives no `rule-unproven` diagnostic for a row of `workspace` or `domain`, or for `workspace.every-file-has-a-kind` or `domain.every-file-has-a-kind`
- **AND** archangel exits with status 0
- **TEST** `crates/archangel/tests/test_command.rs` › "a_table_row_needs_no_written_example"
