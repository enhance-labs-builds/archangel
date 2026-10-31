# no-cycles

A group of files import each other, so each can reach the others. The diagnostic lists every file in the group and shows its shortest cycle.

## Example

```ts
// rule({ files: 'src/**', noCycles: true })
// src/a.ts
import { b } from './b';
// src/b.ts
import { a } from './a';
```

## What to do

Remove one import of the cycle, or move what both files need into a third file that neither imports. Type imports do not count unless the rule says `imports: 'all'`.
