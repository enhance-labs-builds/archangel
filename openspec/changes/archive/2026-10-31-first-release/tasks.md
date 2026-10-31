# Tasks

## 1. Tests first

Each task writes the tests for one spec, one test per TEST line, named exactly as the TEST line. Each
test has a kind from the list in `openspec/config.yaml`: unit, differential, fuzz, integration, editor,
release or benchmark. Run each new test and confirm that it fails because the behaviour is missing, not
because the test cannot build its fixture.

- [x] 1.1 Write the configuration tests: unit tests in `crates/archangel_engine/tests/load.rs`, and in `crates/archangel_config/tests/validate.rs` and `pattern.rs`; unit property tests in `crates/archangel_config/tests/pattern_properties.rs`; unit tests in `crates/archangel_rules/tests/elements.rs`; the unit test for built-in modules in `crates/archangel_graph/tests/resolve.rs`; integration tests in `crates/archangel/tests/config_discovery.rs` and `cli.rs`. Verify each fails for the missing behaviour.
- [x] 1.2 Write the import graph tests: unit tests in `crates/archangel_graph/tests/files.rs`, `imports.rs`, `resolve.rs` and `work.rs`; the differential test in `crates/archangel_graph/tests/resolve_differential.rs`; integration tests in `crates/archangel/tests/cli.rs`. Verify each fails for the missing behaviour.
- [x] 1.3 Write the architecture rule tests: unit tests in `crates/archangel_rules/tests/rules.rs` (the permitted unresolved specifier included) and `work.rs`; unit property tests in `graph_properties.rs` and `rule_properties.rs`. Verify each fails for the missing behaviour.
- [x] 1.4 Write the rule example tests: unit tests in `crates/archangel_rules/tests/examples.rs` and the example work counts in `crates/archangel_engine/tests/test_command.rs`; integration tests in `crates/archangel/tests/test_command.rs` for exit statuses and files written. Verify each fails for the missing behaviour.
- [x] 1.5 Write the exception and baseline tests: unit tests in `crates/archangel_rules/tests/exceptions.rs` and `baseline.rs`; integration tests in `crates/archangel/tests/baseline_command.rs`, `cli.rs` and `output.rs`. Verify each fails for the missing behaviour.
- [x] 1.6 Write the diagnostic tests: unit tests in `crates/archangel_rules/tests/diagnostics.rs` and `crates/archangel_engine/tests/fixes.rs`; integration tests in `crates/archangel/tests/output.rs`, with an `insta` snapshot for the text output. Verify each fails for the missing behaviour.
- [x] 1.7 Write the command line tests: integration tests in `crates/archangel/tests/cli.rs`, `output.rs` and `watch.rs`. The watch tests wait for the output line of the check that answers each change, never for a duration. Verify each fails for the missing behaviour.
- [x] 1.8 Write the language server tests: unit work-count tests in `crates/archangel_engine/tests/work.rs` and the simulated sessions in `crates/archangel_engine/tests/sessions.rs`, both driving `Session::apply` directly; editor tests in `crates/archangel_lsp/tests/server.rs`, driven over an in-memory connection, each waiting for the diagnostics of the exact document version it sent. Verify each fails for the missing behaviour.
- [x] 1.9 Write the fuzz targets `config`, `pattern` and `exception` in `fuzz/fuzz_targets/`, kind fuzz. Verify each builds with `cargo fuzz build` and finds the missing parser.
- [x] 1.10 Write the distribution tests: unit tests of the npm package in `npm/archangel/test/platform.test.mjs` (Node's test runner) and `npm/archangel/test/types.test-d.ts` (`tsd`); release jobs `install-from-npm`, `install-from-crates-io`, `checksums-match` and `provenance-verified` in `.github/workflows/release.yml`, kind release. Verify the npm tests fail for the missing package and `actionlint` accepts the workflow.

## 2. Workspace and test support

- [x] 2.1 Turn `Cargo.toml` into a workspace with the seven crates, move the lints to `[workspace.lints]`, and remove `src/lib.rs`. Verify `scripts/check` passes on the empty crates.
- [x] 2.2 Add the dependencies from the proposal at exact versions, and add each licence that they need to `deny.toml`, with no wildcard. Verify `cargo deny check` passes.
- [x] 2.3 Build `FixtureRepo` in `archangel_testkit`, and its synthetic repository generator. Verify a generated repository of 10,000 files has the requested number of files and imports.
- [x] 2.4 Add the `failpoints` feature to every crate, each turning on the features of the crates it depends on, and add `cargo test --workspace --features failpoints` to `scripts/check`. Verify with `cargo tree -e features --release` that no release build enables it, and that the feature run lists every fault-control test in the specs as run, none skipped.

## 3. Configuration

- [x] 3.1 Implement the config types, JSON loading and validation that reports every problem. Verify the `validate.rs` tests and the JSON tests in `load.rs` pass.
- [x] 3.2 Implement the Node version check, the Node loader and the TypeScript config path, with `config-needs-node` and `config-load-failed`. Verify the TypeScript tests in `load.rs` and `config_discovery.rs` pass.
- [x] 3.3 Implement `Pattern` and `ElementMatcher`, with captures, brace alternatives, instance roots and package names. Verify `pattern.rs` and `pattern_properties.rs` pass.
- [x] 3.4 Implement config discovery and the `--config` option. Verify the discovery tests in `config_discovery.rs` pass.

## 4. Import graph

- [x] 4.1 Implement `SourceSet::discover` with the repository folder (a `.git` folder or file), the list of nested repositories, the walker settings in the design, the list of links not followed, `file-unreadable`, the size check, UTF-16 decoding and `ManifestDependencies`. Verify `files.rs` passes.
- [x] 4.2 Implement `imports::extract` on a 64 MB parse stack, with JSX permitted in JavaScript files: the module record, the walk behind the text search, triple-slash references, type import marks, imported and exported names, exception comment text, computed specifiers, and `parse-failed` with every parser error and the Flow message. Verify `imports.rs` passes, the 10,000-level nesting test included.
- [x] 4.3 Implement `Overlay` and resolution: tsconfig discovery per importing file with `TsconfigDiscovery::Auto`, references and `extends` chains recorded, and the readability check; `resolve_with_context`, `ResolutionInputs` with its contents and existence sets, `cwd`, `node_path: false` and Plug'n'Play only with `.pnp.cjs`; packages, built-ins, Plug'n'Play; the `ResolveError` mapping to plain reasons with its wildcard arm and the variant-list test; and one `resolution-file-invalid` per tsconfig or manifest. Verify `resolve.rs` passes.
- [x] 4.4 Write `scripts/resolve-with-typescript.mjs` and the resolution fixtures, with path aliases, package exports, workspace links and `.js` specifiers. Verify `resolve_differential.rs` passes.
- [x] 4.5 Implement `Work` for the graph stages and the `graph::parse_twice` fail point. Verify both tests in `crates/archangel_graph/tests/work.rs` pass, the second by its failure under the fault.

## 5. Architecture rules

- [x] 5.1 Implement `Architecture::compile`, classification, `element-empty` with `ManifestDependencies`, and `allowUnresolved`. Verify `elements.rs` and the unresolved test in `rules.rs` pass.
- [x] 5.2 Implement `RuleTable` and the evaluation of layers, isolate, entry, only and forbid, with the `imports` option and the `rules::search_every_rule` fail point. Verify the matching tests in `rules.rs`, `work.rs` and `rule_properties.rs` pass.
- [x] 5.3 Implement petgraph's traits on `ImportGraph`, the cycle groups with every file listed, the group identity from its sorted files, the shortest cycle in each group, and the count of files visited. Verify the no-cycles tests in `rules.rs`, the 200,000-file chain included, and `graph_properties.rs` pass.

## 6. Exceptions and baselines

- [x] 6.1 Implement exception comments, runs of consecutive exceptions, and the three exception diagnostics: `exception-without-reason`, `exception-unknown-rule` and `exception-unused`. Verify `exceptions.rs` passes.
- [x] 6.2 Implement `ViolationId::baseline_key`, for imports and for cycle groups, and baseline matching in `archangel_rules`, and the baseline file, `baseline-invalid`, `baseline-stale`, `baseline add` and `baseline prune` in `archangel_engine` and the binary. Verify `baseline.rs` and `baseline_command.rs` pass.
- [x] 6.3 Write `docs/exceptions-and-baselines.md`, which explains when to use each. Verify its examples match the tests' fixtures.

## 7. Engine and rule examples

- [x] 7.1 Implement `Session::open`, which connects the config, the graph, the rules and the baseline. Verify the integration tests in `cli.rs` that need a full check pass.
- [x] 7.2 Implement `ExampleView`, the right-reason rules for both kinds of example, the cycle search over example files and the repository files that reach a replaced file with the `rules::search_whole_graph_per_example` fail point, and `rule-unproven`. Verify `examples.rs` passes, the fault control included.
- [x] 7.3 Implement `archangel test` with the `engine::reread_per_example`, `engine::evaluate_graph_per_example` and `engine::write_examples` fail points. Verify both `test_command.rs` files pass, the fault controls included.

## 8. Diagnostics and fixes

- [x] 8.1 Implement `Diagnostic`, `Code`, `ViolationId`, the help sentence for each rule kind, the documentation link, and the sort order. Verify `diagnostics.rs` passes.
- [x] 8.2 Implement the entry fix in `archangel_engine`: the shorter specifier, the export names through `export *` chains, the check against every rule, and the `engine::reparse_per_fix` fail point. Verify `fixes.rs` passes.
- [x] 8.3 Write `docs/diagnostics/<code>.md` for every code. Verify the documentation test passes.

## 9. Command line

- [x] 9.1 Implement `check`, the text and JSON renderers, `--stats`, and the exit statuses. Verify `cli.rs` and `output.rs` pass, the snapshot and the 5,000-violation test included.
- [x] 9.2 Implement `explain` and `why`. Verify their tests in `cli.rs` pass.
- [x] 9.3 Write the README's guide: install, a first config, the commands, and one example of each rule kind. Verify every command in it runs as written against a fixture.

## 10. Watch mode and the language server

- [x] 10.1 Implement `Session::apply`, the five kinds of change, and `events.rs` in the order the design gives, which nets each batch against the known paths, treats a save by rename as a text change, walks again after an ignore-rule change, and costs nothing for a content change to a found file; with the fail points `engine::skip_netting` and `engine::resolve_per_found_file`; with the merge of waiting file and manifest events and the fail points `engine::full_check_per_edit`, `engine::evaluate_every_file_per_edit`, `engine::full_check_per_event`, `engine::resolve_per_event`, `engine::full_check_per_config` and `engine::full_check_per_baseline`. Verify `crates/archangel_engine/tests/work.rs` passes, the fault controls included.
- [x] 10.2 Implement the language server: initialize, one `**/*` watch registration or the engine's watcher with a polling message, document changes with queued versions collapsed, disk changes to open documents ignored, closing unsaved documents, versioned diagnostics for changed files only, and code actions. Verify `server.rs` passes.
- [x] 10.3 Implement the simulated sessions with `proptest-state-machine` in `crates/archangel_engine/tests/sessions.rs`, with edits to source files, tsconfigs with `extends` and `references`, manifests, `.gitignore` and the config among the random steps. Verify `sessions.rs` passes with 1,000 random sessions.
- [x] 10.4 Implement the engine's `watcher.rs` on `notify`, with a full check after lost events and polling when the system has no watches left, and the fail points `engine::watcher_reports_lost_events` and `engine::watcher_has_no_watches`, and `archangel watch` on it, with one `Session::apply` per batch of events and text and JSON lines. Verify `watch.rs` passes, the watch work count and its fault control included.
- [x] 10.5 Write `docs/editors.md`, which says how to start the server in VS Code, Neovim and Zed and how a coding agent reads watch mode. Verify each config in it starts the server against a fixture.

## 11. Test layers and measurements

- [x] 11.1 Add the scheduled nightly-Rust CI job for the fuzz targets, with the hand-written seeds and each crash input in `fuzz/seeds/`, and the corpus kept in the Actions cache from run to run. Verify each target runs for one minute locally without a crash.
- [x] 11.2 Add `cargo mutants --in-diff` on the pull request's diff file to pull request CI and a weekly full run, with `.cargo/mutants.toml` for exclusions and their reasons. Verify a deliberately weakened test makes the run fail.
- [x] 11.3 Add `scripts/bench` on `hyperfine`, `benches/repos.toml` and the scheduled benchmark job. Measure the costs that the design estimates: classification and rule evaluation, the recording of resolution inputs and the size of those sets, the Kosaraju and shortest-cycle search on a graph with large cycle groups, the walk for require calls and import types, the comment scan, the Node load, an edit to an ordinary file with imports, and memory. Write the numbers into the Cost section. Verify the job uploads one result per repository.

## 12. Distribution

- [x] 12.1 Build `npm/archangel` with `defineConfig`, the types and the launcher, and the seven platform packages. Verify `platform.test.mjs` and `types.test-d.ts` pass.
- [ ] 12.2 Complete `.github/workflows/release.yml`: the build matrix, `cargo publish --workspace` and `npm publish` with trusted publishing and no stored tokens, and the checksums. Verify with a dry run on a tag in a fork, where the four release jobs pass against a local registry.
- [x] 12.3 Add archangel's own config that checks the TypeScript in `npm/`. Verify `archangel check` passes on the repository.

## 13. Launch readiness

An outside review before the announcement found what a first-time user and a contributor would miss.

- [x] 13.1 Add `--format github` and `--format sarif` to `check` and `test`, JSON to `explain` and `why`, and a refusal for a format a command cannot print. Verify `formats.rs` passes.
- [x] 13.2 Add `archangel docs`, `init`, `completions`, `man` and `--color`; compile each code's page into the binary from `crates/archangel_rules/docs/`. Verify `setup.rs` and `diagnostics.rs` pass.
- [x] 13.3 Give config problems a line, a column and the nearest name; accept `$schema` and ship `archangel.schema.json` in the npm package. Verify `validate.rs`, `schema.rs` and `schema.test.mjs` pass.
- [x] 13.4 Test on macOS and Windows in CI, attach the manual and completions to each release, let Dependabot update npm, and add the crate metadata, CHANGELOG, SECURITY, CODE_OF_CONDUCT and the issue and pull request templates.
- [x] 13.5 Ship the VS Code extension in `editors/vscode`, and run its tests in CI. Verify its integration test shows a diagnostic from the server.
- [x] 13.6 Stop the language server cleanly when shutdown and exit arrive in one batch, which the extension's restarts found. Verify `server.rs` passes.

## 14. The rule language, presets and plugins

### 14a. Tests first

Each task writes the tests for its spec, one per TEST line, named exactly as the TEST line, and
confirms that each fails because the behaviour is missing. Kinds are as in `openspec/config.yaml`.

- [x] 14.1 Selectors and the shorthand sentences (`selectors`, and `architecture-rules` from "A rule is one selector and one condition" on): unit tests in `crates/archangel_rules/tests/selectors.rs`, `selector_properties.rs`, `conditions.rs`, `work.rs`, `rule_properties.rs` (the transitive and shorthand properties) and `diagnostics.rs`; unit tests in `crates/archangel_config/tests/validate.rs`; unit tests in `crates/archangel_engine/tests/sessions.rs` and `work.rs`; the integration test `explain_lists_selectors_and_rule_sentences` in `crates/archangel/tests/cli.rs`; the type tests for sentences in `npm/archangel/test/types.test-d.ts`; and the elements test for an uninstalled forbidden package. Rewrite the existing tests of forbid, only and the noCycles kind in `rules.rs`, `work.rs` and `rule_properties.rs` as the sentence tests that the spec now names. Verify each new test fails for the missing behaviour.
- [x] 14.2 Structure and exports (`structure-rules`, `export-rules`, and the identity requirement in `diagnostics`): unit tests in `crates/archangel_rules/tests/structure.rs`, `structure_properties.rs`, `exports.rs`, `baseline.rs`, `exceptions.rs` and `examples.rs` (the exception and example requirements this change touches); unit tests in `crates/archangel_engine/tests/work.rs` and `sessions.rs`. Verify each fails for the missing behaviour.
- [x] 14.3 Presets (`presets`): unit tests in `crates/archangel_config/tests/merge.rs` and `validate.rs`; integration tests in `crates/archangel/tests/presets.rs`; the differential test `crates/archangel/tests/fsd_differential.rs` with Steiger 0.8.1 and its plugin pinned in the root `package.json`, and Feature-Sliced Design fixtures in `tests/fixtures/fsd/` that together break each of the 21 rules. Verify each fails for the missing behaviour.
- [x] 14.4 Plugins (`plugins`): unit tests in `crates/archangel_engine/tests/plugins.rs` and `crates/archangel_config/tests/validate.rs`; integration tests in `crates/archangel/tests/plugins.rs`; editor tests in `crates/archangel_lsp/tests/server.rs`, with a fixture plugin in `crates/archangel_testkit` whose condition can throw, stop its process, or wait on a gate the test opens. Verify each fails for the missing behaviour.

### 14b. Build

- [x] 14.5 Keep each file's class in the session, classify only the edited file, update importer lists for the edited file's old and new targets only, and count classifications in `Work`, with the fail point `engine::classify_every_file_per_edit`. This fixes the per-edit cost the design already promised. Verify `an_edit_classifies_one_file` and its fault control pass, and every test from sections 1 to 13 still passes.
- [x] 14.6 Implement selectors, string resolution on rule sides, inline package elements with `*` and `node:*`, classes and the class-pair table filled for connected pairs before evaluation, the file counts per class, `selector-empty` with its exemptions, the re-check of a file's importers when its class changes, and the fail point `rules::match_per_selector`. Expand layers, isolate and entry into sentences in `parse.rs`; remove the forbid, only and noCycles kinds, with the `config-invalid` message that gives each one's sentence. Update `defineConfig`'s types, `archangel.schema.json`, the docs and the README for `selectors`, `rule()` and the conditions. Verify the 14.1 selector, configuration and shorthand tests pass.
- [x] 14.7 Implement `rule()` and the import conditions with `names`, `specifier`, `instance`, `dependency` and `transitively`; the backward walk for transitive rules with its shortest chains and the fail point `rules::walk_per_file`; and manifest changes that re-check only their package's imports. Verify the rest of the 14.1 tests pass.
- [x] 14.8 Implement the paths of files the walk does not read, the new file facts (export kinds, own statements, directive), names and full names, the five styles and the `{FileName}` spellings, the structure and export conditions, `exports-unknown`, the `export *` walk shared with the entry fix, and the fail points `engine::evaluate_every_folder_per_event` and `engine::evaluate_exports_of_every_file`. Verify the 14.2 tests pass.
- [x] 14.9 Implement the plugin process in `crates/archangel_engine/src/plugins.rs`, which starts `load.mjs` and receives the config first; the protocol with one message per record; standard error on its own thread; filters; changed records after each batch; examples as batches with a revert; `plugin-failed` and restarts; progress messages in the editor; `definePlugin`, `defineCondition` and the view types in `npm/archangel/plugin.d.ts`, with the helpers in `index.js`, and `plugins` in `defineConfig`'s types and `archangel.schema.json`; option validation with `jsonschema`; and the fail points `engine::plugin_dies` and `engine::send_every_file`, and the gated fixture plugin. Verify the 14.4 tests pass.
- [x] 14.10 Implement `extends`, with `extends` and `off` in `defineConfig`'s types and `archangel.schema.json`, `merge.rs` with prefixes, replacement by name, `off`, a preset used twice, empty preset elements in the summary and `preset-empty`; the `fsd` preset with its `strict` option, fourteen built-in rules and the `fsd` plugin's seven conditions, each with examples; and the `nextjs` preset. Verify the 14.3 tests pass, the differential test included.

### 14c. Documentation and measurement

- [x] 14.11 Write the diagnostic pages in `crates/archangel_rules/docs/`, which `archangel docs` builds in, for `selector-empty`, `exports-unknown`, `preset-empty` and `plugin-failed`, a page for each condition, the shorthand pages with their sentences, and the README's guide to selectors, sentences, presets and plugins, with a plugin written from start to end. Verify the documentation tests pass and every command in the README runs as written against a fixture.
- [x] 14.12 Rewrite the dogfood config by hand with selectors and sentences, without the translator, and measure the costs the design lists: the number of classes and connected pairs and the time to fill the table, whether the pattern set compiled as one, the structure, export and transitive conditions, the plugin view's size and time per check, and the Feature-Sliced Design plugin conditions after one edit in the editor. Write the numbers into the Cost section. Verify `archangel check` on the dogfood reports what dependency-cruiser, eslint-plugin-boundaries, eslint-plugin-check-file and Steiger report there for the rules it now holds.

## 15. Before the pull request

- [x] 15.1 Run `scripts/check` and every test the change touches, locally. Verify all pass with zero findings.

## Workflow follow-up

- Archive the change with `openspec archive first-release --yes` as the last commit of the pull request.
- Register the npm organisation for the platform packages, or rename them, before the first tag.
- With the first release, add the crates.io, npm and docs.rs badges to the README, and make the repository public before the tag, as npm provenance needs.
