# documentation

## ADDED Requirements

### Requirement: Each README sends the reader to the docs site

The README and the npm README MUST link to the docs site above their first section. The full documentation MUST be the site only. The repository MUST NOT hold a second copy of its pages.

#### Scenario: The link to the docs

- **GIVEN** the README and the npm README
- **WHEN** the test reads each one
- **THEN** each links to `https://enhancelabs.ai/archangel/` above its first section
- **TEST** `crates/archangel/tests/doc_examples.rs` › "each_readme_links_to_the_docs_before_its_first_section"

## MODIFIED Requirements

### Requirement: Every rule fragment in the docs loads

Each rule written in a TypeScript block, or as inline code, in the README or the npm README MUST load in a config that declares its names. A test MUST run each one, so that a docs example cannot stop working.

#### Scenario: A fragment that does not load

- **GIVEN** a doc with the inline code `rule('a', { files: 'shared', mayNotImport: 'react' })`, whose bare package name the config refuses
- **WHEN** the test reads the doc
- **THEN** the test names the doc and the line of the fragment
- **TEST** `crates/archangel/tests/doc_examples.rs` › "a_fragment_that_does_not_load_is_named"

#### Scenario: The fragments of the docs

- **GIVEN** the README and the npm README
- **WHEN** the test runs each rule fragment in a config
- **THEN** no fragment gives a config error
- **TEST** `crates/archangel/tests/doc_examples.rs` › "every_rule_fragment_in_the_docs_loads"

### Requirement: Every config example in the docs is a checked fixture

Each config example in the README and the npm README MUST be the text of a fixture in `examples/`. A fixture MUST hold the config, the source files, and the expected output of each command that the docs show for it. A test MUST run each command on each fixture and compare the output with the expected output. A test MUST read each config block in the READMEs and find the fixture whose config has the same text.

#### Scenario: An example that disagrees with archangel

- **GIVEN** a fixture whose expected output says no problem, and whose config refuses one of its imports
- **WHEN** the test of the fixtures runs
- **THEN** the test fails and names the fixture and the first line that differs
- **TEST** `crates/archangel/tests/doc_examples.rs` › "a_fixture_whose_output_differs_fails"

#### Scenario: A config block with no fixture

- **GIVEN** a config block in a README whose text is the text of no fixture
- **WHEN** the test of the docs runs
- **THEN** the test fails and names the file and the line of the block
- **TEST** `crates/archangel/tests/doc_examples.rs` › "every_config_example_in_the_docs_is_a_checked_fixture"

### Requirement: The small app fixture shows the whole loop

The fixture is `examples/small-app/`, and its config is the one in design.md, "In short". It MUST have features in `src/features/<name>/`, shared code in `src/shared/`, and the two tables `app` and `feature-code`. Its views MUST import only models and shared code, except one view that imports an api file. The README MUST show its config in full, and the error that `archangel check` gives.

#### Scenario: The small app

- **GIVEN** the small-app fixture
- **WHEN** the test runs `archangel check`, `archangel test` and `archangel rules` on it
- **THEN** `archangel check` gives one violation of `feature-code.views` and exits with status 1
- **AND** `archangel test` exits with status 0 with no written example and no `rule-unproven` diagnostic
- **AND** each output is the expected output of the fixture
- **TEST** `crates/archangel/tests/doc_examples.rs` › "the_small_app_gives_its_documented_output"

#### Scenario: The README shows the small app and its error

- **GIVEN** the README
- **WHEN** the test reads it
- **THEN** it holds one config block whose text is the config of `examples/small-app/`, and one output block whose text is the fixture's expected output of `archangel check`
- **TEST** `crates/archangel/tests/doc_examples.rs` › "the_readme_shows_the_small_app_and_its_error"

### Requirement: The monorepo slice fixture checks a monorepo architecture

The fixture is `examples/monorepo-slice/`. It is a small monorepo of apps and packages. It MUST hold a workspace table of its apps and packages, an API domain table, and the rules `frontend-api-types-only` and `domain-not-to-pool`, with a small set of files. Its files MUST include one import that each table refuses.

#### Scenario: The monorepo slice

- **GIVEN** the monorepo-slice fixture
- **WHEN** the test runs `archangel check` and `archangel test` on it
- **THEN** `archangel check` gives one violation of `workspace.contract` for an import of `web`, and one violation of `domain.entity` for an import of a use case
- **AND** each output is the expected output of the fixture
- **TEST** `crates/archangel/tests/doc_examples.rs` › "the_monorepo_slice_gives_its_documented_output"
