# exceptions-and-baselines Specification

## Purpose
A team must be able to turn on a rule before the code obeys it, and to permit one deliberate import.
This capability covers exceptions and baselines, which permit old violations without a way for new
violations to hide.

## Requirements

### Requirement: An exception permits one import to break one rule

An exception MUST be a line comment or a block comment in the form `archangel-allow <rule name>: <reason>`. A run of exception comments on consecutive lines MUST apply to the import on the line after the run. Each exception MUST permit that import to break the named rule only, and the import MUST still be checked against every other rule.

#### Scenario: An exception above an import

- **GIVEN** the comment `// archangel-allow entry(entity): the migration reads the raw store` directly above an import that breaks the entry rule
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of `entry(entity)` for that import
- **AND** the check summary counts one exception for `entry(entity)`
- **TEST** `crates/archangel_rules/tests/exceptions.rs` › "exception_permits_one_import_to_break_one_rule"

#### Scenario: The import breaks a second rule

- **GIVEN** the same exception, and an import that breaks `entry(entity)` and a layers rule
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of the layers rule for that import
- **TEST** `crates/archangel_rules/tests/exceptions.rs` › "exception_does_not_permit_other_rules"

#### Scenario: Two exceptions above one import

- **GIVEN** an exception for `entry(entity)` on line 3, an exception for the layers rule on line 4, and an import on line 5 that breaks both rules
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation for that import and no `exception-unused` diagnostic
- **TEST** `crates/archangel_rules/tests/exceptions.rs` › "consecutive_exceptions_apply_to_the_import_after_them"

#### Scenario: An exception in a block comment

- **GIVEN** the comment `/* archangel-allow entry(entity): generated code */` directly above an import that breaks that rule
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of `entry(entity)` for that import
- **TEST** `crates/archangel_rules/tests/exceptions.rs` › "exception_in_a_block_comment_permits_the_import"

#### Scenario: A blank line between the exception and the import

- **GIVEN** an exception for `entry(entity)`, then a blank line, then an import that breaks that rule
- **WHEN** archangel makes a check
- **THEN** archangel gives a violation of `entry(entity)` and an `exception-unused` diagnostic
- **TEST** `crates/archangel_rules/tests/exceptions.rs` › "exception_separated_by_a_blank_line_permits_nothing"

### Requirement: Only a violation of an import takes an exception

An exception MUST apply only to a violation of the import on the next line, a transitive violation reported at that import included. A violation about a file, a folder or an instance, and a plugin violation that names no import, MUST be permitted only through the baseline.

#### Scenario: An exception above a file with a bad name

- **GIVEN** a `mustBeNamed` rule that `src/userProfile.ts` breaks, and `// archangel-allow <that rule>: legacy` as its first line
- **WHEN** archangel makes a check
- **THEN** archangel gives the naming violation, and `exception-unused` for the comment, which says the violation can only be baselined
- **TEST** `crates/archangel_rules/tests/exceptions.rs` › "an_exception_does_not_permit_a_violation_about_a_file"

#### Scenario: An exception above the first import of a chain

- **GIVEN** a transitive rule, and a client file whose chain to `server-only` starts at `import './lib/b'`, with an exception for the rule above that import
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation and counts one exception
- **TEST** `crates/archangel_rules/tests/exceptions.rs` › "an_exception_permits_a_transitive_violation_at_its_first_import"

### Requirement: An exception must give its reason

An exception without a reason MUST NOT permit anything. Archangel MUST give an `exception-without-reason` diagnostic for it. An exception that names a rule which the config does not have MUST give an `exception-unknown-rule` diagnostic.

#### Scenario: An exception with no reason

- **GIVEN** the comment `// archangel-allow entry(entity):` above an import that breaks that rule
- **WHEN** archangel makes a check
- **THEN** archangel prints an `exception-without-reason` diagnostic and a violation of `entry(entity)`
- **TEST** `crates/archangel_rules/tests/exceptions.rs` › "exception_without_a_reason_permits_nothing"

#### Scenario: An exception for a rule that does not exist

- **GIVEN** the comment `// archangel-allow entry(entitty): old code` above an import
- **WHEN** archangel makes a check
- **THEN** archangel prints an `exception-unknown-rule` diagnostic that names `entry(entitty)` and the rule names that do exist
- **TEST** `crates/archangel_rules/tests/exceptions.rs` › "exception_for_an_unknown_rule_is_exception_unknown_rule"

#### Scenario: Random comments never make the exception reader fail without a diagnostic

- **GIVEN** random text after `archangel-allow` in a comment
- **WHEN** archangel reads the comment
- **THEN** archangel gives an exception or a diagnostic, and does not stop in any other way
- **TEST** `fuzz/fuzz_targets/exception.rs` › "exception"

### Requirement: An exception that permits nothing is an error

When an exception's import does not break the rule that the exception names, archangel MUST give an `exception-unused` diagnostic. So an exception goes away when the code is fixed.

#### Scenario: The import was fixed

- **GIVEN** an exception for `entry(entity)` above an import of the entity's entry file
- **WHEN** archangel makes a check
- **THEN** archangel prints an `exception-unused` diagnostic for that comment
- **AND** archangel exits with status 1
- **TEST** `crates/archangel_rules/tests/exceptions.rs` › "exception_that_permits_nothing_is_exception_unused"
- **TEST** `crates/archangel/tests/cli.rs` › "exception_unused_exits_one"

### Requirement: A baseline records violations that exist when a rule starts

`archangel baseline add` MUST write every current violation to `archangel.baseline.json` in the config folder. A baseline entry MUST record the rule name, the importing file, the specifier and a count, or, for a violation about a file, a folder or an instance, the rule name, that path and a count. It MUST NOT record a line, so that an edit above an import does not change the entry. The file MUST be sorted, so that two runs on the same code write the same bytes.

#### Scenario: A baseline for a new rule

- **GIVEN** a repository with three violations of a layers rule
- **WHEN** the developer runs `archangel baseline add`
- **THEN** `archangel.baseline.json` has entries for the three violations, sorted by rule, file and specifier
- **TEST** `crates/archangel/tests/baseline_command.rs` › "baseline_add_records_every_violation_sorted"

#### Scenario: Two runs on the same code

- **GIVEN** a repository with violations, and no baseline
- **WHEN** the developer runs `archangel baseline add` two times
- **THEN** the second run writes the same bytes as the first
- **TEST** `crates/archangel/tests/baseline_command.rs` › "baseline_add_two_times_writes_the_same_bytes"

#### Scenario: A recorded cycle gains a file

- **GIVEN** a baseline entry for a no-cycles violation of the group `src/a.ts` and `src/b.ts`
- **AND** an edit that adds `src/c.ts` to the group
- **WHEN** archangel makes a check
- **THEN** archangel gives a violation for the group of three files
- **AND** archangel gives a `baseline-stale` diagnostic for the entry of two files
- **TEST** `crates/archangel_rules/tests/baseline.rs` › "cycle_group_that_gains_a_file_is_a_new_violation"

#### Scenario: Lines move above a recorded import

- **GIVEN** a baseline entry for an import on line 4
- **AND** an edit that adds ten lines above that import
- **WHEN** archangel makes a check
- **THEN** the baseline entry still matches the violation
- **TEST** `crates/archangel_rules/tests/baseline.rs` › "baseline_entry_matches_after_lines_move"

### Requirement: A check fails only for violations that are not in the baseline

A check MUST NOT fail for a violation that matches a baseline entry with a count that is not used yet. Every other violation MUST fail the check. The check summary MUST count the violations that the baseline matched, for each rule.

#### Scenario: A new violation next to recorded ones

- **GIVEN** a baseline with one entry for `src/a.ts` and `./b` with the count 1
- **AND** the file `src/a.ts` that now has two imports of `./b` that break the rule
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation and the summary counts one violation matched by the baseline
- **AND** archangel exits with status 1
- **TEST** `crates/archangel_rules/tests/baseline.rs` › "violation_beyond_the_recorded_count_fails"
- **TEST** `crates/archangel/tests/baseline_command.rs` › "new_violation_beside_a_baseline_exits_one"

### Requirement: Archangel refuses a baseline that it cannot read

When the baseline file does not parse, archangel MUST give a `baseline-invalid` diagnostic with the line and the column, and MUST exit with status 2. A baseline entry for a rule that the config does not have MUST give a `baseline-stale` diagnostic.

#### Scenario: A baseline with a missing comma

- **GIVEN** an `archangel.baseline.json` with a missing comma on line 3
- **WHEN** the developer runs `archangel check`
- **THEN** archangel prints a `baseline-invalid` diagnostic on line 3
- **AND** archangel exits with status 2
- **TEST** `crates/archangel/tests/baseline_command.rs` › "baseline_that_does_not_parse_is_baseline_invalid"

#### Scenario: An entry for a removed rule

- **GIVEN** a baseline entry for the rule `entry(widget)`, which the config does not have
- **WHEN** archangel makes a check
- **THEN** archangel gives a `baseline-stale` diagnostic for that entry that says the rule does not exist
- **TEST** `crates/archangel_rules/tests/baseline.rs` › "entry_for_a_removed_rule_is_baseline_stale"

### Requirement: The baseline can only shrink

A check MUST NOT write the baseline. When a baseline entry matches fewer violations than its count, archangel MUST give a `baseline-stale` diagnostic that names the entry and the command `archangel baseline prune`. `archangel baseline prune` MUST lower or remove those entries and MUST NOT add an entry or raise a count.

#### Scenario: A recorded violation was fixed

- **GIVEN** a baseline entry for a violation that the code no longer has
- **WHEN** archangel makes a check
- **THEN** archangel prints a `baseline-stale` diagnostic that names the entry and `archangel baseline prune`
- **AND** the baseline file has the same bytes as before
- **TEST** `crates/archangel_rules/tests/baseline.rs` › "fixed_violation_makes_the_entry_baseline_stale"
- **TEST** `crates/archangel/tests/baseline_command.rs` › "check_does_not_write_the_baseline"

#### Scenario: Prune removes fixed entries only

- **GIVEN** a baseline with one fixed entry, one entry that still matches, and one new violation that is not in the baseline
- **WHEN** the developer runs `archangel baseline prune`
- **THEN** the baseline has only the entry that still matches
- **TEST** `crates/archangel/tests/baseline_command.rs` › "prune_removes_fixed_entries_and_adds_nothing"

### Requirement: The summary shows how much each rule permits

The check summary MUST show, for each rule, the number of exceptions and the number of violations that the baseline matched. The JSON output MUST contain the same numbers.

#### Scenario: Counts in the JSON output

- **GIVEN** a rule with two exceptions and three violations that the baseline matched
- **WHEN** the developer runs `archangel check --format json`
- **THEN** the output for that rule has the exception count 2 and the baseline count 3
- **TEST** `crates/archangel/tests/output.rs` › "json_summary_counts_exceptions_and_baseline_per_rule"

### Requirement: An exception or a baseline entry for a removed row fails the check

An exception that names a table row that the config does not have MUST give `exception-unknown-rule`. An exception for a row that the import does not break MUST give `exception-unused`. A baseline entry for a row that the config does not have MUST give `baseline-stale`. Each of these diagnostics MUST fail the check.

#### Scenario: A row that was removed from its table

- **GIVEN** the comment `// archangel-allow domain.port: reads the raw store` above an import, and a baseline entry for `domain.port`, and a table `domain` with no row `port`
- **WHEN** archangel makes a check
- **THEN** archangel gives `exception-unknown-rule` for the comment and `baseline-stale` for the entry
- **AND** archangel exits with status 1
- **TEST** `crates/archangel_rules/tests/exceptions.rs` › "an_exception_and_a_baseline_entry_for_a_removed_row_are_stale"

#### Scenario: A row that now permits the import

- **GIVEN** the comment `// archangel-allow domain.entity: needs the use case for now` above an import that the row `domain.entity` now permits
- **WHEN** archangel makes a check
- **THEN** archangel gives `exception-unused` for the comment, and exits with status 1
- **TEST** `crates/archangel_rules/tests/exceptions.rs` › "an_exception_for_a_row_that_permits_the_import_is_unused"
