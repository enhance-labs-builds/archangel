# Spec Delta

## Purpose

The documentation site teaches archangel with guides, concepts and a reference. This capability
makes each example on the site run on the published release, and it publishes the site.

## ADDED Requirements

### Requirement: Each config on a page comes from a fixture

A page MUST show a whole config only by an import of a fixture file. A fixture is a small repository in `site/examples/`, with the output that each command gives for it. A test MUST read each page and refuse a whole config that the page writes itself.

#### Scenario: A page that writes a whole config

- **GIVEN** a page with a TypeScript block that holds `defineConfig(`
- **WHEN** the test reads the pages
- **THEN** the test fails and names the page and the line of the block
- **TEST** `site/test/pages.test.mjs` › "a config that a page writes itself is named"

#### Scenario: The pages of the site

- **GIVEN** each page of the site
- **WHEN** the test reads it
- **THEN** no page writes a whole config
- **TEST** `site/test/pages.test.mjs` › "no page writes a whole config"

### Requirement: Each fixture gives its documented output with the published release

A test MUST run `archangel check`, `archangel test` and `archangel rules` on each fixture with the package that the site pins. It MUST compare each output with the expected output of the fixture. The package MUST be the published `@enhance-eng/archangel`, not a build of this repository.

#### Scenario: An output that differs

- **GIVEN** a fixture whose expected output of `archangel check` is "no problem", and whose config refuses one of its imports
- **WHEN** the test compares the output
- **THEN** the test fails and names the fixture, the command and the first line that differs
- **TEST** `site/test/examples.test.mjs` › "an output that differs is named"

#### Scenario: The fixtures of the site

- **GIVEN** each folder in `site/examples/`
- **WHEN** the test runs the three commands on it
- **THEN** each output is the expected output of the fixture
- **TEST** `site/test/examples.test.mjs` › "each fixture gives its documented output"

#### Scenario: An output that no step makes

- **GIVEN** a file in the `expected/` folder of a fixture that no step of the fixture writes
- **WHEN** the test reads the fixtures
- **THEN** the test fails and names the file, because a page can show it and no test runs it
- **TEST** `site/test/examples.test.mjs` › "each expected file is the output of a step"

#### Scenario: The site uses the published package

- **GIVEN** the package that the site installs
- **WHEN** the test reads its manifest
- **THEN** its version is an exact version, and the package is not a link to this repository
- **TEST** `site/test/examples.test.mjs` › "the site runs the published package"

#### Scenario: The card on the home page

- **GIVEN** the card on the home page, which shows part of a table and part of its error
- **WHEN** the test reads the home page
- **THEN** each line of the card is a line of the config or of the `archangel check` output of the first fixture
- **TEST** `site/test/pages.test.mjs` › "the card on the home page quotes the first table and its error"

### Requirement: Each rule fragment on a page loads

A page MUST write each rule that is not a whole config so that it loads in a config that declares the names that the site uses. A test MUST load each such rule with the published package.

#### Scenario: A fragment that does not load

- **GIVEN** a page with the inline code `rule('a', { files: 'shared', mayNotImport: 'react' })`, whose bare package name the config refuses
- **WHEN** the test loads the fragments of the page
- **THEN** the test fails and names the page and the line of the fragment
- **TEST** `site/test/pages.test.mjs` › "a fragment that does not load is named"

#### Scenario: The fragments of the site

- **GIVEN** each page of the site
- **WHEN** the test loads each rule fragment
- **THEN** no fragment gives a config error
- **TEST** `site/test/pages.test.mjs` › "each rule fragment on a page loads"

### Requirement: Each diagnostic code has a page from the release

The site MUST have one page for each diagnostic code. The text of the page MUST be the text that `archangel docs <code>` of the published release prints. A test MUST find a page for each code that the repository documents in `crates/archangel_rules/docs/`.

#### Scenario: The diagnostic pages

- **GIVEN** each code that has a file in `crates/archangel_rules/docs/`
- **WHEN** the test runs `archangel docs` for the code with the published package
- **THEN** the command gives the text, and the site lists the code
- **TEST** `site/test/diagnostics.test.mjs` › "each documented code has a page"

### Requirement: Each link on the site goes to a page or a heading that exists

The build of the site MUST fail when a link between pages names a page or a heading that does not exist.

#### Scenario: A link to a heading that does not exist

- **GIVEN** a page with a link to `/reference/config/#no-such-heading`
- **WHEN** the site builds
- **THEN** the build fails and names the page and the link
- **TEST** none, because the check is the build itself; `scripts/check` runs the build on each change

### Requirement: The site goes live from main

A workflow MUST build the site and publish it to GitHub Pages after each push to `main` that changes the site. It MUST use only GitHub-hosted runners, and it MUST pin each action to a commit SHA.

#### Scenario: A push to main

- **GIVEN** a push to `main` that changes a file in `site/`
- **WHEN** the workflow runs
- **THEN** the workflow publishes the built site to GitHub Pages
- **TEST** none, because only a real push can prove it; actionlint and zizmor check the workflow in `scripts/check`
