# may-only-import

A file in a `mayOnlyImport` rule's selector imports a file or package that is neither on the rule's list nor in the selector itself.

## Example

```ts
// rule({ files: 'view', mayOnlyImport: ['shared', { packages: ['react'] }] })
// src/views/cart.view.tsx
import { store } from '../model/store';
```

An import of a stylesheet counts like any other, so a rule that permits stylesheets lists them, such as `'**/*.css'`. The options `names`, `specifier`, `instance` and `dependency` narrow the imports the rule checks, as on [`may-not-import`](may-not-import.md).

## What to do

Import only what the rule lists, or add the import's target to the list if the architecture permits it.
