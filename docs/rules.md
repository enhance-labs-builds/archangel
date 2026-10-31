# Rules

This page tells you how to write rules. A rule has two parts:

- **Files**: the files that the rule is about.
- **One condition**: what those files must do, or must not do.

Read a rule as a sentence: "These files may not import React."

```ts
rule({ files: 'shared', mayNotImport: 'react' })
```

## Name the parts of your code

Before you write rules, give names to the parts of your code. Archangel has two kinds of names.

### Elements

An element is a part of your code, found by a path pattern. Each file is in one element at most:
the first element whose pattern finds the file.

```ts
elements: {
  app: 'src/app/**',
  feature: 'src/features/{feature}/**',
  shared: 'src/shared/**',
  react: { packages: ['react', 'react-dom'] },
},
```

- `{feature}` captures one part of the path. Each folder in `src/features` is then one **instance**
  of the `feature` element, for example `login` or `cart`.
- `{ packages: [...] }` is an element of npm packages. Use it on the other side of an import rule.
  `'*'` is every package, `'@acme/*'` is every package in a scope, and `'node:*'` is every Node
  built-in module.

### Selectors

A selector is a named set of files. A file can be in many selectors. Use a selector for files that
cut across elements, for example tests.

```ts
selectors: {
  test: '**/*.test.ts',
  client: { directive: 'use client' },
},
```

### Ways to write a set of files

You can write a set of files in these ways. They work in `elements`, in `selectors`, and in a rule's
`files`.

| You write | The set has |
| --- | --- |
| `'feature'` | the files of the element or selector with this name |
| `'src/**/*.view.tsx'` | the files that the path pattern finds (a pattern has a `/` or a `*`) |
| `['app', 'feature']` | the files in one or more of these sets |
| `{ any: ['app', 'feature'] }` | the files in one or more of these sets |
| `{ all: ['src/**', { not: 'test' }] }` | the files that are in all of these sets |
| `{ not: 'test' }` | all files except these files |
| `{ element: 'feature', capture: { feature: 'login' } }` | the files of one instance |
| `{ element: '*' }` | the files that are in an element |
| `{ directive: 'use client' }` | the files that start with this directive |

If a set has no files, archangel shows the error `selector-empty`. A rule about an empty set can
never find a problem, so archangel does not let it pass silently.

## Conditions

Each rule has one condition.

### Import conditions

| Condition | Example | Archangel reports |
| --- | --- | --- |
| `mayNotImport` | `rule({ files: 'shared', mayNotImport: 'react' })` | an import of `react` in `shared` |
| `mayOnlyImport` | `rule({ files: 'view', mayOnlyImport: ['shared', 'react'] })` | an import in `view` of something that is not in the list |
| `mayOnlyBeImportedBy` | `rule({ files: 'db', mayOnlyBeImportedBy: 'api' })` | an import of `db` by a file that is not in `api` or `db` |
| `noCycles` | `rule({ files: 'src/**', noCycles: true })` | files that import each other in a cycle |

`noCycles` counts only runtime imports. The compiler removes an `import type`, so a cycle of type
imports cannot break when the code runs. To report type cycles too, add `imports: 'all'`.

### Structure conditions

| Condition | Example | Archangel reports |
| --- | --- | --- |
| `mustNotExist` | `rule({ files: { all: ['src/**', { not: { element: '*' } }] }, mustNotExist: true })` | a file that is in no element |
| `mustBeNamed` | `rule({ files: 'src/components/**/*.tsx', mustBeNamed: 'PascalCase' })` | a file name that is not in this style |
| `mustContain` | `rule({ files: 'feature', mustContain: 'index.ts' })` | an instance that does not have this file |

### Export conditions

| Condition | Example | Archangel reports |
| --- | --- | --- |
| `mustExport` | `rule({ files: '**/*.view.tsx', mustExport: '{FileName}' })` | a view that does not export the name of its file |
| `mayNotExport` | `rule({ files: 'src/**', mayNotExport: 'default' })` | a default export |
| `mustOnlyReExport` | `rule({ files: '**/index.ts', mustOnlyReExport: true })` | an index file that has code of its own |

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

### Rules about chains of imports

Add `transitively: true` to `mayNotImport`. The rule then finds chains of imports, not only direct
imports. The report shows the shortest chain. Use `stopAt` to end a chain at some files.

```ts
rule({
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
rule({ folders: 'src/**', mustBeNamed: 'kebab-case' }),
rule({ folders: ['src/shared/*', '!src/shared/{ui,lib}'], mustContain: ['index.ts', 'index.tsx'] }),
```

`mustContain` accepts a list. One file from the list is sufficient.

## Name styles

The styles are `kebab-case`, `camelCase`, `PascalCase`, `snake_case` and `SCREAMING_SNAKE_CASE`.

A name pattern can use the name of the file or its folder, in each style. For example, `{FileName}`
is the file name in PascalCase, and `{folder-name}` is the folder name in kebab-case.

```ts
rule({ files: '**/*.test.ts', mustBeNamed: '{folder-name}.test' })
```

## Shorthands

Three shorthands write common rules for you.

| Shorthand | It is the same as |
| --- | --- |
| `layers(['app', 'feature', 'shared'])` | `feature` may not import `app`, and `shared` may not import `app` or `feature` |
| `isolate('feature')` | `rule({ files: 'feature', mayNotImport: 'feature', instance: 'different' })` |
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
rule({
  name: 'no-ui-in-app',
  sentences: [
    { folders: 'src/app/ui', mustNotExist: true },
    { files: 'src/app/ui.*', mustNotExist: true },
  ],
})
```

## Options for every rule

| Option | What it does |
| --- | --- |
| `name` | The name in each report. Without a name, archangel uses the rule itself, for example `isolate(feature)`. |
| `reason` | The reason, which each report shows. Tell the reader why the rule exists. |
| `link` | A link, which each report shows. |
| `imports` | `'all'`, `'runtime'` or `'types'`. The kind of imports that the rule counts. The default is `'all'`, but `'runtime'` for `noCycles` and for a rule with `transitively`. |
| `examples` | Code that the rule must accept, and code that it must reject. |

## Prove each rule with examples

A rule can be wrong. For example, a typing error in a path can make a rule find no files. Examples
prove that the rule works.

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
shows that the rule can find a problem.
