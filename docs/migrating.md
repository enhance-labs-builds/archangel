# Moving from other tools

This page tells you what changes when you move rules to archangel from dependency-cruiser,
eslint-plugin-boundaries, eslint-plugin-check-file or Steiger.

## From dependency-cruiser

- **An import that does not resolve is an error.** dependency-cruiser treats an import that it
  cannot resolve as a path that matches `pathNot`. So a rule with a typing error in its import path
  passes. Archangel reports `import-unresolved`. When you move, expect some new errors: each one is
  an import or a rule that was wrong before.
- **Type cycles.** dependency-cruiser with `tsPreCompilationDeps` reports cycles of `import type`.
  Archangel's `noCycles` counts only runtime imports by default. Add `imports: 'all'` to keep the
  same result.
- **`no-orphans`.** Archangel has no built-in condition for a file that nothing imports. Write it as
  a [plugin](plugins.md) condition that reads `importers`.
- **A rule that finds no files is an error.** A dependency-cruiser rule whose path matches nothing
  passes silently. In archangel, it gives `selector-empty`.

## From eslint-plugin-boundaries

- **Elements are the same idea.** A boundaries element type becomes an archangel element, and its
  capture becomes `{name}` in the pattern. Read [docs/rules.md](rules.md#elements).
- **One run checks all files.** You do not run ESLint over each file. `archangel check` reads the
  repository one time.

## From eslint-plugin-check-file

- **Name styles.** `mustBeNamed` takes `kebab-case`, `camelCase`, `PascalCase`, `snake_case` and
  `SCREAMING_SNAKE_CASE`, for files and for folders. Archangel checks the name part of a file, for
  example `button` in `button.view.tsx`. It does not have a rule for the first or the last
  character of a name.

## From Steiger

- **Use the `fsd` preset.** `fsd({ root: 'src' })` has Steiger's rules, each with examples. Read
  [docs/presets.md](presets.md).
- **Slices are found by their path.** Each folder in a sliced layer is a slice. Steiger finds slices
  by the segments inside them. The presets spec lists all known differences.
