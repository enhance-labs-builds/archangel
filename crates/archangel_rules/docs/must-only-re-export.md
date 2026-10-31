# must-only-re-export

A file in a `mustOnlyReExport` rule has a top-level statement of its own: a declaration, a type included, or code that runs. It may hold only imports, exports from other files, directives, and `export { … }` of names it imported.

## Example

```ts
// rule({ files: '**/index.ts', mustOnlyReExport: true })
export { a } from './a';
export function b() {}
```

## What to do

Move the declaration into a file of its own and re-export it from there.
