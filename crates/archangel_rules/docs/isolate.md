# isolate

A file in one instance of a part imports a file in a different instance of the same part, such as one feature importing another.

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

Move what both instances need into a part they may both import, such as a shared part.
