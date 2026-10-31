# entry

Code outside an element instance imports a file inside it that is not one of the instance's entry files. A sibling instance counts as outside.

## Example

```ts
// entry('entity', 'index.ts'), with src/entities/{entity}/**
import { store } from '@/entities/canvas/model/store';
```

`entry('entity', 'index.ts')` is one sentence for each instance, such as `canvas`, whose entry files are relative to the instance root:

```ts
rule({
  files: { all: [{ element: 'entity', capture: { entity: 'canvas' } }, { not: 'src/entities/canvas/index.ts' }] },
  mayOnlyBeImportedBy: { element: 'entity', capture: { entity: 'canvas' } },
})
```

## What to do

Import the instance through its entry file, such as `@/entities/canvas`. When the entry file does not export what you need, export it there first.
