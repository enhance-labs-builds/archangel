# Spec Delta

## Purpose

Rules about where files live and what files and folders are called: names, places, files that every
instance must have and files that must not exist, so that the folder structure is checked with the
same speed and proof as the imports.

## ADDED Requirements

### Requirement: Structure rules see every file the walk visits

The structure rules MUST cover every file and folder that the walk visits under the repository's ignore rules, whether or not archangel reads it as a source file.

#### Scenario: A folder of images

- **GIVEN** a rule `{ folders: 'src/**', mustBeNamed: 'kebab-case' }`, and the folder `src/userAssets` that holds only `.png` files
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation for the folder `src/userAssets`
- **TEST** `crates/archangel_rules/tests/structure.rs` › "structure_rules_see_folders_without_source_files"

#### Scenario: A folders pattern that matches no folder

- **GIVEN** a rule `{ name: 'readme', folders: 'modules/*', mustContain: 'README.md' }`, and no folder `modules`
- **WHEN** archangel makes a check
- **THEN** archangel gives a `selector-empty` diagnostic that names the rule "readme" and the pattern
- **TEST** `crates/archangel_rules/tests/structure.rs` › "a_folders_pattern_that_matches_no_folder_is_selector_empty"

### Requirement: A file has a name and a full name

A file's name MUST be its name up to the first dot, and its full name its name without the last extension: `button.view.tsx` has the name `button` and the full name `button.view`.

#### Scenario: Names of a file with several dots

- **GIVEN** the files `src/button.view.tsx` and `src/auth.test.ts`
- **WHEN** archangel reads their names
- **THEN** the names are `button` and `auth`, and the full names are `button.view` and `auth.test`
- **TEST** `crates/archangel_rules/tests/structure.rs` › "a_file_has_a_name_and_a_full_name"

### Requirement: A mustBeNamed rule checks names against a style or a pattern

A `mustBeNamed` rule MUST accept a style (`kebab-case`, `camelCase`, `PascalCase`, `snake_case`, `SCREAMING_SNAKE_CASE`), checked against a file's name, or a name pattern, matched against its full name. With `folders`, it MUST check each folder below the pattern's fixed part once, however many files the folder holds.

#### Scenario: A folder that is not kebab-case

- **GIVEN** a rule `{ folders: 'src/**', mustBeNamed: 'kebab-case' }`, and the files `src/userProfile/a.ts` and `src/userProfile/b.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for the folder `src/userProfile`, that suggests `user-profile`, and none for `src`
- **TEST** `crates/archangel_rules/tests/structure.rs` › "a_folder_name_is_checked_once_against_its_style"

#### Scenario: A component file in PascalCase

- **GIVEN** a rule `{ files: 'src/components/**/*.tsx', mustBeNamed: 'PascalCase' }`, and the files `src/components/Button.tsx`, `src/components/Card.stories.tsx` and `src/components/icon-button.tsx`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for `icon-button.tsx`, that suggests `IconButton.tsx`
- **TEST** `crates/archangel_rules/tests/structure.rs` › "a_style_checks_the_name_up_to_the_first_dot"

#### Scenario: Styles agree with a reference conversion

- **GIVEN** random names, with acronyms and digits such as `HTTPServer` and `v2Api` among them
- **WHEN** archangel decides whether each name is in each style and converts it to each style
- **THEN** the results agree with a reference implementation of case conversion
- **TEST** `crates/archangel_rules/tests/structure_properties.rs` › "name_styles_agree_with_the_reference_conversion"

### Requirement: A name pattern can use the file's and folder's own names

A name pattern MUST accept `{FileName}`, `{fileName}`, `{file-name}`, `{file_name}` and `{FILE_NAME}`, the file's name in each style, and the same five spellings of `{FolderName}` for the name of the folder that holds it.

#### Scenario: A test file named for its folder

- **GIVEN** a rule `{ files: 'src/features/*/*.test.ts', mustBeNamed: '{folder-name}.test' }`, and the files `src/features/login/login.test.ts` and `src/features/login/auth.test.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for `auth.test.ts`, that says the full name must be `login.test`
- **TEST** `crates/archangel_rules/tests/structure.rs` › "a_name_pattern_uses_the_folders_name"

### Requirement: A mustContain rule requires a file in each instance or folder

A `mustContain` rule MUST name a pattern, or a list of patterns any one of which will do, relative to the instance root, when `files` is an element with a capture, or to each folder, for a folder rule, and MUST give one violation for each instance or folder that has no file matching it. Any other `files` MUST be refused with `rule-needs-capture`.

#### Scenario: A slice without its public API

- **GIVEN** an element "slice" with the pattern `src/features/{slice}/**`, and a rule `{ files: 'slice', mustContain: 'index.ts' }`
- **AND** the files `src/features/login/index.ts`, `src/features/login/a.ts` and `src/features/billing/a.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for the instance "billing", that names `src/features/billing/index.ts` as the file it needs
- **TEST** `crates/archangel_rules/tests/structure.rs` › "must_contain_rejects_an_instance_without_the_file"

#### Scenario: Each folder needs a README

- **GIVEN** a rule `{ folders: 'packages/*', mustContain: 'README.md' }`, and the folders `packages/a`, which has `README.md`, and `packages/b`, which does not
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for `packages/b`
- **TEST** `crates/archangel_rules/tests/structure.rs` › "must_contain_rejects_a_folder_without_the_file"

#### Scenario: A file deleted in the editor

- **GIVEN** the same rule, and no violation
- **WHEN** the developer deletes `src/features/login/index.ts`
- **THEN** the language server reports a violation for the instance "login"
- **AND** the result is the same as a full check of the changed repository
- **TEST** `crates/archangel_engine/tests/sessions.rs` › "deleting_a_required_file_reports_its_instance"

#### Scenario: Any of several patterns

- **GIVEN** a rule `{ folders: 'src/features/*', mustContain: ['{ui,model}.*', '{ui,model}/**'] }`, and the files `src/features/a/model.ts`, `src/features/b/ui/button.ts` and `src/features/c/helpers.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for the folder `src/features/c`
- **TEST** `crates/archangel_rules/tests/structure.rs` › "must_contain_accepts_any_of_several_patterns"

#### Scenario: A folder pattern that leaves folders out

- **GIVEN** a rule `{ folders: ['src/shared/*', '!src/shared/{ui,lib}'], mustContain: 'index.*' }`, and the folders `src/shared/api`, `src/shared/ui` and `src/shared/lib` without an index file, and `src/shared/config` with one
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for `src/shared/api`, because a folder pattern that starts with `!` leaves the folders it matches out
- **TEST** `crates/archangel_rules/tests/structure.rs` › "a_folder_pattern_with_an_exclamation_mark_leaves_folders_out"

### Requirement: A mustNotExist rule refuses every file in its selector

A `mustNotExist` rule MUST give one violation for each file in its selector. With `{ not: { element: '*' } }` it MUST report every file in no element. A folder rule with `mustNotExist` MUST give one violation for each folder it covers.

#### Scenario: A file in no element

- **GIVEN** elements for `src/app/**` and `src/shared/**`, and a rule `{ files: { all: ['src/**', { not: { element: '*' } }] }, mustNotExist: true }`
- **AND** the file `src/stray.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation for `src/stray.ts` that lists every declared element
- **TEST** `crates/archangel_rules/tests/structure.rs` › "must_not_exist_reports_a_file_in_no_element"

#### Scenario: A reserved folder name

- **GIVEN** a rule `{ files: 'src/*/*/{utils,helpers}/**', mustNotExist: true, reason: 'Group segments by purpose' }`, and the file `src/features/login/utils/a.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation that shows the reason
- **TEST** `crates/archangel_rules/tests/structure.rs` › "must_not_exist_reports_a_forbidden_folder"

#### Scenario: A folder that must not exist

- **GIVEN** a rule `{ folders: 'src/processes', mustNotExist: true }`, and the files `src/processes/checkout/a.ts` and `src/processes/checkout/b.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for the folder `src/processes`, not one for each file
- **TEST** `crates/archangel_rules/tests/structure.rs` › "a_folder_that_must_not_exist_is_one_violation"

### Requirement: Structure rules cost one look at each file, folder and instance

A full check MUST evaluate each structure rule once for each file, folder or instance it covers. An edit to a file's text MUST NOT evaluate a structure rule unless the file's class changed, and then only for that file. Creating, deleting or renaming files MUST evaluate the structure rules only for the folders and instances that the change touched.

#### Scenario: An edit to a file's text

- **GIVEN** a session with naming and mustContain rules, none on a directive selector
- **WHEN** the developer edits the text of one file
- **THEN** the count of structure evaluations is zero
- **TEST** `crates/archangel_engine/tests/work.rs` › "a_text_edit_evaluates_no_structure_rule"

#### Scenario: An edit that changes a file's class

- **GIVEN** a session with a naming rule and a `mayNotExport` rule on the selector "client", defined as `{ directive: 'use client' }`, and `app/b.tsx`, which has a default export and no directive
- **WHEN** the developer adds `'use client';` to the top of `app/b.tsx`
- **THEN** the language server reports both violations for `app/b.tsx`
- **AND** the result is the same as a full check, and the count of structure evaluations is one
- **TEST** `crates/archangel_engine/tests/sessions.rs` › "a_class_change_re_checks_the_files_structure_and_export_rules"

#### Scenario: A new file in one folder

- **GIVEN** a session with a folder naming rule on a repository of 500 folders
- **WHEN** the developer creates one file in an existing folder
- **THEN** the count of folder evaluations is one
- **TEST** `crates/archangel_engine/tests/work.rs` › "a_new_file_evaluates_only_its_folder"

#### Scenario: A new folder with a bad name

- **GIVEN** a session with the rule `{ folders: 'src/**', mustBeNamed: 'kebab-case' }`
- **WHEN** the developer renames `src/a/x.ts` to `src/newThing/x.ts`
- **THEN** the language server reports a violation for `src/newThing`
- **AND** the result is the same as a full check of the changed repository
- **TEST** `crates/archangel_engine/tests/sessions.rs` › "a_rename_into_a_badly_named_folder_is_reported"

#### Scenario: The fault control for structure evaluations

- **GIVEN** a build of archangel with a fault that evaluates every folder after each file event
- **WHEN** the new-file work test runs against that build
- **THEN** the test fails on the count of folder evaluations
- **TEST** `crates/archangel_engine/tests/work.rs` › "folder_evaluation_count_rejects_every_folder_per_event"
