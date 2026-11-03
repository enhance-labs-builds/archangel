# may-only-be-imported-by

A file imports a file in the `files` of a `mayOnlyBeImportedBy` rule, and the importing file is neither on the rule's list nor in those files. A file in no part counts as outside the list.

## Example

```ts
// rule({ files: 'db', mayOnlyBeImportedBy: ['api'] })
// src/web/page.ts
import { users } from '../db/users';
```

The options `names`, `specifier`, `instance` and `dependency` narrow the imports the rule checks, as on [`may-not-import`](may-not-import.md).

Add `within` to judge only importers in an area. The rule does not judge an importer outside the area.

```ts
// rule({ files: 'db', mayOnlyBeImportedBy: 'api', within: 'apps/**' })
// scripts/seed.ts is outside the area, so the rule does not judge it.
```

## What to do

Go through a file on the list, such as an `api` module, instead of importing the rule's files directly.
