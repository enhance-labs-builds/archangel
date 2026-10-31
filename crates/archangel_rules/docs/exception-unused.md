# exception-unused

An `archangel-allow` comment permits nothing, because the line after it does not break the rule it names. Exceptions go away when the code is fixed.

## Example

```ts
// archangel-allow entry(entity): the store was moved
import { store } from '@/entities/canvas';
```

## What to do

Delete the comment.
