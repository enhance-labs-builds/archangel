# Rules

This page tells you how to write rules. A rule has two parts:

- **Files**: the files that the rule is about.
- **One condition**: what those files must do, or must not do.

Read a rule as a sentence: "These files may not import React."

```ts
rule('no-react-in-shared', { files: 'shared', mayNotImport: 'react' })
```

A table is many of these rules in one place. Use a table for the imports of your architecture, and a
rule for each other decision.

## Name the parts of your code

Before you write rules, give names to the parts of your code. Archangel has three kinds of names.

### Parts

A part is a part of your code, found by a path pattern. Each file is in one part at most. If two
parts find one file, archangel shows the error `part-overlap`.

```ts
parts: {
  feature: 'src/features/{feature}/**',
  shared: 'src/shared/**',
},
```

`{feature}` captures one part of the path. Each folder in `src/features` is then one **instance** of
the part `feature`, for example `login` or `cart`.

### Groups

A group is a named set of files. A file can be in many groups. Use a group for files that cut across
parts, for example tests.

```ts
groups: {
  test: '**/*.test.ts',
  client: { directive: 'use client' },
},
```

### Externals

An externals set is a named set of outside packages. Use it on the other side of an import rule, or
in a table.

```ts
externals: {
  driver: ['pg', '@aws-sdk/*', 'node:fs'],
},
```

- A name matches the package and each of its subpaths, so `pg` matches `pg/lib/client`.
- `@aws-sdk/*` matches each package in a scope, and `*` matches each package.
- Write a Node built-in module with `node:`. Archangel refuses `fs`, and tells you to write `node:fs`.
- Archangel refuses a name with a subpath, a name that starts with `@types/`, and the name of a
  package in your repository.

### Ways to write a set of files

You can write a set of files in these ways. They work in `parts`, in `groups`, and in a rule's
`files`.

| You write | The set has |
| --- | --- |
| `'feature'` | the files of the part or group with this name |
| `'src/**/*.view.tsx'` | the files that the path pattern finds (a pattern has a `/` or a `*`) |
| `['app', 'feature']` | the files in one or more of these sets |
| `{ any: ['app', 'feature'] }` | the files in one or more of these sets |
| `{ all: ['src/**', { not: 'test' }] }` | the files that are in all of these sets |
| `{ not: 'test' }` | all files except these files |
| `{ element: 'feature', capture: { feature: 'login' } }` | the files of one instance |
| `{ element: '*' }` | the files that are in a part |
| `{ directive: 'use client' }` | the files that start with this directive |

If a set has no files, archangel shows the error `selector-empty`. A rule about an empty set can
never find a problem, so archangel does not let it pass silently.

## Tables

A table says what each group of files may import.

```ts
mayImport('feature-code', {
  views: ['models', 'shared'],
  models: ['api', 'shared'],
  api: ['shared'],
}, {
  kinds: 'src/features/*/{kind}/**',
  because: 'A view shows a model. Only the api folder talks to the server.',
})
```

Each row is a rule, named `<table>.<row>`. The name is `feature-code.views` in this example. You use
this name in an exception, in the baseline and in `off`.

### What a row is

A row name is a declared part, a declared group, or a kind. One table can mix them.

A **kind** is a name that the table's `kinds` pattern gives to a file. The pattern has the capture
`{kind}`:

| Pattern | File | Kind |
| --- | --- | --- |
| `**/*.{kind}.ts` | `src/board/board.entity.ts` | `entity` |
| `src/features/*/{kind}/**` | `src/features/cart/views/page.tsx` | `views` |
| `**/{kind}/**/*.{kind}.ts` | `src/entity/board.entity.ts` | `entity` |

`{kind}` can be in a pattern two times. Then the two places must give the same name. A declaration
file counts `.d.ts` as one extension, so `env.d.ts` has no kind `d`.

A kind means the same files in all of the table. Use `files` to choose which files the table judges
as importers. The target of an import has its kind everywhere.

### What a row permits

A row permits only:

- the names on its list. A name in a list is a row, a part, a group, an externals set or a path
  pattern, and archangel refuses a name that is none of these;
- its own name, in the same instance. Use `mayNotImportItself` to remove this;
- what `everyRowMayImport` lists.

The row refuses each other import into the repository. An import of a target that has no row gives
the message "no row for ...". A table judges an import of an outside package only if one of its
cells, or `everyRowMayImport`, names an externals set with that package.

### Each file has a row

Each table makes the rule `<table>.every-file-has-a-kind`. This rule refuses a file in the table's
`files` that has no row. A table without `files` has no list of files to look at, so it does not
check a file that has no kind.

To let a file out of a table, write `not` in the table's `files`, or put the rule's name in `off`.

If two rows find one file, archangel shows the error `table-row-overlap`. If a table has no row, or
its rows find no file, archangel shows the error `table-empty`. A single row with no file is not an
error, because you can write a kind before the first file has it. `archangel check` and
`archangel rules` list that row.

### Options

| Option | What it does |
| --- | --- |
| `kinds` | The pattern that gives a file its kind. |
| `files` | The files that the table judges as importers. |
| `within` | The area that the table judges. The table ignores an import outside it. |
| `everyRowMayImport` | Names that each row has on its list. |
| `mayNotImportItself` | Rows that may not import their own name. |
| `because`, `link`, `examples` | The same as for a rule. |
| `imports`, `names`, `specifier`, `instance`, `dependency` | The same as for an import condition. Each row gets the option. |

A table row does not need a written example. You can write examples for a table: they run against
all its rows, and a rejected example passes when one row rejects it.

## Conditions

Each rule has one condition.

### Import conditions

| Condition | Example | Archangel reports |
| --- | --- | --- |
| `mayNotImport` | `rule('a', { files: 'shared', mayNotImport: 'react' })` | an import of `react` in `shared` |
| `mayOnlyImport` | `rule('a', { files: 'view', mayOnlyImport: ['shared', 'react'] })` | an import in `view` of something that is not in the list |
| `mayOnlyBeImportedBy` | `rule('a', { files: 'db', mayOnlyBeImportedBy: 'api' })` | an import of `db` by a file that is not in `api` or `db` |
| `noCycles` | `rule('a', { files: 'src/**', noCycles: true })` | files that import each other in a cycle |

`noCycles` counts only runtime imports. The compiler removes an `import type`, so a cycle of type
imports cannot break when the code runs. To report type cycles too, add `imports: 'all'`.

`mayOnlyImport` permits the rule's own files. If `files` is one part with a capture, it permits only
the files of the same instance. To permit each instance, add the part to the list.

### Structure conditions

| Condition | Example | Archangel reports |
| --- | --- | --- |
| `mustNotExist` | `rule('a', { files: { all: ['src/**', { not: { element: '*' } }] }, mustNotExist: true })` | a file that is in no part |
| `mustBeNamed` | `rule('a', { files: 'src/components/**/*.tsx', mustBeNamed: 'PascalCase' })` | a file name that is not in this style |
| `mustContain` | `rule('a', { files: 'feature', mustContain: 'index.ts' })` | an instance that does not have this file |

### Export conditions

| Condition | Example | Archangel reports |
| --- | --- | --- |
| `mustExport` | `rule('a', { files: '**/*.view.tsx', mustExport: '{FileName}' })` | a view that does not export the name of its file |
| `mayNotExport` | `rule('a', { files: 'src/**', mayNotExport: 'default' })` | a default export |
| `mustOnlyReExport` | `rule('a', { files: '**/index.ts', mustOnlyReExport: true })` | an index file that has code of its own |

## Make an import condition narrower

Add one of these options to an import condition. The condition then applies only to some imports.

| Option | The condition applies only to |
| --- | --- |
| `names: ['useEffect']` | imports of these names |
| `specifier: 'relative'` | imports that start with `./` or `../` |
| `specifier: 'parent'` | imports that start with `../` |
| `specifier: 'alias'` | imports through a tsconfig path or a package `imports` field |
| `specifier: 'package'` | imports of packages |
| `instance: 'same'` | imports in one instance of `files` (`files` must be an element with a capture) |
| `instance: 'different'` | imports from one instance to a different instance |
| `dependency: 'dev'` | packages that the nearest `package.json` lists only in `devDependencies` |
| `dependency: 'undeclared'` | packages that the nearest `package.json` does not list |
| `within: 'apps/api/src/**'` | imports into this area. It works on `mayOnlyImport` and `mayOnlyBeImportedBy`. |

`within` removes a double negative. To say that a file may import only some of the files in an area,
and may import each thing outside it:

```ts
rule('entity-imports', {
  files: 'entity',
  mayOnlyImport: ['value-object', 'error'],
  within: 'apps/api/src/**',
})
```

Without `within`, `mayOnlyImport` judges each import, and each package. To limit `mayNotImport` to an
area, write the area on the other side: `mayNotImport: { all: ['use-case', 'apps/api/src/**'] }`.

### Rules about chains of imports

Add `transitively: true` to `mayNotImport`. The rule then finds chains of imports, not only direct
imports. The report shows the shortest chain. Use `stopAt` to end a chain at some files.

```ts
rule('server-stays-out', {
  files: { directive: 'use client' },
  mayNotImport: { packages: ['server-only'] },
  transitively: true,
  stopAt: { directive: 'use server' },
})
```

A rule with `transitively` cannot also have `names`, `specifier`, `instance` or `dependency`.

## Rules about folders

Write `folders` instead of `files`. A folder rule can use `mustBeNamed`, `mustContain` or
`mustNotExist`. A pattern that starts with `!` removes folders, as in `.gitignore`.

```ts
rule('folder-names', { folders: 'src/**', mustBeNamed: 'kebab-case' }),
rule('shared-entry', { folders: ['src/shared/*', '!src/shared/{ui,lib}'], mustContain: ['index.ts', 'index.tsx'] }),
```

`mustContain` accepts a list. One file from the list is sufficient.

## Name styles

The styles are `kebab-case`, `camelCase`, `PascalCase`, `snake_case` and `SCREAMING_SNAKE_CASE`.

A name pattern can use the name of the file or its folder, in each style. For example, `{FileName}`
is the file name in PascalCase, and `{folder-name}` is the folder name in kebab-case.

```ts
rule('test-names', { files: '**/*.test.ts', mustBeNamed: '{folder-name}.test' })
```

## Shorthands

Three shorthands write common rules for you.

| Shorthand | It is the same as |
| --- | --- |
| `layers(['app', 'feature', 'shared'])` | `feature` may not import `app`, and `shared` may not import `app` or `feature` |
| `isolate('feature')` | `rule('a', { files: 'feature', mayNotImport: 'feature', instance: 'different' })` |
| `entry('feature', 'index.ts')` | code outside a feature may import that feature only through its `index.ts` |

In a JSON config, write a shorthand with its `kind`:

```json
{ "kind": "layers", "order": ["app", "feature", "shared"] }
{ "kind": "isolate", "element": "feature" }
{ "kind": "entry", "element": "feature", "files": ["index.ts"] }
```

## A rule with more than one condition

Use `sentences` to give one name to a group of conditions. The rule fails when one of them fails.

```ts
rule('no-ui-in-app', {
  sentences: [
    { folders: 'src/app/ui', mustNotExist: true },
    { files: 'src/app/ui.*', mustNotExist: true },
  ],
})
```

## Options for every rule

| Option | What it does |
| --- | --- |
| `name` | The name in each report. Write it first in `rule('<name>', { ... })`, or as an option. Without a name, archangel uses the rule itself, for example `isolate(feature)`. |
| `because` | The reason, which each report shows. Tell the reader why the rule exists. |
| `link` | A link, which each report shows. |
| `imports` | `'all'`, `'runtime'` or `'types'`. The kind of imports that the rule counts. The default is `'all'`, but `'runtime'` for `noCycles` and for a rule with `transitively`. |
| `examples` | Code that the rule must accept, and code that it must reject. |

## Prove each rule with examples

A rule can be wrong. For example, a typing error in a path can make a rule find no files. Examples
prove that the rule works.

Write an example on one line. Archangel makes the files for you:

```ts
rule('no-react-in-shared', {
  files: 'shared',
  mayNotImport: { packages: ['react'] },
  examples: {
    accepted: ['src/shared/price.ts imports src/shared/money.ts'],
    rejected: ['src/shared/price.ts imports react'],
  },
})
```

| One line | The files that archangel makes |
| --- | --- |
| `'src/a.ts imports src/b.ts'` | Two files. `src/a.ts` imports `src/b.ts`. |
| `'src/a.ts imports react'` | One file, which imports the package. |
| `'src/a.ts imports src/b.ts imports src/a.ts'` | Two files that import each other. |
| `'src/app/ui.ts'` | One empty file. |

A target that ends in `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`, `.mjs` or `.cjs` is a file,
unless it starts with `@` or `node:`. Each other target is a package.

A file made from one line holds only runtime imports. So archangel refuses one line on a rule where
the text of the file decides the result: a rule with `names`, `specifier`, `dependency` or
`imports: 'types'`, a rule that selects files by directive, an export condition, and a plugin
condition. For these rules, write the files yourself:

```ts
isolate('feature', {
  examples: {
    accepted: [{ 'src/features/login/a.ts': "import './b';", 'src/features/login/b.ts': '' }],
    rejected: [{ 'src/features/login/a.ts': "import '../signup/c';", 'src/features/signup/c.ts': '' }],
  },
}),
```

Run `archangel test`. Archangel checks each example in memory and does not write files.
`archangel test` fails for a rule that has no rejected example, because only a rejected example
shows that the rule can find a problem. A table row does not need a written example.

## See what your rules do

`archangel rules` shows each rule as a sentence, with its reason, where you wrote it, the files and
imports that it judged, and its problems and exceptions. A rule that covers no file says that it
checks nothing. At the end, it lists the files and imports that no rule judges, and the outside
packages that no rule names.

`archangel explain src/a.ts:3` shows each rule that judged the import on line 3, with its result, and
why each other rule did not judge it.

## Older spellings

A config from archangel 0.1 still loads. A config uses one spelling of each key, and archangel refuses
two spellings in one config. These spellings stop at 1.0.

| Spelling | Older spelling |
| --- | --- |
| `parts` | `elements` |
| `groups` | `selectors` |
| `because` | `reason` |
| `rule('<name>', { ... })` | `rule({ name: '<name>', ... })` |

A package set in `elements`, `{ packages: [...] }`, is now an entry in `externals`.
