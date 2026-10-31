# Archangel for VS Code

[Archangel](https://github.com/enhance-labs-builds/archangel) runs the architecture tests of a
TypeScript or JavaScript repository: imports, import cycles, file and folder names, folder contents,
and exports. This extension shows its diagnostics in the editor as you type, before you save, and offers
each fix that archangel has checked as a quick fix.

The extension runs `archangel lsp`, the language server built into the archangel binary. It starts
in a workspace whose folder has an `archangel.config.ts`, `.mts`, `.js`, `.mjs` or `.json`, and checks
TypeScript and JavaScript files, with or without JSX.

## Install archangel

The extension does not bundle archangel. Install it in the workspace:

```sh
npm install --save-dev @enhance-eng/archangel
```

## Which archangel it runs

The extension looks in three places, in order, and the **Archangel** output channel says which one
it used:

1. The `archangel.path` setting.
2. The workspace's `node_modules/.bin/archangel`.
3. `archangel` on `PATH`, as after `cargo install archangel`.

When none has it, the extension shows an error that names the three places, and starts nothing.
When `archangel.path` names no file, that is an error too: the extension does not fall back to the
other two.

## Settings

| Setting | Default | What it does |
| --- | --- | --- |
| `archangel.path` | empty | The binary to run. A relative path is taken from the workspace folder. |
| `archangel.trace.server` | `off` | `messages` or `verbose` logs the traffic with the server to the output channel. |

## Commands

**Archangel: Restart Server** stops the server and starts it again, finding the binary afresh. Run it
after you change `archangel.path` or install archangel.

## Limits

In a workspace with several folders, the extension checks the first one.
