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

- **Elements become parts.** A boundaries element type becomes an archangel part, and its capture
  becomes `{name}` in the pattern. The `element-types` rule becomes a table. Read
  [docs/rules.md](rules.md#tables).
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

## From archangel 0.1

A 0.1 config still loads. [docs/rules.md](rules.md#older-spellings) lists the new spellings. Two
results change:

- **`mayOnlyImport` on a part with a capture permits only its own instance.** In 0.1, it permitted
  each instance. To keep the 0.1 result, add the part to the rule's list.
- **Type imports follow `verbatimModuleSyntax`.** With this option, `import { type A } from './a'`
  stays in the compiled code, so it is a runtime import. A rule with `imports: 'runtime'`, a
  transitive rule, or `noCycles` can then report more.
