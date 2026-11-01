# selectors

## MODIFIED Requirements

### Requirement: A selector that selects nothing is an error where emptiness means a mistake

A selector that the config declares, and an inline pattern in one of its rules, that matches no file MUST give a `selector-empty` diagnostic, except the files of a `mustNotExist` rule and an inline pattern that a rule writes only inside a `not`, because leaving out nothing changes nothing. What a preset declares and the kinds of a table are exempt, as their specs say. A selector that refers to itself through other selectors MUST be refused with `config-invalid` and the chain of names.

#### Scenario: A misspelt pattern

- **GIVEN** a selector "views" with the pattern `**/*.viewz.tsx` and no file that matches it
- **WHEN** archangel makes a check
- **THEN** archangel gives a `selector-empty` diagnostic that names "views" and its pattern
- **TEST** `crates/archangel_rules/tests/selectors.rs` › "a_selector_that_matches_nothing_is_an_error"

#### Scenario: A misspelt inline pattern

- **GIVEN** a rule `{ name: 'legacy', files: 'src/legasy/**', mayNotImport: 'src/next/**' }` and no file under `src/legasy`
- **WHEN** archangel makes a check
- **THEN** archangel gives a `selector-empty` diagnostic that names the rule "legacy" and the pattern
- **TEST** `crates/archangel_rules/tests/selectors.rs` › "an_inline_pattern_that_matches_nothing_is_an_error"

#### Scenario: An inline pattern only inside not

- **GIVEN** a rule whose files are `{ all: ['src/**', { not: ['src/generated/**', { any: ['src/vendor/**', 'src/app/**'] }] }] }` and whose side leaves out `src/legacy/**` with a `not`
- **AND** another rule whose side names `src/vendor/**` both inside and outside a `not`
- **AND** no file under `src/generated`, `src/vendor` or `src/legacy`
- **WHEN** archangel makes a check
- **THEN** archangel gives one `selector-empty` diagnostic, for `src/vendor/**` in the second rule
- **TEST** `crates/archangel_rules/tests/selectors.rs` › "an_inline_pattern_only_inside_not_may_match_nothing"

#### Scenario: A mustNotExist rule on a clean repository

- **GIVEN** elements that cover every file in `src`, and a rule `{ files: { all: ['src/**', { not: { element: '*' } }] }, mustNotExist: true }`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation and no `selector-empty` diagnostic
- **TEST** `crates/archangel_rules/tests/selectors.rs` › "an_empty_must_not_exist_selector_passes"

#### Scenario: A selector of files that no import reaches

- **GIVEN** a selector "images" defined as `**/*.png`, and the file `src/assets/logo.png`, which no import reaches
- **WHEN** archangel makes a check
- **THEN** archangel gives no `selector-empty` diagnostic, because a selector counts every file the walk visits
- **TEST** `crates/archangel_rules/tests/selectors.rs` › "a_selector_of_files_no_import_reaches_is_not_empty"

#### Scenario: Selectors that refer to each other

- **GIVEN** a selector "a" defined as `{ not: 'b' }` and a selector "b" defined as `{ any: ['a'] }`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` that shows the chain "a", "b", "a"
- **TEST** `crates/archangel_config/tests/validate.rs` › "selectors_that_refer_to_each_other_are_refused"

#### Scenario: A selector becomes empty in the editor

- **GIVEN** a session with a selector "client" defined as `{ directive: 'use client' }` and one file that has the directive
- **WHEN** the developer removes the directive from that file
- **THEN** the language server reports `selector-empty` for "client"
- **AND** the result is the same as a full check, and no other file is classified
- **TEST** `crates/archangel_engine/tests/sessions.rs` › "a_selector_that_empties_in_the_editor_is_reported"

#### Scenario: A kind of a table that matches nothing

- **GIVEN** a table with the kind pattern `**/*.{kind}.ts` and a row `aggregate-root`, and no file that ends in `.aggregate-root.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives no `selector-empty` diagnostic for the kind `aggregate-root`
- **TEST** `crates/archangel_rules/tests/selectors.rs` › "a_kind_of_a_table_that_matches_nothing_is_not_selector_empty"
