# Spec Delta

## Purpose

A rule that checks nothing looks the same as a rule that works. This capability covers how the test
command proves each rule with its examples, quickly and without a change to the repository.

## ADDED Requirements

### Requirement: The test command checks every example of every rule

`archangel test` MUST check each accepted example and each rejected example of each rule. It MUST print one line for each example, with the rule name and the result. It MUST exit with status 0 only when every example passes.

#### Scenario: All examples pass

- **GIVEN** a config with two rules, each with one accepted example and one rejected example that are correct
- **WHEN** the developer runs `archangel test`
- **THEN** archangel prints four passed examples and exits with status 0
- **TEST** `crates/archangel/tests/test_command.rs` › "all_examples_pass_with_status_zero"

#### Scenario: One example fails

- **GIVEN** a config with a rejected example that its rule does not reject
- **WHEN** the developer runs `archangel test`
- **THEN** archangel prints that example as failed, with the files of the example
- **AND** archangel exits with status 1
- **TEST** `crates/archangel/tests/test_command.rs` › "failed_example_gives_status_one"

### Requirement: Examples never change the repository

Archangel MUST put the files of an example into a check in memory. It MUST NOT write, change or delete a file on disk for an example. A file of an example MAY import files of the repository. A file of an example at the path of a repository file MUST replace that file for that example only.

#### Scenario: The repository after a test

- **GIVEN** a repository and a config with ten examples
- **WHEN** the developer runs `archangel test`
- **THEN** every file in the repository has the same bytes and modification time as before
- **AND** the work count of files written is zero
- **TEST** `crates/archangel/tests/test_command.rs` › "test_command_writes_no_file"

#### Scenario: The fault control for files written

- **GIVEN** a build of archangel with a fault that writes each example's files to disk
- **WHEN** the files-written test runs against that build
- **THEN** the test fails on the count of files written
- **TEST** `crates/archangel/tests/test_command.rs` › "files_written_count_rejects_examples_on_disk"

#### Scenario: An example imports a repository file

- **GIVEN** a rejected example with one file in "shared" that imports the repository file `src/features/login/index.ts`
- **WHEN** archangel checks the example against a layers rule
- **THEN** the layers rule rejects the import, and the example passes
- **TEST** `crates/archangel_rules/tests/examples.rs` › "example_file_can_import_a_repository_file"

#### Scenario: An example file replaces a repository file

- **GIVEN** a repository file `src/shared/a.ts` with no imports
- **AND** a rejected example with a file at `src/shared/a.ts` that imports a file in "feature"
- **WHEN** archangel checks the example
- **THEN** the example uses the example file, and its rule rejects the import
- **AND** the next example sees the repository file again
- **TEST** `crates/archangel_rules/tests/examples.rs` › "example_file_replaces_a_repository_file_for_that_example_only"

### Requirement: An example that does not parse fails

When a file of an example has a syntax error, the example MUST fail with a `parse-failed` diagnostic for that file. A syntax error MUST NOT make a rejected example pass.

#### Scenario: A rejected example with a syntax error

- **GIVEN** a rejected example of a layers rule whose only file has a syntax error
- **WHEN** the developer runs `archangel test`
- **THEN** the example fails with a `parse-failed` diagnostic for its file
- **TEST** `crates/archangel_rules/tests/examples.rs` › "example_with_a_syntax_error_fails"

### Requirement: The cycle search for an example covers every file that can reach it

An example file at a new path has no importers among repository files, because their imports were resolved before the example existed. An example file that replaces a repository file keeps that file's importers. So archangel MUST search for cycles among the example files and the repository files that import a replaced file, directly or through other files, and among no other files.

#### Scenario: A rejected example of a cycle

- **GIVEN** a rejected example of a `noCycles` rule with two new files that import each other
- **WHEN** the developer runs `archangel test`
- **THEN** the example passes
- **AND** the work count of files that the cycle search visits for the example is 2
- **TEST** `crates/archangel_rules/tests/examples.rs` › "cycle_example_searches_only_its_own_files"

#### Scenario: A replaced file closes a cycle through a repository file

- **GIVEN** the repository file `src/b.ts`, which imports `./a`
- **AND** an accepted example of a `noCycles` rule that replaces `src/a.ts` with a file that imports `./b`
- **WHEN** the developer runs `archangel test`
- **THEN** the example fails, because the rule finds the cycle `src/a.ts`, `src/b.ts`, `src/a.ts`
- **TEST** `crates/archangel_rules/tests/examples.rs` › "replaced_file_cycle_through_a_repository_file_is_found"

#### Scenario: The fault control for the example cycle search

- **GIVEN** a build of archangel with a fault that searches the whole graph for cycles for each example
- **WHEN** the two-file cycle example test runs against that build
- **THEN** the test fails on the count of files that the cycle search visits
- **TEST** `crates/archangel_rules/tests/examples.rs` › "cycle_search_count_rejects_a_whole_graph_search_per_example"

### Requirement: A rejected example passes only when its own rule rejects it

A rejected example MUST pass only when its own rule gives a violation that involves a file of the example. When only other rules reject the example, it MUST fail, and archangel MUST name the rules that rejected it.

#### Scenario: Another rule rejects the example

- **GIVEN** a rejected example of an entry rule whose import also breaks a layers rule, but not the entry rule
- **WHEN** the developer runs `archangel test`
- **THEN** the example fails
- **AND** archangel says that the layers rule rejected it and the entry rule did not
- **TEST** `crates/archangel_rules/tests/examples.rs` › "rejected_example_caught_by_another_rule_fails_and_names_it"

### Requirement: An accepted example passes only when its rule checked it

An accepted example MUST pass only when its rule gives no violation that involves a file of the example, and its rule evaluated at least one of them: an import for an import rule, or a file, folder or instance for a naming, structure or export rule. Otherwise it MUST fail with a diagnostic that says it proves nothing.

#### Scenario: An accepted example that its rule never looks at

- **GIVEN** an accepted example of an isolate rule on "feature", whose only file is in "shared"
- **WHEN** the developer runs `archangel test`
- **THEN** the example fails with an `example-proves-nothing` diagnostic
- **TEST** `crates/archangel_rules/tests/examples.rs` › "accepted_example_that_its_rule_never_checks_proves_nothing"

#### Scenario: An accepted example that its rule checks

- **GIVEN** an accepted example of the same isolate rule, with a file in the instance "login" that imports another file in "login"
- **WHEN** the developer runs `archangel test`
- **THEN** the example passes
- **TEST** `crates/archangel_rules/tests/examples.rs` › "accepted_example_that_its_rule_checks_passes"

#### Scenario: An accepted example of a structure rule

- **GIVEN** a rule `{ files: 'slice', mustContain: 'README.md' }` with the accepted example `{ 'src/features/x/README.md': '# x', 'src/features/x/a.ts': '' }`
- **WHEN** the developer runs `archangel test`
- **THEN** the example passes, and `README.md` is not parsed
- **TEST** `crates/archangel_rules/tests/examples.rs` › "an_accepted_structure_example_passes_and_non_source_files_are_not_parsed"

### Requirement: Every rule proves that it can reject

`archangel test` MUST fail when a rule has no rejected example, with a `rule-unproven` diagnostic that names the rule. A rule without a rejected example has no proof that it can find a violation.

#### Scenario: A rule without a rejected example

- **GIVEN** a config with a `mayNotImport` rule that has only accepted examples
- **WHEN** the developer runs `archangel test`
- **THEN** archangel prints a `rule-unproven` diagnostic that names the `mayNotImport` rule
- **AND** archangel exits with status 1
- **TEST** `crates/archangel/tests/test_command.rs` › "rule_without_a_rejected_example_is_rule_unproven"

### Requirement: The cost of an example does not grow with the repository

To check one example, archangel MUST parse only the files of the example, and resolve only their imports. It MUST NOT read or parse the repository again for each example.

#### Scenario: Work for many examples

- **GIVEN** a repository with 1,000 source files, and a config with 50 examples of one file each
- **WHEN** the developer runs `archangel test`
- **THEN** the work counts are 1,000 repository files parsed and 50 example files parsed
- **AND** the resolutions and rule evaluations for the examples are those of the example files' imports only
- **TEST** `crates/archangel_engine/tests/test_command.rs` › "examples_parse_only_their_own_files"

#### Scenario: The fault control for example evaluations

- **GIVEN** a build of archangel with a fault that evaluates every rule over the whole graph for each example
- **WHEN** the example work test runs against that build
- **THEN** the test fails on the count of rule evaluations
- **TEST** `crates/archangel_engine/tests/test_command.rs` › "example_work_count_rejects_a_full_evaluation_per_example"

#### Scenario: The fault control for the example work count

- **GIVEN** a build of archangel with a fault that reads the repository again for each example
- **WHEN** the example work test runs against that build
- **THEN** the test fails on the count of files parsed
- **TEST** `crates/archangel_engine/tests/test_command.rs` › "example_work_count_rejects_a_reread_per_example"
