# table-row-overlap

A file in a table's files is in two rows of that table. Each of the two rows judges the file.

## Example

```ts
groups: { model: 'src/**/*.model.ts', feature: 'src/features/**' },
rules: ({ mayImport }) => [mayImport('app', { model: [], feature: [] })],
```

The file `src/features/cart/cart.model.ts` is in `model` and in `feature`.

## What to do

Narrow one of the two rows with `not`, or leave the file out of the table's files. Fix this problem in the config, because the baseline does not hold it.
