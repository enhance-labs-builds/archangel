# exception-unused

An `archangel-allow` comment permits nothing, because the import after it does not break the rule it names. The import after it is the one whose specifier is on the next line, or whose statement starts on the next line. Exceptions go away when the code is fixed.

## Example

```ts
// archangel-allow entry(entity): the store was moved
import { store } from '@/entities/ledger';
```

## What to do

Delete the comment.
