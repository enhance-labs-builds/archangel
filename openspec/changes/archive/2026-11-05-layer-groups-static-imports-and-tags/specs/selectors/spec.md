# selectors

## ADDED Requirements

### Requirement: A selector can select files by a docblock tag

A selector MUST accept `{ tag: <tag> }`, which selects the files whose comments before the first statement carry that JSDoc tag: the tag at the start of a comment line, after optional whitespace and `*`, followed by whitespace or the end of the line. A tag MUST start with `@`; a value without it MUST be `config-invalid`, with the fix.

#### Scenario: A tagged file

- **GIVEN** a selector "pure" defined as `{ tag: '@side-effect-free' }`
- **AND** `src/a.ts` that starts with a license comment, then `/** @side-effect-free */`, `src/b.ts` with no tag, and `src/c.ts` whose tag is in a comment after its first statement
- **WHEN** archangel classifies the files
- **THEN** "pure" holds `src/a.ts` only
- **TEST** `crates/archangel_rules/tests/selectors.rs` › "a_tag_selector_selects_files_by_their_docblock_tag"

#### Scenario: A tag without `@`

- **GIVEN** a selector `{ tag: 'side-effect-free' }`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid`, whose message says to write `@side-effect-free`
- **TEST** `crates/archangel_config/tests/selectors.rs` › "a_tag_without_at_is_config_invalid"
