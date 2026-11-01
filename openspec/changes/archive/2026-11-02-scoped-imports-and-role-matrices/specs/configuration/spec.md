# configuration

## ADDED Requirements

### Requirement: Parts and groups are the names of elements and selectors

The config MUST accept the key `parts` with the values of `elements`, and the key `groups` with the values of `selectors`. A rule MUST accept `because` with the value of `reason`. The rule builder MUST accept a name as its first argument, as in `rule('<name>', { ... })`, with the same result as `rule({ name: '<name>', ... })`.

#### Scenario: The new spellings

- **GIVEN** a config with `parts: { feature: 'src/features/{feature}/**' }`, `groups: { tests: '**/*.test.ts' }`, and `rule('no-tests-in-features', { files: 'feature', mayNotImport: 'tests', because: 'Production code never imports a test.' })`
- **AND** the same config written with `elements`, `selectors`, `name` and `reason`
- **WHEN** archangel checks one repository with each config
- **THEN** the two checks give the same diagnostics, and each violation carries the reason
- **TEST** `crates/archangel_config/tests/spellings.rs` › "the_new_spellings_mean_the_old_ones"

### Requirement: A config needs a part or a rule, not both

A config MUST load with a rule and no part, because a table with `kinds` or a rule of path patterns names no part. A config with no part, no rule and no preset checks nothing, and MUST be refused with a message that names `parts`.

#### Scenario: A rule and no part

- **GIVEN** in turn, a config with only a table with `kinds`, and a config with only a rule of path patterns
- **WHEN** archangel loads the config
- **THEN** the config is valid
- **TEST** `crates/archangel_config/tests/merge.rs` › "a_config_with_a_rule_needs_no_part"

#### Scenario: No part and no rule

- **GIVEN** a config with no part, no rule and no preset
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at `parts`, and the message says to declare a part or write a rule
- **TEST** `crates/archangel_config/tests/merge.rs` › "a_config_with_no_part_no_rule_and_no_preset_is_refused"

### Requirement: A config uses one spelling for each key

A config that has both `parts` and `elements`, or both `groups` and `selectors`, MUST be refused with `config-invalid`. A rule of the config that has both `because` and `reason` MUST be refused with `config-invalid`. A rule builder call with a name as its first argument and a `name` key MUST be refused with `config-invalid`. The rules of a preset MAY use the other spelling.

#### Scenario: Two spellings of one key

- **GIVEN** a config with both `parts` and `elements`
- **AND** a config with both `groups` and `selectors`
- **AND** a config with a rule that has both `because` and `reason`
- **AND** a config with `rule('a', { name: 'b', files: 'src/**', mayNotImport: 'tests' })`
- **WHEN** archangel loads each config
- **THEN** each gives `config-invalid` at the second spelling, saying that a config uses one spelling for each key
- **TEST** `crates/archangel_config/tests/spellings.rs` › "a_config_cannot_mix_two_spellings_of_one_key"

#### Scenario: A preset written with the old spellings

- **GIVEN** a config with `parts` and `because`, which extends a preset whose rules use `reason`
- **WHEN** archangel loads the config
- **THEN** archangel gives no diagnostic about spellings
- **TEST** `crates/archangel_config/tests/spellings.rs` › "a_preset_may_use_the_other_spelling"

### Requirement: An externals set is a named list of package patterns

The config MUST accept the key `externals`, an object of named lists of package patterns. A pattern MUST mean what it means in a 0.1 package element: a package name, `@scope/*` for every package in a scope, `node:x` for one built-in module, `node:*` for every built-in module, `*` for every package, and any other `*` as a wildcard in the name. A package MUST be in every set that lists it.

#### Scenario: A scope

- **GIVEN** the externals set `aws` as `['@aws-sdk/*']`, and a rule that refuses `aws`
- **AND** a file that imports `@aws-sdk/client-s3` and `@aws-sdkx/y`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for `@aws-sdk/client-s3`
- **TEST** `crates/archangel_rules/tests/externals.rs` › "a_scope_name_matches_every_package_in_the_scope"

#### Scenario: A pattern means what it means in a 0.1 package element

- **GIVEN** the externals sets `builtins` as `['node:*']`, `db` as `['knex*']` and `all` as `['*']`, each in a rule that refuses it, in its own config
- **AND** a file that imports `node:path`, `knex-utils` and `react`
- **WHEN** archangel makes a check of each config
- **THEN** the first gives one violation, for `node:path`; the second one, for `knex-utils`; and the third one for each of the three imports
- **TEST** `crates/archangel_rules/tests/externals.rs` › "an_externals_name_is_a_package_pattern_as_in_0_1"

### Requirement: An import counts as the package it resolves to

An import of a subpath MUST count as an import of its package, because archangel records the package that an import resolves to. An import of a built-in module MUST count under its `node:` name, written with or without the prefix.

#### Scenario: A package and its subpaths

- **GIVEN** the externals set `driver` as `['pg']`, and a rule `rule('no-driver', { files: 'src/**', mayNotImport: 'driver' })`
- **AND** a file that imports `pg`, `pg/lib/client` and `pg-format`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation for each of `pg` and `pg/lib/client`, and none for `pg-format`
- **TEST** `crates/archangel_rules/tests/externals.rs` › "a_name_matches_its_package_and_every_subpath"

#### Scenario: Built-in modules

- **GIVEN** the externals set `fs` as `['node:fs', 'node:fs/promises']`, and a rule that refuses `fs`
- **AND** a file that imports `fs`, `node:fs`, `fs/promises`, `node:fs/promises` and `node:path`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation for each of the first four imports, and none for `node:path`
- **TEST** `crates/archangel_rules/tests/externals.rs` › "node_names_match_the_built_in_with_and_without_the_prefix"

### Requirement: An externals set has a name of its own

An externals set with the name of a part or a group MUST be refused with `config-invalid`, which names both declarations.

#### Scenario: An externals set with the name of a group

- **GIVEN** the group `driver` as `src/drivers/**` and the externals set `driver` as `['pg']`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at the externals set `driver`, saying that it is also a group
- **TEST** `crates/archangel_config/tests/externals_and_areas.rs` › "an_externals_set_with_the_name_of_a_part_or_group_is_refused"

### Requirement: An externals name that could never match is refused

A bare name of a built-in module MUST be refused with `config-invalid` and the fix `node:x`; the built-in names MUST be the list that the resolver uses. A name that starts with `@types/` MUST be refused with `config-invalid`. A name of a package of the workspace MUST be refused with `config-invalid`.

#### Scenario: Names that could never match

- **GIVEN** the externals sets `a` as `['fs']`, `b` as `['@types/react']`, and `c` as `['@enhance/contract']`, where `@enhance/contract` is a package of the workspace
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` for `fs` with the fix `node:fs`
- **AND** archangel gives `config-invalid` for `@types/react`, saying that archangel judges the package that an import names, such as `react`
- **AND** archangel gives `config-invalid` for `@enhance/contract`, saying that it resolves to a repository file and its part judges it
- **TEST** `crates/archangel_config/tests/externals_and_areas.rs` › "an_externals_name_that_could_never_match_is_refused"

### Requirement: A node name must be a built-in module

A name that starts with `node:`, has no wildcard, and names no built-in module MUST be refused with `config-invalid`. A built-in with a mandatory prefix, a built-in subpath and `node:*` MUST be accepted.

#### Scenario: A misspelt built-in

- **GIVEN** an externals set with `node:fss`
- **WHEN** archangel loads the config
- **THEN** the config is refused, and the message names `node:fss`
- **TEST** `crates/archangel_config/tests/externals_and_areas.rs` › "a_node_name_that_is_no_built_in_is_refused"

#### Scenario: Node names that exist

- **GIVEN** an externals set with `node:fs`, `node:fs/promises`, `node:test`, `node:sqlite` and `node:*`
- **WHEN** archangel loads the config
- **THEN** the config loads
- **TEST** `crates/archangel_config/tests/externals_and_areas.rs` › "node_names_that_exist_and_wildcards_are_accepted"

### Requirement: An externals name with a subpath is refused

A name with a subpath, such as `pg/lib/client`, MUST be refused with `config-invalid` and the fix `pg`, because archangel records the package that an import resolves to. A `node:` name with a subpath is a built-in module and MUST be accepted.

#### Scenario: A subpath of a package, and of a built-in

- **GIVEN** the externals sets `a` as `['pg/lib/client']` and `b` as `['node:fs/promises']`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` for `pg/lib/client` with the fix `pg`, and accepts `node:fs/promises`
- **TEST** `crates/archangel_config/tests/externals_and_areas.rs` › "an_externals_name_with_a_subpath_is_refused_unless_it_is_a_built_in"

### Requirement: A part holds no packages

A value of `parts` with packages MUST be refused with `config-invalid` and the fix to move it to `externals`.

#### Scenario: Packages in parts

- **GIVEN** a config with `parts: { react: { packages: ['react'] } }`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at the part, saying to move it to `externals`
- **TEST** `crates/archangel_config/tests/externals_and_areas.rs` › "parts_refuse_packages"

### Requirement: Every pattern starts at the config folder

A pattern in parts, groups, files, within, a kind pattern or a cell MUST start at the folder of the config. Archangel MUST NOT add `**/` before a pattern. An entry pattern MUST start at the instance root. For a part without a capture, the instance root MUST be the folders before the first wildcard of its pattern.

#### Scenario: A kind pattern in a monorepo

- **GIVEN** a table with the kind pattern `src/domain/{kind}/**`, and the files `src/domain/entities/a.ts` and `apps/api/src/domain/entities/b.ts`
- **WHEN** archangel makes a check
- **THEN** only `src/domain/entities/a.ts` has the kind `entities`
- **TEST** `crates/archangel_config/tests/pattern.rs` › "a_pattern_starts_at_the_config_folder"

#### Scenario: An entry on a part without a capture

- **GIVEN** the part `canvas-document` as `packages/canvas-document/**`, and `entry('canvas-document', 'src/*/index.ts')`
- **AND** a file in `web` that imports `packages/canvas-document/src/flex-grid/index.ts`, and another that imports `packages/canvas-document/src/flex-grid/grid.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of the entry rule, for the import of `grid.ts`
- **TEST** `crates/archangel_rules/tests/rules.rs` › "an_entry_pattern_starts_at_the_root_of_a_part_without_a_capture"

### Requirement: A brace with one name is a capture

A brace without a comma MUST be a capture, also when its name looks like an extension. `archangel explain` MUST show each capture of a file's part as an instance.

#### Scenario: One word in braces

- **GIVEN** the part `code` as `src/**/*.{ts}`, and the file `src/a.ts`
- **WHEN** the developer runs `archangel explain src/a.ts`
- **THEN** the output shows the part `code` with the capture `ts` and the value `ts`, because the capture matches the text after the dot
- **TEST** `crates/archangel/tests/cli.rs` › "explain_shows_a_one_word_brace_as_a_capture"

## MODIFIED Requirements

### Requirement: Archangel refuses a pattern that cannot mean what it says

Archangel MUST refuse a pattern that has an empty folder name, a `**` that is not a full folder name, a capture without a name, or the same capture name two times. A kind pattern MAY contain the capture `{kind}` two or more times, because each place takes the same kind. Another capture name two times in a kind pattern MUST be refused. The diagnostic MUST be `pattern-invalid`. It MUST name the pattern, and the element, or the table for a kind pattern.

#### Scenario: A double star inside a folder name

- **GIVEN** an element "ui" with the pattern `src/ui**/x`
- **WHEN** the developer runs `archangel check`
- **THEN** archangel prints a `pattern-invalid` diagnostic that names "ui" and `src/ui**/x`
- **AND** archangel exits with status 2
- **TEST** `crates/archangel_config/tests/validate.rs` › "double_star_inside_a_name_is_pattern_invalid"
- **TEST** `crates/archangel/tests/config_discovery.rs` › "invalid_pattern_exits_two"

#### Scenario: An empty folder name

- **GIVEN** an element "x" with the pattern `src//x`
- **WHEN** archangel loads the config
- **THEN** archangel gives a `pattern-invalid` diagnostic that names "x" and `src//x`
- **TEST** `crates/archangel_config/tests/validate.rs` › "empty_folder_name_is_pattern_invalid"

#### Scenario: A capture without a name

- **GIVEN** an element "x" with the pattern `src/{}/x`
- **WHEN** archangel loads the config
- **THEN** archangel gives a `pattern-invalid` diagnostic that names "x" and `src/{}/x`
- **TEST** `crates/archangel_config/tests/validate.rs` › "capture_without_a_name_is_pattern_invalid"

#### Scenario: One capture name two times

- **GIVEN** an element "x" with the pattern `src/{a}/{a}`
- **WHEN** archangel loads the config
- **THEN** archangel gives a `pattern-invalid` diagnostic that names "x", `src/{a}/{a}` and the capture "a"
- **TEST** `crates/archangel_config/tests/validate.rs` › "repeated_capture_name_is_pattern_invalid"

#### Scenario: The kind capture two times in a kind pattern

- **GIVEN** a table `domain` with the kind pattern `**/{kind}s/*.{kind}.ts`
- **AND** a table `apps` with the kind pattern `apps/{app}/{app}/*.{kind}.ts`
- **WHEN** archangel loads the config
- **THEN** archangel gives no diagnostic for the pattern of `domain`
- **AND** archangel gives a `pattern-invalid` diagnostic that names `apps`, its pattern and the capture "app"
- **TEST** `crates/archangel_config/tests/tables.rs` › "only_the_kind_capture_may_repeat_in_a_kind_pattern"

#### Scenario: A kind pattern that cannot mean what it says

- **GIVEN** a table `domain` with the kind pattern `**//*.{kind}.ts`
- **WHEN** archangel loads the config
- **THEN** archangel gives a `pattern-invalid` diagnostic that names the table `domain` and the pattern `**//*.{kind}.ts`
- **TEST** `crates/archangel_config/tests/tables.rs` › "a_kind_pattern_that_cannot_mean_what_it_says_is_pattern_invalid"

#### Scenario: Random text never makes the pattern parser fail without a diagnostic

- **GIVEN** random text as a pattern
- **WHEN** archangel parses the pattern
- **THEN** archangel gives a pattern or a `pattern-invalid` diagnostic, and does not stop in any other way
- **TEST** `fuzz/fuzz_targets/pattern.rs` › "pattern"

### Requirement: Each file is in one element at most

With `elements`, archangel MUST put each repository file in the first file element, in declaration order, that has a pattern which matches the file. With `parts`, a file that two of the config's own parts match MUST give one `part-overlap` diagnostic, which names the file and both parts, and the check MUST fail. The message MUST say to narrow one of the two patterns with `not` or to make one of them a group, and that every overlap is fixed in the config because the baseline does not hold it. The parts of each preset MUST be a separate set, so a file MAY be in one of the config's parts and in one part of each preset. A file that no pattern matches MUST be in no element.

#### Scenario: Two elements match the same file

- **GIVEN** the element "model" with `src/**/*.model.ts`, declared before the element "feature" with `src/features/{feature}/**`
- **WHEN** archangel classifies `src/features/login/session.model.ts`
- **THEN** the file is in the element "model" only
- **TEST** `crates/archangel_config/tests/pattern.rs` › "first_declared_element_wins"

#### Scenario: Two parts match the same file

- **GIVEN** the parts "model" with `src/**/*.model.ts` and "feature" with `src/features/{feature}/**`
- **WHEN** archangel makes a check on a repository with `src/features/login/session.model.ts`
- **THEN** archangel gives one `part-overlap` diagnostic that names the file, "model" and "feature"
- **AND** the message says to narrow one of the two patterns with `not` or to make one of them a group, and that the baseline does not hold it
- **AND** archangel exits with status 1
- **TEST** `crates/archangel_rules/tests/elements.rs` › "two_parts_that_match_one_file_are_an_error"

#### Scenario: A part overlap appears and goes in the editor

- **GIVEN** a session with the parts "model" and "feature" of this requirement, and no file that both match
- **WHEN** the developer creates `src/features/login/session.model.ts`
- **THEN** the language server reports `part-overlap` for that file, and the result is the same as a full check
- **AND** when the developer deletes the file, the diagnostic is gone, and the result is the same as a full check
- **TEST** `crates/archangel_engine/tests/sessions.rs` › "a_part_overlap_appears_and_goes_with_the_file_in_the_editor"

#### Scenario: A part of the config and a part of a preset

- **GIVEN** the part "web" with `apps/web/**`, and the preset `fsd({ root: 'apps/web/src', name: 'fsd-web' })`
- **AND** a rule on "web" and a rule of the preset on `fsd-web/features`
- **WHEN** archangel makes a check on a repository with `apps/web/src/features/cart/a.ts`
- **THEN** archangel gives no `part-overlap` diagnostic, and both rules judge the file
- **TEST** `crates/archangel_rules/tests/elements.rs` › "a_presets_parts_are_a_separate_set"
