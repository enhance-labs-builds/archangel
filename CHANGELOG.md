# Changelog

Every release, newest first. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and archangel follows [semantic versioning](https://semver.org/) for its command line, its config,
its JSON output (`schemaVersion`) and its language server. The library crates have no stability
guarantee.

## 0.2.0 - 2026-11-01

A config can now read as your architecture, and `archangel rules` shows what it does. Each 0.1
config still loads, with the two changes of result below. Read
[docs/migrating.md](docs/migrating.md#from-archangel-01).

### Added

- Import tables: `mayImport('<table>', rows, options)`, where each row is a rule named
  `<table>.<row>`. A row is a kind of file, a part or a group. A table row does not need a written
  example.
- Kinds: a pattern such as `**/*.{kind}.ts` gives each file its kind. It does not need a file name
  format, because `src/features/*/{kind}/**` works too.
- The rule `<table>.every-file-has-a-kind`, which each table makes.
- `parts`, `groups`, `externals`, `because`, and `rule('<name>', { ... })`. `elements`, `selectors`
  and `reason` still load until 1.0, and one config cannot mix two spellings of one key.
- `within` on `mayOnlyImport` and `mayOnlyBeImportedBy`: the area whose imports the rule judges.
- One-line examples, such as `'src/a.ts imports src/b.ts'`, in place of a set of files.
- `archangel rules [path]`: each rule as a sentence, with its reason, its place in the config and
  what it judged, and what no rule judges.
- `archangel explain <file>:<line>`: each rule that judged one import, and why each other rule did
  not.
- A config can have rules and no parts.
- The diagnostic codes `may-import`, `part-overlap`, `table-empty` and `table-row-overlap`.
- `archangel init` writes the new spellings and a one-line example.

### Changed

- `mayOnlyImport` on a part with a capture permits only the files of its own instance. In 0.1, it
  permitted each instance. To keep the 0.1 result, add the part to the rule's list.

### Fixed

- Type imports follow `verbatimModuleSyntax`. With this option, `import { type A } from './a'` and
  `export { type A } from './a'` stay in the compiled code, so they are runtime imports. The file's
  tsconfig decides the mark, so a change to the tsconfig marks the imports again with no parse.
- In the editor and in `watch`, a change to a tsconfig re-checks the files whose import marks moved.

## 0.1.1 - 2026-11-01

### Fixed

- In `archangel test`, a plugin condition sees the importers of an example file: the example files
  that import it, and the repository files that the example does not replace.
- An example's own `package.json` decides its dependency rules, so a `dependency: 'dev'` rule can
  be proven. An example without a manifest sees the repository's manifest.
- `archangel test` in text format no longer panics on a failed example. It shows the example's own
  text, and a span that does not fit its text is shown without the snippet.
- A pattern that a rule writes only inside `not` may match no file without `selector-empty`.
- When an accepted example proves nothing, the message names what the rule looks at: imports,
  files, folders or instances. For `mustNotExist`, it says that only rejected examples can prove it.
- `defineConfig`'s types reject a slash in a rule name that the config writes, except after the
  name of a preset that the config extends.

### Documentation

- `noCycles` counts only runtime imports by default; `imports: 'all'` adds type imports.
- `docs/migrating.md`: what changes when you move from dependency-cruiser, eslint-plugin-boundaries,
  eslint-plugin-check-file or Steiger.

## 0.1.0 - 2026-10-31

The first release.

### Added

- Rules as sentences: a set of files and one condition, with a name, a reason, a link, and
  examples that the rule must accept and reject.
- Elements and selectors, named by path patterns, with captures such as `{feature}`.
- Import conditions: `mayNotImport`, `mayOnlyImport`, `mayOnlyBeImportedBy` and `noCycles`, with
  `names`, `specifier`, `instance`, `dependency` and `transitively`.
- Structure and export conditions: `mustNotExist`, `mustBeNamed`, `mustContain`, `mustExport`,
  `mayNotExport` and `mustOnlyReExport`, for files and folders.
- The shorthands `layers`, `isolate` and `entry`.
- Presets with `extends` and `off`: `fsd`, with Steiger's rules, and `nextjs`.
- Plugins: conditions written in TypeScript with `definePlugin` and `defineCondition`.
- `archangel check`, `test`, `explain`, `why`, `watch`, `baseline`, `init`, `docs`, `completions`
  and `lsp`.
- Output as text, JSON, GitHub Actions annotations and SARIF.
- Exceptions in comments, each with a reason, and a baseline file for existing violations.
- A config in TypeScript, with `defineConfig` types, or in JSON, with a published JSON schema.
- `@enhance-eng/archangel` on npm, with a binary for macOS, Linux (glibc and musl) and Windows, and
  with provenance; `archangel` on crates.io; and a VS Code extension in `editors/vscode`.
