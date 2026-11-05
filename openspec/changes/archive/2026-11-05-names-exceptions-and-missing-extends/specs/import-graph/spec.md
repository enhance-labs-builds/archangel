# import-graph

## ADDED Requirements

### Requirement: The config can permit a tsconfig base that is not installed

When a tsconfig's `extends` names a package that is not installed, and an `allowUnresolved` entry matches the package name, archangel MUST NOT give the `resolution-file-invalid` diagnostic for it, and the summary MUST count it as a permitted unresolved import. Without such an entry, the diagnostic MUST name the entry that would permit it. The files below that tsconfig MUST still be checked, as they are when the diagnostic is given.

#### Scenario: A permitted missing base

- **GIVEN** `tools/job/tsconfig.json` with `"extends": "@tsconfig/missing/tsconfig.json"`, no such package installed, and a config whose `allowUnresolved` maps `@tsconfig/missing` to "the job installs its own packages"
- **WHEN** archangel makes a check
- **THEN** archangel gives no `resolution-file-invalid` diagnostic
- **AND** the summary counts one permitted unresolved import
- **TEST** `crates/archangel_rules/tests/rules.rs` › "permitted_missing_tsconfig_base_is_counted_not_reported"

#### Scenario: A missing base that the config does not permit

- **GIVEN** the same tsconfig, and no `allowUnresolved` entry
- **WHEN** archangel makes a check
- **THEN** archangel gives one `resolution-file-invalid` diagnostic whose help says to add `@tsconfig/missing` to `allowUnresolved` with the reason, or to install it
- **TEST** `crates/archangel_rules/tests/rules.rs` › "missing_tsconfig_base_names_the_entry_that_permits_it"
