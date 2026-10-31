# import-unresolved

Archangel cannot find the file or package that an import names. The message says why in plain words.

## Example

```ts
import { missing } from './missing';
```

## What to do

Fix the specifier. For a module that a tool makes at build time, add the specifier to `allowUnresolved` with the reason, such as `{ 'astro:*': 'Astro makes these at build time' }`.
