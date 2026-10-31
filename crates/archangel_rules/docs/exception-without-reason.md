# exception-without-reason

An `archangel-allow` comment gives no reason after the colon, so it permits nothing.

## Example

```ts
// archangel-allow entry(entity):
import { store } from '@/entities/canvas/model/store';
```

## What to do

Write why the import is allowed after the colon.
