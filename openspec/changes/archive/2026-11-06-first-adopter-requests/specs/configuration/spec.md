# configuration

## MODIFIED Requirements

### Requirement: Two parts that match one file fail the check

With `parts`, a file that two of the config's own parts match MUST give one `part-overlap` diagnostic, which names the file and both parts, and the check MUST fail. The message MUST say to leave the file out of one part with a `!` pattern in its list or to make one of them a group, and that every overlap is fixed in the config because the baseline does not hold it.

#### Scenario: Two parts match the same file

- **GIVEN** the parts "model" with `src/**/*.model.ts` and "feature" with `src/features/{feature}/**`
- **WHEN** archangel makes a check on a repository with `src/features/login/session.model.ts`
- **THEN** archangel gives one `part-overlap` diagnostic that names the file, "model" and "feature"
- **AND** the message says to leave the file out of one part with a `!` pattern or to make one of them a group, and that the baseline does not hold it
- **AND** archangel exits with status 1
- **TEST** `crates/archangel_rules/tests/elements.rs` › "two_parts_that_match_one_file_are_an_error"

#### Scenario: A part overlap appears and goes in the editor

- **GIVEN** a session with the parts "model" and "feature" of this requirement, and no file that both match
- **WHEN** the developer creates `src/features/login/session.model.ts`
- **THEN** the language server reports `part-overlap` for that file, and the result is the same as a full check
- **AND** when the developer deletes the file, the diagnostic is gone, and the result is the same as a full check
- **TEST** `crates/archangel_engine/tests/sessions.rs` › "a_part_overlap_appears_and_goes_with_the_file_in_the_editor"

## ADDED Requirements

### Requirement: A part can leave files out with a `!` pattern

A pattern that starts with `!` in a part's list MUST leave the files it matches out of the part. A list of only `!` patterns MUST be `config-invalid`.

#### Scenario: A nested part

- **GIVEN** a part "pack" with `["src/shared/{pack}/**", "!src/shared/widgets/**"]` and a part "widget" with `src/shared/widgets/{widget}/**`
- **WHEN** archangel makes a check
- **THEN** the files below `src/shared/widgets` are in "widget" only, and there is no `part-overlap`
- **TEST** `crates/archangel_rules/tests/elements.rs` › "a_bang_pattern_leaves_a_nested_part_out_of_its_part"

#### Scenario: Only `!` patterns

- **GIVEN** a part with `["!src/shared/widgets/**"]`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid`, whose message says that the list has only `!` patterns
- **TEST** `crates/archangel_rules/tests/elements.rs` › "a_part_of_only_bang_patterns_is_refused"

### Requirement: `./` names a file at the root

A pattern that starts with `./` MUST name the path after it, from the config folder. A name that is not declared and ends in an extension MUST give `rule-unknown-element`, whose message suggests the same text after `./`.

#### Scenario: A root file in a group

- **GIVEN** a group with `["./app.config.ts", "./src/**"]`
- **WHEN** archangel loads the config
- **THEN** the group's patterns are `app.config.ts` and `src/**`
- **TEST** `crates/archangel_config/tests/selectors.rs` › "a_root_file_is_named_with_dot_slash"

#### Scenario: A bare file name

- **GIVEN** a group with `["app.config.ts"]`
- **WHEN** archangel loads the config
- **THEN** the message ends with `, so a file at the root is written './app.config.ts'`
- **TEST** `crates/archangel_config/tests/selectors.rs` › "a_bare_file_name_suggests_dot_slash"

### Requirement: A rule can warn

A rule MUST accept `severity`, `'error'` (the default) or `'warn'`. A violation of a rule that warns MUST be reported as a warning in every format, and MUST NOT fail `check`. `check --max-warnings <count>` MUST fail the check when more warnings than the count are shown.

#### Scenario: A deprecation

- **GIVEN** a rule with `severity: 'warn'` and one file that breaks it
- **WHEN** the developer runs `archangel check`
- **THEN** the text shows `warning[may-not-import]`, the summary says `1 warning`, the JSON gives `"severity": "warning"` and `summary.warnings` 1, GitHub output uses `::warning`, and the exit status is 0
- **TEST** `crates/archangel/tests/cli.rs` › "a_rule_that_warns_reports_and_passes"

#### Scenario: A limit on warnings

- **GIVEN** the same rule and file
- **WHEN** the developer runs `archangel check --max-warnings 1`, then `--max-warnings 0`
- **THEN** the first exits with status 0, and the second with status 1
- **TEST** `crates/archangel/tests/cli.rs` › "max_warnings_fails_the_check_past_its_count"
