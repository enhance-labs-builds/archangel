# table-empty

A table has no row, or its rows together have no file, so the table checks nothing.

## Example

```ts
mayImport('domain', { entity: [] }, { kinds: '**/*.{kind}.tz' })
```

The repository has only `.ts` files, so no file has a kind.

## What to do

Add a row, check the kind pattern and the row names, or remove the table. A row with no file is not an error when the table has other files: the summary lists it. A table in a preset gives one line in the summary, not this error.
