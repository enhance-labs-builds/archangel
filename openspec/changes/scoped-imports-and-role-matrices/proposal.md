# Proposal: scoped imports and import tables (archangel 0.2.0)

## Why

Haziq Nordin, archangel's author, asked for this change on 2026-11-01. His top requirement: a config
written with archangel reads as clean and delightful, and a developer can know what their rules are
and reason about them with confidence. Today enhance3, a 9,800-file monorepo, passes its check, but
its config is an 828-line builder that produces about 380 rules with computed names. (enhance3 is
the Enhance product monorepo, https://github.com/enhance-labs-builds/enhance3, which the owner can
open; every `packages/architecture/` path in this change is there, not in archangel.) Two cold
readers tested a proposal for a better interface. Both liked the direction and rated it 2 of 5, because its
own examples broke its own meaning. So this change writes every meaning down as a tested scenario,
and makes every config example in the docs a fixture that archangel checks.

## What Changes

- **Tables.** `mayImport('<table>', { <row>: [<names>] }, { <options> })`. A row is a named group of
  files: a kind of file, a part or a group, mixed freely. Tables fail closed for repository files: an
  import of a repository file that a row does not list is refused, and the error names the row, or
  says "no row for" the target. An outside package is judged only when a cell names an externals set
  that holds it. A row may import itself in the same instance (the same captured folder, such as the
  same feature), unless `mayNotImportItself` lists it. `everyRowMayImport` names what every row may
  also import. Each row is a rule named `<table>.<row>`, and a table row needs no written example.
- **Kinds.** A kind comes from a table's one pattern with `{kind}`, such as `**/*.{kind}.ts`. A row
  is a kind, or a declared part or group. Every table also makes the rule
  `<table>.every-file-has-a-kind`, which refuses a file in the table's files that has no row.
- **Parts, groups and externals.** `parts` and `groups` are the new spellings of `elements` and
  `selectors`; `externals` names sets of outside packages, as package patterns.
- **`within`** on `mayOnlyImport` and `mayOnlyBeImportedBy`: the area whose imports the rule judges.
- **Rules read as sentences**: `rule('<name>', { ... })` and `because`.
- **`archangel rules [path]`** prints every rule as the sentence it means, with its reason, where it
  is written, and its reach: the files and imports it judged, its problems, its exceptions and its
  baseline matches; a rule that checks nothing says so. **`archangel explain <file>:<line>`**
  shows every rule that judged or skipped one import, and why.
- **Exact meaning** for outside packages, overlapping parts, type imports under
  `verbatimModuleSyntax`, an import that breaks two rules, and where patterns start.
- **Checked examples**: every config example in the docs and the README is a fixture under test.

Every 0.1 config keeps working, with two exceptions that the design names for a decision:
`mayOnlyImport` on a part with a capture now permits only its own instance (a deliberate change of
default, so that tables and rules agree), and type imports under `verbatimModuleSyntax` are fixed
(0.1 gives a plausible wrong answer). The release is 0.2.0.

Out of scope: the rewrite of enhance3's config (estimated at 1,150 to 1,350 lines), one-line
examples (an example written as one import string, such as `'entity -> use-case'`),
`mayOnlyImportTypesFrom`, layers and isolate over groups, an explicit word to replace a preset
rule, a map printed by `archangel init`, and a new capture syntax.

## Capabilities

### New Capabilities

- `import-tables`: tables, rows, cells, kinds, the own-instance permission, the
  `every-file-has-a-kind` rule, overlaps, empty rows and `table-empty`, and their cost.
- `documentation`: config examples in the docs are checked fixtures.

### Modified Capabilities

- `architecture-rules`: `within`, the self default of `mayOnlyImport`, one import that breaks
  several rules, the table as the fourth shorthand.
- `configuration`: `parts`, `groups`, `externals`, `because`, name-first rules, part overlap, where
  patterns start, and `{kind}`.
- `import-graph`: type imports under `verbatimModuleSyntax` and `export type *`.
- `command-line`: `archangel rules` and `explain` of one import.
- `rule-examples`: a table row needs no written example, and a table's written examples run
  against all its rows.
- `exceptions-and-baselines`: stale entries for table rows.
- `selectors`: kinds are exempt from `selector-empty`.
- `distribution`: the types of the new forms.

## Impact

- The config gains the keys and the `mayImport` kind above, in TypeScript and in the JSON schema.
- New diagnostics: `table-empty`, `table-row-overlap`, `part-overlap`.
- New command `archangel rules`; `explain` takes `<file>:<line>`.
- The docs, README, glossary, changelog and diagnostic pages change; the version becomes 0.2.0.

design.md, "Where in the code", lists the files.
