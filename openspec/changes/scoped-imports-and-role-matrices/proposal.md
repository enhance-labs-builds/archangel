# Proposal: scoped imports and role matrices (archangel 0.2.0)

## Why

Haziq Nordin, archangel's author, asked for this change on 2026-11-01, after the 0.1.1 release moved
a 9,800-file TypeScript monorepo (enhance3) onto archangel. The check works: no problems over 9,677
files and 51,241 imports, and 688 passing examples. But the config does not read as an architecture.
It is one line that re-exports an 828-line builder, which reads a 2,896-line vocabulary, globs the
repository at load time and produces about 380 rules with computed names. A developer who opens a
config written with archangel should like what they read; three gaps in archangel prevent that:

1. A developer cannot say "these files may import only these, within this part of the code".
   `mayOnlyImport` judges every import, packages and other apps included, so teams write the double
   negative `mayNotImport: { all: [area, { not: allowed }] }` instead.
2. A developer cannot write an import table as one table. A role is a kind of file found by its
   name, such as a file that ends in `.entity.ts`. A role matrix says, for each role, what its files
   may import. enhance3 has 9 role tables, which it applies 14 times to different parts of the code,
   and each row becomes one generated rule. Its package dependencies are the same kind of table, with
   packages as rows, written as two rules per package.
3. A developer cannot say "every file here has one of the table's roles" without listing the roles
   again, so enhance3 has one naming rule per folder that repeats them.

## What Changes

- **`within` on import rules.** `rule({ files: 'entity', mayOnlyImport: ['value-object'], within:
  'apps/api/src/**' })` judges only the imports whose other end is within that area. A package or a
  file of another app is not judged by that rule. `within` takes the same values as a rule side, and
  combines with `names`, `specifier`, `instance`, `dependency` and `imports`. On `mayNotImport` it
  narrows the side, and on `mayOnlyBeImportedBy` it limits the importers.
- **A `matrix` shorthand**, beside `layers`, `isolate` and `entry`. The developer writes the table:
  one row per role, found by one role pattern such as `**/*.{role}.ts`, or, without a role pattern,
  one row per declared element or selector, such as each package. A row permits exactly what it
  lists. Each row is a rule named `<matrix>.<row>`, such as `api-domain.entity`, so `off`,
  exceptions and the baseline can name it.
- **`everyFileHasARow: true`** makes a matrix refuse a file in its files that no row has, with a
  message that names the roles the file could have. It is off by default, and the summary then
  counts such files for each matrix.
- **Empty rows are declared intent.** A row whose role no file has yet is not an error. The summary
  lists it. A matrix with no row, or whose rows together match no file, gives the new diagnostic
  `matrix-empty`, as a preset gives `preset-empty`.
- **A matrix is proven by its author's examples.** One written rejected example proves a matrix and
  all its rows, as one proves a `layers` rule. A matrix without one is `rule-unproven`.
- **Types.** `defineConfig` checks the row names, the cells, the area and the matrix name, with no
  computed names.
- **Documentation** in ASD-STE100 Simplified Technical English: `docs/rules.md`, the README where it
  shows the rule forms, the diagnostic pages and the changelog.
- **Additive.** Every 0.1 config keeps working and gives the same results. The release is 0.2.0.

Out of scope: the rewrite of enhance3's config (a later task, which the design estimates at 1,400 to
1,600 lines and breaks down), examples written as one line, a naming condition that takes a list of
roles, the VS Code Marketplace, and type-aware rules.

## Capabilities

### New Capabilities

- `role-matrices`: the matrix shorthand in both forms: its rows and cells, the names of its rows,
  `everyFileHasARow`, its empty rows and `matrix-empty`, how `explain` shows it, and what it costs.

### Modified Capabilities

- `architecture-rules`: `within` on `mayNotImport`, `mayOnlyImport` and `mayOnlyBeImportedBy`, its
  refusals, its cost, and matrix as the fourth shorthand kind.
- `configuration`: `{role}` may appear twice in a role pattern; other repeated captures stay refused.
- `rule-examples`: the written rejected example that proves a matrix and its rows.
- `selectors`: the roles of a matrix are exempt from `selector-empty`.
- `distribution`: the config types check a matrix and `within`.

## Impact

- The config gains `within`, the `matrix` kind and `everyFileHasARow`, in the TypeScript types and
  the JSON schema.
- The check summary, in text and JSON, gains the matrix rows with no file and the count of files with
  no row, and `archangel test` names the row that rejected a matrix example.
- There is one new diagnostic, `matrix-empty`, with its page.
- The docs, the README, the diagnostic pages, the glossary (area, role, role pattern, matrix, matrix
  row, cell) and the changelog change, and the version becomes 0.2.0.

design.md, "Where in the code", lists the files.
