# may-not-import

A file in a `mayNotImport` rule's selector imports a file or package on the rule's other side, directly or, for a transitive rule, through a chain of imports.

## Example

```ts
// rule({ files: 'entity', mayNotImport: { packages: ['react'] } })
// src/entities/canvas/model.ts
import { useState } from 'react';
```

Options narrow the imports the rule checks:

- `names: ['useEffect']`: only imports that bring in one of these names. A default import brings in `default`; a namespace import, an `export *`, a dynamic import and a `require` bring in every name.
- `specifier`: only imports written `'relative'` (`./` or `../`), `'parent'` (`../`), `'alias'` (a tsconfig path or a package `imports` field) or `'package'`.
- `instance: 'same'` or `'different'`: only imports within one instance of `files`, or between two; `files` must be an element with a capture.
- `dependency: 'dev'` or `'undeclared'`: only imports of packages that the importing file's nearest `package.json` lists only as development dependencies, or does not list.
- `transitively: true`: chains of imports count. The diagnostic shows the shortest chain, such as `app/a.tsx → lib/b.ts → server-only`, and points at the chain's first import. A chain stops at packages and at files in `stopAt`, such as `{ directive: 'use server' }`. A transitive rule checks runtime imports unless it says otherwise, and takes none of the four options above.

## What to do

Remove the import, or move the code to files that may import it. For a chain, remove one of its imports.
