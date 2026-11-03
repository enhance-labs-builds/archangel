# structure-rules

## ADDED Requirements

### Requirement: A folder that must not exist passes when no folder matches

A `folders` pattern of a `mustNotExist` sentence MUST NOT give `selector-empty` when it matches no folder, because matching nothing is how the sentence passes. A `folders` pattern of every other condition MUST still give `selector-empty` when it matches no folder.

#### Scenario: No folder matches a folder that must not exist

- **GIVEN** a rule `{ name: 'no-utils', folders: 'src/**/utils', mustNotExist: true }`, and a repository with `src/app/main.ts` and no folder named `utils`
- **WHEN** archangel makes a check
- **THEN** archangel gives no diagnostic
- **TEST** `crates/archangel_rules/tests/structure.rs` › "a_folder_that_must_not_exist_passes_when_no_folder_matches"

#### Scenario: Another folder condition still reports an empty pattern

- **GIVEN** a rule `{ name: 'readme', folders: 'modules/*', mustContain: 'README.md' }`, and no folder `modules`
- **WHEN** archangel makes a check
- **THEN** archangel gives a `selector-empty` diagnostic that names the rule "readme" and the pattern
- **TEST** `crates/archangel_rules/tests/structure.rs` › "a_folders_pattern_that_matches_no_folder_is_selector_empty"
