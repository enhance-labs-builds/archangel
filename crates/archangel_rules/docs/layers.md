# layers

An import goes from one part of a layers rule to a part earlier in its list.

## Example

```ts
// layers(['app', 'feature', 'shared'])
// src/shared/format.ts
import { login } from '../features/login';
```

`layers(['app', 'feature', 'shared'])` is one sentence for each part after the first:

```ts
rule({ files: 'feature', mayNotImport: ['app'] })
rule({ files: 'shared', mayNotImport: ['app', 'feature'] })
```

## What to do

Move the code that both need into the later part, or import it from a part later in the list. Parts outside the rule's list are not checked by it.
