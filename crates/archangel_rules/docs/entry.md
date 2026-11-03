# entry

Code outside an instance of a part imports a file inside it that is not one of the instance's entry files. A sibling instance counts as outside.

## Example

```ts
// entry('entity', 'index.ts'), with src/entities/{entity}/**
import { store } from '@/entities/ledger/model/store';
```

`entry('entity', 'index.ts')` is one sentence for each instance, such as `ledger`, whose entry files are relative to the instance root:

```ts
rule({
  files: { all: [{ element: 'entity', capture: { entity: 'ledger' } }, { not: 'src/entities/ledger/index.ts' }] },
  mayOnlyBeImportedBy: { element: 'entity', capture: { entity: 'ledger' } },
})
```

## What to do

Import the instance through its entry file, such as `@/entities/ledger`. When the entry file does not export what you need, export it there first.
