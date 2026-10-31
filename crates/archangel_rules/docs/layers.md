# layers

An import goes from one element of a layers rule to an element earlier in its list.

## Example

```ts
// layers(['app', 'feature', 'shared'])
// src/shared/format.ts
import { login } from '../features/login';
```

`layers(['app', 'feature', 'shared'])` is one sentence for each element after the first:

```ts
rule({ files: 'feature', mayNotImport: ['app'] })
rule({ files: 'shared', mayNotImport: ['app', 'feature'] })
```

## What to do

Move the code that both need into the later element, or import it from an element later in the list. Elements outside the rule's list are not checked by it.
