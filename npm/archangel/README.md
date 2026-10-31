# Archangel

**Architecture rules for TypeScript.** You write down how your code is organized. Archangel tells
you when the code breaks those rules, and what to do about it. It checks a large repository in about
a second, so it can run in your editor as you type.

```sh
npm install --save-dev @enhance-eng/archangel
npx archangel init --typescript
npx archangel check
```

Archangel checks imports, import cycles, file and folder names, the files that folders must contain,
and exports. It has presets for Feature-Sliced Design and Next.js, and you can write your own
conditions in TypeScript.

- [Quick start](https://github.com/enhance-labs-builds/archangel/blob/main/README.md#quick-start)
- [Rules](https://github.com/enhance-labs-builds/archangel/blob/main/docs/rules.md)
- [Presets](https://github.com/enhance-labs-builds/archangel/blob/main/docs/presets.md)
- [Plugins](https://github.com/enhance-labs-builds/archangel/blob/main/docs/plugins.md)
- [Commands](https://github.com/enhance-labs-builds/archangel/blob/main/docs/commands.md)

The package includes a prebuilt binary for macOS, Linux and Windows. You do not need Rust.

Licensed under MIT or Apache-2.0, at your option.
