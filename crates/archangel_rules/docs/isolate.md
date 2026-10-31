# isolate

A file in one instance of an element imports a file in a different instance of the same element, such as one feature importing another.

## Example

```ts
// isolate('feature'), with src/features/{feature}/**
// src/features/login/form.ts
import { price } from '../billing/price';
```

`isolate('feature')` is the sentence:

```ts
rule({ files: 'feature', mayNotImport: 'feature', instance: 'different' })
```

## What to do

Move what both instances need into an element they may both import, such as a shared element.
