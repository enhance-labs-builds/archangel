# exceptions-and-baselines

## MODIFIED Requirements

### Requirement: An exception permits one import to break one rule

An exception MUST be a line comment or a block comment in the form `archangel-allow <rule name>: <reason>`. A run of exception comments on consecutive lines MUST apply to the import on the line after the run: the import whose specifier is on that line, or whose statement starts on that line. Each exception MUST permit that import to break the named rule only, and the import MUST still be checked against every other rule.

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

#### Scenario: An exception above a multi-line import

- **GIVEN** an exception for `entry(entity)` directly above `import {`, and the specifier that breaks the rule two lines lower, after the closing brace
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of `entry(entity)` and no `exception-unused` diagnostic
- **TEST** `crates/archangel_rules/tests/exceptions.rs` › "exception_above_a_multi_line_import_permits_it"

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
