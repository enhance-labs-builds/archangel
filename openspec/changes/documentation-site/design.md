# Design: A documentation site whose examples run

## In short

Today a developer learns archangel from the README and eight Markdown pages on GitHub. The pages
are correct, but they have no order for a new user, no guides for real tasks, and no search.

After this change, archangel has a public site with four sections. **Start** takes a new user from
install to a first table that catches a real import. **Guides** each do one task, for example "Add
archangel to a repository that already breaks its rules". **Concepts** explain the words that every
config uses: parts, groups, externals, captures, kinds, tables, rows and cells. **Reference** lists
each config key, rule form, condition, option, command and diagnostic code.

**How it works:**

1. Each example on the site is a fixture: a small repository in `site/examples/<name>/`, with its
   config, its source files and the output of `archangel check`, `test` and `rules`.
2. A page imports the files of a fixture to show them. The page does not copy the text.
3. A test runs the three commands on each fixture with the published package, and compares the
   output with the fixture's expected output.
4. `scripts/check` builds the site and runs the test. CI runs `scripts/check`, so a pull request
   cannot change archangel's output without a change to the site.
5. After a merge to `main`, a workflow builds the site and publishes it to GitHub Pages.

The site documents one release: the version that `site/package.json` pins, which is 0.3.1. When a
release changes behaviour, a pull request after the release moves the pin and updates the
fixtures. The test then shows each output that changed.

### Decisions that need you

1. **The URL is `https://enhancelabs.ai/archangel/`.** A page in a folder of enhancelabs.ai adds
   to the search rank of that domain, and a subdomain adds less. The Enhance Labs site is on Webflow,
   which cannot serve a folder from somewhere else. So a CloudFront distribution sits in front of
   enhancelabs.ai: `/archangel/*` goes to GitHub Pages, and every other path goes to Webflow
   unchanged and uncached. GitHub Pages serves the build at `/archangel/` too, so no path is
   rewritten. The switch is one Route 53 record for enhancelabs.ai, and the way back is the same
   record. Webflow does not support a proxy in front of it except on its Enterprise plan, so the
   way back must stay ready.
2. **GitHub Pages must be on.** Pages is a setting of the repository: Settings, Pages, Source
   "GitHub Actions". The publish workflow fails until it is on, so it stays off until you agree.
3. **The Markdown pages in `docs/` stay.** They are what a reader sees on GitHub and on npm, and
   `crates/archangel/tests/doc_examples.rs` tests them against the code on `main`. The site tests
   the published release. If you prefer one source, a later change can make `docs/` pointers to
   the site.

## How it works

### The pages

| Section | Pages |
| --- | --- |
| Start | Introduction · Getting started · How archangel thinks |
| Guides | Add archangel to an existing repository · Layer a backend · Feature-Sliced Design · Next.js · Monorepos and workspace tables · File and folder names · Write a plugin · Use archangel in CI · Editors and coding agents · Exceptions and baselines · Migrate from dependency-cruiser · Migrate from eslint-plugin-boundaries · Migrate from Steiger |
| Concepts | Parts, groups and externals · Captures and instances · Tables, rows and cells · Kinds · Rules and conditions · Examples prove rules · What a rule judges |
| Reference | Config file · Sets of files · Tables · Conditions · Import options · Rule options · Shorthands · Examples · Presets · Plugin API · Commands · Output formats and exit codes · Diagnostic codes (one page each) |

A guide has one aim, a list of what the reader needs, numbered steps, and the output that the
reader sees at each step. Each step has one instruction.

### The words

The text uses ASD-STE100 Simplified Technical English: one instruction in each sentence, the
active voice, the simple tenses, approved words, and one term for one meaning. The terms are the
terms of `docs/rules.md`: part, group, externals set, capture, instance, kind, table, row, cell,
judge. The site does not use words that the docs do not use, such as "element" for a part.

### The examples

A fixture folder holds a config, the source files, a `package.json` and an `expected/` folder. By
default the test runs `archangel check`, `test` and `rules` on it. A fixture can list other
**steps** in `expected/commands.json` instead. A step runs one command and keeps its output, and it
can check the exit status. A step can also write files first, as a developer does between two
commands: the text is in the step, or in a file of the fixture's `later/` folder that a page can
show. A step can keep a file that the command writes, such as the config that `init` writes or the
baseline file. A migration guide keeps the config of the old tool in `before/`. The test does not
copy `expected/`, `later/` or `before/` into the repository that archangel checks.

The test copies the fixture to a temporary folder. It links the published package into the copy's
`node_modules`, links each workspace package as npm does, and writes an empty package for each
other dependency, as `crates/archangel/tests/support/fixtures.rs` does. Then it runs each step with
`--color never` and compares the text. With `UPDATE=1`, it writes what archangel prints, and a
person reads the change in review.

A page shows a fixture file with the `Example` component, and a command with the `Output`
component. `Output` takes the command from the fixture's steps, so a page cannot show one command
with the output of another. A file in `expected/` that no step writes fails the test, so a page
cannot show output that no test runs.

A small rule on a page, for example `rule('no-react-in-shared', { ... })`, can be inline. The test
finds each one, as `doc_examples.rs` does, and loads it in a config that declares the names that the
site uses. A whole config is never inline. The card on the home page quotes lines of the first
fixture, and a test checks each line.

### The diagnostic pages

Before the build, a script runs `archangel docs <code>` of the pinned package for each code, and
writes one page for each code. The pages are not in Git. So a diagnostic page always says what
`archangel docs` says in the terminal.

### The look

Starlight's own layout, with a purple accent that matches the badges in the README, the speed chart
from `docs/assets/` on the home page, and a splash home page with the problem, one table, and the
error that it gives. The site has light and dark themes and works at phone width.

## Decisions

**1. Astro Starlight.** Starlight is a standard generator for documentation. It gives search
(Pagefind) without a service, MDX for raw imports of fixture files, code blocks with titles and
marks (Expressive Code), and a check for broken links (`starlight-links-validator`). VitePress has
the same features, but its raw imports of a file outside the site are less direct.

**2. The site tests the published package, not a build of `main`.** A reader installs the published
package, so the site must describe it. `doc_examples.rs` already tests the Markdown docs against
`main`.

**3. Fixtures, not copied text.** The Markdown docs already work this way, and it is why their
examples are correct. The site takes the same approach.

**4. The site is in `scripts/check`.** AGENTS.md says that CI runs `scripts/check` and nothing else.
The build and the test take about 30 seconds.

## Cost

No change to the check path. Archangel itself does not change.

`scripts/check` does more work: one `npm ci` in `site/`, one build, and three commands for each
fixture.

## Risks

- **A fixture shows a result that is wrong, and the test keeps it.** The test proves that the page
  and archangel agree, not that the result is what a reader wants. Each fixture has one problem that
  a reader can see in its source, and the review reads each expected output.
- **The site and `docs/` say different things.** Both are tested, but against different versions.
  The site says which version it documents, on each page.
- **Text that is not STE.** No tool checks STE. The review reads each page.

## Compatibility and release

No change to the config, the output or the commands. No release.

## Limits

- The test checks the text of each command, not its colour. A fixture runs with `--color never`.
- The test runs only the steps that a fixture lists. A file in `expected/` that no step writes fails
  the test, so no page shows output that the test does not run.
- A page can show a code fragment that is not a rule, for example a GitHub Actions step or a Lua
  config for Neovim. The test cannot run these. Each one is short, and the review reads it.

## Where in the code

This section is for whoever builds the change.

| Part | Where |
|---|---|
| The site | `site/astro.config.mjs`, `site/src/content/docs/` |
| Fixtures | `site/examples/<name>/` |
| The test of fixtures | `site/test/examples.test.mjs` |
| The test of pages and fragments | `site/test/pages.test.mjs` |
| The diagnostic pages | `site/scripts/diagnostics.mjs`, `site/test/diagnostics.test.mjs` |
| CI | `scripts/check`, `.github/workflows/ci.yml`, `.github/workflows/site.yml` |
| Precedent | `crates/archangel/tests/doc_examples.rs`, `crates/archangel/tests/support/fixtures.rs` |
