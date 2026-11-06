# architecture-rules

## ADDED Requirements

### Requirement: A package pattern can name a subpath

A package pattern with more names than a package has, such as `lodash-es/*` or `@scope/tokens/core/*`, MUST name only the subpaths it matches, compared with the specifier's package and subpath. Any other package pattern MUST name the package, its root and each subpath. A transitive rule whose side names a subpath MUST be `config-invalid`.

#### Scenario: Deep imports and a subtree

- **GIVEN** rules that `src/**` may not import `utils/*` and may not import `@scope/tokens/core/*`
- **AND** files that import `utils`, `utils/get`, `@scope/tokens/core/a` and `@scope/tokens/base/a`
- **WHEN** archangel makes a check
- **THEN** archangel gives violations for `utils/get` and `@scope/tokens/core/a` only
- **TEST** `crates/archangel_rules/tests/subpaths_and_templates.rs` › "a_package_pattern_with_a_subpath_bans_deep_imports_only"

#### Scenario: A package without a subpath

- **GIVEN** a rule that `src/**` may not import `utils`
- **WHEN** archangel makes a check on files that import `utils` and `utils/get`
- **THEN** archangel gives a violation for each
- **TEST** `crates/archangel_rules/tests/subpaths_and_templates.rs` › "a_package_pattern_without_a_subpath_still_names_every_subpath"

#### Scenario: A transitive rule

- **GIVEN** a transitive rule whose side is `{ packages: ['lodash-es/*'] }`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid`, which says that `lodash-es/*` cannot end a chain
- **TEST** `crates/archangel_config/tests/selectors.rs` › "a_transitive_rule_refuses_a_package_subpath"
