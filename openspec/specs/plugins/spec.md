# plugins Specification

## Purpose
Plugins let a team or a library author add conditions that the built-in ones cannot say, written in
TypeScript over a view of the graph, and have them checked, proven, baselined and shown in the editor
like any other rule.

## Requirements

### Requirement: A plugin rule names its plugin's condition

A rule MUST be able to use a plugin condition with `{ files, plugin: '<plugin>/<condition>', options }`. Archangel MUST validate `options` against the condition's JSON Schema before any check, and MUST refuse with `config-invalid` a rule that names a plugin or condition the config does not load, listing the conditions it does load.

#### Scenario: A misspelt condition

- **GIVEN** a config that loads the plugin "team" with the condition `one-store-per-entity`, and a rule with `plugin: 'team/one-store-per-entities'`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at the rule, suggesting `team/one-store-per-entity`
- **TEST** `crates/archangel_config/tests/validate.rs` › "a_rule_that_names_a_missing_condition_is_refused"

#### Scenario: Options that break the schema

- **GIVEN** a condition whose schema needs a number `max`, and a rule that gives `max: 'twenty'`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at the rule's `options`, saying `max` must be a number
- **TEST** `crates/archangel_config/tests/validate.rs` › "plugin_options_are_checked_against_the_schema"

### Requirement: A plugin condition sees only what its filter covers

Archangel MUST evaluate each condition's filter itself, and MUST send the plugin process only the files that some condition's filter covers, with only the facts those conditions read. After a change it MUST send only the records that changed, where a record changes when its file's facts change or, for a condition that reads importers, when the files importing it change.

#### Scenario: A filter that covers one folder

- **GIVEN** a repository of 1,000 files, and one plugin condition that reads imports, on a rule whose `files` covers `src/entities/**`, 40 of the files
- **WHEN** archangel makes a check
- **THEN** the view that archangel sends holds 40 file records, each with its imports and without exports or importers
- **TEST** `crates/archangel_engine/tests/plugins.rs` › "the_view_holds_only_the_files_and_facts_a_filter_covers"

#### Scenario: An edit outside every filter

- **GIVEN** a session with that condition
- **WHEN** the developer edits a file outside `src/entities/**`
- **THEN** archangel sends nothing to the plugin process
- **TEST** `crates/archangel_engine/tests/plugins.rs` › "an_edit_outside_every_filter_runs_no_plugin"

#### Scenario: An edit outside the filter that adds an importer

- **GIVEN** a session with a condition that reads importers, on a rule whose `files` covers `src/entities/**`
- **WHEN** the developer adds, to a file outside `src/entities/**`, an import of `src/entities/user/index.ts`
- **THEN** archangel sends one changed record, for `src/entities/user/index.ts`, with the new importer
- **TEST** `crates/archangel_engine/tests/plugins.rs` › "a_new_importer_changes_the_imported_files_record"

#### Scenario: The fault control for the filter

- **GIVEN** a build of archangel with a fault that sends every file to the plugin process
- **WHEN** the view-size test runs against that build
- **THEN** the test fails on the number of records in the view
- **TEST** `crates/archangel_engine/tests/plugins.rs` › "view_size_rejects_a_view_of_every_file"

### Requirement: A plugin violation is a diagnostic like any other

Each violation that a condition reports MUST become a diagnostic with the rule's name, reason and link, subject to exceptions, the baseline, the sort order and every output format. Its identity MUST be the rule, the file and, when it names an import, the import's specifier and occurrence, and MUST NOT include the message. A violation MAY name a folder instead of a file, when the folder holds a file of the condition's view; it then names no import.

#### Scenario: A plugin violation in the baseline

- **GIVEN** a plugin rule with one violation whose message carries a count, and `archangel baseline add` run once
- **WHEN** the count in the message changes and archangel makes a check
- **THEN** archangel gives no violation and counts one baseline match for the plugin rule
- **TEST** `crates/archangel/tests/plugins.rs` › "a_plugin_violation_is_baselined_without_its_message"

#### Scenario: An exception for a plugin rule

- **GIVEN** a plugin rule that reports an import, and `// archangel-allow team/one-store: migrating` above that import
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation and counts one exception for the plugin rule
- **TEST** `crates/archangel/tests/plugins.rs` › "an_exception_permits_a_plugin_violation"

#### Scenario: Plugin violations in every format

- **GIVEN** a plugin rule with violations in two files, and a built-in rule with one
- **WHEN** archangel makes a check with each output format
- **THEN** each format lists the three violations, sorted by file, line, column and rule, with the plugin rule's name
- **TEST** `crates/archangel/tests/plugins.rs` › "plugin_violations_appear_in_every_format_in_order"

#### Scenario: A violation of a folder

- **GIVEN** a plugin rule over `entity` whose condition reports the folder `src/entities/user`, which holds a file of its view
- **WHEN** archangel makes a check
- **THEN** archangel gives one `plugin` violation whose file is `src/entities/user`, which only the baseline can permit, as a structure rule's folder violation
- **TEST** `crates/archangel/tests/plugins.rs` › "a_plugin_violation_may_name_a_folder_that_holds_a_file_of_its_view"

#### Scenario: A violation of a folder outside the view

- **GIVEN** the same rule, whose condition reports the folder `src/shared`, which holds no file of its view
- **WHEN** archangel makes a check
- **THEN** archangel gives one `plugin-failed` diagnostic that names `src/shared`
- **TEST** `crates/archangel/tests/plugins.rs` › "a_plugin_violation_of_a_folder_outside_its_view_is_plugin_failed"

### Requirement: A plugin rule is proven by its examples

A plugin rule MUST be proven by `archangel test` like any other rule: the condition runs on each example over the view, the right-reason rules apply, and a plugin rule with no rejected example MUST fail as `rule-unproven`. Archangel MUST send the view once and each example as a batch of changed records that it takes back after the example.

#### Scenario: A plugin rule proven by its examples

- **GIVEN** a plugin rule with one accepted and one rejected example that the condition treats correctly
- **WHEN** the developer runs `archangel test`
- **THEN** both examples pass, nothing is written to disk, and the view is sent once
- **TEST** `crates/archangel/tests/plugins.rs` › "a_plugin_rule_is_proven_by_its_examples"

#### Scenario: A plugin rule without a rejected example

- **GIVEN** a plugin rule with only an accepted example
- **WHEN** the developer runs `archangel test`
- **THEN** the rule fails as `rule-unproven`
- **TEST** `crates/archangel/tests/plugins.rs` › "a_plugin_rule_without_a_rejected_example_is_unproven"

### Requirement: A plugin that fails never passes in silence

A condition that throws, a violation for a file outside the condition's view or for a folder that holds no file of it, and a plugin process that stops MUST each give one `plugin-failed` diagnostic that names the plugin, says what happened, and says the rule can be turned off with `off` or reported to the plugin's author; the check MUST fail. In the editor, the server MUST start the plugin process again on the next change.

#### Scenario: A condition that throws

- **GIVEN** a condition that throws `new Error('boom')`
- **WHEN** archangel makes a check
- **THEN** archangel gives one `plugin-failed` that names the plugin and `boom`, and exits with status 1
- **TEST** `crates/archangel/tests/plugins.rs` › "a_plugin_that_throws_fails_the_check"

#### Scenario: A plugin process that stops

- **GIVEN** a build of archangel with a fault that stops the plugin process
- **WHEN** archangel makes a check
- **THEN** archangel gives one `plugin-failed` that says the plugin process stopped, and exits with status 1
- **TEST** `crates/archangel/tests/plugins.rs` › "a_plugin_process_that_stops_fails_the_check"

#### Scenario: A violation for a file the condition was not given

- **GIVEN** a condition that reports a violation for a file outside its view
- **WHEN** archangel makes a check
- **THEN** archangel gives `plugin-failed` that names the plugin and the file, and exits with status 1
- **TEST** `crates/archangel/tests/plugins.rs` › "a_violation_outside_the_view_is_a_plugin_defect"

#### Scenario: A plugin process that stops in the editor

- **GIVEN** a language server session with a plugin, and a fault that stops the plugin process
- **WHEN** the developer edits a file in the plugin's filter, and then edits it again
- **THEN** the server publishes `plugin-failed` after the first edit, and the plugin's diagnostics again after the second
- **TEST** `crates/archangel_lsp/tests/server.rs` › "a_stopped_plugin_process_is_started_again"

### Requirement: Everything a plugin prints reaches the developer

Every line the config or a plugin prints MUST reach standard error, or the editor's log, with the name of the code that printed it: the plugin's name during its check, `config` while the config loads, and `plugin process` otherwise, when no one plugin is known to have printed it. A line written straight to standard output MUST be shown with `standard output`. A command MUST end the plugin process and wait for it to exit before it prints what the process printed, so that no line is lost.

#### Scenario: What a plugin printed during a check

- **GIVEN** a plugin condition that prints a line and stops its process
- **WHEN** archangel makes a check
- **THEN** standard error has the line, with the plugin's name
- **TEST** `crates/archangel/tests/plugins.rs` › "what_a_plugin_printed_goes_to_standard_error_after_a_check"

#### Scenario: What a plugin prints as its process ends

- **GIVEN** a plugin condition that prints a line only when its process's input closes
- **WHEN** archangel makes a check
- **THEN** standard error has the line, with `plugin process`
- **TEST** `crates/archangel/tests/plugins.rs` › "what_a_plugin_prints_as_its_process_ends_goes_to_standard_error"

#### Scenario: A line written straight to standard output

- **GIVEN** a plugin condition that writes a line to standard output past the console
- **WHEN** archangel makes a check
- **THEN** standard error has the line, with `standard output`
- **TEST** `crates/archangel/tests/plugins.rs` › "a_line_a_plugin_writes_to_standard_output_goes_to_standard_error"

### Requirement: Plugin diagnostics follow archangel's own in the editor

In watch mode and the language server, archangel MUST publish its own diagnostics for a change without waiting for the plugin process, MUST show that the plugin is running while it waits, and MUST publish the plugin diagnostics for the same document version when the process answers. A config with no plugin condition MUST leave no Node process running.

#### Scenario: An edit while a plugin is still working

- **GIVEN** a language server session with a plugin whose condition waits on a gate that the test holds closed
- **WHEN** the developer adds an import that a built-in rule refuses
- **THEN** the server publishes the built-in violation and a progress message that names the plugin, while the gate is closed
- **AND** after the test opens the gate, the server publishes the plugin's diagnostics for the same document version and ends the progress message
- **TEST** `crates/archangel_lsp/tests/server.rs` › "built_in_diagnostics_do_not_wait_for_plugins"

#### Scenario: A config without plugins

- **GIVEN** a TypeScript config with no plugin condition
- **WHEN** the language server starts
- **THEN** the Node process that loaded the config has exited before the first diagnostics are published
- **TEST** `crates/archangel_lsp/tests/server.rs` › "no_plugin_process_runs_without_plugin_conditions"

### Requirement: The view has a schema version and grows without breaking plugins

The view MUST carry `schemaVersion`. Adding a field to the view MUST NOT change the schema version, and a field's meaning MUST NOT change without a new one. A plugin that declares a schema version archangel does not send MUST be refused with `config-invalid` that names both versions.

#### Scenario: A plugin that ignores a new field

- **GIVEN** a plugin written against the view without its newest field
- **WHEN** archangel makes a check
- **THEN** the plugin's violations are the same as with the older view
- **TEST** `crates/archangel_engine/tests/plugins.rs` › "a_new_view_field_does_not_change_an_old_plugins_result"

#### Scenario: A plugin for another schema version

- **GIVEN** a plugin that declares `schemaVersion: 2`
- **WHEN** archangel, which sends version 1, loads the config
- **THEN** archangel gives `config-invalid` that names the plugin, version 2 and version 1
- **TEST** `crates/archangel_config/tests/validate.rs` › "a_plugin_for_another_schema_version_is_refused"

#### Scenario: The view types match the engine

- **GIVEN** the view types that the npm package declares
- **WHEN** the test compares them with the view that the engine sends
- **THEN** every field the engine sends is declared, with the same type
- **TEST** `crates/archangel_engine/tests/plugins.rs` › "the_declared_view_types_match_the_engine"
