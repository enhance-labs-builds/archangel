# rule-examples

## ADDED Requirements

### Requirement: A matrix needs one rejected example from its author

A matrix MUST have at least one rejected example that its author writes. Otherwise `archangel test` MUST give one `rule-unproven` diagnostic that names the matrix, and none for its rows. The examples of a matrix MUST run against all its rules: its rows, and the rule of `everyFileHasARow` when the matrix has it. A rejected example of a matrix MUST pass when one of these rules rejects it, and archangel MUST name that rule.

#### Scenario: A matrix without an example of its own

- **GIVEN** a matrix `api-domain` with the rows `entity` and `policy`, which both have files, and no `examples`
- **WHEN** the developer runs `archangel test`
- **THEN** archangel gives one `rule-unproven` diagnostic, which names the matrix `api-domain` and says that one written rejected example must show what its roles mean
- **AND** archangel gives no `rule-unproven` diagnostic for `api-domain.entity` or `api-domain.policy`
- **AND** archangel exits with status 1
- **TEST** `crates/archangel/tests/test_command.rs` › "a_matrix_without_a_written_rejected_example_is_unproven"

#### Scenario: A written example that one row rejects

- **GIVEN** a matrix `api-domain` with a rejected example in which an entity imports a use case
- **WHEN** the developer runs `archangel test`
- **THEN** the example passes, and archangel names `api-domain.entity` as the row that rejected it
- **TEST** `crates/archangel_rules/tests/examples.rs` › "a_written_matrix_example_passes_when_one_row_rejects_it"

#### Scenario: A written example that no row rejects

- **GIVEN** a matrix `api-domain` whose row `entity` lists `value-object`, with a rejected example in which an entity imports a value object
- **WHEN** the developer runs `archangel test`
- **THEN** the example fails, and the message says that no row of `api-domain` rejected it
- **AND** archangel exits with status 1
- **TEST** `crates/archangel_rules/tests/examples.rs` › "a_written_matrix_example_fails_when_no_row_rejects_it"

### Requirement: An accepted example of a matrix passes when no row rejects it

An accepted example of a matrix MUST pass when no row of the matrix gives a violation and at least one row judged one of its imports. It MUST fail when a row gives a violation, and archangel MUST name that row. When no row judged an import, it MUST fail with `example-proves-nothing`, and the message MUST name the matrix, say that no row looked at an import of the example, and tell the developer to give it an import from a file of a row into the area, which it names.

#### Scenario: A written accepted example

- **GIVEN** a matrix `api-domain` whose row `entity` lists `value-object`, with an accepted example in which an entity imports a value object
- **AND** the same matrix with an accepted example in which an entity imports a use case
- **WHEN** the developer runs `archangel test` with each
- **THEN** the first example passes
- **AND** the second example fails, and archangel names `api-domain.entity` as the row that rejected it
- **TEST** `crates/archangel_rules/tests/examples.rs` › "a_written_matrix_accepted_example_passes_when_no_row_rejects_it"

#### Scenario: An accepted example that no row judges

- **GIVEN** the matrix `api-domain` with the area `apps/api/src/**`, and an accepted example in which an entity imports only `zod`
- **WHEN** the developer runs `archangel test`
- **THEN** the example fails with `example-proves-nothing`
- **AND** the message names `api-domain`, says that no row looked at an import of the example, and says to give it an import from a file of a row into `apps/api/src/**`
- **TEST** `crates/archangel_rules/tests/examples.rs` › "a_matrix_accepted_example_that_no_row_judges_proves_nothing"

### Requirement: An accepted example outside a rule's area proves nothing

An accepted example of a rule with an area MUST fail with `example-proves-nothing` when the rule judged none of its imports, and the message MUST name the area.

#### Scenario: An accepted example that imports only outside the area

- **GIVEN** a rule `{ files: 'entity', mayOnlyImport: ['value-object'], within: 'apps/api/src/**' }`, and an accepted example in which an entity imports only `zod`
- **WHEN** the developer runs `archangel test`
- **THEN** the example fails with `example-proves-nothing`, and the message says to give it an import from the rule's files into `apps/api/src/**`
- **TEST** `crates/archangel_rules/tests/examples.rs` › "an_accepted_example_outside_the_area_proves_nothing"

## MODIFIED Requirements

### Requirement: Every rule proves that it can reject

`archangel test` MUST fail when a rule has no rejected example, with a `rule-unproven` diagnostic that names the rule. A rule without a rejected example has no proof that it can find a violation. A matrix row, and the rule of a matrix's `everyFileHasARow`, MUST count the rejected examples of the matrix as their own.

#### Scenario: A rule without a rejected example

- **GIVEN** a config with a `mayNotImport` rule that has only accepted examples
- **WHEN** the developer runs `archangel test`
- **THEN** archangel prints a `rule-unproven` diagnostic that names the `mayNotImport` rule
- **AND** archangel exits with status 1
- **TEST** `crates/archangel/tests/test_command.rs` › "rule_without_a_rejected_example_is_rule_unproven"

#### Scenario: A matrix row without an example that names it

- **GIVEN** a matrix `api-domain` with one written rejected example, which the row `entity` rejects, and a row `policy` that has files
- **WHEN** the developer runs `archangel test`
- **THEN** archangel gives no `rule-unproven` diagnostic for `api-domain.policy`, because the matrix has a rejected example
- **AND** archangel exits with status 0 when every example passes
- **TEST** `crates/archangel/tests/test_command.rs` › "a_matrix_row_is_proven_by_the_examples_of_its_matrix"

#### Scenario: The rule that every file has a row

- **GIVEN** a matrix `api-domain` with `everyFileHasARow: true` and one written rejected example, which the row `entity` rejects
- **WHEN** the developer runs `archangel test`
- **THEN** archangel gives no `rule-unproven` diagnostic for `api-domain.every-file-has-a-row`
- **TEST** `crates/archangel/tests/test_command.rs` › "the_every_file_rule_is_proven_by_the_examples_of_its_matrix"
