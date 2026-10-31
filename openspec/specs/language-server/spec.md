# language-server Specification

## Purpose
A broken rule is cheapest to fix while the import is still being written. This capability covers the
language server, which gives editors and agents archangel's diagnostics and fixes as code changes.

## Requirements

### Requirement: The language server gives the diagnostics of a full check

`archangel lsp` MUST speak the Language Server Protocol on standard input and output. After the editor starts it, the server MUST make a full check and publish the diagnostics of every file that has them.

#### Scenario: The server starts

- **GIVEN** a repository with violations in two files
- **WHEN** an editor starts `archangel lsp` and sends the initialize messages
- **THEN** the server publishes the diagnostics of the two files
- **TEST** `crates/archangel_lsp/tests/server.rs` › "server_publishes_full_check_diagnostics_after_initialize"

#### Scenario: Ranges count in UTF-16 code units

- **GIVEN** an import whose line has a character outside the Basic Multilingual Plane before the specifier, and an accented letter in the specifier
- **WHEN** the server publishes the import's diagnostic and offers its quick fix
- **THEN** the diagnostic's range covers exactly the specifier and its quotes, and the fix's range exactly the text between the quotes, counted in the UTF-16 code units the protocol uses
- **TEST** `crates/archangel_lsp/tests/server.rs` › "ranges_count_utf16_code_units_as_the_protocol_does"

### Requirement: Diagnostics follow text that is not saved

When the editor sends a change to an open document, the server MUST check the document's new text, which is not saved yet. It MUST publish the diagnostics with the version number of that change. While a document is open, the server MUST ignore changes to its file on disk, because its text belongs to the editor.

#### Scenario: The developer types an import

- **GIVEN** an open document with no violations
- **WHEN** the editor sends version 7 of the document, which has an import that breaks a rule
- **THEN** the server publishes the violation for version 7 of the document
- **TEST** `crates/archangel_lsp/tests/server.rs` › "unsaved_text_gets_diagnostics_for_its_version"

#### Scenario: A document closes without a save

- **GIVEN** an open document whose unsaved text breaks a rule, and whose text on disk does not
- **WHEN** the editor closes the document without a save
- **THEN** the server publishes no violation for that file
- **TEST** `crates/archangel_lsp/tests/server.rs` › "closing_an_unsaved_document_returns_to_the_text_on_disk"

#### Scenario: Edits arrive during a check

- **GIVEN** an open document
- **WHEN** the editor sends versions 3, 4 and 5 before the server finishes the check of version 3
- **THEN** the last diagnostics that the server publishes for the document are for version 5
- **AND** the server publishes nothing for version 4 after it publishes version 5
- **TEST** `crates/archangel_lsp/tests/server.rs` › "queued_edits_publish_only_the_newest_version"

### Requirement: An edit costs work for the edited file only

For a change to one document, the server MUST parse only that document and resolve only its imports. It MUST publish diagnostics only for files whose diagnostics changed. The cost of an edit MUST NOT grow with the number of files in the repository, except for the search for cycles.

#### Scenario: Work for one edit

- **GIVEN** a repository with 2,000 source files and an open document with 6 imports
- **WHEN** the editor sends a change to that document
- **AND** the editor then saves the document, which also reports a change to its file on disk
- **THEN** the work counts for the change and the save together are 1 file parsed, 6 resolutions, and rule evaluations for those 6 imports only
- **AND** the server publishes diagnostics for that document only
- **TEST** `crates/archangel_engine/tests/work.rs` › "one_edit_parses_one_file_and_resolves_its_imports"
- **TEST** `crates/archangel_lsp/tests/server.rs` › "edit_publishes_only_files_whose_diagnostics_changed"

#### Scenario: The fault control for the edit work count

- **GIVEN** a build of archangel with a fault that makes a full check after each edit
- **WHEN** the edit work test runs against that build
- **THEN** the test fails on the count of files parsed
- **TEST** `crates/archangel_engine/tests/work.rs` › "edit_work_count_rejects_a_full_check_per_edit"

#### Scenario: The fault control for edit evaluations

- **GIVEN** a build of archangel with a fault that evaluates every file's imports after each edit
- **WHEN** the edit work test runs against that build
- **THEN** the test fails on the count of rule evaluations
- **TEST** `crates/archangel_engine/tests/work.rs` › "edit_work_count_rejects_an_evaluation_of_every_file"

### Requirement: The server follows files that are created, deleted and renamed

The server MUST ask the editor to report changes to every file that the check depends on: source files, the config, the baseline, `.gitignore` files, and every file that a resolution read or found. When the editor cannot report them, the server MUST watch the files itself, with the same watcher as watch mode. After files are created, deleted or renamed, the server MUST resolve all imports again, one time for all the changes that wait. It MUST parse only a created or renamed file.

#### Scenario: A deleted file

- **GIVEN** a file `src/b.ts` that `src/a.ts` imports
- **WHEN** the editor reports that `src/b.ts` was deleted
- **THEN** the server publishes an `import-unresolved` diagnostic for the import in `src/a.ts`
- **AND** the work count of files parsed for the change is zero
- **TEST** `crates/archangel_lsp/tests/server.rs` › "deleted_file_makes_its_importers_unresolved"
- **TEST** `crates/archangel_engine/tests/work.rs` › "file_deletion_parses_no_file"

#### Scenario: The fault control for a deletion

- **GIVEN** a build of archangel with a fault that makes a full check after each file event
- **WHEN** the file deletion work test runs against that build
- **THEN** the test fails on the count of files parsed
- **TEST** `crates/archangel_engine/tests/work.rs` › "file_event_work_count_rejects_a_full_check_per_event"

#### Scenario: A deleted CSS file

- **GIVEN** a source file that imports `./a.module.css`
- **WHEN** the editor reports that `a.module.css` was deleted
- **THEN** the server publishes an `import-unresolved` diagnostic for that import
- **TEST** `crates/archangel_lsp/tests/server.rs` › "deleted_css_file_makes_its_importer_unresolved"

#### Scenario: A deleted file that the walk ignores

- **GIVEN** a source file that imports `./generated`, and `src/generated.ts`, which a `.gitignore` pattern matches
- **WHEN** the editor reports that `src/generated.ts` was deleted
- **THEN** the server publishes an `import-unresolved` diagnostic for that import
- **TEST** `crates/archangel_lsp/tests/server.rs` › "deleted_ignored_file_makes_its_importer_unresolved"

#### Scenario: A save by temporary file and rename

- **GIVEN** a repository with 2,000 source files, and `src/a.ts` with 6 imports
- **WHEN** the editor reports that a temporary file was created and renamed over `src/a.ts`
- **THEN** the work counts for the change are 1 file parsed and 6 resolutions
- **TEST** `crates/archangel_engine/tests/work.rs` › "save_by_rename_is_a_text_change"

#### Scenario: The fault control for a save by rename

- **GIVEN** a build of archangel with a fault that skips the comparison of waiting events with the known paths
- **WHEN** the save-by-rename test runs against that build
- **THEN** the test fails on the count of resolutions
- **TEST** `crates/archangel_engine/tests/work.rs` › "rename_save_work_count_rejects_events_that_are_not_netted"

#### Scenario: A save to an imported CSS file

- **GIVEN** a repository with 2,000 source files, and a source file that imports `./a.module.css`
- **WHEN** the editor reports a change to the text of `a.module.css`
- **THEN** the work counts for the change are zero files parsed and zero resolutions
- **TEST** `crates/archangel_engine/tests/work.rs` › "save_to_an_imported_css_file_costs_nothing"

#### Scenario: The fault control for an imported CSS file

- **GIVEN** a build of archangel with a fault that resolves all imports again for any file that a resolution found
- **WHEN** the CSS save test runs against that build
- **THEN** the test fails on the count of resolutions
- **TEST** `crates/archangel_engine/tests/work.rs` › "css_save_work_count_rejects_a_resolution_per_found_file"

#### Scenario: A branch switch changes many files

- **GIVEN** a repository with 2,000 source files and 9,000 imports
- **WHEN** the editor reports 500 created files in five notifications before the server handles the first
- **THEN** the work counts for all five are 500 files parsed and one resolution of every import, the new files' imports included
- **TEST** `crates/archangel_engine/tests/work.rs` › "many_file_events_resolve_all_imports_one_time"

#### Scenario: The fault control for many file events

- **GIVEN** a build of archangel with a fault that resolves all imports again for each file event
- **WHEN** the many-file-events test runs against that build
- **THEN** the test fails on the count of resolutions
- **TEST** `crates/archangel_engine/tests/work.rs` › "file_event_work_count_rejects_a_resolution_per_event"

#### Scenario: An editor that cannot report file changes

- **GIVEN** an editor whose client capabilities do not permit the server to register file watches
- **WHEN** a file that another file imports is deleted on disk
- **THEN** the server publishes an `import-unresolved` diagnostic for the importing file
- **TEST** `crates/archangel_lsp/tests/server.rs` › "server_watches_files_itself_when_the_editor_cannot"

#### Scenario: The server's watcher lost events

- **GIVEN** a build of archangel whose file watcher reports lost events as it starts, which the `engine::watcher_reports_lost_events` fail point causes
- **WHEN** the server watches the files itself
- **THEN** it makes a full check after the start's, so its work counts hold two parses of every file
- **TEST** `crates/archangel_lsp/tests/server.rs` › "lost_events_make_a_full_check"

#### Scenario: A change to a tsconfig

- **GIVEN** a server that runs, and a path alias in a tsconfig
- **WHEN** the editor reports that the tsconfig now maps the alias to another folder
- **THEN** the server publishes diagnostics for the new resolutions
- **AND** the work counts for the change are zero files parsed and one resolution of every import
- **TEST** `crates/archangel_lsp/tests/server.rs` › "tsconfig_change_publishes_the_new_resolutions"
- **TEST** `crates/archangel_engine/tests/work.rs` › "tsconfig_change_parses_nothing_and_resolves_every_import_once"

#### Scenario: A tsconfig reached through extends

- **GIVEN** a tsconfig that extends `configs/base.json`, which maps a path alias
- **WHEN** the editor reports that `configs/base.json` maps the alias to another folder
- **THEN** the server publishes diagnostics for the new resolutions
- **TEST** `crates/archangel_lsp/tests/server.rs` › "extended_tsconfig_change_publishes_the_new_resolutions"

#### Scenario: A tsconfig reached through a link

- **GIVEN** a `tsconfig.json` that is a link to `configs/tsconfig.shared.json`, or that extends `shared/tsconfig.base.json` where `shared` is a link to the folder `configs`
- **WHEN** the files that decide resolutions are recorded
- **THEN** they hold the file at its real path in `configs`, where the watcher reports an edit to it
- **TEST** `crates/archangel_graph/tests/resolve.rs` › "resolution_inputs_hold_the_real_path_of_a_linked_tsconfig"
- **TEST** `crates/archangel_graph/tests/resolve.rs` › "resolution_inputs_hold_the_real_path_of_a_tsconfig_in_a_linked_folder"

#### Scenario: Many manifest changes at once

- **GIVEN** a server that runs on a repository with 9,000 imports
- **WHEN** the editor reports changes to 40 package manifests in four notifications before the server handles the first
- **THEN** the work counts for all four are zero files parsed and 9,000 resolutions
- **TEST** `crates/archangel_engine/tests/work.rs` › "many_manifest_changes_resolve_all_imports_one_time"

#### Scenario: The fault control for resolution inputs

- **GIVEN** a build of archangel with a fault that resolves all imports again for each manifest event
- **WHEN** the manifest change test runs against that build
- **THEN** the test fails on the count of resolutions
- **TEST** `crates/archangel_engine/tests/work.rs` › "manifest_change_work_count_rejects_a_resolution_per_event"

#### Scenario: A change to the baseline

- **GIVEN** a server that runs with three violations
- **WHEN** the developer runs `archangel baseline add`, and the editor reports the change to the baseline
- **THEN** the server publishes the three files with no violations
- **AND** the work counts for the change are zero files parsed, zero resolutions and zero rule evaluations
- **TEST** `crates/archangel_lsp/tests/server.rs` › "baseline_change_publishes_the_files_with_no_violations"
- **TEST** `crates/archangel_engine/tests/work.rs` › "baseline_change_applies_without_a_check"

#### Scenario: The fault control for a baseline change

- **GIVEN** a build of archangel with a fault that makes a full check after a baseline change
- **WHEN** the baseline change test runs against that build
- **THEN** the test fails on the count of files parsed
- **TEST** `crates/archangel_engine/tests/work.rs` › "baseline_change_work_count_rejects_a_full_check"

#### Scenario: A change to the config

- **GIVEN** a server that runs with a config
- **WHEN** the editor reports a change to the config that adds a `mayNotImport` rule
- **THEN** the server publishes the violations of the new rule
- **AND** the work counts for the change are zero files parsed and zero resolutions
- **TEST** `crates/archangel_lsp/tests/server.rs` › "config_change_publishes_the_new_rules_violations"
- **TEST** `crates/archangel_engine/tests/work.rs` › "config_change_parses_no_file"

#### Scenario: The fault control for a config change

- **GIVEN** a build of archangel with a fault that makes a full check after each config change
- **WHEN** the config change work test runs against that build
- **THEN** the test fails on the count of files parsed
- **TEST** `crates/archangel_engine/tests/work.rs` › "config_change_work_count_rejects_a_full_check"

#### Scenario: A tsconfig named by project references

- **GIVEN** a root `tsconfig.json` with `references` to `tsconfig.app.json`, which maps the path alias `@/*`
- **WHEN** the editor reports that `tsconfig.app.json` maps `@/*` to another folder
- **THEN** the server publishes diagnostics for the new resolutions
- **TEST** `crates/archangel_lsp/tests/server.rs` › "referenced_tsconfig_change_publishes_the_new_resolutions"

#### Scenario: A package manifest that no resolution read

- **GIVEN** a server that runs, and a workspace package whose manifest no resolution read
- **WHEN** the editor reports that its manifest now lists `react` as a dependency
- **THEN** the work counts for the change are zero files parsed and zero resolutions
- **TEST** `crates/archangel_engine/tests/work.rs` › "unread_manifest_change_parses_and_resolves_nothing"

#### Scenario: The fault control for an unread manifest

- **GIVEN** a build of archangel with a fault that resolves all imports again for each manifest event
- **WHEN** the unread manifest test runs against that build
- **THEN** the test fails on the count of resolutions
- **TEST** `crates/archangel_engine/tests/work.rs` › "unread_manifest_work_count_rejects_a_resolution_per_event"

#### Scenario: A manifest that the resolver read once

- **GIVEN** a server that runs, and an edit that adds an import of a package whose manifest the resolver already read for another file
- **WHEN** the editor then reports a change to that manifest's `exports` field
- **THEN** the work counts for the change are one resolution of every import
- **TEST** `crates/archangel_engine/tests/work.rs` › "manifest_read_before_an_edit_still_triggers_a_resolution"

#### Scenario: A change to the Plug'n'Play file

- **GIVEN** a server that runs on a Yarn Plug'n'Play repository
- **WHEN** the editor reports a change to `.pnp.cjs` that removes the package `left-pad`
- **THEN** the server publishes an `import-unresolved` diagnostic for each import of `left-pad`
- **TEST** `crates/archangel_lsp/tests/server.rs` › "pnp_file_change_publishes_the_new_resolutions"

#### Scenario: A change to the config's ignore list

- **GIVEN** a server that runs, and a source file `fixtures/a.ts` with a violation
- **WHEN** the editor reports that the config now ignores `fixtures/`
- **THEN** the server publishes no diagnostics for `fixtures/a.ts`
- **TEST** `crates/archangel_lsp/tests/server.rs` › "config_ignore_change_removes_ignored_files_diagnostics"

#### Scenario: The server says when it polls

- **GIVEN** an editor that cannot register file watches, and a build whose watcher cannot add a watch, which the `engine::watcher_has_no_watches` fail point causes
- **WHEN** the server starts
- **THEN** the server sends a message to the editor that says it polls for changes and names the limit
- **TEST** `crates/archangel_lsp/tests/server.rs` › "server_says_when_it_polls"

#### Scenario: A change to .gitignore

- **GIVEN** a server that runs, and a source file `src/gen/a.ts` with a violation
- **WHEN** the editor reports that `.gitignore` now ignores `src/gen/`
- **THEN** the server publishes no diagnostics for `src/gen/a.ts`
- **AND** the work counts for the change are zero files parsed and one resolution of every import
- **TEST** `crates/archangel_lsp/tests/server.rs` › "gitignore_change_removes_ignored_files_diagnostics"
- **TEST** `crates/archangel_engine/tests/work.rs` › "gitignore_change_parses_nothing_and_resolves_once"

#### Scenario: A config that is not valid

- **GIVEN** a server that runs with a valid config
- **WHEN** the developer saves the config with an unknown field
- **THEN** the server publishes a `config-invalid` diagnostic on the config file and removes the other diagnostics
- **TEST** `crates/archangel_lsp/tests/server.rs` › "invalid_config_replaces_the_diagnostics_with_its_own"

### Requirement: Editor sessions give the same result as a full check

After any sequence of edits, creations, deletions and renames of source files, tsconfigs, package manifests, `.gitignore` files and the config, the diagnostics MUST be the same as the diagnostics of a full check of the same files from the start.

#### Scenario: Random editor sessions

- **GIVEN** random sequences of edits, creations, deletions and renames of source files, tsconfigs with `extends` and `references`, package manifests, `.gitignore` files and the config, on a fixture repository
- **WHEN** the server receives each sequence
- **THEN** after each step, the server's diagnostics are the same as a full check of the files at that step
- **TEST** `crates/archangel_engine/tests/sessions.rs` › "random_sessions_agree_with_a_full_check"

### Requirement: The server offers checked fixes as code actions

When the editor asks for code actions on a range that has a violation with a fix, the server MUST offer the fix as a quick fix. It MUST NOT offer an action for a violation without a fix.

#### Scenario: A quick fix for an entry violation

- **GIVEN** an open document with an entry violation that has a fix
- **WHEN** the editor asks for code actions on the specifier
- **THEN** the server offers one quick fix that changes the specifier to the entry file
- **TEST** `crates/archangel_lsp/tests/server.rs` › "entry_violation_offers_its_fix_as_a_quick_fix"

### Requirement: The language server stops when the editor says so

On a shutdown request followed by the exit notification, the server MUST answer the request and stop without an error, so that the editor sees exit status 0, however the two messages are queued.

#### Scenario: Shutdown and exit arrive together

- **GIVEN** an editor that queues initialize, initialized, shutdown and exit before the server reads any of them
- **WHEN** the server runs
- **THEN** it stops without an error
- **TEST** `crates/archangel_lsp/tests/server.rs` › "shutdown_then_exit_in_one_batch_stops_the_server_cleanly"
