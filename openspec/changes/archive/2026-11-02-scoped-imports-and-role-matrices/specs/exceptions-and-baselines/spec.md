# exceptions-and-baselines

## ADDED Requirements

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
