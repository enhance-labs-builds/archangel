# Archangel

[![CI](https://github.com/enhance-labs-builds/archangel/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/enhance-labs-builds/archangel/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

**Architecture rules for TypeScript.** You write down how your code is organized. Archangel tells
you when the code breaks those rules, and what to do about it. It checks a large repository in about
a second, so it can run in your editor as you type.

```text
error[layers]: "shared" may not import "feature", an earlier layer, through "../features/cart"
 --> src/shared/price.ts:1:22
  |
1 | import { cart } from '../features/cart';
  |                      ^^^^^^^^^^^^^^^^^^ "shared" may import only "shared"
  |
  = note: rule: layers(feature, shared)
  = note: reason: Shared code must not depend on features
```

> **Status:** the first release is almost ready. Until it is on npm, build archangel from source
> (see [Install from source](#install-from-source)).

## Quick start

These steps take about five minutes. You need Node 22.18 or later.

**1. Install archangel** in your project:

```sh
npm install --save-dev @enhance-eng/archangel
```

**2. Make a config.** This command writes `archangel.config.ts`, with one element for each folder of
source code, and a rule against import cycles:

```sh
npx archangel init --typescript
```

**3. Run a check:**

```sh
npx archangel check
```

**4. Write your rules.** Open `archangel.config.ts`. This example has features in
`src/features/<name>/` and shared code in `src/shared/`:

```ts
import { defineConfig } from '@enhance-eng/archangel';

export default defineConfig({
  // Give a name to each part of your code.
  elements: {
    feature: 'src/features/{feature}/**', // each folder in src/features is one feature
    shared: 'src/shared/**',
  },
  rules: ({ layers, isolate, entry }) => [
    // Code in a later layer may not import an earlier layer.
    layers(['feature', 'shared'], { reason: 'Shared code must not depend on features' }),
    // One feature may not import a different feature.
    isolate('feature', { reason: 'Features must not depend on each other' }),
    // Other code may import a feature only through its index.ts.
    entry('feature', 'index.ts', { reason: 'Other code uses a feature only through its index.ts' }),
  ],
});
```

**5. Run the check again.** Archangel shows each problem, the rule, your reason, and what to do:

```text
error[isolate]: "cart" of "feature" imports "login" of "feature" through "../login/session", and instances of "feature" may not import each other
 --> src/features/cart/index.ts:1:25
  |
1 | import { session } from '../login/session';
  |                         ^^^^^^^^^^^^^^^^^^ move what both need into an element that instances of "feature" may share
  |
  = note: rule: isolate(feature)
  = note: reason: Features must not depend on each other

checked 6 files and 5 imports: 3 problems
```

**6. Prove your rules.** A rule with a typing error can find no files and pass silently. To prevent
this, give each rule an example that it must reject, then run:

```sh
npx archangel test
```

[docs/rules.md](docs/rules.md#prove-each-rule-with-examples) shows how to write examples.

## Use it every day

| Where | How |
| --- | --- |
| **In your editor** | Problems show as you type. The VS Code extension is in [`editors/vscode`](editors/vscode), and it is not on the Marketplace yet. For other editors, read [docs/editors.md](docs/editors.md). |
| **In CI** | Add `npx archangel check --format github` and `npx archangel test --format github` to your workflow. Each problem shows on its line in the pull request. |
| **With a coding agent** | Run `npx archangel watch`. Archangel checks again after each change and prints what is new and what is fixed. |
| **To understand a file** | Run `npx archangel explain src/features/cart/index.ts`. It shows the file's element, its rules and its imports. |
| **To find a chain of imports** | Run `npx archangel why src/app.tsx react`. It shows the shortest chain from one file to the other. |
| **For an old codebase** | Run `npx archangel baseline add`. Archangel records the problems of today, and the check fails only for new problems. Read [docs/exceptions-and-baselines.md](docs/exceptions-and-baselines.md). |

## What archangel can check

- **Imports:** which code may import which code, directly or through a chain of imports. Which
  names an import brings in. How an import is written. Whether a package is in `package.json`.
- **Cycles:** files that import each other.
- **Structure:** where files must be, which files a folder must contain, and which files must not
  exist.
- **Names:** the style of file and folder names, for example `kebab-case` or `PascalCase`.
- **Exports:** what a file must export, or must not export.

Archangel does not check code style or types. ESLint and the TypeScript compiler do that well.

There is more:

- **Presets** give you a set of rules in one line. Archangel has presets for
  [Feature-Sliced Design](https://feature-sliced.design) and for Next.js. Read
  [docs/presets.md](docs/presets.md).
- **Plugins** let you write your own conditions in TypeScript. Read
  [docs/plugins.md](docs/plugins.md).

## Why archangel

Many teams use several tools for architecture rules: dependency-cruiser or an ESLint plugin for
imports, another plugin for file names, and Steiger for Feature-Sliced Design. Each tool reads the
whole repository again. The checks are slow, so they usually run only in CI. And a rule with a wrong
path can find no files and still pass.

Archangel puts these rules in one config. A Rust core reads each file one time, and the editor, the
watcher and the command line use the same results.

On a 9,800-file monorepo, archangel checks 400 rules in 1.9 seconds. dependency-cruiser, two ESLint
plugins and Steiger take about 68 seconds for the same rules. In the editor, archangel shows the
result of an edit in 13 milliseconds.

| Repository | Files | dependency-cruiser | archangel |
| --- | --- | --- | --- |
| [VS Code](https://github.com/microsoft/vscode) 1.144.0: layers and no cycles | 7,652 | 38.7 s | 0.36 s |
| A TypeScript monorepo with 257 import rules | 9,704 | 19.7 s | 1.26 s |

These numbers are from a 14-core laptop with a warm file cache. `scripts/bench` reproduces the
archangel numbers.

## Learn more

| Page | What it tells you |
| --- | --- |
| [docs/rules.md](docs/rules.md) | Elements, selectors, every condition, and how to prove a rule |
| [docs/presets.md](docs/presets.md) | The Feature-Sliced Design and Next.js presets, and how to change them |
| [docs/plugins.md](docs/plugins.md) | How to write your own conditions in TypeScript |
| [docs/commands.md](docs/commands.md) | Every command and option, the output formats, and the exit codes |
| [docs/exceptions-and-baselines.md](docs/exceptions-and-baselines.md) | How to permit a problem, and how to start on an old codebase |
| [docs/editors.md](docs/editors.md) | How to use archangel in VS Code, Neovim and other editors |
| [Diagnostic codes](crates/archangel_rules/docs/) | What each error means, and how to fix it (`archangel docs <code>` shows the same text) |

## Install from source

With Rust 1.93 or later, from a clone of this repository:

```sh
cargo install --path crates/archangel
```

After the first release, `cargo install archangel` also works.

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
