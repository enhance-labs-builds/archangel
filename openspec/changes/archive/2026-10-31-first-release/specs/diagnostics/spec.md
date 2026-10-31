# Spec Delta

## Purpose

A diagnostic arrives exactly when a person or an agent makes a mistake, so it must be enough on its
own to fix the code. This capability covers what each diagnostic contains, its formats, its order,
and the fixes that archangel offers.

## ADDED Requirements

### Requirement: A violation says what is wrong, what to do and where to read more

Each violation of an import MUST give the importing file, line and column, the specifier, the rule's name, its shorthand kind or condition, and the element instance of each file that has one. It MUST give the rule's reason and documentation link when the config has them. It MUST say what the code may do instead, in the terms of the rule.

#### Scenario: A layers violation

- **GIVEN** a layers rule "app", "feature", "shared" with the reason "Shared code must not know about features"
- **AND** a file in "shared" that imports a file in "feature"
- **WHEN** archangel makes a check
- **THEN** the violation names the file, the line, the specifier and the rule
- **AND** it gives the reason
- **AND** it says that "shared" may import only "shared"
- **TEST** `crates/archangel_rules/tests/diagnostics.rs` › "layers_violation_says_what_shared_may_import"

#### Scenario: An entry violation

- **GIVEN** an entry rule on "entity" with the entry file `index.ts`
- **AND** an import of `@/entities/canvas/model/store` from outside the instance "canvas"
- **WHEN** archangel makes a check
- **THEN** the violation says that the import must go through `src/entities/canvas/index.ts`
- **TEST** `crates/archangel_rules/tests/diagnostics.rs` › "entry_violation_names_the_entry_file_to_use"

#### Scenario: A rule with a documentation link

- **GIVEN** a `mayNotImport` rule with `link: 'docs/architecture.md#entities'`
- **WHEN** archangel gives a violation of the rule
- **THEN** the violation shows `docs/architecture.md#entities`
- **TEST** `crates/archangel_rules/tests/diagnostics.rs` › "violation_shows_the_rules_documentation_link"

#### Scenario: Every diagnostic code has a page

- **GIVEN** the list of every diagnostic code that archangel can give
- **WHEN** the binary is built, with each page in `crates/archangel_rules/docs/` compiled into it
- **THEN** each code has a page that opens with its name and says in one sentence what it means,
  followed by an example and what to do about it, and `archangel docs <code>` prints it
- **TEST** `crates/archangel_rules/tests/diagnostics.rs` › "every_code_has_a_page_that_names_it_and_says_what_it_means"

### Requirement: Archangel offers a fix only after it checks the fix

Archangel MUST offer a fix only when it applied the fix in memory and the fix removes the diagnostic without a new one. For an entry violation, the fix MUST change the specifier so that it resolves to an entry file that exports every name the import uses. Archangel MUST NOT parse a file again to check a fix.

#### Scenario: A specifier that can point to the entry file

- **GIVEN** an import of `@/entities/canvas/model/store` that breaks an entry rule
- **AND** the entry file `src/entities/canvas/index.ts`, which the specifier `@/entities/canvas` resolves to
- **WHEN** archangel makes a check
- **THEN** the violation has a fix that changes the specifier to `@/entities/canvas`
- **TEST** `crates/archangel_engine/tests/fixes.rs` › "entry_fix_points_the_specifier_at_the_entry_file"

#### Scenario: The entry file does not export the imported name

- **GIVEN** the same import of the name `store`, and an entry file that does not export `store`
- **WHEN** archangel makes a check
- **THEN** the violation has no fix
- **AND** the violation says that the entry file does not export `store`
- **TEST** `crates/archangel_engine/tests/fixes.rs` › "entry_fix_is_not_offered_when_the_entry_does_not_export_the_name"

#### Scenario: The entry file exports the name through a chain

- **GIVEN** the same import of `store`, and an entry file with `export * from './model'`, whose file `model/index.ts` exports `store`
- **WHEN** archangel makes a check
- **THEN** the violation has a fix that changes the specifier to `@/entities/canvas`
- **TEST** `crates/archangel_engine/tests/fixes.rs` › "entry_fix_follows_export_star_chains"

#### Scenario: The fix would break another rule

- **GIVEN** an import that breaks an entry rule
- **AND** a `mayNotImport` rule that an import of the entry file from the importing file would break
- **WHEN** archangel makes a check
- **THEN** the entry violation has no fix
- **TEST** `crates/archangel_engine/tests/fixes.rs` › "entry_fix_is_not_offered_when_it_adds_a_violation"

#### Scenario: No specifier resolves to the entry file

- **GIVEN** an entry violation whose specifier cannot be shortened to one that resolves to the entry file
- **WHEN** archangel makes a check
- **THEN** the violation has no fix and names the entry file
- **TEST** `crates/archangel_engine/tests/fixes.rs` › "entry_fix_is_not_offered_when_no_specifier_resolves"

#### Scenario: Work for fixes

- **GIVEN** a repository with 30 source files and 10 entry violations
- **WHEN** archangel makes a check
- **THEN** the count of files parsed is 30
- **TEST** `crates/archangel_engine/tests/fixes.rs` › "fixes_parse_no_file_again"

#### Scenario: The fault control for fix work

- **GIVEN** a build of archangel with a fault that parses the entry file again for each fix
- **WHEN** the fix work test runs against that build
- **THEN** the test fails on the count of files parsed
- **TEST** `crates/archangel_engine/tests/fixes.rs` › "fix_work_count_rejects_a_parse_per_fix"

### Requirement: Text output is for people and JSON output is for programs

The text format MUST show each diagnostic with the line of source code and a mark under the specifier. It MUST use color only when the output is a terminal and `NO_COLOR` is not set. The JSON format MUST be one document with a schema version, the diagnostics and the summary.

#### Scenario: The source line and the mark

- **GIVEN** a check with one entry violation
- **WHEN** the developer runs `archangel check`
- **THEN** the output shows the line of source code and a mark under the specifier, as in the recorded example output
- **TEST** `crates/archangel/tests/output.rs` › "text_output_shows_the_source_line_and_a_mark"

#### Scenario: A terminal with NO_COLOR

- **GIVEN** a check with one violation, output to a terminal, and the environment variable `NO_COLOR=1`
- **WHEN** the developer runs `archangel check`
- **THEN** the output contains no color codes
- **TEST** `crates/archangel/tests/output.rs` › "no_color_turns_color_off_on_a_terminal"

#### Scenario: Output to a pipe

- **GIVEN** a check with one violation, and output to a pipe
- **WHEN** the developer runs `archangel check`
- **THEN** the output contains no color codes
- **TEST** `crates/archangel/tests/output.rs` › "text_output_to_a_pipe_has_no_color"

#### Scenario: The JSON document

- **GIVEN** a check with one violation
- **WHEN** the developer runs `archangel check --format json`
- **THEN** the output is one JSON document with `schemaVersion` 1, a `diagnostics` list with one item, and a `summary`
- **TEST** `crates/archangel/tests/output.rs` › "json_output_is_one_versioned_document"

### Requirement: Each diagnostic has a stable code and identity

Each diagnostic MUST have a code from a fixed list, such as `layers` or `import-unresolved`. Each violation of an import MUST have an identity made from the rule name, the importing file, the specifier and its position among imports of the same specifier in that file. Each violation of a `noCycles` rule MUST have an identity made from the rule name and the sorted paths of every file in the group. An edit that moves lines MUST NOT change an identity.

#### Scenario: Lines move above a violation

- **GIVEN** a violation and its identity in a first check
- **AND** an edit that adds five lines at the top of the importing file
- **WHEN** archangel makes a second check
- **THEN** the violation has the same identity as in the first check
- **TEST** `crates/archangel_rules/tests/diagnostics.rs` › "violation_identity_survives_moved_lines"

#### Scenario: Two imports of one specifier

- **GIVEN** `src/a.ts` with two imports of `./b` that break one rule
- **WHEN** archangel makes a check
- **THEN** the two violations have different identities that differ only in their position
- **TEST** `crates/archangel_rules/tests/diagnostics.rs` › "two_imports_of_one_specifier_have_different_identities"

### Requirement: A violation that is not about an import is identified by its path

A violation about a file, a folder or an instance MUST have an identity made from the rule name and that path, so that a change to what the folder holds does not change it.

#### Scenario: A folder violation in the baseline

- **GIVEN** a violation of `{ folders: 'src/**', mustBeNamed: 'kebab-case' }` for `src/userProfile`, recorded with `archangel baseline add`
- **WHEN** a file is added to `src/userProfile` and archangel makes a check
- **THEN** the baseline still matches the violation
- **TEST** `crates/archangel_rules/tests/baseline.rs` › "a_folder_violations_identity_is_its_rule_and_path"

### Requirement: The same input gives the same output

Two checks of the same files with the same config MUST give the same bytes of output, on any machine and with any number of threads. Diagnostics MUST be in order of file path, then line, then column, then rule name. Paths MUST be relative to the config folder, with `/` between folders.

#### Scenario: The order of diagnostics

- **GIVEN** violations in `src/b.ts` on line 2, in `src/a.ts` on line 9 and in `src/a.ts` on line 2
- **WHEN** the developer runs `archangel check`
- **THEN** archangel prints them in the order `src/a.ts` line 2, `src/a.ts` line 9, `src/b.ts` line 2
- **TEST** `crates/archangel/tests/output.rs` › "diagnostics_are_sorted_by_path_then_line"

#### Scenario: One thread and many threads

- **GIVEN** a repository with 500 source files and 40 violations
- **WHEN** archangel checks it with one thread and then with sixteen threads
- **THEN** the two outputs have the same bytes
- **TEST** `crates/archangel/tests/output.rs` › "output_is_the_same_with_one_thread_and_with_many"

### Requirement: Archangel prints every diagnostic

Archangel MUST NOT stop at a number of diagnostics, a length of output or a length of cycle. Each command MUST print every diagnostic that it finds, and each cycle MUST show every step.

#### Scenario: A large number of violations

- **GIVEN** a generated repository with 5,000 violations and one cycle through 300 files
- **WHEN** the developer runs `archangel check --format json`
- **THEN** the `diagnostics` list has 5,001 items
- **AND** the cycle shows all 300 files
- **TEST** `crates/archangel/tests/output.rs` › "output_is_never_truncated"
