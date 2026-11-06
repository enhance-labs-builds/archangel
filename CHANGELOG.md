# Changelog

Every release, newest first. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and archangel follows [semantic versioning](https://semver.org/) for its command line, its config,
its JSON output (`schemaVersion`) and its language server. The library crates have no stability
guarantee.

## 0.3.5 - 2026-11-06

### Fixed

- A type import of a package that only `@types` installs, such as `import type { Point } from
  'geojson'`, resolves to that package, as TypeScript does. A runtime import of a package that is
  not installed is still unresolved. (#36)
- A part can leave files out with a `!` pattern in its list, so nested parts can be separated.
  `part-overlap` now says so, not to use `not`. (#37)
- When the config's `ignore` list empties a group or a part, its error names the line, and says
  when a line matches at any depth. (#40)
- `./app.config.ts` names the file at the root; before, it matched nothing. A bare file name's
  error suggests it. (#41)
- `test --format json` lists its problems under `diagnostics`, as `check` does. When the config
  stops a command, its JSON or SARIF document goes to standard output. (#44)
- The summary lists each import with a computed specifier by file and line, in text and in
  `summary.unfollowedImports`, instead of pointing to `archangel explain`. (#45)

### Added

- A package pattern can name a subpath: `lodash-es/*` bans deep imports and permits the root. A
  transitive rule refuses one, because a chain ends at a package. (#34)
- `severity: 'warn'` on a rule reports its violations as warnings, in every format and in the
  editor, and the check passes. `check --max-warnings <count>` fails past a count. Each JSON
  diagnostic has `severity`, and the summary has `warnings`. (#42)
- The entry fix reaches an entry file in a sibling folder: `@/shared/ui/components/Button` becomes
  `@/shared/ui/public`. (#43)
- An `import()` of a template literal with a static relative start and a written extension is an
  import of each file that it can load, as bundlers treat it. A template that matches no file is
  `import-unresolved`. (#46)

## 0.3.4 - 2026-11-05

### Fixed

- A rule with `names` judges a namespace or dynamic import of a package module by the names that
  the module exports. Before, `import * as loc from 'dates/locale/fr'` broke a rule that bans
  `format`, although the locale exports only `fr`. When archangel cannot see every name, the import
  still brings in each name. (#35)
- An `archangel-allow` comment above the first line of a multi-line import permits the import.
  Before, it was `exception-unused`, and it worked only inside the braces. (#38)
- An `allowUnresolved` entry for a package permits a tsconfig whose `extends` names it when it is
  not installed. Before, nothing could permit it, and `resolution-file-invalid` failed the check.
  The diagnostic now says which entry to add. (#39)

### Added

- The `order` of a `layers` rule can name groups, so a file can be in a layer and in a part of its
  own, such as a feature with an `entry` rule. (#47)
- `imports: 'static'` counts the runtime imports other than `import()`. A transitive rule with it
  stops each chain at an `import()`, which is where a bundler starts a new chunk, so it can say
  "the initial bundle may not reach this". (#48)
- A `{ tag: '@side-effect-free' }` selector selects the files whose comments before the first
  statement carry that JSDoc tag. (#49)
- `summary.work.packageModulesParsed` counts the package files that archangel parses for those
  names.

## 0.3.3 - 2026-11-04

### Fixed

- A `mustNotExist` rule on `folders` passes when no folder matches, as a `mustNotExist` rule on
  `files` already did. Before, it gave `selector-empty`, so `{ folders: 'src/**/utils',
  mustNotExist: true }` failed in a repository with no `utils` folder.

## 0.3.2 - 2026-11-03

### Fixed

- A rule of several sentences with an `entry` sentence, such as `fsd/no-public-api-sidestep`, says
  which entry file to import through. Before, its message ended with an empty list.
- A `mayNotImport` message names the limits of its rule. `fsd/import-locality` now says "through a
  relative specifier", so it no longer reads as a ban on a whole layer.
- A package pattern reads as words: `*` is "any package", and `@scope/*` is "a package in
  `@scope`". Before, an undeclared-package message said "the package `*`".
- A part that matches only files that another part also matches is no longer `element-empty`.
  `part-overlap` reports the file.
- The list of commands after an unknown command is in two columns, and it does not show the hidden
  `man` command.
- `archangel explain <file>` says, for each rule, whether it judges the file's own imports, imports
  of the file, or both. The JSON output gives the same as `judges`.
- `archangel docs` and the help say "part" and "group", as the config does.

## 0.3.1 - 2026-11-03

### Fixed

- The npm types declare the rule builders `rule`, `mayImport`, `layers`, `isolate` and `entry` as
  function properties, not methods. A config can destructure them, as in
  `rules: ({ mayImport, rule }) => [...]`, and typescript-eslint's `unbound-method` rule no longer
  reports them.

## 0.3.0 - 2026-11-03

A rule can now say which of several names a file may have. Each 0.2 config still loads and gives the
same result.

### Added

- The npm types accept a condition of the `fsd` preset in a config's own rule, such as
  `plugin: 'fsd/insignificant-slice'`, so a rule can use it with its own files. The types refuse a
  misspelt name.
- `mustBeNamed` takes a list of styles and name patterns, such as
  `mustBeNamed: ['*.view', '*.container', '*.styles']`. A file or folder passes when one entry
  matches. The message of a failing file gives its full name and each entry.
- An empty `mustBeNamed` list and an entry that is not a string are config errors. The npm types
  refuse both.

### Changed

- In the library crates, `Condition::MustBeNamed` holds a list of `NameRule`.

## 0.2.0 - 2026-11-01

A config can now read as your architecture, and `archangel rules` shows what it does. Each 0.1
config still loads, with the two changes of result below. Read
[the 0.2.0 upgrade notes](https://github.com/enhance-labs-builds/archangel/blob/v0.2.0/docs/migrating.md#from-archangel-01).

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
- The diagnostic codes `may-import`, `every-file-has-a-kind`, `part-overlap`, `table-empty` and
  `table-row-overlap`.
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
