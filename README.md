<div align="center">

# Archangel

**Make architecture verifiable.**

Architecture tests for TypeScript, checked in milliseconds.<br>
For your editor, your CI and your coding agents.

[![npm](https://img.shields.io/npm/v/@enhance-eng/archangel?color=8250df&label=npm)](https://www.npmjs.com/package/@enhance-eng/archangel)
[![crates.io](https://img.shields.io/crates/v/archangel?color=8250df)](https://crates.io/crates/archangel)
[![CI](https://github.com/enhance-labs-builds/archangel/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/enhance-labs-builds/archangel/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

<br>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/speed-dark.svg">
  <img alt="Seconds to check the same 400 architecture rules on a 9,800-file TypeScript monorepo: archangel 1.9 s; dependency-cruiser, ESLint and Steiger together 67.7 s" src="docs/assets/speed-light.svg" width="760">
</picture>

<sub>Time to check the same 400 architecture rules on a 9,800-file TypeScript monorepo, on a
laptop. Without archangel, these rules need three tools.</sub>

</div>

<br>

Archangel tests the structure of your TypeScript code: which code imports which code, where files
are, and what they export. You write each architecture decision as a test. When the code breaks a
test, archangel tells you, or your coding agent, where, why, and what to do.

## An example

The main form of a test is a **table**. Each row of a table is a group of files, and the list on that
row is what those files may import. The table refuses each other import into the repository.

```ts
import { defineConfig } from '@enhance-eng/archangel';

export default defineConfig({
  parts: {
    feature: 'src/features/{feature}/**',   // each folder in src/features is one feature
    shared: 'src/shared/**',
  },
  rules: ({ mayImport }) => [
    mayImport('app', {
      feature: ['shared'],
      shared: [],
    }, {
      because: 'Features share code only through src/shared, and shared code never depends on a feature.',
    }),

    mayImport('feature-code', {
      views: ['models', 'shared'],
      models: ['api', 'shared'],
      api: ['shared'],
    }, {
      kinds: 'src/features/*/{kind}/**',     // src/features/cart/views/cart-page.tsx is in views
      because: 'A view shows a model. Only the api folder talks to the server.',
    }),
  ],
});
```

- **`parts`** give names to the parts of your code. `{feature}` makes each folder in `src/features`
  one instance of `feature`.
- **The table `app`** says that a feature may import `shared` and itself, and not a different feature.
- **The table `feature-code`** says what each kind of file may import. `kinds` is the pattern that
  gives a file its kind, so `src/features/cart/views/cart-page.tsx` is in the row `views`.

A view in this app imports an api file, so `archangel check` shows:

```
error[may-import]: views may not import api, through "../api/fetch-cart": views may import only models, shared, and itself
 --> src/features/cart/views/cart-page.tsx:2:27
  |
2 | import { fetchCart } from '../api/fetch-cart';
  |                           ^^^^^^^^^^^^^^^^^^^ views may import only models, shared, and itself
  |
  = note: rule: feature-code.views
  = note: reason: A view shows a model. Only the api folder talks to the server.

checked 8 files and 10 imports: 1 problem
For what a code means, run `archangel docs may-import`.
```

`archangel rules` shows each row as a sentence, with the files and imports that it judged, so you can
see what your tests do.

## Highlights

- **Fast.** A Rust core reads each file one time. It checks 9,800 files in 1.9 seconds, 35 times
  faster than the three tools it replaces. Your editor shows the result of a change in 13
  milliseconds.
- **One tool, one config.** It holds the rules of dependency-cruiser, eslint-plugin-boundaries,
  eslint-plugin-check-file and Steiger.
- **Errors that say what to do.** Each error shows the line, the rule, your reason for the rule,
  and what the rule permits.
- **Tests that cannot lie.** A table that finds no file fails the check. A file in a table's files
  that has no row is a problem. Each rule that is not a table row needs an example of code that it
  must reject, and `archangel test` fails if the rule does not reject it.
- **Made for coding agents.** Watch mode, a language server, and JSON output give an agent the
  answer while it writes the code.
- **Presets and plugins.** Use the Feature-Sliced Design and Next.js presets, or write your own
  conditions in TypeScript.

## Get started

You need Node 22.18 or later.

```sh
npm install --save-dev @enhance-eng/archangel
npx archangel init --typescript
npx archangel check
```

`init` writes `archangel.config.ts`, with one part for each folder of source files and one rule
against import cycles. Add a table for your architecture, as in the example above. Then run
`npx archangel rules` to see what each row judges.

A rule that is not a table row needs a rejected example. Write it on one line:

```ts
rule('no-react-in-shared', {
  files: 'shared',
  mayNotImport: { packages: ['react'] },
  because: 'Shared code runs on the server too.',
  examples: { rejected: ['src/shared/price.ts imports react'] },
})
```

`npx archangel test` runs each example in memory. A rule with a wrong path finds no files and cannot
reject its example, so the test fails.
[docs/rules.md](docs/rules.md#prove-each-rule-with-examples) tells you how.

## Why archangel

Part of your architecture is in the source code: which code depends on which code, where files are,
and what each part exports. A reviewer sees a diff, not the graph of imports, so this part decays a
little with each change. Coding agents write code faster than people can review it, so it now
decays faster.

Archangel makes this part of your architecture testable. Unit tests check behavior. The type
checker checks types. Archangel tests architecture.

Archangel checks structure only. Use ESLint for code style and the TypeScript compiler for types.
[docs/principles.md](docs/principles.md) gives our opinions, and the ideas that archangel comes from.

## Use it every day

| Where | How |
| --- | --- |
| **In your editor** | Problems show as you type. The VS Code extension is in [`editors/vscode`](editors/vscode), and it is not on the Marketplace yet. For other editors, read [docs/editors.md](docs/editors.md). |
| **In CI** | Add `npx archangel check --format github` and `npx archangel test --format github` to your workflow. Each problem shows on its line in the pull request. |
| **With a coding agent** | Run `npx archangel watch`. Archangel checks again after each change and prints what is new and what is fixed. |
| **To see your tests** | Run `npx archangel rules`. It shows each rule as a sentence, what it judged, and what no rule judges. |
| **To understand a file** | Run `npx archangel explain src/features/cart/index.ts`. It shows the file's part, its rules and its imports. |
| **To understand one import** | Run `npx archangel explain src/features/cart/index.ts:3`. It shows each rule that judged the import on this line, and why each other rule did not. |
| **To find a chain of imports** | Run `npx archangel why src/app.tsx react`. It shows the shortest chain from one file to the other. |
| **For an old codebase** | Run `npx archangel baseline add`. Archangel records the problems of today, and the check fails only for new problems. Read [docs/exceptions-and-baselines.md](docs/exceptions-and-baselines.md). |

## What archangel can check

- **Imports:** which code may import which code, in a table, directly or through a chain of imports. Which
  names an import brings in. How an import is written. Whether a package is in `package.json`.
- **Cycles:** files that import each other.
- **Structure:** where files must be, which files a folder must contain, and which files must not
  exist.
- **Names:** the style of file and folder names, for example `kebab-case` or `PascalCase`.
- **Exports:** what a file must export, or must not export.

## Documentation

| Page | What it tells you |
| --- | --- |
| [docs/rules.md](docs/rules.md) | Parts, groups, tables, each condition, and how to prove a rule |
| [docs/presets.md](docs/presets.md) | The Feature-Sliced Design and Next.js presets |
| [docs/plugins.md](docs/plugins.md) | How to write your own conditions in TypeScript |
| [docs/commands.md](docs/commands.md) | Each command and option, the output formats, and the exit codes |
| [docs/exceptions-and-baselines.md](docs/exceptions-and-baselines.md) | How to permit a problem, and how to start on an old codebase |
| [docs/editors.md](docs/editors.md) | VS Code, Neovim and other editors |
| [docs/migrating.md](docs/migrating.md) | What changes when you move from dependency-cruiser, eslint-plugin-boundaries, check-file or Steiger |
| [docs/principles.md](docs/principles.md) | What archangel believes about architecture, and why |
| [Diagnostic codes](crates/archangel_rules/docs/) | What each error means, and how to fix it |

## Install with Rust

```sh
cargo install archangel
```

To build from a clone of this repository, run `cargo install --path crates/archangel`. You need
Rust 1.93 or later.

## Contributing

Each change starts as a design, and the design's tests define when the change is done.
[CONTRIBUTING.md](CONTRIBUTING.md) tells you how to take part, and [AGENTS.md](AGENTS.md) lists the
rules of this repository.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT),
at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
archangel by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without
any additional terms or conditions.
