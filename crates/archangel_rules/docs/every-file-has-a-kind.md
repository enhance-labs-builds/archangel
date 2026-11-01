# every-file-has-a-kind

A file in the `files` of an import table has no row in that table.

## Example

```ts
mayImport('domain', { entity: ['value-object'], 'value-object': [] }, {
  files: 'apps/api/src/*/domain/**',
  kinds: '**/*.{kind}.ts',
})
```

The file `apps/api/src/board/domain/board.helper.ts` has the kind `helper`, and the table has no row for it. A file with no kind gives the same problem.

## What to do

Add a row for the kind, rename the file so that it has a kind with a row, or use `not` in the table's `files` to leave the file out. To stop this check for all the table, put `<table>.every-file-has-a-kind` in `off`.

An exception comment cannot permit this problem, because no import is on its line. The baseline can hold it.

Only source files count. The config file is in no table.
