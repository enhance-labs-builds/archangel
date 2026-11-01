# import-tables

## Purpose

A table states, for each named group of files, what those files may import. Its rows are kinds of
file, parts or groups. This capability lets a team write the table once, read it as a table, and
have archangel check each row as a rule with a stable name. A table fails closed for repository
files: an import of a repository file that a row does not list is refused; an outside package is
judged only when a cell names an externals set that holds it.

## ADDED Requirements

### Requirement: A row refuses each import that its cells do not list

A table MUST have a name and one or more rows. Each row MUST be one rule. The rule MUST judge each import from a file of the row into a repository file. The rule MUST give a violation when the target is not in a cell of the row. A cell MUST mean the files of its name in the whole repository. The message MUST name the row and list the cells. An import that does not resolve MUST NOT be a violation of a row.

#### Scenario: A view imports an api file

- **GIVEN** the small-app config, with the table `feature-code`, the kind pattern `src/features/*/{kind}/**` and the row `views: ['models', 'shared']`
- **AND** the file `src/features/cart/views/cart-page.tsx`, which imports `src/features/cart/api/fetch-cart.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of `feature-code.views` for that import
- **AND** the message says that views may import only models and shared
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_row_refuses_an_import_that_it_does_not_list"

#### Scenario: A view imports a model and shared code

- **GIVEN** the same config
- **AND** the file `src/features/cart/views/cart-page.tsx`, which imports `src/features/cart/models/cart.ts` and `src/shared/money.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the table `feature-code`
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_row_permits_an_import_of_a_name_that_it_lists"

#### Scenario: A cell names a kind outside the table's files

- **GIVEN** a table `domain` with the files `apps/api/src/*/domain/**`, the kind pattern `**/*.{kind}.ts` and the rows `entity: ['value-object']` and `value-object: []`
- **AND** the file `apps/api/src/board/domain/board.entity.ts`, which imports `apps/api/src/board/application/draft.value-object.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the table, because the cell `value-object` means every file of that kind
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_cell_means_its_name_in_the_whole_repository"

#### Scenario: An import that does not resolve

- **GIVEN** the small-app config, and a view file that imports `./missing`
- **WHEN** archangel makes a check
- **THEN** archangel gives one `import-unresolved` diagnostic and no violation of the table
- **TEST** `crates/archangel_rules/tests/tables.rs` › "an_unresolved_import_is_not_a_violation_of_a_row"

### Requirement: A target with no row is refused, and the message says no row for it

A row MUST refuse an import whose target is in no row of the table, unless a cell lists the target. The message MUST say "no row for" and name the target. The name MUST be the kind that the kind pattern gives the target, or else its part, or else its path with the words "which is in no part".

#### Scenario: An entity imports a use case

- **GIVEN** the enhance3-slice config, with the table `domain` and the kind pattern `**/*.{kind}.ts`
- **AND** the file `apps/api/src/board/domain/board.entity.ts`, which imports `apps/api/src/board/application/save.use-case.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of `domain.entity` whose message says "no row for use-case"
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_target_with_a_kind_and_no_row_is_named_by_its_kind"

#### Scenario: An entity imports a file of a part that is not a row

- **GIVEN** the same config
- **AND** the file `apps/api/src/board/domain/board.entity.ts`, which imports `packages/contract/src/canvas.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of `domain.entity` whose message says "no row for contract"
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_target_without_a_kind_is_named_by_its_part"

#### Scenario: A feature imports a file in no part

- **GIVEN** the small-app config, and the file `src/features/cart/models/cart.ts`, which imports `src/main.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of `app.feature` whose message says "no row for src/main.ts, which is in no part"
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_target_in_no_part_is_named_by_its_path"

### Requirement: A table judges an outside package only when it names its externals set

A table MUST judge an import of an outside package only when a cell of the table, or its everyRowMayImport list, names an externals set that holds the package. A row MUST then refuse the import unless one of its cells names such a set. A table MUST NOT judge any other import of an outside package.

#### Scenario: A table that names a driver

- **GIVEN** the externals set `driver` as `['pg']`, and a table whose row `infrastructure` lists `driver` and whose row `domain` does not
- **AND** a file of `domain` that imports `pg/lib/client` and `zod`, and a file of `infrastructure` that imports `pg`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of the row `domain`, for the import of `pg/lib/client`
- **AND** archangel gives no violation for the import of `zod`, or for the import in `infrastructure`
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_table_judges_only_the_outside_packages_that_it_names"

#### Scenario: A table that names no externals set

- **GIVEN** the small-app config, and a view file that imports `react`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of either table
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_table_that_names_no_externals_set_ignores_outside_packages"

#### Scenario: A package of the workspace

- **GIVEN** the enhance3-slice config, where the package `@enhance/contract` resolves to `packages/contract/src/index.ts`
- **AND** a file in `canvas-format` that imports `@enhance/contract`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of `workspace.canvas-format`, which names `contract` as the target
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_workspace_package_is_judged_by_the_part_of_its_file"

### Requirement: A row permits its own name in the same instance

A row MUST permit an import of its own name in the same instance. The instance MUST come from the captures of a part, or from the captures other than kind in a kind pattern. A group, an inline pattern and a name without a capture MUST have one instance. A cell that names the row's own name MUST permit every instance. A row that mayNotImportItself lists MUST NOT have this permission. A row that mayNotImportItself lists and that names itself in a cell MUST be refused with `config-invalid`.

#### Scenario: One feature imports another

- **GIVEN** the small-app config, with the part `feature` as `src/features/{feature}/**` and the row `feature: ['shared']`
- **AND** the file `src/features/cart/models/cart.ts`, which imports `src/features/cart/api/fetch-cart.ts` and `src/features/login/models/session.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of `app.feature`, for the import of `session.ts`
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_row_permits_its_own_name_only_in_the_same_instance"

#### Scenario: A row that names itself

- **GIVEN** a table with the part `feature` as `src/features/{feature}/**` and the row `feature: ['feature', 'shared']`
- **AND** a file in the instance `cart` that imports a file in the instance `login`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the table
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_cell_that_names_the_row_permits_every_instance"

#### Scenario: A kind that may not import its own kind

- **GIVEN** the enhance3-slice config, where mayNotImportItself lists `domain-event`
- **AND** the file `apps/api/src/board/domain/created.domain-event.ts`, which imports `apps/api/src/board/domain/renamed.domain-event.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of `domain.domain-event` for that import
- **TEST** `crates/archangel_rules/tests/tables.rs` › "may_not_import_itself_removes_the_permission"

#### Scenario: A row with a capture that may not import itself

- **GIVEN** the part `feature` as `src/features/{feature}/**`, and a table whose mayNotImportItself lists `feature`
- **AND** a file in the instance `cart` that imports a file in the instance `login`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of the row `feature`
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_row_with_a_capture_that_may_not_import_itself_gives_one_violation"

#### Scenario: A kind that may import its own kind

- **GIVEN** the same config, and the file `apps/api/src/board/domain/board.entity.ts`, which imports `apps/api/src/board/domain/column.entity.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the table
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_kind_may_import_its_own_kind_by_default"

#### Scenario: A kind pattern with another capture

- **GIVEN** a table with the kind pattern `apps/api/src/{context}/domain/**/*.{kind}.ts` and the row `entity: []`
- **AND** the file `apps/api/src/board/domain/board.entity.ts`, which imports `apps/api/src/board/domain/column.entity.ts` and `apps/api/src/canvas/domain/canvas.entity.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of the row `entity`, for the import of `canvas.entity.ts`
- **TEST** `crates/archangel_rules/tests/tables.rs` › "another_capture_in_a_kind_pattern_makes_an_instance"

#### Scenario: A row that both lists itself and may not import itself

- **GIVEN** a table with the row `port: ['port', 'entity']` and mayNotImportItself `['port']`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at the row `port`, saying that the row both names itself and may not import itself
- **TEST** `crates/archangel_config/tests/tables.rs` › "a_row_that_names_itself_and_may_not_import_itself_is_refused"

### Requirement: Every row may also import what everyRowMayImport lists

A table MAY have the option everyRowMayImport, with cells. Each row MUST permit the imports that those cells permit, as if each row listed them.

#### Scenario: The shared Result type

- **GIVEN** the enhance3-slice config, where everyRowMayImport lists `apps/api/src/shared/result.ts`
- **AND** the file `apps/api/src/board/domain/money.value-object.ts`, which imports `apps/api/src/shared/result.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the table
- **TEST** `crates/archangel_rules/tests/tables.rs` › "every_row_may_import_what_every_row_may_import_lists"

#### Scenario: Every row may import a driver

- **GIVEN** the externals sets `driver` as `['pg']` and `orm` as `['mysql2']`, and a table with `everyRowMayImport: ['driver']`, whose row `infrastructure` lists `orm` and whose row `domain` does not
- **AND** a file of `domain` that imports `pg` and `mysql2`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of the row `domain`, for the import of `mysql2`, and none for `pg`
- **TEST** `crates/archangel_rules/tests/tables.rs` › "every_row_may_import_an_externals_set"

### Requirement: A kind pattern finds the kind of each file

A table MAY have one kind pattern, which MUST contain the capture `{kind}` one or more times. The kind of a file MUST be the text that `{kind}` matches, and `{kind}` MUST match as any capture does. Where a `*` comes before `{kind}`, the `*` MUST take the most it can, so the kind is the last name before what follows the capture. A declaration file MUST count `.d.ts`, `.d.mts` or `.d.cts` as its extension. A kind pattern without `{kind}` MUST be refused with `config-invalid`.

#### Scenario: Kinds by suffix

- **GIVEN** a table with the kind pattern `**/*.{kind}.ts`
- **WHEN** archangel makes a check
- **THEN** `board.entity.ts` has the kind `entity`, `board.entity.test.ts` has the kind `test`, and `board.ts` has no kind
- **TEST** `crates/archangel_rules/tests/tables.rs` › "the_kind_is_the_name_before_the_extension"

#### Scenario: A declaration file

- **GIVEN** the same table
- **WHEN** archangel makes a check
- **THEN** `board.entity.d.ts` has the kind `entity`, and `env.d.ts` has no kind
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_declaration_file_counts_d_ts_as_its_extension"

#### Scenario: Kinds by folder

- **GIVEN** a table with the kind pattern `src/domain/{kind}/**`
- **WHEN** archangel makes a check
- **THEN** `src/domain/entities/board.ts` has the kind `entities`, and `packages/x/src/domain/entities/a.ts` has no kind, because the pattern starts at the config folder
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_kind_pattern_by_folder_starts_at_the_config_folder"

#### Scenario: A folder kind with a dot

- **GIVEN** the small-app config, with the kind pattern `src/features/*/{kind}/**` and no row `api.v2`
- **AND** the file `src/features/cart/api.v2/fetch-cart.ts`
- **WHEN** archangel makes a check
- **THEN** the file has the kind `api.v2`, and archangel gives one violation of `feature-code.every-file-has-a-kind` that says the table has no row for `api.v2`
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_folder_kind_may_hold_a_dot"

#### Scenario: The kind capture in two places

- **GIVEN** a table with the kind pattern `**/{kind}s/*.{kind}.ts`
- **WHEN** archangel makes a check
- **THEN** `src/entitys/board.entity.ts` has the kind `entity`, and `src/entitys/board.policy.ts` has no kind
- **TEST** `crates/archangel_rules/tests/tables.rs` › "the_kind_capture_in_two_places_takes_one_name"

#### Scenario: A kind pattern without the capture

- **GIVEN** a table with the kind pattern `**/*.entity.ts`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at the kind pattern, saying that it must contain `{kind}`
- **TEST** `crates/archangel_config/tests/tables.rs` › "a_kind_pattern_without_the_kind_capture_is_refused"

### Requirement: A row key means a part or group before a kind

A row key MUST mean the declared part or group of that name. Otherwise, in a table with a kind pattern, it MUST mean a kind.

#### Scenario: Rows that mix kinds and parts

- **GIVEN** the groups `repository` as `**/*Repository.ts` and `use-case` as `src/application/**`, the part `shared` as `src/shared/**`, and a table with the rows `repository: []`, `use-case: ['repository', 'shared']` and no kind pattern
- **AND** a file in `use-case` that imports `src/shared/clock.ts` and `src/infra/UserRepository.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the table
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_table_can_mix_groups_and_parts"

### Requirement: A row key or cell that names nothing is refused with the closest name

A row key that names no part, group or kind, in a table without a kind pattern, MUST give `rule-unknown-element` with the closest name. A cell MUST name a row of the table, a declared part, group or externals set, or a path pattern. Any other cell MUST give `rule-unknown-element` with the closest name.

#### Scenario: A misspelt cell

- **GIVEN** a table with the kind pattern `**/*.{kind}.ts` and the rows `entity: ['value-objects']` and `value-object: []`
- **WHEN** archangel loads the config
- **THEN** archangel gives `rule-unknown-element` at the cell, and names `value-object` as the closest name
- **TEST** `crates/archangel_config/tests/tables.rs` › "a_misspelt_cell_is_refused_with_the_closest_name"

#### Scenario: An unknown key without a kind pattern

- **GIVEN** the part `canvas-document`, and a table with no kind pattern and the row `canvas-documents`
- **WHEN** archangel loads the config
- **THEN** archangel gives `rule-unknown-element` at the row, and names `canvas-document` as the closest name
- **TEST** `crates/archangel_config/tests/tables.rs` › "a_row_key_that_names_nothing_is_refused_with_the_closest_name"

### Requirement: A row that everyRowMayImport names may not refuse itself

A row in `mayNotImportItself` that `everyRowMayImport` names MUST be refused with `config-invalid`, because each row has those names on its list.

#### Scenario: Every row may import a row that may not import itself

- **GIVEN** a table with `everyRowMayImport: ['port']` and `mayNotImportItself: ['port']`
- **WHEN** archangel loads the config
- **THEN** the config is refused with `config-invalid`, and the message names `everyRowMayImport`
- **TEST** `crates/archangel_config/tests/tables.rs` › "a_row_that_every_row_may_import_lists_and_may_not_import_itself_is_refused"

### Requirement: A rule and a table have different names

A rule and a table with one name MUST be refused with `rule-name-duplicate`, because `off` refers to both by name.

#### Scenario: A rule and a table named t

- **GIVEN** a rule named `t` and a table named `t`
- **WHEN** archangel loads the config
- **THEN** the config is refused with `rule-name-duplicate`
- **TEST** `crates/archangel_config/tests/tables.rs` › "a_rule_and_a_table_with_one_name_are_refused"

### Requirement: A row key that names an externals set is refused

A row key that names an externals set MUST be refused with `config-invalid`, because outside packages have no files that import.

#### Scenario: A row of outside packages

- **GIVEN** the externals set `driver`, and a table with the row `driver`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at the row, saying that outside packages have no files that import
- **TEST** `crates/archangel_config/tests/tables.rs` › "a_row_key_that_names_an_externals_set_is_refused"

### Requirement: A table's files choose whose imports it judges

A table MAY have the option files. Only a file in the table's files MUST be judged by a row. Without the option, the table's files MUST be the files that its kind pattern matches with any kind, and the files of its part and group rows. The files option MUST NOT change which files a cell or a row name means as a target.

#### Scenario: A kind file outside the table's files

- **GIVEN** the enhance3-slice config, and the file `apps/api/src/board/application/draft.entity.ts`, which imports `apps/api/src/board/application/save.use-case.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the table `domain`, because the importer is not in the table's files
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_file_outside_the_tables_files_is_not_judged"

#### Scenario: The default files of a kind table

- **GIVEN** the small-app config, and the file `src/features/cart/index.ts`, which imports `src/main.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the table `feature-code`, because the kind pattern does not match the importer
- **TEST** `crates/archangel_rules/tests/tables.rs` › "the_default_files_of_a_table_are_the_files_its_kind_pattern_matches"

#### Scenario: The default files of a table of parts

- **GIVEN** the parts `web` and `api`, a table with the rows `web: ['api']` and `api: []` and no `files`, and the file `scripts/seed.ts`, in neither part, which imports a file of `api`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the table, because the importer is in none of its rows
- **TEST** `crates/archangel_rules/tests/tables.rs` › "the_default_files_of_a_table_of_parts_are_the_files_of_its_rows"

### Requirement: A file in no row of a table without a kind pattern is told the rows

For a table without a kind pattern, the message of `<table>.every-file-has-a-kind` MUST say that the file is in none of the table's rows, name them, and say to put the file in one of those parts or groups, or to leave it out of the table's files with `not`.

#### Scenario: A table of parts, and a file in neither part

- **GIVEN** a table `app` with the rows `feature` and `shared`, the option `files: 'src/**'`, and the file `src/stray.ts`, which is in neither part
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of `app.every-file-has-a-kind` at `src/stray.ts`, whose message names the rows `feature` and `shared` and says to put the file in one of them or to leave it out with `not`
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_file_in_no_row_of_a_table_of_parts_is_told_the_rows"

### Requirement: Every file in a table's files must have a row

Every table MUST make the rule `<table>.every-file-has-a-kind`; there is no option to leave it out, and `off` turns it off as any rule. Each file in the table's files that is in no row MUST give one violation of that rule. The message MUST say that the file has no kind, or name its kind and the closest row, and say to add a row, rename the file, or leave it out of the table's files with `not`. A table without `files` MUST NOT have a file with no kind in its files.

#### Scenario: A domain file with no kind

- **GIVEN** the enhance3-slice config, and the file `apps/api/src/board/domain/board.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of `domain.every-file-has-a-kind` for that file, whose message says that the file has no kind and shows the kind pattern
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_file_with_no_kind_is_refused_by_default"

#### Scenario: A file with a kind that has no row

- **GIVEN** the small-app config, and the file `src/features/cart/hooks/use-cart.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of `feature-code.every-file-has-a-kind`, whose message says that the table has no row for `hooks`
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_file_whose_kind_has_no_row_is_refused"

#### Scenario: A misspelt kind

- **GIVEN** a table with the kind pattern `**/*.{kind}.ts` and the row `aggregate-rot`, and the file `apps/api/src/board/domain/board.aggregate-root.ts` in its files
- **WHEN** archangel makes a check
- **THEN** the violation of `every-file-has-a-kind` names `aggregate-rot` as the closest row
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_file_whose_kind_has_no_row_names_the_closest_row"

#### Scenario: Tests, index files and declaration files left out

- **GIVEN** the group `tests` as `**/*.test.ts`, and a table with the files `{ all: ['apps/api/src/*/domain/**', { not: ['tests', '**/index.ts', '**/*.d.ts'] }] }` and the kind pattern `**/*.{kind}.ts`
- **AND** the files `index.ts`, `board.entity.test.ts` and `env.d.ts` in `apps/api/src/board/domain`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of `every-file-has-a-kind` for those files
- **TEST** `crates/archangel_rules/tests/tables.rs` › "files_left_out_of_a_table_need_no_kind"

#### Scenario: A file with no kind outside a table without files

- **GIVEN** a table with the kind pattern `**/*.{kind}.ts`, no `files`, and the row `entity: []`, and no other rule
- **AND** the file `apps/api/src/board/domain/board.ts`, which imports `apps/api/src/board/application/save.use-case.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the table for that file or its import
- **AND** `archangel rules` lists `board.ts` as a file that no rule judges
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_file_with_no_kind_is_outside_a_table_without_files"

### Requirement: A file in two rows of one table is an error

A file in the table's files that is in two rows of one table MUST give one `table-row-overlap` diagnostic, which names the file and both rows, and says to narrow one of the two with `not` or to leave the file out of the table's files. The message MUST say that the baseline does not hold this diagnostic, so the overlap is fixed in the config. Each of the two rows MUST still judge the file.

#### Scenario: A file in two groups that are rows

- **GIVEN** the groups `model` as `src/**/*.model.ts` and `feature` as `src/features/**`, and a table with the rows `model: []` and `feature: []`
- **AND** the file `src/features/cart/cart.model.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one `table-row-overlap` diagnostic that names the file, `model` and `feature`
- **AND** the message says to narrow `model` or `feature` with `not`, or to leave the file out of the table's files, and that the baseline does not hold it
- **AND** archangel exits with status 1
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_file_in_two_rows_of_a_table_is_an_error"

### Requirement: A row key that is a group means the group, also when the kind pattern gives that name

When a row key names a declared group and the kind pattern gives that name as a kind to a file, the key MUST mean the group. A file in the table's files with that kind that is not in the group MUST give the violation of `<table>.every-file-has-a-kind`, whose message MUST say that the row key of that name means the group, not the kind. `archangel explain` and `archangel rules` MUST show that the key means the group.

#### Scenario: A group named types

- **GIVEN** the group `types` as `apps/api/src/shared/types/**`, a table with the kind pattern `**/*.{kind}.ts` and the row `types: []`, and the file `apps/api/src/board/domain/id.types.ts` in the table's files
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of `every-file-has-a-kind` for `id.types.ts`, whose message says that the row `types` means the group `apps/api/src/shared/types/**`, not the kind
- **AND** `archangel rules` shows the row `types` with the files of the group
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_row_key_that_is_a_group_means_the_group"

### Requirement: Each row is a rule named after its table and row

Each row MUST be one rule, named with the name of the table, a dot and the row key, such as `domain.entity`. A table that a preset declares MUST have the preset's prefix, so its rows are named such as `fsd-x/roles.entity`. A config rule with the prefixed name of a row MUST replace that row. Two tables with one name MUST give `rule-name-duplicate`. A table without a name MUST be refused with `config-invalid`.

#### Scenario: A row name that another rule has

- **GIVEN** a table `domain` with the row `entity`, and another rule named `domain.entity`
- **WHEN** archangel loads the config
- **THEN** archangel gives `rule-name-duplicate` for `domain.entity`
- **TEST** `crates/archangel_config/tests/tables.rs` › "a_row_name_that_another_rule_has_is_a_duplicate"

#### Scenario: Two tables with one name

- **GIVEN** two tables named `domain`
- **WHEN** archangel loads the config
- **THEN** archangel gives `rule-name-duplicate` that names the table `domain` and says to give each table its own name
- **TEST** `crates/archangel_config/tests/tables.rs` › "two_tables_with_one_name_are_refused"

#### Scenario: A table in a JSON config without a name

- **GIVEN** a JSON config with a rule of the kind `mayImport` that has rows and no name
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at the rule, saying that a table needs a name
- **TEST** `crates/archangel_config/tests/tables.rs` › "a_table_without_a_name_is_refused"

#### Scenario: A row of a table in a preset

- **GIVEN** a config that extends a preset `fsd-x`, which declares a table `roles` with the row `entity`, and an import that breaks that row
- **WHEN** archangel makes a check
- **THEN** the violation names the rule `fsd-x/roles.entity`
- **AND** a config rule named `fsd-x/roles.entity` replaces that row
- **TEST** `crates/archangel/tests/presets.rs` › "a_presets_table_row_is_named_with_the_preset_prefix"

### Requirement: Off, exceptions and the baseline refer to a row by its name

`off`, exception comments and the baseline MUST refer to a row by its name. A table's name in `off` MUST turn off every rule that the table makes: each row, and its `every-file-has-a-kind` rule, including a row added to the table later. `archangel rules` MUST show those rules as off.

#### Scenario: Turn off one row

- **GIVEN** a table `domain` with the rows `entity` and `policy`, and `off: ['domain.policy']`, and an import that breaks the row `policy`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation for that import, and the row `entity` still applies
- **TEST** `crates/archangel_rules/tests/tables.rs` › "off_drops_one_row_of_a_table"

#### Scenario: Turn off a whole table by its name

- **GIVEN** a table `domain` with the rows `entity` and `policy`, `off: ['domain']`, an import that breaks the row `entity`, and a file in the table's files with no row
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of `domain.entity`, `domain.policy` or `domain.every-file-has-a-kind`
- **AND** `archangel rules` shows each of the three as off
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_table_name_in_off_drops_every_rule_of_the_table"

#### Scenario: An exception for one row

- **GIVEN** an import that breaks the row `domain.entity`, under the comment `// archangel-allow domain.entity: migration in progress`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation for that import and counts one exception for `domain.entity`
- **TEST** `crates/archangel_rules/tests/tables.rs` › "an_exception_names_a_row_of_a_table"

#### Scenario: A baseline entry for one row

- **GIVEN** a violation of the row `domain.entity`
- **WHEN** the developer runs `archangel baseline add`
- **THEN** the baseline entry records the rule name `domain.entity`
- **TEST** `crates/archangel/tests/baseline_command.rs` › "a_baseline_entry_records_the_name_of_a_table_row"

### Requirement: A table or row name with a colon or a slash is refused

The name of a table and each row key MUST NOT contain a colon or a slash. Archangel MUST refuse such a name with `config-invalid` at the name.

#### Scenario: Table names with a refused character

- **GIVEN** tables named `api:domain` and `api/domain`, each in its own config
- **WHEN** archangel loads each config
- **THEN** each gives `config-invalid` at the name, which names the refused character
- **AND** the message for `api:domain` says that an exception comment ends the rule name at the first colon
- **TEST** `crates/archangel_config/tests/tables.rs` › "a_table_name_with_a_colon_or_slash_is_refused"

#### Scenario: Row keys with a refused character

- **GIVEN** tables with the rows `aggregate:root` and `aggregate/root`, each in its own config
- **WHEN** archangel loads each config
- **THEN** each gives `config-invalid` at the row, which names the refused character
- **TEST** `crates/archangel_config/tests/tables.rs` › "a_row_key_with_a_colon_or_slash_is_refused"

### Requirement: A dot or a space in a table or row name is accepted

A dot and white space in the name of a table or a row MUST be accepted, as in any rule name. A row name that is then the name of another rule MUST give `rule-name-duplicate`.

#### Scenario: Names that load, and a clash

- **GIVEN** a table `feature-code` with the rows `'api.v2': ['shared']` and `'my views': ['shared']`
- **AND** another config with a table `a.b` that has the row `c`, and a table `a` that has the row `b.c`
- **WHEN** archangel loads each config
- **THEN** the first config loads with no diagnostic
- **AND** the second config gives `rule-name-duplicate` for `a.b.c`
- **TEST** `crates/archangel_config/tests/tables.rs` › "a_dot_or_a_space_in_a_name_is_accepted_and_a_clash_is_a_duplicate"

#### Scenario: A row with such a name judges and takes an exception

- **GIVEN** the `feature-code` table with the kind pattern `src/features/*/{kind}/**`, a file of kind `api.v2` that imports a file no cell lists, and the same import in a `my views` file under `// archangel-allow feature-code.my views: reason`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of `feature-code.api.v2`, none of `feature-code.my views`, and counts one exception for `feature-code.my views`
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_rule_named_with_a_dot_or_a_space_judges_its_kind_and_takes_an_exception"

### Requirement: A row with no file is listed, not refused

A kind row whose files are empty MUST NOT give a diagnostic. The check summary MUST list it, and the JSON summary MUST list it under `rowsWithNoFile`. A row that names a part or group MUST NOT give a diagnostic of its own; the part or group gives `element-empty` or `selector-empty` at its declaration.

#### Scenario: No aggregate root yet

- **GIVEN** the enhance3-slice config, on a repository with entity files and no aggregate-root file
- **WHEN** archangel makes a check
- **THEN** archangel gives no diagnostic about the row `aggregate-root`
- **AND** the text summary has the line `domain.aggregate-root has no file in this repository`
- **AND** the JSON summary lists `domain.aggregate-root` under `rowsWithNoFile`
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_row_with_no_file_is_listed_in_the_summary"
- **TEST** `crates/archangel/tests/cli.rs` › "a_row_with_no_file_is_listed_in_the_summary_as_text_and_json"

#### Scenario: A misspelt part in a row

- **GIVEN** the part `ui` as `packages/uii/**`, no file under `packages/uii`, and a table with the row `ui: []`
- **WHEN** archangel makes a check
- **THEN** archangel gives one `element-empty` diagnostic for `ui`, and no other diagnostic about the row
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_row_that_names_an_empty_part_gives_one_diagnostic"

#### Scenario: The first file of a kind in the editor

- **GIVEN** a session with the enhance3-slice config and no aggregate-root file
- **WHEN** the developer creates `apps/api/src/board/domain/board.aggregate-root.ts`, which imports `apps/api/src/board/application/save.use-case.ts`
- **THEN** the language server reports a violation of `domain.aggregate-root` for that import
- **AND** the summary no longer lists the row, and the result is the same as a full check
- **TEST** `crates/archangel_engine/tests/sessions.rs` › "a_row_becomes_active_when_its_first_file_is_created"

#### Scenario: The last file of a kind in the editor

- **GIVEN** a session with the same config and one aggregate-root file
- **WHEN** the developer deletes that file
- **THEN** the summary lists `domain.aggregate-root` as a row with no file, and the result is the same as a full check
- **TEST** `crates/archangel_engine/tests/sessions.rs` › "a_row_is_listed_again_when_its_last_file_is_deleted"

### Requirement: A table that checks nothing is an error

A table with no row, or whose rows together have no file, MUST give a `table-empty` diagnostic, and the check MUST fail. This MUST NOT occur when the table's files or its area give `selector-empty`. The message MUST name the table and its kind pattern, if it has one. It MUST tell the developer to add a row, to check the pattern and the row names, or to remove the table.

#### Scenario: A misspelt kind pattern

- **GIVEN** a table `domain` with the kind pattern `**/*.{kind}.tz`, on a repository that has only `.ts` files
- **WHEN** archangel makes a check
- **THEN** archangel gives `table-empty` that names `domain` and the pattern `**/*.{kind}.tz`, and tells the developer to check the pattern and the row names, or to remove the table
- **AND** archangel exits with status 1
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_table_whose_rows_match_no_file_is_table_empty"

#### Scenario: A table without a row

- **GIVEN** a table `domain` with a kind pattern and an empty object of rows
- **WHEN** archangel makes a check
- **THEN** archangel gives `table-empty` that names `domain`, says that the table has no row, and tells the developer to add a row or to remove the table
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_table_without_a_row_is_table_empty"

#### Scenario: Files that match nothing

- **GIVEN** a table with the files `apps/api/src/*/domains/**`, and no folder named `domains`
- **WHEN** archangel makes a check
- **THEN** archangel gives one `selector-empty` diagnostic for the files pattern, and no `table-empty` diagnostic
- **TEST** `crates/archangel_rules/tests/tables.rs` › "files_that_match_nothing_are_selector_empty_not_table_empty"

#### Scenario: An area that matches nothing

- **GIVEN** a table with `within: 'apps/apii/**'`, and no folder named `apii`
- **WHEN** archangel makes a check
- **THEN** archangel gives one `selector-empty` diagnostic for the area pattern, and no `table-empty` diagnostic
- **TEST** `crates/archangel_rules/tests/tables.rs` › "an_area_that_matches_nothing_is_selector_empty_not_table_empty"

### Requirement: A table holds only source files, and never the config file

A file that is not a source file MUST NOT count as a file of a table: it MUST NOT give `every-file-has-a-kind`, and MUST NOT keep a row or a table from counting as having no file. The config file MUST be in no table, as an importer or under `every-file-has-a-kind`.

#### Scenario: A table whose only files are not source files

- **GIVEN** a table with the kind pattern `docs/{kind}/**`, and only `docs/guide/a.md` under it
- **WHEN** archangel makes a check
- **THEN** archangel gives `table-empty`
- **TEST** `crates/archangel_rules/tests/table_files.rs` › "a_table_whose_only_files_are_not_source_files_is_empty"

#### Scenario: A row whose only files are not source files

- **GIVEN** a table with the rows `domain` and `assets`, and only `src/assets/s.css` under `assets`
- **WHEN** archangel makes a check
- **THEN** the summary lists `t.assets` as a row with no file
- **TEST** `crates/archangel_rules/tests/table_files.rs` › "a_row_with_only_files_that_are_not_source_files_is_a_row_with_no_file"

#### Scenario: A file in the table's files that is not a source file

- **GIVEN** a table with `files: 'src/**'`, and the files `src/docs/r.md` and `src/tsconfig.json`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation for these files
- **TEST** `crates/archangel_rules/tests/table_files.rs` › "a_file_that_is_not_source_needs_no_row"

#### Scenario: The reach of a table counts source files

- **GIVEN** the same table, and a row whose only file is a stylesheet
- **WHEN** the developer runs `archangel rules`
- **THEN** the table's own rule counts only source files, and the row checks nothing
- **TEST** `crates/archangel/tests/rules_command.rs` › "a_table_counts_only_source_files"

#### Scenario: The config file has a kind

- **GIVEN** `archangel.config.ts`, the kind pattern `**/*.{kind}.ts`, and a row `config` with an empty list
- **WHEN** archangel makes a check
- **THEN** the imports of the config file give no violation
- **TEST** `crates/archangel_rules/tests/table_files.rs` › "the_config_file_is_never_in_a_table"

#### Scenario: The config file in the table's files

- **GIVEN** `archangel.config.ts`, the kind pattern `**/*.{kind}.ts`, `files: '**'`, and no row `config`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation for the config file
- **TEST** `crates/archangel_rules/tests/table_files.rs` › "the_config_file_needs_no_row"

### Requirement: A file with no row has its own code

A violation of `<table>.every-file-has-a-kind` MUST have the code `every-file-has-a-kind`. Only the baseline MUST be able to permit it.

#### Scenario: A file with no row

- **GIVEN** a table with `files: 'src/**'`, and a file with a kind that has no row
- **WHEN** archangel makes a check
- **THEN** the code is `every-file-has-a-kind`, which is about a path
- **TEST** `crates/archangel_rules/tests/table_files.rs` › "a_file_with_no_row_has_the_code_every_file_has_a_kind"

### Requirement: A preset's tables count toward preset-empty

`preset-empty` MUST count a preset's tables with its file parts, so that a preset with only tables and no file fails, and a preset whose table has a file does not.

#### Scenario: A preset of tables with no file

- **GIVEN** a preset with only a table, whose kind pattern is misspelt
- **WHEN** archangel makes a check
- **THEN** archangel gives `preset-empty`
- **TEST** `crates/archangel_rules/tests/table_files.rs` › "a_preset_that_is_only_tables_and_has_no_file_is_preset_empty"

#### Scenario: A preset of tables with a file

- **GIVEN** a preset with only a table, and a file that has a kind of a row
- **WHEN** archangel makes a check
- **THEN** archangel gives no `preset-empty`
- **TEST** `crates/archangel_rules/tests/table_files.rs` › "a_preset_whose_table_has_a_file_is_not_preset_empty"

### Requirement: An instance comes from the values of the captures

Two files MUST be in one instance of a part when their captures have the same values, whichever pattern of the part found them. Rows, `mayOnlyImport` and the `instance` option MUST agree. An `entry` rule MUST keep its comparison of the folder.

#### Scenario: A row over two patterns of a part

- **GIVEN** the part `f` with the patterns `a/{x}/**` and `b/{x}/**`, and the row `f: []`
- **WHEN** `a/foo/x.ts` imports `b/foo/y.ts`, and `b/foo/y.ts` imports `a/bar/z.ts`
- **THEN** only the second import is a violation
- **TEST** `crates/archangel_rules/tests/table_files.rs` › "an_instance_comes_from_the_captures_across_two_patterns_of_a_part"

#### Scenario: A rule and the instance option over two patterns of a part

- **GIVEN** the same part, a `mayOnlyImport` rule on it, and a `mayNotImport` rule on it with `instance: 'different'`
- **WHEN** `a/foo/x.ts` imports `b/foo/y.ts`
- **THEN** neither rule gives a violation
- **TEST** `crates/archangel_rules/tests/table_files.rs` › "may_only_import_and_the_instance_option_agree_on_an_instance"

### Requirement: A table with files and a captured kind means its rows

A table with `files` and a kind pattern with another capture MUST agree with the same rules written with paths.

#### Scenario: Random imports of such a table

- **GIVEN** a table of two kinds with the kind pattern `{x}/*.{kind}.ts`, with and without `files`, and the same rows written by hand with path patterns
- **WHEN** archangel judges any import between the files
- **THEN** both give the same broken rows
- **TEST** `crates/archangel_rules/tests/rule_properties.rs` › "a_table_with_files_and_a_captured_kind_agrees_with_one_rule_per_row"

### Requirement: A preset's table that checks nothing is shown, not failed

A table that a preset declares MUST NOT give `table-empty`, because `preset-empty` covers the preset. When such a table checks nothing, the summary MUST give one line that names it and its kind pattern. The JSON summary MUST list it under `presetNamesMatchingNothing`.

#### Scenario: A table in a preset

- **GIVEN** a preset `fsd-x` that declares a table `roles` with the kind pattern `**/*.{kind}.ts`, whose rows match no file in the team's repository, and a part that matches files
- **WHEN** archangel makes a check
- **THEN** archangel gives no `table-empty` diagnostic and exits with status 0
- **AND** the text summary has the line `fsd-x/roles matches no file: no file matches its kind pattern **/*.{kind}.ts`
- **AND** the JSON summary lists `fsd-x/roles` under `presetNamesMatchingNothing`
- **TEST** `crates/archangel/tests/presets.rs` › "a_presets_table_with_no_file_is_listed_not_failed"

### Requirement: The options of a table apply to each row

The reason, link, import kinds, names, specifier, dependency and instance of a table MUST apply to each row, as on any rule. With instance, each row MUST judge only the imports whose two ends compare that way, and a row whose name has no capture MUST give `rule-needs-capture`.

#### Scenario: Type imports only

- **GIVEN** a table with `imports: 'runtime'`, and a file of a row that has `import type` of a file that the row does not list
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the row
- **TEST** `crates/archangel_rules/tests/table_options.rs` › "the_import_kinds_of_a_table_apply_to_each_row"

#### Scenario: Because and link

- **GIVEN** a table `domain` with `because: 'Domain stays pure'` and `link: 'https://example.test/adr/7'`, and an import that breaks the row `entity`
- **WHEN** archangel makes a check
- **THEN** the violation of `domain.entity` carries that reason and that link
- **TEST** `crates/archangel_rules/tests/table_options.rs` › "the_reason_and_link_of_a_table_apply_to_each_row"

#### Scenario: An import limit

- **GIVEN** a table with `specifier: 'parent'`, and a file of the row `entity` that imports a use case through `../application/save.use-case` and another through the alias `@api/application/load.use-case`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of the row `entity`, for the import that starts with `../`
- **TEST** `crates/archangel_rules/tests/table_options.rs` › "the_import_limits_of_a_table_apply_to_each_row"

#### Scenario: Names and dependency

- **GIVEN** a table with `names: ['default']`, and a file of the row `entity` that imports `{ save }` from a use case that the row does not list
- **AND** the externals set `tools` as `['zod', 'vitest']`, a table with `dependency: 'dev'` whose row `infrastructure` lists `tools` and whose row `entity` lists nothing, and a manifest that lists `zod` under `dependencies` and `vitest` only under `devDependencies`, with a file of `entity` that imports `zod` and `vitest`
- **WHEN** archangel makes a check with each
- **THEN** the first check gives no violation of `entity`
- **AND** the second check gives one violation of `entity`, for `vitest`, and none for `zod`, because `dependency: 'dev'` makes each row judge only the imports of dev dependencies
- **TEST** `crates/archangel_rules/tests/table_options.rs` › "the_names_and_dependency_of_a_table_apply_to_each_row"

#### Scenario: A table with instance

- **GIVEN** the part `feature` as `src/features/{feature}/**`, and a table with the row `feature: ['shared']` and `instance: 'different'`
- **AND** a file in the instance `cart` that imports a file in the instance `login`, and a file in `src/features/cart/views` that imports a file in `src/features/cart/models`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of the row `feature`, for the import into `login`, and does not judge the import inside `cart`
- **TEST** `crates/archangel_rules/tests/table_options.rs` › "the_instance_of_a_table_applies_to_each_row"

#### Scenario: Instance on a row without a capture

- **GIVEN** the part `shared` as `src/shared/**`, and a table with the row `shared: []` and `instance: 'different'`
- **WHEN** archangel loads the config
- **THEN** archangel gives `rule-needs-capture` at the row `shared`
- **TEST** `crates/archangel_config/tests/tables.rs` › "instance_on_a_row_without_a_capture_is_refused"

### Requirement: A table with an area judges only targets in the area

A table MAY have an area, written with within. Its rows MUST then judge only imports whose target is in the area.

#### Scenario: A domain table limited to the API

- **GIVEN** the enhance3-slice domain table with `within: 'api'`
- **AND** the file `apps/api/src/board/domain/board.entity.ts`, which imports `packages/contract/src/canvas.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the table for that import
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_table_with_an_area_judges_only_targets_in_the_area"

### Requirement: Explain shows a file's rows as sentences

`archangel explain` MUST give the kind of the file in each table, list each row whose files include the file, and show each row as the sentences that it stands for.

#### Scenario: Explain an entity

- **GIVEN** the enhance3-slice config and the file `apps/api/src/board/domain/board.entity.ts`
- **WHEN** the developer runs `archangel explain apps/api/src/board/domain/board.entity.ts`
- **THEN** the output gives the kind `entity` in the table `domain`, and names the rule `domain.entity`
- **AND** the output shows the row as one `mayOnlyImport` sentence with its cells, the everyRowMayImport cell `apps/api/src/shared/result.ts`, and itself, because the kind pattern `**/*.{kind}.ts` makes no instance
- **TEST** `crates/archangel/tests/cli.rs` › "explain_shows_a_table_row_as_its_sentences"

#### Scenario: Explain a row that may not import itself

- **GIVEN** the same config and the file `apps/api/src/board/domain/created.domain-event.ts`
- **WHEN** the developer runs `archangel explain` on the file
- **THEN** the output shows the row as the `mayOnlyImport` sentence and a second sentence, `mayNotImport` of its own kind
- **TEST** `crates/archangel/tests/cli.rs` › "explain_shows_the_second_sentence_of_a_row_that_may_not_import_itself"

### Requirement: The work of a check does not grow with rows that no import connects

The rule evaluations and the pattern passes of a check MUST NOT grow with table rows that no import connects. A kind pattern MUST NOT add a pattern pass or a file classification. The rule `<table>.every-file-has-a-kind` MUST add no rule evaluation and one structure evaluation per file in the table's files with no row. An import between two files of a row whose name has an instance capture MUST cost one rule evaluation and no pattern match; between two files of a row without one, none.

#### Scenario: Five rows and fifty rows

- **GIVEN** one import graph, and two configs: one with a table of 5 rows, and one with the same table and 45 more rows that no import connects
- **WHEN** archangel makes a check with each config
- **THEN** the two checks have the same count of rule evaluations and the same count of pattern passes
- **TEST** `crates/archangel_rules/tests/work.rs` › "rule_evaluations_do_not_grow_with_rows_that_cannot_apply"

#### Scenario: The fault control for table rows

- **GIVEN** a build of archangel with a fault that tests every row of a table for every import
- **WHEN** the table work test runs against that build
- **THEN** the test fails on the count of rule evaluations
- **TEST** `crates/archangel_rules/tests/work.rs` › "rule_evaluation_count_rejects_a_test_of_every_row"

#### Scenario: A kind pattern and the pattern passes

- **GIVEN** one repository, and two configs that differ only by one table with the kind pattern `**/*.{kind}.ts`
- **WHEN** archangel makes a check with each config
- **THEN** the two checks have the same count of pattern passes and the same count of files classified
- **TEST** `crates/archangel_rules/tests/work.rs` › "a_kind_pattern_adds_no_pattern_pass"

#### Scenario: The same check with the rule on and off

- **GIVEN** one import graph with 3 files in the table's files that have no row, and one table `domain`, checked once as written and once with `off: ['domain.every-file-has-a-kind']`
- **WHEN** archangel makes a check with each config
- **THEN** the two checks have the same count of rule evaluations
- **AND** the check with the rule on has 3 more structure evaluations than the check with it off
- **TEST** `crates/archangel_rules/tests/work.rs` › "every_file_has_a_kind_costs_one_structure_evaluation_per_file_with_no_row"

#### Scenario: Imports inside a captured row and inside an uncaptured row

- **GIVEN** a table with the part row `feature`, whose pattern is `src/features/{feature}/**`, and the kind row `entity`, whose kind pattern has no other capture
- **AND** a second table `contexts` with the kind pattern `apps/api/src/{context}/domain/**/*.{kind}.ts` and the row `entity`
- **AND** one repository with N imports between files of one feature, M imports between entity files of the first table, and P imports between entity files of one context
- **WHEN** archangel makes a check, and the same check without the table `contexts`
- **THEN** the imports inside the feature count N rule evaluations for `feature`, the imports between entities count 0 for the first table's `entity`, and the imports inside one context count P for `contexts.entity`
- **AND** the two checks have the same count of pattern passes
- **TEST** `crates/archangel_rules/tests/work.rs` › "imports_inside_a_captured_row_cost_one_evaluation_each"

### Requirement: A table means the sentences of its rows

A table MUST give exactly the violations of one rule per row, under the row's name. Each such rule is `{ files: { all: [files, row] }, mayOnlyImport: [cells, everyRowMayImport], within: area }` with the permission of its own name in the same instance, and with `mayNotImport` of its own name when mayNotImportItself lists the row.

#### Scenario: Random tables and their rules

- **GIVEN** random tables with kinds, parts and groups as rows, with and without an area, externals cells and mayNotImportItself, and for each row its rule written by hand
- **WHEN** archangel checks random repositories with both
- **THEN** each pair of checks gives the same violations under the same rule names
- **TEST** `crates/archangel_rules/tests/rule_properties.rs` › "a_table_agrees_with_one_rule_per_row"

### Requirement: The area of a row

When the table has no `within`, the area of each row MUST be every repository file and the externals sets that the table names in a cell or in everyRowMayImport. With `within`, the area of each row MUST be that area.

#### Scenario: A row without within

- **GIVEN** the small-app config, and a view file that imports `react` and `src/main.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of `feature-code.views`, for `src/main.ts`, and does not judge the import of `react`
- **TEST** `crates/archangel_rules/tests/tables.rs` › "a_row_without_within_judges_every_repository_file_and_no_unnamed_package"
