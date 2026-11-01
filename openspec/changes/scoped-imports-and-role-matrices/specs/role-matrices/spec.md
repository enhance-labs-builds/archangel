# role-matrices

## Purpose

A matrix is a table of import permissions. Its rows are kinds of files, found by their names, or
parts of the code that the config declares. This capability lets a team write the table once, read
it as a table, and have archangel check each row as a rule with a stable name.

## ADDED Requirements

### Requirement: A matrix states what the files of each row may import within an area

A matrix MUST have a name and a table of rows. It MAY have an area and a role pattern. With a role pattern, each row MUST be a role, and the files of a row MUST be the files of its role that are in the matrix's files. The matrix's files MUST be the area's files by default. A row MUST give a violation for an import from its files into the area when the cells of the row do not include the other end. A row MUST NOT judge an import whose other end is outside the area.

#### Scenario: An entity imports a use case

- **GIVEN** a matrix `api-domain` with the files `apps/api/src/*/domain/**`, the area `apps/api/src/**`, the role pattern `**/*.{role}.ts`, and the row `entity: ['entity', 'value-object']`
- **AND** the file `apps/api/src/canvas/domain/canvas.entity.ts` that imports `apps/api/src/canvas/application/save.use-case.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of `api-domain.entity` for that import
- **AND** the message names `entity` and `value-object` as what the file may import, and the area
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "a_row_refuses_an_import_into_the_area_that_it_does_not_list"

#### Scenario: An entity imports a value object

- **GIVEN** the same matrix
- **AND** the file `apps/api/src/canvas/domain/canvas.entity.ts` that imports `apps/api/src/canvas/domain/name.value-object.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the matrix
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "a_row_permits_an_import_of_a_cell_that_it_lists"

#### Scenario: An entity imports a package and a file of another app

- **GIVEN** the same matrix
- **AND** the file `apps/api/src/canvas/domain/canvas.entity.ts` that imports `zod` and `apps/web/src/shared/lib/format.lib.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the matrix, because both ends are outside the area
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "a_row_does_not_judge_an_import_outside_the_area"

#### Scenario: A file in the area that has no role

- **GIVEN** the same matrix
- **AND** the file `apps/api/src/canvas/domain/canvas.entity.ts` that imports `apps/api/src/canvas/domain/index.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of `api-domain.entity`, because the area includes files that have no role
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "a_row_refuses_a_file_in_the_area_that_has_no_role"

#### Scenario: A role cell outside the matrix's files

- **GIVEN** the same matrix
- **AND** the file `apps/api/src/canvas/domain/canvas.entity.ts` that imports `apps/api/src/canvas/application/draft.entity.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the matrix, because the cell `entity` means every entity file in the area
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "a_role_cell_means_every_file_of_the_role_in_the_area"

#### Scenario: A matrix without files takes the files of its area

- **GIVEN** a matrix with the area `apps/api/src/**`, the role pattern `**/*.{role}.ts`, the row `entity: ['value-object']` and no `files`
- **AND** the file `apps/api/src/canvas/application/draft.entity.ts`, which imports `apps/api/src/canvas/application/save.use-case.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of the row `entity` for that import
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "a_matrix_without_files_takes_the_files_of_its_area"

### Requirement: A matrix without an area judges every import

A matrix without an area MUST judge every import from the files of its rows, packages included, as a `mayOnlyImport` rule without an area does. Its files MUST be every file by default.

#### Scenario: A matrix without an area

- **GIVEN** a matrix `api-domain` with the role pattern `**/*.{role}.ts`, the row `entity: ['value-object']`, and no `within` and no `files`
- **AND** the file `apps/api/src/canvas/domain/canvas.entity.ts` that imports `apps/api/src/canvas/domain/name.value-object.ts` and `zod`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of `api-domain.entity`, for the import of `zod`, which its cells do not list
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "a_matrix_without_an_area_judges_every_import"

### Requirement: A row permits exactly the cells that it lists

A row MUST permit an import of its own files only when the row lists itself. An empty row MUST permit no import into the area.

#### Scenario: A domain event imports another domain event

- **GIVEN** a matrix with the row `'domain-event': ['value-object']`
- **AND** the file `apps/api/src/canvas/domain/created.domain-event.ts` that imports `apps/api/src/canvas/domain/renamed.domain-event.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of the row `domain-event` for that import
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "a_row_that_does_not_list_its_own_role_refuses_it"

#### Scenario: An entity imports another entity

- **GIVEN** a matrix with the row `entity: ['entity', 'value-object']`
- **AND** the file `apps/api/src/canvas/domain/canvas.entity.ts` that imports `apps/api/src/canvas/domain/board.entity.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the row `entity`
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "a_row_that_lists_its_own_role_permits_it"

#### Scenario: An empty row

- **GIVEN** a matrix with the row `consts: []`
- **AND** a file of the role `consts` that imports a file of the role `types` in the area
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of the row `consts`
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "an_empty_row_refuses_every_import_into_the_area"

### Requirement: A role pattern finds the files of each role by their name

A matrix MUST accept one role pattern that contains the capture `{role}` one or more times. A file MUST be in a role when the role pattern matches the file with the name of that role, as literal text, in place of every `{role}`. The rows of the matrix MUST give the names of its roles. Another capture in a role pattern MUST match as `*` does, but MUST NOT match an empty part. The matrix MUST NOT make an instance from it.

#### Scenario: Roles found by the end of the file name

- **GIVEN** a matrix with the role pattern `**/*.{role}.ts` and the rows `entity` and `value-object`
- **WHEN** archangel makes a check
- **THEN** `apps/api/src/canvas/domain/canvas.entity.ts` is in the role `entity`, and `apps/api/src/canvas/domain/canvas.ts` is in no role
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "a_role_pattern_puts_a_file_in_the_role_that_its_name_ends_with"

#### Scenario: The capture in two places

- **GIVEN** a matrix with the role pattern `**/{role}s/*.{role}.ts` and the rows `entity` and `policy`
- **WHEN** archangel makes a check
- **THEN** `apps/api/src/entitys/canvas.entity.ts` is in the role `entity`, and `apps/api/src/entitys/canvas.policy.ts` is in no role
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "a_role_capture_in_two_places_takes_the_same_name"

#### Scenario: A role pattern with another capture

- **GIVEN** a matrix with the role pattern `apps/{app}/**/*.{role}.ts` and the row `entity`
- **WHEN** archangel makes a check
- **THEN** archangel gives no diagnostic about the capture `{app}`
- **AND** `apps/api/src/canvas.entity.ts` is in the role `entity`, and `apps/canvas.entity.ts` is in no role
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "a_role_pattern_with_another_capture_matches_it_as_a_wildcard"

### Requirement: A file that two rows have is judged by each row

A file that is in the files of two rows MUST be judged by each of the two rows. Each row MUST give its own violation for an import that it does not permit.

#### Scenario: A file that two roles match

- **GIVEN** a matrix with the area `apps/api/**`, the role pattern `**/{role}/**`, and the rows `domain: []` and `application: ['domain']`
- **AND** the file `apps/api/domain/application/x.ts`, which imports `apps/api/infra/db.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of the row `domain` and one violation of the row `application` for that import
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "a_file_that_two_roles_match_is_judged_by_both_rows"

### Requirement: A role pattern without the capture is refused

A role pattern that does not contain the capture `{role}` MUST be refused with `config-invalid`.

#### Scenario: A role pattern without the capture

- **GIVEN** a matrix with the role pattern `**/*.entity.ts`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at the role pattern, saying that it must contain `{role}`
- **TEST** `crates/archangel_config/tests/matrices.rs` › "a_role_pattern_without_the_role_capture_is_refused"

### Requirement: A matrix without a role pattern has declared names as rows

In a matrix without a role pattern, each row MUST be the name of an element or a selector that the config declares. The files of the row MUST be the files of that element or selector that are in the matrix's files. A row name that names no declared element or selector MUST give `rule-unknown-element` with the closest declared name. A row name that names a package element MUST be refused with `config-invalid`, because packages have no files that import.

#### Scenario: A package imports a package that its row does not list

- **GIVEN** the elements `canvas-format` as `packages/canvas-format/src/**` and `contract` as `packages/contract/src/**`
- **AND** a matrix `packages` with no role pattern, no area and the row `'canvas-format': ['canvas-format']`
- **AND** the file `packages/canvas-format/src/version.lib.ts`, which imports `packages/contract/src/canvas.schema.ts` and `zod`
- **WHEN** archangel makes a check
- **THEN** archangel gives two violations of `packages.canvas-format`, one for each import
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "an_element_row_refuses_an_import_that_it_does_not_list"

#### Scenario: A package imports a package that its row lists

- **GIVEN** the same elements, and a matrix `packages` with the row `'canvas-document': ['canvas-document', 'contract', { packages: ['*'] }]` and the element `canvas-document`
- **AND** a file in `canvas-document` that imports a file in `contract`, a file in `canvas-document` and `zod`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the matrix
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "an_element_row_permits_an_import_of_a_cell_that_it_lists"

#### Scenario: A misspelt row name in a matrix without a role pattern

- **GIVEN** the element `canvas-document` and a matrix with no role pattern and the row `canvas-documnet`
- **WHEN** archangel loads the config
- **THEN** archangel gives `rule-unknown-element` at the row, and names `canvas-document` as the closest name
- **TEST** `crates/archangel_config/tests/matrices.rs` › "a_row_key_that_names_no_declared_name_is_refused_with_the_closest_name"

#### Scenario: A row that names a package element

- **GIVEN** the element `react` as `{ packages: ['react'] }` and a matrix with no role pattern and the row `react`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at the row, saying that a package element has no files that import
- **TEST** `crates/archangel_config/tests/matrices.rs` › "a_row_key_that_names_packages_is_refused"

### Requirement: A matrix can refuse a file that no row has

A matrix MAY have the option `everyFileHasARow`, which is off by default. With the option, each file in the matrix's files that is in no row MUST give one violation of the rule `<matrix>.every-file-has-a-row`. The message MUST name the file and the matrix, and the role pattern and roles, or the rows. A file that the matrix's files leave out MUST NOT give this violation.

#### Scenario: A domain file with no role

- **GIVEN** a matrix `api-domain` with the files `apps/api/src/*/domain/**`, the role pattern `**/*.{role}.ts`, the rows `entity` and `value-object`, and `everyFileHasARow: true`
- **AND** the file `apps/api/src/canvas/domain/canvas.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of `api-domain.every-file-has-a-row` for that file
- **AND** the message names `api-domain`, the role pattern `**/*.{role}.ts`, and the roles `entity` and `value-object`
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "a_file_with_no_row_is_refused_when_every_file_must_have_a_row"

#### Scenario: A file in no row of a matrix without a role pattern

- **GIVEN** the selectors `api-domain` and `api-shared`, and a matrix `api-layers` with the files `apps/api/src/**`, those two rows and `everyFileHasARow: true`
- **AND** the file `apps/api/src/stray.ts`, which is in neither selector
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of `api-layers.every-file-has-a-row` for that file
- **AND** the message names `api-domain` and `api-shared` as the rows
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "a_file_in_no_row_of_an_element_matrix_is_refused"

#### Scenario: Files that the matrix leaves out

- **GIVEN** the selector `role-exempt` as `['**/index.ts', '**/*.test.ts', '**/*.d.ts']`
- **AND** a matrix with the files `{ all: ['apps/api/src/*/domain/**', { not: 'role-exempt' }] }`, the role pattern `**/*.{role}.ts`, the row `entity` and `everyFileHasARow: true`
- **AND** the files `index.ts`, `canvas.entity.test.ts` and `env.d.ts` in `apps/api/src/canvas/domain`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the matrix for those files
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "files_left_out_of_a_matrix_need_no_row"

### Requirement: The summary counts the files with no row

For each matrix without `everyFileHasARow` that has files in no row, the text summary MUST give their count. The JSON summary MUST give the same counts under `filesWithNoRow`, by matrix name.

#### Scenario: The option is off

- **GIVEN** a matrix `api-domain` without `everyFileHasARow`, and three files in its files that are in no row
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation for those files
- **AND** the text summary has the line `api-domain has 3 files that no row has; everyFileHasARow: true refuses them`
- **AND** the JSON summary gives `{ "api-domain": 3 }` under `filesWithNoRow`
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "files_with_no_row_are_counted_in_the_summary"

### Requirement: A cell names a role, a declared name, a pattern or packages

A cell MUST accept a role of its matrix, the name of a declared element or selector, a path pattern, packages, and the other values of a rule side. A cell that names a role MUST mean every file of that role in the area. Archangel MUST look up a cell name first among the roles of the matrix, and then among the declared names, so a role shadows a declared name of the same name inside its own matrix.

#### Scenario: A cell names an element

- **GIVEN** the element `api-domain` and a matrix row `command: ['api-domain', 'dto']`, where `dto` is a role of the matrix
- **AND** a file of the role `command` that imports a file in `api-domain`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the row `command`
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "a_cell_can_name_a_declared_element"

#### Scenario: A cell of packages

- **GIVEN** a matrix with the area `['apps/api/src/**', { packages: ['*'] }]` and the row `entity: ['value-object', { packages: ['zod'] }]`
- **AND** a file of the role `entity` that imports `zod` and `lodash`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of the row `entity`, for the import of `lodash`
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "a_cell_can_name_packages"

#### Scenario: A role with the name of a selector

- **GIVEN** a declared selector `types` defined as `apps/api/src/shared/types/**`, and a matrix `api-domain` with the area `apps/api/src/**`, the role pattern `**/*.{role}.ts` and the rows `entity: ['types']` and `types: []`
- **AND** a file of the role `entity` that imports `apps/api/src/shared/types/id.ts`, and another that imports `apps/api/src/canvas/domain/id.types.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of the row `entity`, for the import of `apps/api/src/shared/types/id.ts`, because the cell `types` means the role
- **AND** archangel gives no diagnostic about the name `types`
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "a_role_shadows_a_declared_name_inside_its_matrix"

#### Scenario: A misspelt cell

- **GIVEN** a matrix with the roles `entity` and `value-object`, and the row `entity: ['value-objekt']`
- **WHEN** archangel loads the config
- **THEN** archangel gives `rule-unknown-element` at the cell, and names `value-object` as the closest name
- **TEST** `crates/archangel_config/tests/matrices.rs` › "a_misspelt_cell_is_refused_with_the_closest_role"

### Requirement: Each row is a rule named after its matrix and row

Each row MUST be one rule, named with the name of the matrix, a dot and the name of the row, such as `api-domain.entity`. A matrix that a preset declares MUST have the preset's prefix, so its rows are named such as `fsd-x/roles.entity`. A config rule with the prefixed name of a row MUST replace that row, as for any preset rule. Two matrices with one name MUST give `rule-name-duplicate`.

#### Scenario: A row name that another rule has

- **GIVEN** a matrix `api-domain` with the row `entity`, and another rule named `api-domain.entity`
- **WHEN** archangel loads the config
- **THEN** archangel gives `rule-name-duplicate` for `api-domain.entity`
- **TEST** `crates/archangel_config/tests/matrices.rs` › "a_row_name_that_another_rule_has_is_a_duplicate"

#### Scenario: Two matrices with one name

- **GIVEN** two matrices named `api-domain`, one with the row `entity` and one with the row `policy`
- **WHEN** archangel loads the config
- **THEN** archangel gives `rule-name-duplicate` that names the matrix `api-domain` and says to give each matrix its own name
- **TEST** `crates/archangel_config/tests/matrices.rs` › "two_matrices_with_one_name_are_refused"

#### Scenario: A row of a matrix in a preset

- **GIVEN** a config that extends a preset `fsd-x`, which declares a matrix `roles` with the row `entity`
- **AND** an import that breaks that row
- **WHEN** archangel makes a check
- **THEN** the violation names the rule `fsd-x/roles.entity`
- **AND** a config rule named `fsd-x/roles.entity` replaces that row
- **TEST** `crates/archangel/tests/presets.rs` › "a_presets_matrix_row_is_named_with_the_preset_prefix"

### Requirement: Off, exceptions and the baseline refer to a row by its name

`off`, exception comments and the baseline MUST refer to a row by its name. A matrix name in `off` MUST be refused with `config-invalid` and the closest row names, as any name in `off` that matches no rule is.

#### Scenario: Turn off one row

- **GIVEN** a matrix `api-domain` with the rows `entity` and `policy`, and `off: ['api-domain.policy']`
- **AND** an import that breaks the row `policy`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation for that import, and the row `entity` still applies
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "off_drops_one_row_of_a_matrix"

#### Scenario: Turn off a whole matrix by its name

- **GIVEN** a matrix `api-domain` with the rows `entity` and `policy`, and `off: ['api-domain']`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at `off`, and names `api-domain.entity` and `api-domain.policy` as the closest names
- **TEST** `crates/archangel_config/tests/matrices.rs` › "a_matrix_name_in_off_is_refused_with_its_row_names"

#### Scenario: An exception for one row

- **GIVEN** an import that breaks the row `api-domain.entity`, under the comment `// archangel-allow api-domain.entity: migration in progress`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation for that import and counts one exception for `api-domain.entity`
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "an_exception_names_a_row_of_a_matrix"

#### Scenario: A baseline entry for one row

- **GIVEN** a violation of the row `api-domain.entity`
- **WHEN** the developer runs `archangel baseline add`
- **THEN** the baseline entry records the rule name `api-domain.entity`
- **TEST** `crates/archangel/tests/baseline_command.rs` › "a_baseline_entry_records_the_name_of_a_matrix_row"

### Requirement: A matrix or row name with a dot, colon, slash or white space is refused

The name of a matrix and the name of a row MUST NOT contain a dot, a colon, a slash or white space. Archangel MUST refuse such a name with `config-invalid` at the name.

#### Scenario: Matrix names with a refused character

- **GIVEN** matrices named `api.domain`, `api:domain`, `api/domain` and `api domain`, each in its own config
- **WHEN** archangel loads each config
- **THEN** each gives `config-invalid` at the name, which names the refused character
- **AND** the message for `api:domain` says that an exception comment ends the rule name at the first colon
- **TEST** `crates/archangel_config/tests/matrices.rs` › "a_matrix_name_with_a_dot_colon_slash_or_space_is_refused"

#### Scenario: A role name with a refused character

- **GIVEN** a matrix with the row `aggregate.root`, and matrices with the rows `aggregate:root`, `aggregate/root` and `aggregate root`, each in its own config
- **WHEN** archangel loads each config
- **THEN** each gives `config-invalid` at the row, which names the refused character
- **TEST** `crates/archangel_config/tests/matrices.rs` › "a_role_name_with_a_dot_colon_slash_or_space_is_refused"

### Requirement: A matrix without a name is refused

A matrix without a name MUST be refused with `config-invalid`, because the name of each row starts with it.

#### Scenario: A matrix without a name

- **GIVEN** a matrix with an area, a role pattern and rows, and no `name`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at the matrix, saying that a matrix needs a name
- **TEST** `crates/archangel_config/tests/matrices.rs` › "a_matrix_without_a_name_is_refused"

### Requirement: A row with no file is listed, not refused

A row whose files are empty MUST NOT give a diagnostic. The check summary MUST list it as a row with no file, and the JSON summary MUST contain the same list under `rowsWithNoFile`. The role pattern MUST NOT give `selector-empty`.

#### Scenario: No aggregate root yet

- **GIVEN** a matrix `api-domain` with the rows `entity` and `aggregate-root`, on a repository with entity files and no aggregate-root file
- **WHEN** archangel makes a check
- **THEN** archangel gives no diagnostic about the row `aggregate-root`
- **AND** the text summary has the line `api-domain.aggregate-root has no file in this repository`
- **AND** the JSON summary lists `api-domain.aggregate-root` under `rowsWithNoFile`
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "a_row_with_no_file_is_listed_in_the_summary"

#### Scenario: The first file of a role in the editor

- **GIVEN** a session with the same matrix and no aggregate-root file
- **WHEN** the developer creates `apps/api/src/canvas/domain/board.aggregate-root.ts`, which imports a file of the role `policy` that its row does not list
- **THEN** the language server reports a violation of `api-domain.aggregate-root`
- **AND** the summary no longer lists the row, and the result is the same as a full check
- **TEST** `crates/archangel_engine/tests/sessions.rs` › "a_row_becomes_active_when_its_first_file_is_created"

#### Scenario: The last file of a role in the editor

- **GIVEN** a session with the same matrix and one aggregate-root file
- **WHEN** the developer deletes that file
- **THEN** the summary lists `api-domain.aggregate-root` as a row with no file
- **AND** the result is the same as a full check
- **TEST** `crates/archangel_engine/tests/sessions.rs` › "a_row_is_listed_again_when_its_last_file_is_deleted"

### Requirement: A matrix whose rows match no file is an error

A matrix whose rows together have no file MUST give a `matrix-empty` diagnostic, and the check MUST fail. This MUST NOT occur when the matrix's area or its files pattern gives `selector-empty`. The message MUST name the matrix, its files and its role pattern if it has one, and say that no file in its files is in a row. It MUST tell the developer to check the pattern's extension and the row names, or to remove the matrix.

#### Scenario: A misspelt role pattern

- **GIVEN** a matrix `api-domain` with the role pattern `**/*.{role}.tz`, on a repository that has only `.ts` files
- **WHEN** archangel makes a check
- **THEN** archangel gives `matrix-empty` that names `api-domain` and the pattern `**/*.{role}.tz`, and tells the developer to check the pattern's extension and the row names, or to remove the matrix
- **AND** archangel exits with status 1
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "a_matrix_whose_rows_match_no_file_is_matrix_empty"

#### Scenario: An area that matches nothing

- **GIVEN** a matrix with the area `apps/apii/src/**` and no file under `apps/apii`
- **WHEN** archangel makes a check
- **THEN** archangel gives one `selector-empty` diagnostic for the area pattern, and no `matrix-empty` diagnostic
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "an_area_that_matches_nothing_is_selector_empty_not_matrix_empty"

#### Scenario: Files that match nothing

- **GIVEN** a matrix with the area `apps/api/src/**` and the files `apps/api/src/*/domian/**`, and no folder named `domian`
- **WHEN** archangel makes a check
- **THEN** archangel gives one `selector-empty` diagnostic for the files pattern, and no `matrix-empty` diagnostic
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "files_that_match_nothing_are_selector_empty_not_matrix_empty"

### Requirement: A matrix with no row is an error

A matrix with no row MUST give a `matrix-empty` diagnostic, and the check MUST fail. The message MUST say that the matrix has no row, and tell the developer to add a row or to remove the matrix.

#### Scenario: A matrix without a row

- **GIVEN** a matrix `api-domain` with an area, a role pattern and an empty table
- **WHEN** archangel makes a check
- **THEN** archangel gives `matrix-empty` that names `api-domain`, says that the matrix has no row, and tells the developer to add a row or to remove the matrix
- **AND** archangel exits with status 1
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "a_matrix_without_a_row_is_matrix_empty"

### Requirement: A preset's matrix that matches no file is shown, not failed

A matrix that a preset declares MUST NOT give `matrix-empty`, because `preset-empty` covers the preset. When such a matrix has no row with a file, the text summary MUST give one line that names the matrix and its role pattern and says that the matrix matches no file, and the JSON summary MUST list the matrix's name under `presetNamesMatchingNothing`.

#### Scenario: A matrix in a preset

- **GIVEN** a preset that declares a matrix whose rows match no file in the team's repository, and an element that matches files
- **WHEN** archangel makes a check
- **THEN** archangel gives no `matrix-empty` diagnostic and exits with status 0
- **AND** the text summary has the line `fsd-x/roles matches no file: no file matches its role pattern **/*.{role}.ts`, for a matrix `roles` with that role pattern in the preset `fsd-x`
- **AND** the JSON summary lists `fsd-x/roles` under `presetNamesMatchingNothing`
- **TEST** `crates/archangel/tests/presets.rs` › "a_presets_matrix_with_no_file_is_listed_not_failed"

### Requirement: The options of a matrix apply to each row

The reason, link, import kinds, names, specifier, instance and dependency of a matrix MUST apply to each row.

#### Scenario: Type imports only

- **GIVEN** a matrix with `imports: 'runtime'`, and a file of a row that has `import type` of a file the row does not list
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the row
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "the_import_kinds_of_a_matrix_apply_to_each_row"

#### Scenario: Reason and link

- **GIVEN** a matrix `api-domain` with `reason: 'Domain stays pure'` and `link: 'https://example.test/adr/7'`
- **AND** an import that breaks the row `entity`
- **WHEN** archangel makes a check
- **THEN** the violation of `api-domain.entity` carries that reason and that link
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "the_reason_and_link_of_a_matrix_apply_to_each_row"

#### Scenario: An import limit

- **GIVEN** a matrix with `specifier: 'parent'`, and a file of the row `entity` that imports a use case through `../application/save.use-case` and another through the alias `@api/application/load.use-case`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of `api-domain.entity`, for the import that starts with `../`
- **AND** the same holds for `names`, `instance` and `dependency`, each applied to each row as on a rule
- **TEST** `crates/archangel_rules/tests/matrices.rs` › "the_import_limits_of_a_matrix_apply_to_each_row"

### Requirement: Explain shows a file's matrix rows as sentences

`archangel explain` MUST list each matrix row whose files include the file, and MUST show each row as the sentences that it stands for. For a cell name that is both a role of the matrix and a declared name, it MUST say that the cell means the role.

#### Scenario: Explain an entity

- **GIVEN** the matrix `api-domain` and the file `apps/api/src/canvas/domain/canvas.entity.ts`
- **WHEN** the developer runs `archangel explain apps/api/src/canvas/domain/canvas.entity.ts`
- **THEN** the output names the rule `api-domain.entity`
- **AND** the output shows `rule({ files: { all: ['apps/api/src/*/domain/**', '**/*.entity.ts'] }, mayOnlyImport: ['**/*.entity.ts', '**/*.value-object.ts'], within: 'apps/api/src/**' })`
- **TEST** `crates/archangel/tests/cli.rs` › "explain_shows_a_matrix_row_as_its_sentences"

#### Scenario: Explain a cell that a role shadows

- **GIVEN** the declared selector `types` and a matrix with the rows `entity: ['types']` and `types: []`
- **WHEN** the developer runs `archangel explain` on an entity file and on a file of the role `types`
- **THEN** the output for the entity shows the cell `types` as the role pattern `**/*.types.ts`, and says that it means the role and not the declared selector `types`
- **AND** the output for the `types` file shows its row as the `mayOnlyImport` sentence and the second sentence, `mayNotImport` of its own files, because the row does not list its own role
- **TEST** `crates/archangel/tests/cli.rs` › "explain_shows_which_meaning_a_cell_has"

### Requirement: The work of a check does not grow with rows that no import connects

The rule evaluations and the pattern passes of a check MUST NOT grow with matrix rows whose roles no import connects.

#### Scenario: Five rows and fifty rows

- **GIVEN** one import graph, and two configs: one with a matrix of 5 rows, and one with the same matrix and 45 more rows whose roles no import connects
- **WHEN** archangel makes a check with each config
- **THEN** the two checks have the same count of rule evaluations and the same count of pattern passes
- **TEST** `crates/archangel_rules/tests/work.rs` › "rule_evaluations_do_not_grow_with_rows_that_cannot_apply"

#### Scenario: The fault control for matrix rows

- **GIVEN** a build of archangel with a fault that tests every row of a matrix for every import from the area
- **WHEN** the matrix work test runs against that build
- **THEN** the test fails on the count of rule evaluations
- **TEST** `crates/archangel_rules/tests/work.rs` › "rule_evaluation_count_rejects_a_test_of_every_row"

### Requirement: A matrix that refuses a file with no row adds no rule evaluation

The option `everyFileHasARow` MUST NOT add a rule evaluation for any import.

#### Scenario: The same check with and without the option

- **GIVEN** one import graph, and one matrix with and without `everyFileHasARow`
- **WHEN** archangel makes a check with each config
- **THEN** the two checks have the same count of rule evaluations
- **TEST** `crates/archangel_rules/tests/work.rs` › "every_file_has_a_row_adds_no_rule_evaluation"
