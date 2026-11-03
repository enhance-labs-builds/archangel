# A documentation site for archangel

## Why

The docs of archangel are eight Markdown pages in `docs/`. They are correct, but a new user has no
path through them. There is no getting-started page, no guide for a task such as "add archangel to
my repository", and no reference that lists every config key and every diagnostic code in one
place. A developer who reads the docs on GitHub cannot search them.

The brief asks for a full documentation site that is public, clear and easy to read, with many
examples and guides. It asks for these things:

- A getting-started page.
- Guides by task: add archangel to an existing repository, layer a backend, Feature-Sliced Design
  with the `fsd` preset, Next.js, monorepos and workspace tables, file and folder names with
  `mustBeNamed` lists, plugins, CI, the editor (the language server and VS Code), exceptions and
  baselines, and migration from dependency-cruiser, eslint-plugin-boundaries and Steiger.
- A full reference for every config key, rule form, condition, diagnostic code and command.
- Concepts: parts, groups, kinds, tables and cells.
- All text in ASD-STE100 Simplified Technical English.
- Examples that are real and that run. A test runs them on the published
  `@enhance-eng/archangel@0.3.1`, as the tests of `docs/examples/` already do for the Markdown docs.
- A standard generator, hosted on GitHub Pages from the public repository by a GitHub Actions
  workflow, with pinned action SHAs.

## What changes

- A new folder, `site/`, holds an Astro Starlight site: about 35 pages in four sections, Start,
  Guides, Concepts and Reference.
- Each config and each command output on a page comes from a fixture: a small repository in
  `site/examples/`, with the output that archangel gives for it. A page imports the fixture's files,
  so the page and the fixture cannot have different text.
- A test runs `archangel check`, `test` and `rules` on each fixture with the published package, and
  compares each output with the fixture's expected output. The test also loads each rule fragment
  that a page writes inline, and checks that each diagnostic code has a page.
- Each diagnostic code page is the text that `archangel docs <code>` of the published release prints.
- `scripts/check` builds the site and runs its test, so CI checks the site on each pull request.
- A new workflow publishes the site to GitHub Pages after each push to `main` that changes it.

## Capabilities

### New capabilities

- `documentation-site`: the public site, how its examples are proved, and how it is published.

## Out of scope

- The Markdown pages in `docs/` stay, and their tests do not change. The README links to the site.
- The site lives at `https://enhancelabs.ai/archangel/`, so its pages count for the Enhance Labs
  domain in search. GitHub Pages serves the build, and a CloudFront distribution in front of
  enhancelabs.ai sends `/archangel/*` there. That distribution and its DNS record are set up in AWS,
  not in this repository.
- No docs for more than one version. The site describes the release that its package pins.
- No translation.

## Impact

- New: `site/` (Astro, Starlight, the fixtures, the test), `.github/workflows/site.yml`.
- Changed: `scripts/check` and the `Checks` job of `.github/workflows/ci.yml` install and test the
  site. `README.md` links to the site.
- No change to archangel itself, to its config format, its output or its commands. No release.
