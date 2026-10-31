# exports-unknown

A `mustExport` rule needs the names a file exports, and the file passes names on through an `export *` from a package or from a specifier that does not resolve, so archangel cannot know them.

## Example

```ts
// rule({ files: '**/*.view.tsx', mustExport: '{FileName}' })
// src/chip.view.tsx
export * from '@acme/chips';
```

Archangel follows `export *` from files in the repository, through any number of files, and counts every name the chain brings in except `default`. It gives `exports-unknown` only when the name the rule needs is not among the names it found and some link of the chain leads into a package or does not resolve. The message names the rule, the file and each specifier it could not follow. It reports no violation of the rule for that file, because it cannot prove one, and it does not pass the rule either: `exports-unknown` fails the check.

A file that does not parse gets [`parse-failed`](parse-failed.md) instead, and no export diagnostic.

## What to do

Export the names the rule needs by name, such as `export { Chip } from '@acme/chips'`, or fix the specifier that does not resolve.
