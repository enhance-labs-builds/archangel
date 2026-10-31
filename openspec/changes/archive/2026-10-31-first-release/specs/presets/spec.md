# Spec Delta

## Purpose

Presets let a team adopt a known architecture, such as Feature-Sliced Design, in one line, and let
anyone publish one: a function that returns elements, selectors, rules and the examples that prove
them, composed into the config with `extends`.

## ADDED Requirements

### Requirement: Everything a preset declares carries its prefix

Archangel MUST name each element, selector and rule that a preset returns with the preset's name, a slash and the name the preset gave it, such as `fsd/shared`. A rule side MUST refer to a preset's element or selector by that name.

#### Scenario: Two presets that both declare shared

- **GIVEN** a config that extends two presets, "a" and "b", which each declare an element `shared` with a different pattern
- **WHEN** archangel loads the config
- **THEN** the config has the elements `a/shared` and `b/shared`, each with its own pattern
- **TEST** `crates/archangel_config/tests/merge.rs` › "preset_names_carry_the_presets_prefix"

#### Scenario: A team element with the same name as a preset's

- **GIVEN** a config with its own element `shared` that extends a preset "fsd" which declares `shared`
- **WHEN** archangel loads the config
- **THEN** the config has both `shared` and `fsd/shared`, and the preset's rules use `fsd/shared`
- **TEST** `crates/archangel_config/tests/merge.rs` › "a_config_element_never_replaces_a_preset_element"

### Requirement: The config's rules replace preset rules of the same name

The config's `extends` MUST take a list of preset results, and `extends` MUST need a TypeScript or JavaScript config. A config rule whose name is a preset rule's prefixed name MUST replace that rule. Two rules with one name within one preset, or within the config, MUST still be `rule-name-duplicate`.

#### Scenario: The config replaces a preset rule

- **GIVEN** a preset "team" with the rule `views-thin` that has `mayOnlyImport: ['team/shared']`, and a config that extends it and declares the rule `team/views-thin` with `mayOnlyImport: ['team/shared', 'team/model']`
- **WHEN** archangel loads the config
- **THEN** the config has one rule `team/views-thin`, which permits imports of `team/shared` and `team/model`
- **TEST** `crates/archangel_config/tests/merge.rs` › "a_config_rule_replaces_the_preset_rule_of_the_same_name"

#### Scenario: A misspelt replacement

- **GIVEN** the same preset, and a config rule named `team/view-thin`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at the rule's `name`, suggesting `team/views-thin`, rather than adding a rule that replaces nothing
- **TEST** `crates/archangel_config/tests/merge.rs` › "a_config_rule_with_a_presets_prefix_must_replace_one_of_its_rules"

#### Scenario: A JSON config with extends

- **GIVEN** an `archangel.config.json` with an `extends` field
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at `extends`, saying presets need a TypeScript or JavaScript config
- **TEST** `crates/archangel_config/tests/validate.rs` › "a_json_config_cannot_extend_a_preset"

### Requirement: Off drops preset rules by name

The config's `off` MUST list rule names to drop after the merge. A name in `off` that matches no rule MUST be refused with `config-invalid` and the rule names closest to it.

#### Scenario: Turning off one Feature-Sliced Design rule

- **GIVEN** a config with `extends: [fsd({ root: 'src' })]` and `off: ['fsd/insignificant-slice']`, on a repository with an insignificant slice
- **WHEN** archangel makes a check
- **THEN** no diagnostic names the rule `fsd/insignificant-slice`
- **TEST** `crates/archangel/tests/presets.rs` › "off_drops_a_preset_rule"

#### Scenario: A name in off that matches nothing

- **GIVEN** a config with `off: ['fsd/insignificant-slices']`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at `off`, suggesting `fsd/insignificant-slice`
- **TEST** `crates/archangel_config/tests/merge.rs` › "an_off_name_that_matches_nothing_is_refused"

#### Scenario: Off after the merge, and examples under the root

- **GIVEN** a preset "fsd" with root `apps/web/src` and the rules `a`, with a rejected example at `features/x/a.ts`, and `b`, and a config with `off: ['fsd/b']`
- **WHEN** archangel loads the config
- **THEN** the config has the one rule `fsd/a`, whose example file is `apps/web/src/features/x/a.ts`
- **TEST** `crates/archangel_config/tests/merge.rs` › "off_drops_a_rule_and_examples_move_under_the_root"

### Requirement: Every preset rule has a name

Each rule that a preset returns MUST have a name. A preset rule without one MUST be refused with `config-invalid`, naming the preset and its package and saying to report it to the preset's author.

#### Scenario: A preset rule with no name

- **GIVEN** a preset "team" from the package `archangel-plugin-team` that returns a rule without a name
- **WHEN** archangel loads a config that extends it
- **THEN** archangel gives `config-invalid` that names the preset "team" and the package `archangel-plugin-team`, and says the preset's author must name the rule
- **TEST** `crates/archangel_config/tests/merge.rs` › "a_preset_rule_without_a_name_is_refused"

### Requirement: A preset used more than once has its own name

A preset given a `name` option MUST use it as its prefix. A preset used twice under one name MUST be refused with `config-invalid` that says the second use needs its own `name`.

#### Scenario: Two apps in one repository

- **GIVEN** a repository with `extends: [fsd({ root: 'apps/web/src' }), fsd({ root: 'apps/admin/src', name: 'fsd-admin' })]`, and a feature in the web app that imports a feature in the admin app
- **WHEN** archangel makes a check
- **THEN** the rules named `fsd/…` apply to the web app and the rules named `fsd-admin/…` to the admin app, and neither gives a violation for the import between apps
- **TEST** `crates/archangel/tests/presets.rs` › "one_preset_twice_under_two_roots"

#### Scenario: One preset twice with the same name

- **GIVEN** a config with `extends: [fsd({ root: 'apps/web/src' }), fsd({ root: 'apps/admin/src' })]`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at the second entry, saying the preset `fsd` is used twice and the second needs its own `name`
- **TEST** `crates/archangel_config/tests/merge.rs` › "one_preset_twice_with_one_name_is_refused"

### Requirement: A preset's empty elements are listed, and an empty preset is an error

An element or selector that a preset declares and that matches nothing MUST NOT be an error, and MUST be listed in the summary. A preset whose elements together match no file MUST give a `preset-empty` diagnostic that names the preset and its root.

#### Scenario: A Feature-Sliced Design app without widgets

- **GIVEN** a config with `extends: [fsd({ root: 'src' })]`, on a repository with no `src/widgets` and no `src/processes`
- **WHEN** archangel makes a check
- **THEN** archangel gives no `selector-empty` or `element-empty` diagnostic, and the summary lists `fsd/widgets` and `fsd/processes` as matching nothing
- **TEST** `crates/archangel/tests/presets.rs` › "a_presets_unused_layers_are_listed_not_failed"

#### Scenario: A wrong root

- **GIVEN** a config with `extends: [fsd({ root: 'scr' })]` and no folder `scr`
- **WHEN** archangel makes a check
- **THEN** archangel gives `preset-empty` that names `fsd` and `scr`
- **TEST** `crates/archangel/tests/presets.rs` › "a_preset_that_matches_nothing_is_preset_empty"

### Requirement: A preset's examples prove it in the team's repository

`archangel test` MUST check the examples of each preset rule over the team's repository, with paths relative to the root the preset was given, as it checks the config's own examples.

#### Scenario: A preset applied under one app

- **GIVEN** a config with `extends: [fsd({ root: 'apps/web/src' })]`
- **WHEN** the developer runs `archangel test`
- **THEN** each preset rule's examples are placed under `apps/web/src` and checked, and every rule passes
- **TEST** `crates/archangel/tests/presets.rs` › "preset_examples_are_checked_under_the_presets_root"

### Requirement: The Feature-Sliced Design preset reports what Steiger reports

The `fsd` preset MUST declare a rule for each of the 21 rules of the pinned Steiger and its Feature-Sliced Design plugin. It MUST turn on the rules Steiger recommends, and all 21 with `strict: true`. On the same repository it MUST report each rule for exactly the files and folders Steiger reports, except for the intended differences that the next requirement lists, which the differential test names one by one.

#### Scenario: Archangel and Steiger on the same repositories

- **GIVEN** the Feature-Sliced Design fixture repositories, which together break each of the 21 rules
- **WHEN** Steiger with every rule on, and archangel with `fsd({ root, strict: true })`, check each repository
- **THEN** for each Steiger rule, the files that Steiger reports are the files that archangel reports for the matching preset rule
- **TEST** `crates/archangel/tests/fsd_differential.rs` › "the_fsd_preset_reports_what_steiger_reports"

#### Scenario: The default turns on what Steiger recommends

- **GIVEN** the same fixtures
- **WHEN** Steiger with its recommended rules, and archangel with `fsd({ root })`, check each repository
- **THEN** the two report the same files for the same rules, and archangel reports nothing for Steiger's four opt-in rules
- **TEST** `crates/archangel/tests/fsd_differential.rs` › "the_fsd_default_matches_steigers_recommended_rules"

### Requirement: The Next.js preset keeps server-only code out of client components

The `nextjs` preset MUST refuse, transitively through runtime imports, imports from files whose first statement is `'use client'` into the package `server-only` and into Node's built-in modules, and its chains MUST stop at files whose first statement is `'use server'`.

#### Scenario: A client component reaches server-only code

- **GIVEN** a config with `extends: [nextjs()]`, and a client component that imports a helper that imports `server-only`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of `nextjs/client-stays-client` that shows the chain from the component to `server-only`
- **TEST** `crates/archangel/tests/presets.rs` › "nextjs_refuses_server_only_code_reached_from_a_client_component"

#### Scenario: A client component reaches a Node module

- **GIVEN** the same config, and a client component that imports a helper that imports `node:fs`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of `nextjs/client-stays-client` that shows the chain from the component to `node:fs`
- **TEST** `crates/archangel/tests/presets.rs` › "nextjs_refuses_node_modules_reached_from_a_client_component"

#### Scenario: A client component calls a server action

- **GIVEN** the same config, and a client component that imports `app/actions.ts`, which starts with `'use server';` and imports `server-only` and `node:fs`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation, because Next.js turns the import of a server action into a network call
- **TEST** `crates/archangel/tests/presets.rs` › "nextjs_permits_server_actions_in_client_components"

### Requirement: The Feature-Sliced Design preset finds slices by path

The built-in rules of the `fsd` preset MUST take every folder directly in Entities, Features, Widgets or Pages as a slice, with or without segments, and MUST NOT understand slice groups. Steiger finds a slice by the segments inside it, which would make a file's element depend on its siblings and break the one classification an edit costs. The plugin rules see the whole tree and MUST find slices and slice groups as Steiger does.

#### Scenario: A folder without segments in a sliced layer

- **GIVEN** the fixture with `src/features/stuff/helpers.ts`, where `stuff` has no segment
- **WHEN** Steiger and archangel check it
- **THEN** archangel alone reports `fsd/segments-by-purpose` for `src/features/stuff/helpers.ts`, since `stuff` is a slice for it and `helpers` names what the code is, and both report `fsd/no-segmentless-slices` for `src/features/stuff`
- **TEST** `crates/archangel/tests/fsd_differential.rs` › "the_fsd_preset_reports_what_steiger_reports"

### Requirement: Each folder in shared/ui and shared/lib has its own public API

The `fsd` preset's `public-api` MUST require an index file in each folder of `shared/ui` and `shared/lib`, even when the segment has an index of its own; Steiger checks those folders only when the segment has none. A folder rule cannot depend on its parent's files, and Feature-Sliced Design asks each component for its own public API.

#### Scenario: A component folder without an index in a segment with one

- **GIVEN** the fixture with `src/shared/ui/index.ts` and `src/shared/ui/modal/modal.ts`
- **WHEN** Steiger and archangel check it
- **THEN** archangel alone reports `fsd/public-api` for `src/shared/ui/modal`
- **TEST** `crates/archangel/tests/fsd_differential.rs` › "the_fsd_preset_reports_what_steiger_reports"

### Requirement: A misspelt layer is reported beside the real one

The `fsd` preset's `typo-in-layer-name` MUST report a folder in the root whose name is one to three edits from a layer name, even when that layer exists; Steiger then stays silent. The rule's rejected example must be rejected on any repository, and a misspelt folder beside the real layer is still a misspelling.

#### Scenario: A misspelt features folder beside features

- **GIVEN** the fixture with `src/featurez/index.ts` and `src/features`
- **WHEN** Steiger and archangel check it
- **THEN** archangel alone reports `fsd/typo-in-layer-name` for `src/featurez`
- **TEST** `crates/archangel/tests/fsd_differential.rs` › "the_fsd_preset_reports_what_steiger_reports"

### Requirement: The Feature-Sliced Design preset names layers exactly

The `fsd` preset's built-in rules MUST take only a folder named exactly as a layer as that layer, not `1_shared` or `_app`, which would put eleven alternatives into every pattern, and MUST take any file in a slice's `@x` folder as a cross-import public API, because a pattern cannot name the importing file's slice.

#### Scenario: A numbered layer folder

- **GIVEN** `extends: [fsd({ root: 'src' })]`, and `src/1_shared/lib/a.ts` that imports `src/features/login/index.ts`
- **WHEN** archangel makes a check
- **THEN** archangel reports no `fsd/forbidden-imports` for the import, where Steiger, which reads `1_shared` as Shared, reports one
- **TEST** none

### Requirement: The Feature-Sliced Design preset reads imports as archangel does

Every rule of the `fsd` preset MUST count `export … from` as an import, because a re-export loads the module, and MUST read the source files archangel reads. Its `no-public-api-sidestep` MUST check imports from files outside the layers too, such as tests, because a deep import from a test is still a sidestep.

#### Scenario: A test imports a slice's inner file

- **GIVEN** `extends: [fsd({ root: 'src' })]`, and `test/login.test.ts` that imports `src/features/login/model/login.ts`
- **WHEN** archangel makes a check
- **THEN** archangel reports `fsd/no-public-api-sidestep` for the test's import, where Steiger, which reads only the files in the layers, reports nothing
- **TEST** none
