# may-only-be-imported-by

A file imports a file in a `mayOnlyBeImportedBy` rule's selector, and the importing file is neither on the rule's list nor in the selector itself. A file in no element counts as outside the list.

## Example

```ts
// rule({ files: 'db', mayOnlyBeImportedBy: ['api'] })
// src/web/page.ts
import { users } from '../db/users';
```

The options `names`, `specifier`, `instance` and `dependency` narrow the imports the rule checks, as on [`may-not-import`](may-not-import.md).

## What to do

Go through a file on the list, such as an `api` module, instead of importing the selector's files directly.
