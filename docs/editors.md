# Editors and coding agents

`archangel lsp` is a language server on standard input and output. It gives the diagnostics of a
full check as the editor starts, follows unsaved text as you type, and offers each checked fix as a
quick fix. It finds the config the same way the command line does, from the folder the editor
opened.

## Neovim

Neovim 0.11 and later start a language server from plain config. In `init.lua`:

```lua
vim.lsp.config('archangel', {
  cmd = { 'npx', 'archangel', 'lsp' },
  filetypes = { 'typescript', 'typescriptreact', 'javascript', 'javascriptreact' },
  root_markers = {
    'archangel.config.ts',
    'archangel.config.mts',
    'archangel.config.js',
    'archangel.config.mjs',
    'archangel.config.json',
  },
})
vim.lsp.enable('archangel')
```

With archangel installed by Cargo instead of npm, use `cmd = { 'archangel', 'lsp' }`.

## VS Code and Zed

VS Code starts the server through the Archangel extension, in
[`editors/vscode`](../editors/vscode/README.md). It starts in a workspace whose folder has an
archangel config, and runs the first archangel it finds: the `archangel.path` setting, then the
workspace's `node_modules/.bin/archangel`, then `archangel` on `PATH`. The **Archangel** output
channel says which one it ran. When it finds none, it shows an error naming the three places, and
**Archangel: Restart Server** looks again after you install archangel.

Zed starts a language server only through an extension, and this release has none. Until it does,
run `archangel watch` in Zed's terminal: it prints each new and each gone diagnostic after every
save.

## Coding agents

An agent reads watch mode best as JSON:

```sh
archangel watch --format json
```

Each check prints one line: a document with `schemaVersion`, the diagnostics that are `new`, the
ones that are `gone`, and a `summary` with the counts and the work the check took. The first line
lists every diagnostic as new. An agent can start it beside its work, read the line after each
edit, and fix what is new before it moves on. When the system has no file watches left, archangel
polls, and every summary says so in `polling`.
