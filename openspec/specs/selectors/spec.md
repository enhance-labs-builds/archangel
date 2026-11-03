# selectors Specification

## Purpose
A selector names a set of files that any number of other selectors may overlap, such as
"test files" or "client components", so that a rule can say which files it is about without a
generated element for each combination.

## Requirements

### Requirement: A file can be in many selectors

A selector MUST be declared under `selectors` by a pattern, a list of patterns, or a combination with `all`, `any` and `not` of patterns, element names, other selectors and the forms below. A file MUST be in every selector that it matches, whatever other selectors and elements it is in.

#### Scenario: A test file in a feature

- **GIVEN** an element "feature" with the pattern `src/features/{feature}/**`, and a selector "test" with the pattern `**/*.test.ts`
- **AND** the file `src/features/login/a.test.ts`
- **WHEN** archangel classifies the files
- **THEN** the file is in the element "feature" and in the selector "test"
- **TEST** `crates/archangel_rules/tests/selectors.rs` › "a_file_is_in_its_element_and_every_matching_selector"

#### Scenario: A selector made of others

- **GIVEN** a selector "production" defined as `{ all: ['src/**', { not: 'test' }] }`
- **AND** the files `src/a.ts` and `src/a.test.ts`
- **WHEN** archangel classifies the files
- **THEN** `src/a.ts` is in "production" and `src/a.test.ts` is not
- **TEST** `crates/archangel_rules/tests/selectors.rs` › "all_any_and_not_combine_selectors"

#### Scenario: Selectors agree with a reference matcher

- **GIVEN** random selector definitions and random paths
- **WHEN** archangel classifies the paths
- **THEN** each path is in the same selectors as a slow reference matcher puts it in
- **TEST** `crates/archangel_rules/tests/selector_properties.rs` › "selector_membership_agrees_with_the_reference_matcher"

### Requirement: A selector can name an element, an instance, or any element

A selector MUST accept `{ element: <name> }`, `{ element: <name>, capture: { <capture>: <value> } }` for one instance, and `{ element: '*' }` for a file in any element.

#### Scenario: Files in no element

- **GIVEN** an element "app" with the pattern `src/app/**`, and a selector `{ not: { element: '*' } }`
- **AND** the files `src/app/a.ts` and `src/stray.ts`
- **WHEN** archangel classifies the files
- **THEN** only `src/stray.ts` is in the selector
- **TEST** `crates/archangel_rules/tests/selectors.rs` › "not_any_element_selects_unclassified_files"

#### Scenario: One instance

- **GIVEN** an element "feature" with the pattern `src/features/{feature}/**`, and a selector `{ element: 'feature', capture: { feature: 'login' } }`
- **AND** the files `src/features/login/a.ts` and `src/features/billing/b.ts`
- **WHEN** archangel classifies the files
- **THEN** only `src/features/login/a.ts` is in the selector
- **TEST** `crates/archangel_rules/tests/selectors.rs` › "an_instance_selector_selects_one_capture_value"

### Requirement: A selector can select files by their directive

A selector MUST accept `{ directive: <text> }`, which selects the files whose first statement is that directive, such as `'use client'`.

#### Scenario: A client component

- **GIVEN** a selector "client" defined as `{ directive: 'use client' }`
- **AND** the file `app/a.tsx` that starts with `'use client';` and the file `app/b.tsx` that does not
- **WHEN** archangel classifies the files
- **THEN** only `app/a.tsx` is in "client"
- **TEST** `crates/archangel_rules/tests/selectors.rs` › "a_directive_selector_selects_files_by_their_first_statement"

#### Scenario: An edit adds the directive

- **GIVEN** the selector "client", a rule that "client" may not import the package `server-only`, and the file `app/b.tsx` that imports `server-only` without the directive
- **WHEN** the developer adds `'use client';` to the top of `app/b.tsx` in the editor
- **THEN** the language server reports a violation for the import of `server-only`
- **AND** the result is the same as a full check of the edited repository
- **TEST** `crates/archangel_engine/tests/sessions.rs` › "a_directive_edit_changes_the_files_class"

#### Scenario: A class change re-checks the file's importers

- **GIVEN** a rule `{ files: 'src/server/**', mayNotImport: 'client' }`, and `src/server/c.ts` that imports `app/b.tsx`, which has no directive
- **WHEN** the developer adds `'use client';` to the top of `app/b.tsx`
- **THEN** the language server reports a violation for the import in `src/server/c.ts`
- **AND** the result is the same as a full check, and the rule evaluations are those of the imports of `app/b.tsx` and the imports into it
- **TEST** `crates/archangel_engine/tests/sessions.rs` › "a_class_change_re_checks_the_files_importers"

#### Scenario: The directive must come first

- **GIVEN** the selector "client", the file `app/c.tsx` with `"use client";` in double quotes as its first statement, and the file `app/d.tsx` with `import x from './x';` before `'use client';`
- **WHEN** archangel classifies the files
- **THEN** `app/c.tsx` is in "client" and `app/d.tsx` is not
- **TEST** `crates/archangel_rules/tests/selectors.rs` › "a_directive_counts_only_as_the_first_statement"

#### Scenario: A file that does not parse

- **GIVEN** the selector "client", and a file that starts with `'use client';` and has a syntax error further down
- **WHEN** archangel makes a check
- **THEN** archangel gives `parse-failed` for the file, and the file is in no directive selector
- **TEST** `crates/archangel_rules/tests/selectors.rs` › "a_file_that_does_not_parse_has_no_directive"

### Requirement: A rule side names packages through package elements

A selector MUST select files only. A rule side MUST name packages with a package element or an inline `{ packages: [...] }`, whose names may be a package, a scope ending in `/*`, `node:*` for Node's built-in modules, or `*` for every package.

#### Scenario: A Node built-in module

- **GIVEN** a rule `{ files: 'src/web/**', mayNotImport: { packages: ['node:*'] } }`
- **AND** the file `src/web/a.ts` that imports `fs` and the file `src/web/b.ts` that imports `node:path`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation for each import
- **TEST** `crates/archangel_rules/tests/selectors.rs` › "node_star_selects_built_in_modules_with_and_without_the_prefix"

#### Scenario: Every package

- **GIVEN** a rule `{ files: 'src/domain/**', mayNotImport: { packages: ['*'] } }`, and a domain file that imports `zod`, `@acme/ui` and `./b`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation for each of the two packages, and none for `./b`
- **TEST** `crates/archangel_rules/tests/selectors.rs` › "a_star_package_name_selects_every_package"

#### Scenario: A package written as a selector

- **GIVEN** a selector "react" defined as `{ packages: ['react'] }`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at the selector, saying that packages belong in a package element
- **TEST** `crates/archangel_config/tests/validate.rs` › "a_selector_of_packages_is_refused"

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

### Requirement: Classification costs one pass per file, and an edit classifies one file

Archangel MUST put every file into its element and selectors with one pass of the patterns over its path, and the cost of checking an import MUST NOT grow with the number of rules that cannot apply to the two files' classes. An edit to one file's text MUST classify that file only.

#### Scenario: Ten selectors and a thousand

- **GIVEN** one repository and two configs: one with 10 selectors, and one with the same 10 and 990 more that each match one file and that no rule uses
- **WHEN** archangel makes a check with each config
- **THEN** the two checks have the same count of pattern passes and the same count of rule evaluations per import
- **TEST** `crates/archangel_rules/tests/work.rs` › "classification_does_not_grow_with_selectors"

#### Scenario: The fault control for classification

- **GIVEN** a build of archangel with a fault that matches each selector's patterns in a separate pass
- **WHEN** the classification work test runs against that build
- **THEN** the test fails on the count of pattern passes
- **TEST** `crates/archangel_rules/tests/work.rs` › "classification_count_rejects_a_pass_per_selector"

#### Scenario: An edit classifies one file

- **GIVEN** a session on a repository of 1,000 files
- **WHEN** the developer edits the text of one file
- **THEN** the count of files classified is one
- **TEST** `crates/archangel_engine/tests/work.rs` › "an_edit_classifies_one_file"

#### Scenario: The fault control for classification per edit

- **GIVEN** a build of archangel with a fault that classifies every file after each edit
- **WHEN** the edit classification test runs against that build
- **THEN** the test fails on the count of files classified
- **TEST** `crates/archangel_engine/tests/work.rs` › "classification_count_rejects_every_file_per_edit"

### Requirement: Explain lists a file's selectors

`archangel explain` MUST list every selector that the file is in, beside its element, and MUST show each rule that applies to the file as its sentence.

#### Scenario: Explain a test file

- **GIVEN** the file `src/features/login/a.test.ts` in the element "feature" and the selector "test", and an isolate rule on "feature"
- **WHEN** the developer runs `archangel explain src/features/login/a.test.ts`
- **THEN** the output names the element "feature" with its capture "login", the selector "test", and the isolate rule written as "files in feature may not import feature in a different instance"
- **TEST** `crates/archangel/tests/cli.rs` › "explain_lists_selectors_and_rule_sentences"

### Requirement: A string on a rule side is a name or a path pattern

A string on a rule side MUST first be looked up as the name of a declared element or selector, a preset's prefixed names included. A string that names nothing MUST be a path pattern when it contains `/` or `*`, and MUST otherwise be refused with `rule-unknown-element` and the declared names closest to it.

#### Scenario: A package written as a bare string

- **GIVEN** no element or selector named "lodash", and a rule `{ files: 'src/**', mayNotImport: 'lodash' }`
- **WHEN** archangel loads the config
- **THEN** archangel gives `rule-unknown-element` at the rule, saying `lodash` is not a declared name and that a package is written `{ packages: ['lodash'] }`
- **TEST** `crates/archangel_config/tests/validate.rs` › "a_bare_string_that_names_nothing_is_refused"

#### Scenario: An inline pattern

- **GIVEN** a rule `{ files: 'src/legacy/**', mayNotImport: 'src/next/**' }` with no selector declared for either
- **AND** the file `src/legacy/a.ts` that imports `src/next/b.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of that rule
- **TEST** `crates/archangel_rules/tests/selectors.rs` › "a_string_with_a_slash_is_an_inline_pattern"

#### Scenario: A preset's name with a slash

- **GIVEN** a config with `extends: [fsd({ root: 'src' })]` and a rule `{ files: 'fsd/shared', mayNotImport: 'src/app/**' }`
- **WHEN** archangel loads the config
- **THEN** `fsd/shared` is the preset's element, not a pattern
- **TEST** `crates/archangel_config/tests/merge.rs` › "a_declared_name_wins_over_a_pattern"
