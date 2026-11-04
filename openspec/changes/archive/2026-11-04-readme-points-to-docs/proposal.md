# Make the README a front door to the docs site

## Why

The docs site at enhancelabs.ai/archangel has the guides, concepts and reference, and its examples
run on the published package. The repository also holds eight Markdown pages under `docs/` with the
same material, and the README is a long page whose link to the site is near the end. A reader on
GitHub or npm reads a second, shorter set of docs and does not find the full ones.

## What changes

- The README and the npm README are short. Each puts a link to the docs above its first section,
  then shows how to install archangel, one example with its error, and links into the site.
- The pages `docs/rules.md`, `presets.md`, `plugins.md`, `commands.md`,
  `exceptions-and-baselines.md`, `editors.md`, `migrating.md` and `principles.md` are deleted. The
  site has each of them.
- The fixtures move from `docs/examples/` to `examples/`. The speed charts move to `.github/assets/`.
  The release steps move to `RELEASING.md`. The first-release brief moves to the archive of the
  first-release change. No `docs/` folder is left.
- The doc tests read both READMEs, and a new test checks that each one links to the docs before its
  first section.
- The pages of the diagnostic codes in `crates/archangel_rules/docs/` do not change.
  `archangel docs` prints them, so they stay with the code.

## Capabilities

### Modified capabilities

- `documentation`: the docs in the repository are the two READMEs. The fixtures are in `examples/`.

## Impact

- No change to archangel itself. The npm README changes on the next release. Until then, the npm
  page of 0.3.3 links to the deleted pages and shows no chart.
