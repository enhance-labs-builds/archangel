# rule-examples Specification

## Purpose
A rule that checks nothing looks the same as a rule that works. This capability covers how the test
command proves each rule with its examples, quickly and without a change to the repository.

## Requirements

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

#### Scenario: A failed example shows its own text

- **GIVEN** an empty repository file `src/shared/s.ts`
- **AND** a rejected example that replaces it with a file whose import another rule rejects
- **WHEN** the developer runs `archangel test` with the text format
- **THEN** the failed example's diagnostic shows the example's import line with a mark under its specifier, not the repository file's text
- **TEST** `crates/archangel/tests/test_command.rs` › "failed_example_shows_its_own_text_where_it_replaces_a_repository_file"

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

### Requirement: An example sees its own package manifests first

A `package.json` among an example's files, outside `node_modules`, MUST be a package manifest for that example. It replaces the repository's manifest at its path, and a file's nearest manifest MUST be found among the example's manifests laid over the repository's, as its files are laid over the repository's tsconfigs. An example without a manifest MUST see the repository's. An example MUST NOT add a workspace package, which is reached through its link on disk.

#### Scenario: An example with its own manifest

- **GIVEN** a repository whose `package.json` lists no dependency
- **AND** a rejected example of a `dependency: 'dev'` rule whose files are a `package.json`, at the example's root or in `src`, that lists `vitest` under `devDependencies`, the installed package in `node_modules/vitest`, and `src/a.ts`, which imports `vitest`
- **WHEN** the developer runs `archangel test`
- **THEN** the `dependency: 'dev'` rule rejects the import and names the example's manifest, and a `dependency: 'undeclared'` rule does not
- **TEST** `crates/archangel_rules/tests/examples.rs` › "an_example_sees_its_own_manifest"

#### Scenario: An example without a manifest

- **GIVEN** a repository whose `package.json` lists `vitest` under `devDependencies`, with `vitest` installed
- **AND** a rejected example of a `dependency: 'dev'` rule whose only file imports `vitest`
- **WHEN** the developer runs `archangel test`
- **THEN** the rule rejects the import and names the repository's `package.json`
- **TEST** `crates/archangel_rules/tests/examples.rs` › "an_example_without_a_manifest_sees_the_repository_manifest"

#### Scenario: An example manifest replaces the repository's, and the nearest wins

- **GIVEN** a repository whose `package.json` lists `vitest` under `dependencies`
- **WHEN** an example's own `package.json` lists `vitest` under `devDependencies`
- **THEN** the `dependency: 'dev'` rule rejects the example's import of `vitest`
- **AND WHEN** an example instead has `src/package.json`, which lists nothing
- **THEN** the `dependency: 'undeclared'` rule rejects the import and names `src/package.json`
- **TEST** `crates/archangel_rules/tests/examples.rs` › "an_example_manifest_replaces_the_repository_manifest_at_its_path_and_the_nearest_wins"

#### Scenario: Which files of an example are manifests

- **GIVEN** an example with a `package.json` at its root, a `src/package.json` that is not JSON, and `node_modules/vitest/package.json`
- **WHEN** archangel lays the example over the repository
- **THEN** the example's manifests are the two outside `node_modules`, and the one that is not JSON lists nothing
- **AND** the nearest manifest of `src/a.ts` is `src/package.json`
- **TEST** `crates/archangel_graph/tests/overlay.rs` › "an_overlays_manifests_are_those_the_walk_would_visit"

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

### Requirement: An example that proves nothing is told what its rule looks at

The `example-proves-nothing` message MUST name what the rule looks at: imports, files, folders, instances, or the files of a plugin rule's view. For a `mustNotExist` rule it MUST say that no accepted example can prove the rule.

#### Scenario: What each kind of rule never looked at

- **GIVEN** an accepted example whose only file is `src/app/a.ts`, for each kind of rule: `mustNotExist` on files and on folders, `mustBeNamed` on files and on folders, `mustContain` on folders and on an element, `mustExport`, `mayNotExport`, `mustOnlyReExport`, `noCycles` and `mayNotImport`, none of which covers that file
- **WHEN** the developer runs `archangel test`
- **THEN** each example fails with `example-proves-nothing`, and its message names what the rule looks at and what to give the example
- **AND** the message for a `mustNotExist` rule says that every file or folder it looks at breaks it, so the rule keeps only rejected examples
- **TEST** `crates/archangel_rules/tests/examples.rs` › "an_accepted_example_its_rule_never_checks_is_told_what_the_rule_looks_at"

#### Scenario: A plugin rule never saw the example

- **GIVEN** an accepted example of a plugin rule on "entity", whose only file is in "shared"
- **WHEN** the developer runs `archangel test`
- **THEN** the example fails with `example-proves-nothing`, saying the rule never saw a file of the example and to give it a file in the rule's files
- **TEST** `crates/archangel_engine/tests/plugins.rs` › "an_accepted_example_outside_a_plugin_rules_files_proves_nothing_about_files"

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

### Requirement: A written example of a table runs against all its rows

A table MAY have written examples. They MUST run once against all its rules. A rejected example MUST pass when a row rejects one of its imports, and archangel MUST name that row. The rule `every-file-has-a-kind` MUST NOT prove it, because a misspelt path would then prove the example. It MUST fail when no row rejects it, and the message MUST say that no row of the table rejected it and name each file of the example that has no row.

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

#### Scenario: A rejected example with a misspelt path

- **GIVEN** a table with the rows `infra` and `domain`, the kind pattern `src/{kind}/**`, and the rejected example `'src/domian/a.ts imports src/infra/b.ts'`
- **WHEN** the developer runs `archangel test`
- **THEN** the example fails, and the message says that no row rejected it and names `src/domian/a.ts`
- **TEST** `crates/archangel_rules/tests/table_proof.rs` › "a_rejected_table_example_with_a_misspelt_path_fails"

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

### Requirement: A limit of a table decides whether a row judged an import

A row MUST NOT count as judging an import that its `names`, `specifier`, `instance` or `dependency` leaves out. A rule that is not a table row MUST keep the proof of 0.1, before those limits.

#### Scenario: An accepted table example that every row skips

- **GIVEN** a table with `specifier: 'parent'`, and an accepted example whose only import is `./y`
- **WHEN** the developer runs `archangel test`
- **THEN** the example fails with `example-proves-nothing`
- **TEST** `crates/archangel_rules/tests/table_proof.rs` › "an_accepted_table_example_that_every_row_skips_proves_nothing"

#### Scenario: A plain rule whose limit skips the import of its accepted example

- **GIVEN** a `mayNotImport` rule with `specifier: 'parent'`, and an accepted example whose only import goes through an alias to the rule's target
- **WHEN** the developer runs `archangel test`
- **THEN** the example passes, as in 0.1
- **TEST** `crates/archangel_rules/tests/table_proof.rs` › "a_plain_rule_keeps_its_accepted_example_whose_import_a_limit_skips"

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

#### Scenario: A name with a source extension and no folder

- **GIVEN** the string `'src/a.ts imports chart.js'`
- **WHEN** archangel loads the config
- **THEN** it makes one file that imports the package `chart.js`
- **TEST** `crates/archangel_config/tests/examples.rs` › "a_name_with_a_source_extension_and_no_folder_is_a_package"

#### Scenario: A file at the config folder

- **GIVEN** the string `'src/a.ts imports ./b.ts'`
- **WHEN** archangel loads the config
- **THEN** it makes `src/a.ts`, which imports `../b.ts`, and `b.ts`
- **TEST** `crates/archangel_config/tests/examples.rs` › "a_file_at_the_config_folder_is_a_target_with_dot_slash"

#### Scenario: A target is a file or a package

- **GIVEN** the strings `'a.ts imports src/b.tsx'`, `'a.ts imports react'`, `'a.ts imports @aws-sdk/client-s3'` and `'a.ts imports node:fs'`
- **WHEN** archangel loads the config
- **THEN** the first makes two files, and each of the others makes one file that imports the package
- **TEST** `crates/archangel_config/tests/examples.rs` › "a_target_is_a_file_when_it_has_a_source_extension"

#### Scenario: A chain and a single path

- **GIVEN** the strings `'src/a.ts imports src/b.ts imports src/a.ts'` and `'src/app/ui.ts'`
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

- **GIVEN** the strings `'a.ts import b.ts'`, `'a.ts imports'`, `'../a.ts imports src/b.ts'`, `'./a.ts imports src/b.ts'` and `'a.ts imports react imports src/b.ts'`
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
