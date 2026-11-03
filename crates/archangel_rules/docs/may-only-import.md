# may-only-import

A file in a `mayOnlyImport` rule's selector imports a file or package that is neither on the rule's list nor in the selector itself. When the selector is a part with a capture, only the files of the same instance count as the selector itself.

## Example

```ts
// rule({ files: 'view', mayOnlyImport: ['shared', { packages: ['react'] }] })
// src/views/cart.view.tsx
import { store } from '../model/store';
```

An import of a stylesheet counts like any other, so a rule that permits stylesheets lists them, such as `'**/*.css'`. The options `names`, `specifier`, `instance` and `dependency` narrow the imports the rule checks, as on [`may-not-import`](may-not-import.md).

## Limit the rule to an area

Add `within` to judge only imports into an area. The rule does not judge an import whose target is outside the area. An empty list is permitted only with an area.

```ts
// rule({ files: 'entity', mayOnlyImport: ['value-object'], within: 'apps/api/src/**' })
// apps/api/src/board/domain/board.entity.ts
import { save } from '../application/save.use-case'; // reported
import { z } from 'zod'; // outside the area, so not judged
```

## Instances

When `files` names exactly one part with a capture, the rule permits its own files only in the same instance. With a list, such as `files: ['feature']`, the rule permits all instances of its own files. To permit all instances of one part, list the part in `mayOnlyImport`.

## What to do

Import only what the rule lists, or add the import's target to the list if the architecture permits it.
