# must-contain

An instance of an element, or a folder, in a `mustContain` rule has no file that matches the rule's pattern.

## Example

```ts
// rule({ files: 'slice', mustContain: 'index.ts' }), with slice: 'src/features/{slice}/**'
// src/features/billing, which has no index.ts
```

A list of patterns is met by any one of them, so `['model.*', 'model/**']` takes a file or a folder. In `folders`, a pattern that starts with `!` leaves the folders it matches out, as in `.gitignore`:

```ts
rule({ folders: ['src/shared/*', '!src/shared/{ui,lib}'], mustContain: 'index.*' })
```

## What to do

Add the file the message names. The pattern is relative to the instance's root, or to each folder for a rule about `folders`, and any file counts, whether or not archangel reads it, so `README.md` is found.
