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

### Requirement: An example may be one line

An item of `accepted` or `rejected` MUST be a file set or a string. A string MUST be a path, followed by any number of `imports <target>`. Archangel MUST make one file for each path, which holds one runtime import with a relative specifier for each import that the string gives that path, and nothing else. The example MUST then run as the same file set would.

#### Scenario: A one-line example gives the verdict of its file set

- **GIVEN** a rule with the rejected example `'src/shared/a.ts imports src/features/b.ts'`
- **AND** the same rule with a rejected file set that has these two files and this import
- **WHEN** the developer runs `archangel test` with each
- **THEN** both give the same verdict and the same messages
- **TEST** `crates/archangel_rules/tests/examples.rs` › "a_one_line_example_gives_the_verdict_of_its_file_set"

#### Scenario: A rejected one-liner that the rule does not reject

- **GIVEN** a rule with the rejected example `'src/features/a.ts imports src/shared/b.ts'`, which the rule permits
- **WHEN** the developer runs `archangel test`
- **THEN** the example fails with the message of a file set that the rule does not reject
- **TEST** `crates/archangel_rules/tests/examples.rs` › "a_rejected_one_liner_that_the_rule_permits_fails"

#### Scenario: An accepted one-liner that the rule does not judge

- **GIVEN** the small-app table `feature-code`, and the accepted example `'src/features/a/views/b.ts imports react'`
- **WHEN** the developer runs `archangel test`
- **THEN** the example fails with `example-proves-nothing`
- **TEST** `crates/archangel_rules/tests/examples.rs` › "an_accepted_one_liner_that_no_row_judges_proves_nothing"

#### Scenario: A target is a file or a package

- **GIVEN** the strings `'a.ts imports src/b.tsx'`, `'a.ts imports react'`, `'a.ts imports @aws-sdk/client-s3'` and `'a.ts imports node:fs'`
- **WHEN** archangel loads the config
- **THEN** the first makes two files, and each of the others makes one file that imports the package
- **TEST** `crates/archangel_config/tests/examples.rs` › "a_target_is_a_file_when_it_has_a_source_extension"

#### Scenario: A chain and a single path

- **GIVEN** the strings `'a.ts imports b.ts imports a.ts'` and `'src/app/ui.ts'`
- **WHEN** archangel loads the config
- **THEN** the first makes two files that import each other, and the second makes one empty file
- **TEST** `crates/archangel_config/tests/examples.rs` › "a_chain_gives_each_file_its_imports_and_a_single_path_is_empty"

#### Scenario: A cycle rule proved by one line

- **GIVEN** a `noCycles` rule with the rejected example `'src/a.ts imports src/b.ts imports src/a.ts'`
- **WHEN** the developer runs `archangel test`
- **THEN** the example passes
- **TEST** `crates/archangel_rules/tests/examples.rs` › "a_cycle_rule_is_proved_by_a_one_line_chain"

#### Scenario: A structure rule proved by one path

- **GIVEN** a `mustNotExist` rule on `src/app/ui.*`, with the rejected example `'src/app/ui.ts'`
- **WHEN** the developer runs `archangel test`
- **THEN** the example passes
- **TEST** `crates/archangel_rules/tests/examples.rs` › "a_structure_rule_is_proved_by_one_path"

#### Scenario: One list with both forms

- **GIVEN** a rule whose `rejected` list has a string and a file set
- **WHEN** the developer runs `archangel test`
- **THEN** archangel runs both examples
- **TEST** `crates/archangel_rules/tests/examples.rs` › "one_list_may_hold_both_forms"

#### Scenario: A string with any other shape

- **GIVEN** the strings `'a.ts import b.ts'`, `'a.ts imports'`, `'../a.ts imports b.ts'` and `'a.ts imports react imports b.ts'`
- **WHEN** archangel loads the config
- **THEN** each gives a config problem, whose message names the field and gives the form `<file> imports <file or package>`
- **TEST** `crates/archangel_config/tests/examples.rs` › "a_string_with_any_other_shape_is_refused_with_the_form"

### Requirement: A one-line example is refused where file text decides the verdict

A rule with `names`, `specifier`, `dependency` or `imports: 'types'`, a rule whose sides or `stopAt` select by directive, an export condition, and a plugin condition MUST refuse a one-line example. The message MUST name the rule and say to write a file set.

#### Scenario: A one-liner on a rule that needs file text

- **GIVEN** in turn, a one-line example on a rule with each of `names`, `specifier`, `dependency` and `imports: 'types'`, on a `mustExport` rule, on a plugin rule, and on a rule whose `files` select by directive
- **WHEN** archangel loads the config
- **THEN** each gives a config problem that names the rule and says to write a file set
- **TEST** `crates/archangel_config/tests/examples.rs` › "a_one_liner_is_refused_where_file_text_decides_the_verdict"

#### Scenario: A one-liner on a table with a narrowing option

- **GIVEN** a table with `imports: 'types'` and a one-line example
- **WHEN** archangel loads the config
- **THEN** archangel gives the same config problem, and it names the table
- **TEST** `crates/archangel_config/tests/examples.rs` › "a_one_liner_on_a_narrowed_table_is_refused"

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
