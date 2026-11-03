# may-import

A file of a row of an import table imports something that its row does not list.

## Example

```ts
mayImport('feature-code', {
  views: ['models', 'shared'],
  models: ['api', 'shared'],
  api: ['shared'],
}, { kinds: 'src/features/*/{kind}/**' })
```

The file `src/features/cart/views/cart-page.tsx` imports `src/features/cart/api/fetch-cart.ts`. The row `views` does not list `api`, so archangel reports a violation of the rule `feature-code.views`.

## What a row means

Each row is one rule, with the name `<table>.<row>`. A row has one of two shapes.

A row that permits its own name is this sentence:

```ts
rule('feature-code.views', {
  files: 'views',
  mayOnlyImport: ['models', 'shared'],
  within: 'the repository and the externals sets that the table names',
})
```

A row in `mayNotImportItself` has this sentence too, without the permission of its own name, and a second sentence:

```ts
rule({ files: 'port', mayNotImport: 'port' })
```

A row permits its own name in the same instance. A capture in a part, or a capture other than `{kind}` in a kind pattern, makes an instance. A row that lists its own name in a cell permits every instance. The own-instance permission applies only when `files` names exactly one part or the row names one kind: a rule with `files: ['feature']`, a list, permits all instances.

A table judges an outside package only when one of its cells, or `everyRowMayImport`, names an externals set that holds the package.

Each table also makes the rule `<table>.every-file-has-a-kind`. A file in the table's files that is in no row breaks this rule. A table without `files` leaves a file with no kind outside the table. Write `files` to close that gap.

## What to do

Remove the import, or move the code to a file whose row permits the import. If the design has changed, add the target to the row. To permit one import for a time, write an exception that names the row.
