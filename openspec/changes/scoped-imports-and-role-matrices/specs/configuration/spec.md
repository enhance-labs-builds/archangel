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

### Requirement: A config uses one spelling for each key

A config that has both `parts` and `elements`, or both `groups` and `selectors`, MUST be refused with `config-invalid`. A rule of the config that has both `because` and `reason` MUST be refused with `config-invalid`. The rules of a preset MAY use the other spelling.

#### Scenario: Two spellings of one key

- **GIVEN** a config with both `parts` and `elements`
- **AND** a config with a rule that has both `because` and `reason`
- **WHEN** archangel loads each config
- **THEN** each gives `config-invalid` at the second key, saying that a config uses one spelling for each key
- **TEST** `crates/archangel_config/tests/spellings.rs` › "a_config_cannot_mix_two_spellings_of_one_key"

#### Scenario: A preset written with the old spellings

- **GIVEN** a config with `parts` and `because`, which extends a preset whose rules use `reason`
- **WHEN** archangel loads the config
- **THEN** archangel gives no diagnostic about spellings
- **TEST** `crates/archangel_config/tests/spellings.rs` › "a_preset_may_use_the_other_spelling"

### Requirement: An externals set names outside packages exactly

The config MUST accept the key `externals`, an object of named lists of package names. A name MUST match the package of that name and every subpath of it. A name with a subpath MUST match only that subpath and the subpaths below it. A name that ends with `/*` MUST match every package in that scope. A name `node:x` MUST match the built-in module `x`, written with or without the prefix, and its subpaths.

#### Scenario: A package and its subpaths

- **GIVEN** the externals set `driver` as `['pg']`, and a rule `rule('no-driver', { files: 'src/**', mayNotImport: 'driver' })`
- **AND** a file that imports `pg`, `pg/lib/client` and `pg-format`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation for each of `pg` and `pg/lib/client`, and none for `pg-format`
- **TEST** `crates/archangel_rules/tests/externals.rs` › "a_name_matches_its_package_and_every_subpath"

#### Scenario: A name with a subpath

- **GIVEN** the externals set `client` as `['pg/lib/client']`, and a rule that refuses `client`
- **AND** a file that imports `pg`, `pg/lib/client` and `pg/lib/client/pool`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation for each of `pg/lib/client` and `pg/lib/client/pool`, and none for `pg`
- **TEST** `crates/archangel_rules/tests/externals.rs` › "a_name_with_a_subpath_matches_that_subpath_and_below"

#### Scenario: Built-in modules

- **GIVEN** the externals set `fs` as `['node:fs']`, and a rule that refuses `fs`
- **AND** a file that imports `fs`, `node:fs`, `fs/promises`, `node:fs/promises` and `node:path`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation for each of the first four imports, and none for `node:path`
- **TEST** `crates/archangel_rules/tests/externals.rs` › "node_names_match_the_built_in_with_and_without_the_prefix"

#### Scenario: A scope

- **GIVEN** the externals set `aws` as `['@aws-sdk/*']`, and a rule that refuses `aws`
- **AND** a file that imports `@aws-sdk/client-s3` and `@aws-sdkx/y`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for `@aws-sdk/client-s3`
- **TEST** `crates/archangel_rules/tests/externals.rs` › "a_scope_name_matches_every_package_in_the_scope"

### Requirement: An externals name that could never match is refused, and a part holds no packages

A bare name of a built-in module MUST be refused with `config-invalid` and the fix `node:x`. A name that starts with `@types/` MUST be refused with `config-invalid`. A name of a package of the workspace MUST be refused with `config-invalid`. A value of `parts` with packages MUST be refused with `config-invalid` and the fix to move it to `externals`.

#### Scenario: Names that could never match

- **GIVEN** the externals sets `a` as `['fs']`, `b` as `['@types/react']`, and `c` as `['@enhance/contract']`, where `@enhance/contract` is a package of the workspace
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` for `fs` with the fix `node:fs`
- **AND** archangel gives `config-invalid` for `@types/react`, saying that archangel judges the package that an import names, such as `react`
- **AND** archangel gives `config-invalid` for `@enhance/contract`, saying that it resolves to a repository file and its part judges it
- **TEST** `crates/archangel_config/tests/validate.rs` › "an_externals_name_that_could_never_match_is_refused"

#### Scenario: Packages in parts

- **GIVEN** a config with `parts: { react: { packages: ['react'] } }`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at the part, saying to move it to `externals`
- **TEST** `crates/archangel_config/tests/validate.rs` › "parts_refuse_packages"

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
- **THEN** the output shows the part `code` with the capture `ts` and the value `a`
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

With `elements`, archangel MUST put each repository file in the first file element, in declaration order, that has a pattern which matches the file. With `parts`, a file that two of the config's own parts match MUST give one `part-overlap` diagnostic, which names the file and both parts, and the check MUST fail. The parts of each preset MUST be a separate set, so a file MAY be in one of the config's parts and in one part of each preset. A file that no pattern matches MUST be in no element.

#### Scenario: Two elements match the same file

- **GIVEN** the element "model" with `src/**/*.model.ts`, declared before the element "feature" with `src/features/{feature}/**`
- **WHEN** archangel classifies `src/features/login/session.model.ts`
- **THEN** the file is in the element "model" only
- **TEST** `crates/archangel_config/tests/pattern.rs` › "first_declared_element_wins"

#### Scenario: Two parts match the same file

- **GIVEN** the parts "model" with `src/**/*.model.ts` and "feature" with `src/features/{feature}/**`
- **WHEN** archangel makes a check on a repository with `src/features/login/session.model.ts`
- **THEN** archangel gives one `part-overlap` diagnostic that names the file, "model" and "feature"
- **AND** archangel exits with status 1
- **TEST** `crates/archangel_rules/tests/elements.rs` › "two_parts_that_match_one_file_are_an_error"

#### Scenario: A part of the config and a part of a preset

- **GIVEN** the part "web" with `apps/web/**`, and the preset `fsd({ root: 'apps/web/src', name: 'fsd-web' })`
- **AND** a rule on "web" and a rule of the preset on `fsd-web/features`
- **WHEN** archangel makes a check on a repository with `apps/web/src/features/cart/a.ts`
- **THEN** archangel gives no `part-overlap` diagnostic, and both rules judge the file
- **TEST** `crates/archangel_rules/tests/elements.rs` › "a_presets_parts_are_a_separate_set"
