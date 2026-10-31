# archangel

[![CI](https://github.com/enhance-labs-builds/archangel/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/enhance-labs-builds/archangel/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

Architecture rules for TypeScript, checked on every keystroke.

> **Status: first release in progress.** The packages are not published yet; build from source
> with the commands below. The design is the archived OpenSpec change in
> [`openspec/changes/archive/2026-10-31-first-release/`](openspec/changes/archive/2026-10-31-first-release/),
> and the behaviour it specifies is in [`openspec/specs/`](openspec/specs/).

## Why

Most TypeScript repositories that care about their architecture spread the rules across
several tools. Import boundaries live in dependency-cruiser or an ESLint plugin, folder and naming
rules in another plugin, slice rules in Steiger, and the rest in custom scripts. Each tool parses
the whole repository again, so the checks are slow. They also run late, usually in CI or a
pre-push hook, not in the editor while the code is written. And when one of them is configured
wrong, it checks nothing and still reports success.

Archangel is meant to be one tool for the rules about where code lives, what it may depend on and
who owns what:

- **One config, as data.** The config names the kinds of files in the repository and the rules
  between them. Because the config is data, archangel can explain any file's place in the
  architecture, generate documentation from the rules, and import configs from other tools.
- **Fast enough for every keystroke.** A Rust core reads each file once and checks the rules
  against an in-memory graph of imports. The same process serves the command line, a watcher and
  a language server, so editors and coding agents see a broken rule as it is written.
- **Rules that prove themselves.** Every rule can carry examples of code it must accept and code
  it must reject, checked in memory. A pattern that matches no files is an error, not a pass.
- **Diagnostics that say what to do.** Each result names the rule, the reason the team gave for
  it, and the allowed alternative, with a fix archangel has checked before it offers it.

Archangel is not a general linter, a formatter or a type checker. Those already have good tools.

## How fast

The same rules, checked by dependency-cruiser 18.1 and by archangel, on a 14-core laptop with a warm
file cache:

| Repository | Files | dependency-cruiser | archangel |
| --- | --- | --- | --- |
| [VS Code](https://github.com/microsoft/vscode) 1.144.0: layers and no cycles | 7,652 | 38.7 s | 0.36 s |
| A TypeScript monorepo with 257 import rules | 9,704 | 19.7 s | 1.26 s |
| The same monorepo, proving the rules can find a violation | | 143.5 s for 119 rules | 1.49 s for 205 rules |

dependency-cruiser proves a rule by writing a violating file into the repository and checking it
again; archangel checks each rule's examples in memory. `scripts/bench` reproduces the archangel
numbers.

## Guide

### Install

Once the first release is published:

```sh
npm install --save-dev @enhance-eng/archangel
```

or, with Rust:

```sh
cargo install archangel
```

Until then, build it from a clone of this repository:

```sh
cargo install --path crates/archangel
```

### A first config

`archangel init` writes a starter `archangel.config.json`, with one element for each folder of
source files and a `noCycles` rule; `archangel init --typescript` writes `archangel.config.ts`
instead. Or write one yourself: put `archangel.config.ts` at the top of the repository. Elements name the parts of the code by
path pattern, or packages by name; `{name}` in a pattern captures one instance, such as one feature.
Selectors name other sets of files. Rules say what each part may import.

```ts
import { defineConfig } from '@enhance-eng/archangel';

export default defineConfig({
  elements: {
    app: 'src/app/**',
    feature: 'src/features/{feature}/**',
    shared: 'src/shared/**',
    react: { packages: ['react', 'react-dom'] },
  },
  selectors: {
    test: '**/*.test.ts',
  },
  rules: ({ layers, isolate, entry, rule }) => [
    layers(['app', 'feature', 'shared'], { reason: 'Shared code must not know about features' }),
    isolate('feature'),
    entry('feature', 'index.ts'),
    rule({ files: 'shared', mayNotImport: 'react', imports: 'runtime' }),
    rule({ files: { not: 'test' }, mayNotImport: 'test' }),
    rule({ files: 'src/**', noCycles: true }),
  ],
});
```

A TypeScript config needs Node 22.18 or later, which runs it without a build step. The same config
as `archangel.config.json`, with each rule as the object inside `rule(...)`, or such as
`{ "kind": "isolate", "element": "feature" }` for a shorthand, needs no Node at all. With
`defineConfig`, the TypeScript compiler already reports a rule that names an undeclared element or
selector, a rule with two conditions, and an isolate rule on an element without a capture. A JSON
config gets completion and checks in the editor from the schema the npm package ships:
`"$schema": "./node_modules/@enhance-eng/archangel/archangel.schema.json"`.

### Selectors

A selector is a set of files, and so are a rule's `files` and the other side of its condition.
Each is written one of these ways:

| Form | The files |
| --- | --- |
| `'feature'` | of a declared element or selector |
| `'src/**/*.view.tsx'` | that match a path pattern, which has a `/` or a `*` |
| `['app', 'feature']` | in any of these |
| `{ all: ['src/**', { not: 'test' }] }` | in every one of these |
| `{ any: ['app', 'feature'] }` | in any of these |
| `{ not: 'test' }` | in all but these |
| `{ element: 'feature', capture: { feature: 'login' } }` | of one instance of an element |
| `{ element: '*' }` | of any element |
| `{ directive: 'use client' }` | that start with this directive |

A file is in at most one element, the first whose pattern matches it, but it can be in any number
of selectors. A test of a
feature is in `feature` and in `test`, so these two rules both apply to it:

```ts
selectors: {
  test: '**/*.test.ts',
  client: { directive: 'use client' },
},
rules: ({ rule }) => [
  rule({ files: { not: 'test' }, mayNotImport: 'test' }),
  rule({ files: 'client', mayNotImport: { packages: ['server-only'] }, transitively: true }),
],
```

A selector that matches no file is an error, `selector-empty`, because no rule about it could fire.

### Rules

A rule is a sentence: `files`, and one condition about them.

| Condition | Example | It refuses |
| --- | --- | --- |
| `mayNotImport` | `rule({ files: 'shared', mayNotImport: 'react' })` | an import of `react` from `shared` |
| `mayOnlyImport` | `rule({ files: 'view', mayOnlyImport: ['shared', 'react'] })` | an import from `view` of anything else |
| `mayOnlyBeImportedBy` | `rule({ files: 'db', mayOnlyBeImportedBy: 'api' })` | an import of `db` from anything but `api` and `db` |
| `mustNotExist` | `rule({ files: { all: ['src/**', { not: { element: '*' } }] }, mustNotExist: true })` | a file in no element |
| `noCycles` | `rule({ files: 'src/**', noCycles: true })` | files that import each other in a cycle |
| `mustBeNamed` | `rule({ files: 'src/components/**/*.tsx', mustBeNamed: 'PascalCase' })` | a file whose name is not in the style |
| `mustContain` | `rule({ files: 'feature', mustContain: 'index.ts' })` | an instance without the file |
| `mustExport` | `rule({ files: '**/*.view.tsx', mustExport: '{FileName}' })` | a view that does not export the name of its file |
| `mayNotExport` | `rule({ files: 'src/**', mayNotExport: 'default' })` | a default export |
| `mustOnlyReExport` | `rule({ files: '**/index.ts', mustOnlyReExport: true })` | an index file that declares or runs anything of its own |

The other side of an import condition is a selector, an element of packages,
`{ packages: ['react', '@acme/*', 'node:*'] }` (`'*'` is every package), or a list of these. Options
narrow an import condition to some imports:

- `names: ['useEffect']`: only imports of these names.
- `specifier`: only imports written a certain way: `'relative'` (starting with `./` or `../`),
  `'parent'` (starting with `../`), `'alias'` (through a tsconfig path or a package `imports`
  field) or `'package'`.
- `instance: 'same'` or `'different'`: only imports within one instance of `files`, or between two;
  `files` must be an element with a capture.
- `dependency: 'dev'` or `'undeclared'`: only imports of packages that the importing file's nearest
  `package.json` lists only as development dependencies, or does not list at all.
- `transitively: true`, on `mayNotImport` only: chains of imports count, not just direct ones, and
  the diagnostic shows the shortest chain. `stopAt` names files where a chain ends, such as
  `{ directive: 'use server' }`. A transitive rule cannot also take `names`, `specifier`,
  `instance` or `dependency`.

A rule about folders says `folders` instead of `files`, with path patterns, and `mustBeNamed`,
`mustContain` or `mustNotExist`; a pattern that starts with `!` leaves folders out, as in
`.gitignore`. The styles are `kebab-case`, `camelCase`, `PascalCase`, `snake_case` and
`SCREAMING_SNAKE_CASE`, and a name pattern can use the file's or folder's own name in any of them,
as `{FileName}` or `{folder-name}`. `mustContain` takes a list, met by any one of its patterns:

```ts
rule({ folders: 'src/**', mustBeNamed: 'kebab-case' }),
rule({ folders: ['src/shared/*', '!src/shared/{ui,lib}'], mustContain: ['index.ts', 'index.tsx'] }),
rule({ files: '**/*.test.ts', mustBeNamed: '{folder-name}.test' }),
```

A rule may also list `sentences` under one `name`, and is broken when any of them is:
`rule({ name: 'no-ui-in-app', sentences: [{ folders: 'src/app/ui', mustNotExist: true }, { files: 'src/app/ui.*', mustNotExist: true }] })`.

Three shorthands write common sentences for you:

| Shorthand | It stands for |
| --- | --- |
| `layers(['app', 'feature', 'shared'])` | `rule({ files: 'feature', mayNotImport: ['app'] })` and `rule({ files: 'shared', mayNotImport: ['app', 'feature'] })` |
| `isolate('feature')` | `rule({ files: 'feature', mayNotImport: 'feature', instance: 'different' })` |
| `entry('feature', 'index.ts')` | for each feature, such as `login`, a `mayOnlyBeImportedBy` rule: outside `login`, only `src/features/login/index.ts` may be imported |

In a JSON config a shorthand keeps its `kind`: `{ "kind": "layers", "order": [...] }`,
`{ "kind": "isolate", "element": "feature" }` and
`{ "kind": "entry", "element": "feature", "files": ["index.ts"] }`.

Every rule takes options: a `name`, a `reason` and a `link` that each violation shows, `imports`
(`'all'`, `'runtime'` or `'types'`), and `examples` of code it must accept and reject. A rule
without a `name` is named after itself, such as `mayNotImport(shared, react)` or
`isolate(feature)`. `archangel test` checks the examples in memory, and fails for a rule with no
rejected example, because nothing else shows that the rule can find a violation:

```ts
isolate('feature', {
  examples: {
    accepted: [{ 'src/features/login/a.ts': "import './b';", 'src/features/login/b.ts': '' }],
    rejected: [{ 'src/features/login/a.ts': "import '../signup/c';", 'src/features/signup/c.ts': '' }],
  },
}),
```

### Presets

A preset is a function that returns part of a config: elements, selectors, rules with their
examples, and the plugins its rules use. `extends` puts presets under the config. Archangel ships
two:

- `fsd({ root })`: [Feature-Sliced Design](https://feature-sliced.design), with the rules of its
  linter Steiger. By default it turns on the rules Steiger recommends; `strict: true` turns on
  Steiger's four opt-in rules too.
- `nextjs({ root })`: a file that starts with `'use client'`, and every file it reaches through
  runtime imports, may not import `server-only` or a Node built-in module. A chain stops at a file
  that starts with `'use server'`.

```ts
import { defineConfig, fsd, nextjs } from '@enhance-eng/archangel';

export default defineConfig({
  extends: [
    fsd({ root: 'apps/web/src' }),
    fsd({ root: 'apps/admin/src', name: 'fsd-admin', strict: true }),
    nextjs({ root: 'apps/web' }),
  ],
  off: ['fsd/insignificant-slice'],
  rules: ({ rule }) => [
    rule({ name: 'fsd/public-api', folders: 'apps/web/src/features/*', mustContain: ['index.ts', 'index.tsx'] }),
  ],
});
```

Everything a preset declares is named with its prefix, `fsd/features` or `fsd/public-api`, so two
uses of one preset never collide and a team's own `shared` element never replaces the preset's. A
second use of a preset takes its own `name`. The config's own rules come after the presets: a rule
with the name of a preset rule replaces it, and `off` drops rules by name. A misspelt name in
either is refused with the nearest real one, rather than added as a new rule or ignored. The config's
own names cannot contain a slash.

A preset declares every part of its architecture, and most repositories have only some, so a
preset's element that matches nothing is listed in the summary and is not an error. A preset whose
elements match no file at all, most likely a wrong `root`, is `preset-empty`. `archangel test`
checks a preset's examples in your repository, under its `root`, with your tsconfig.

### Plugins

A plugin adds conditions written in TypeScript, for what the built-in conditions cannot say. A
condition receives a view of the graph, its rule's files with their element, captures, selectors
and the facts it reads (imports, importers or exports), and reports violations. Archangel runs
plugins in the Node process that loaded the config, never in its own, and turns each violation into
a diagnostic like any other, so exceptions, the baseline and every format apply.

This plugin keeps each entity to a number of stores, a file ending in `.store.ts`. Put it in
`plugins/team.ts`:

```ts
import { defineCondition, definePlugin } from '@enhance-eng/archangel';

const oneStorePerEntity = defineCondition({
  name: 'one-store-per-entity',
  description: 'Each entity has at most `max` stores.',
  docs: 'https://example.com/team/one-store-per-entity',
  // JSON Schema: archangel refuses a rule whose options break it when it loads the config.
  options: {
    type: 'object',
    properties: { max: { type: 'integer', minimum: 1 } },
    required: ['max'],
    additionalProperties: false,
  },
  // The facts of each file the condition reads: none here, only paths and captures.
  reads: [],
  check(view, options: { max: number }) {
    const stores = new Map<string, string[]>();
    for (const file of view.files) {
      if (!file.path.endsWith('.store.ts')) continue;
      const entity = file.captures.entity ?? '';
      stores.set(entity, [...(stores.get(entity) ?? []), file.path]);
    }
    return [...stores.values()]
      .filter((files) => files.length > options.max)
      .flatMap((files) =>
        files.map((file) => ({
          file,
          message: `${file} is one of ${files.length} stores in its entity, and an entity has at most ${options.max}`,
        })),
      );
  },
});

export default definePlugin({ name: 'team', schemaVersion: 1, conditions: [oneStorePerEntity] });
```

The config loads it under a name, and a rule uses its condition as `<name>/<condition>`, with
`options` and the examples that prove it:

```ts
import { defineConfig } from '@enhance-eng/archangel';
import team from './plugins/team.ts';

export default defineConfig({
  elements: { entity: 'src/entities/{entity}/**' },
  plugins: { team },
  rules: ({ rule }) => [
    rule({
      name: 'one-store',
      files: 'entity',
      plugin: 'team/one-store-per-entity',
      options: { max: 1 },
      reason: 'An entity keeps its state in one place',
      examples: {
        accepted: [{ 'src/entities/example/user.store.ts': 'export const user = 1;\n' }],
        rejected: [
          {
            'src/entities/example/user.store.ts': 'export const user = 1;\n',
            'src/entities/example/session.store.ts': 'export const session = 1;\n',
          },
        ],
      },
    }),
  ],
});
```

`archangel test` sends the view once and each example as a change to it, and fails for a plugin
rule with no rejected example, like any other. `archangel check` then reports each entity with two
stores. A violation names a file of the view, or a folder that holds one, and optionally an
import, `{ file, import: { specifier, occurrence }, message }`, which an exception above that
import can permit. A condition that throws, reports a file outside its view, or whose process
stops gives `plugin-failed` and fails the check: a plugin rule never passes because its code did not
run. In the editor, archangel's own diagnostics appear at once and the plugin's when Node answers.

Publish a plugin as `archangel-plugin-<name>` or `@scope/archangel-plugin`, and a preset as a
plain function beside it. The view's types are in `archangel`: `View`, `FileRecord`,
`ImportRecord` and `ExportsRecord`. A field added to the view never changes `schemaVersion`, so a
plugin reads only the fields it knows.

### The commands

```sh
archangel check                 # every diagnostic, and a summary
archangel check src/a.ts        # only that file's diagnostics; the whole repository is still checked
archangel test                  # every rule's examples, which prove the rule can find a violation
archangel explain src/a.ts      # the file's element, its rules, its imports and its importers
archangel why src/a.ts react    # the shortest chain of imports from the file to the other
archangel watch                 # check again after each change, printing what is new and gone
archangel baseline add          # record today's violations, so the check fails only for new ones
archangel baseline prune        # drop the entries that the code no longer needs
archangel lsp                   # the language server, which an editor starts
archangel init                  # a starter config
archangel docs layers           # what a diagnostic code means, and what to do about it
archangel completions zsh       # the completion script for bash, zsh, fish, elvish or PowerShell
```

`--format json` gives one JSON document from `check`, `test`, `explain` and `why`, and one JSON line
per check from `watch`. `--stats` adds the work counts and the time of each stage, `--color` takes
`auto`, `always` or `never`, and `--config` names a config instead of the nearest one at or above
the current folder. Every command exits with 0 when all is well, 1 when it found something, and 2
when the config, the baseline or the command line stops it.

### In CI

`--format github` prints each diagnostic as a GitHub Actions annotation, on its line of the pull
request. `--format sarif` writes a SARIF 2.1.0 log for GitHub code scanning or any dashboard that
reads SARIF; each result carries the violation's identity, so a moved line is still the same alert.

```yaml
- run: npx archangel check --format github
- run: npx archangel test --format github
```

[docs/exceptions-and-baselines.md](docs/exceptions-and-baselines.md) says when to permit a violation
and how; [docs/editors.md](docs/editors.md) says how to start the server in an editor and how a
coding agent reads watch mode; [crates/archangel_rules/docs/](crates/archangel_rules/docs/) explains every diagnostic, as
`archangel docs <code>` does.

## How this project works

Every non-trivial change starts as a design that is reviewed before any code is written, and the
tests in each design are its definition of done. [AGENTS.md](AGENTS.md) lists the rules, and
[CONTRIBUTING.md](CONTRIBUTING.md) says how to take part.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT),
at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
archangel by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without
any additional terms or conditions.
