# command-line

## ADDED Requirements

### Requirement: The JSON documents share their shape and their stream

`test --format json` MUST list each failed example and each unproven rule under `diagnostics`, as `check` lists its diagnostics. When the config stops a command, the JSON or SARIF document MUST go to standard output, where every other document goes.

#### Scenario: A stopped command

- **GIVEN** a config with an unknown field
- **WHEN** the developer runs `archangel check --format json`
- **THEN** standard output holds the document of the diagnostic, standard error is empty, and the exit status is 2
- **TEST** `crates/archangel/tests/formats.rs` › "json_format_prints_the_config_that_stops_a_command_as_one_document"

#### Scenario: The problems of `test`

- **GIVEN** a rule whose rejected example passes, and a rule with no rejected example
- **WHEN** the developer runs `archangel test --format json`
- **THEN** `diagnostics` holds `example-failed` and `rule-unproven`
- **TEST** `crates/archangel/tests/formats.rs` › "test_as_json_lists_its_problems_as_diagnostics"

### Requirement: The summary names each computed import

The summary MUST list each import whose specifier archangel cannot follow, by file and line: in text, one line each under the count; in JSON, `summary.unfollowedImports`.

#### Scenario: Two computed specifiers

- **GIVEN** a file with `require(name)` on line 3 and `import(\`./${name}\`)` on line 4
- **WHEN** the developer runs `archangel check`
- **THEN** the summary says `2 imports with a computed specifier`, then `src/a.ts:3` and `src/a.ts:4`
- **TEST** `crates/archangel/tests/output.rs` › "text_summary_counts_permitted_unresolved_and_computed_imports"
