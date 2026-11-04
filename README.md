<div align="center">

# Archangel

**Make architecture verifiable.**

Architecture tests for TypeScript, checked in milliseconds.<br>
For your editor, your CI and your coding agents.

### [Read the docs →](https://enhancelabs.ai/archangel/)

[![npm](https://img.shields.io/npm/v/@enhance-eng/archangel?color=fc622d&label=npm)](https://www.npmjs.com/package/@enhance-eng/archangel)
[![crates.io](https://img.shields.io/crates/v/archangel?color=fc622d)](https://crates.io/crates/archangel)
[![CI](https://github.com/enhance-labs-builds/archangel/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/enhance-labs-builds/archangel/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

<br>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset=".github/assets/speed-dark.svg">
  <img alt="Seconds to check the same 400 architecture rules on a 9,800-file TypeScript monorepo: archangel 1.9 s; dependency-cruiser, ESLint and Steiger together 67.7 s" src=".github/assets/speed-light.svg" width="760">
</picture>

<sub>The same 400 architecture rules on a 9,800-file TypeScript monorepo, on a laptop.</sub>

</div>

<br>

Archangel tests the structure of your TypeScript code: which code imports which code, where files
are, what they are called, and what they export. You write each architecture decision as a test.
When the code breaks a test, archangel tells you, or your coding agent, where, why, and what to do.

## Get started

You need Node 22.18 or later.

```sh
npm install --save-dev @enhance-eng/archangel
npx archangel init --typescript
npx archangel check
```

**[Getting started](https://enhancelabs.ai/archangel/start/getting-started/)** takes you from this to a table that finds a real
problem, in about ten minutes.

## An example

Each row of a table is a set of files, and the list on that row is what those files may import:

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

A view in this app imports an api file, so `archangel check` shows:

```
error[may-import]: views may not import api, through "../api/fetch-cart": views may import only models, shared, and itself
 --> src/features/cart/views/cart-page.tsx:2:27
  |
2 | import { fetchCart } from '../api/fetch-cart';
  |                           ^^^^^^^^^^^^^^^^^^^ views may import only models, shared, and itself
  |
  = help: remove the import, or move the code to a file whose row permits the import
  = note: rule: feature-code.views
  = note: because: A view shows a model. Only the api folder talks to the server.

checked 8 files and 10 imports: 1 problem
For what a code means, run `archangel docs may-import`.
```

## Where to go next

| To | Read |
| --- | --- |
| Learn how archangel thinks | [How archangel works](https://enhancelabs.ai/archangel/start/how-archangel-works/) |
| Add it to an old codebase | [Start on an existing repository](https://enhancelabs.ai/archangel/guides/existing-repository/) |
| Move from another tool | [dependency-cruiser](https://enhancelabs.ai/archangel/guides/from-dependency-cruiser/), [eslint-plugin-boundaries](https://enhancelabs.ai/archangel/guides/from-eslint-plugin-boundaries/), [eslint-plugin-check-file](https://enhancelabs.ai/archangel/guides/from-eslint-plugin-check-file/), [Steiger](https://enhancelabs.ai/archangel/guides/from-steiger/) |
| Run it in CI, an editor or an agent | [CI](https://enhancelabs.ai/archangel/guides/ci/), [Editors and coding agents](https://enhancelabs.ai/archangel/guides/editors-and-agents/) |
| Look up a condition, a command or an error | [Reference](https://enhancelabs.ai/archangel/reference/conditions/) |
| Give the docs to a coding agent | [llms.txt](https://enhancelabs.ai/archangel/llms.txt) |

## Install with Rust

```sh
cargo install archangel
```

## Contributing

Each change starts as a design, and the design's tests define when the change is done.
[CONTRIBUTING.md](CONTRIBUTING.md) tells you how to take part, [AGENTS.md](AGENTS.md) lists the
rules of this repository, and [RELEASING.md](RELEASING.md) tells maintainers how to release.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT),
at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
archangel by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without
any additional terms or conditions.
