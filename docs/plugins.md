# Plugins

A plugin lets you write your own rules in TypeScript. Use a plugin when the built-in conditions in
[docs/rules.md](rules.md) cannot say what you want.

## How it works

1. **Archangel makes a map of your code.** For each file, the map has its path, its element, what
   it imports, what imports it, and what it exports.
2. **A rule asks a question about part of the map.** For example: "Do files in `shared` import a
   feature?" The built-in conditions, such as `mayNotImport`, are questions that archangel already
   knows.
3. **A plugin is a question that you write as a function.** Archangel gives your function the part
   of the map that the rule is about. Your function gives back a list of problems.

## The parts of a condition

A plugin has one or more **conditions**. A condition is an object with these fields:

| Field | What it is |
| --- | --- |
| `name` | The name that a rule uses, for example `max-imports`. |
| `description` | One sentence that says what the condition requires. |
| `reads` | The facts that your function needs: `'imports'`, `'importers'` or `'exports'`. Archangel sends only these facts. |
| `optionsSchema` | Optional. A JSON Schema for the settings that a rule can give. Archangel refuses a rule whose settings do not match it. |
| `check` | Your function. Archangel calls it as `check(view, options)`. |

`check(view, options)` gets two arguments:

- **`view`** is the part of the map. `view.files` is a list of files. Each file is an object, for
  example `{ path: 'src/a.ts', element: 'feature', imports: [...] }`.
- **`options`** are the settings that the rule gives. If the rule has `options: { max: 5 }`, then
  `options` is `{ max: 5 }`.

`check` returns a list of problems. Each problem is `{ file, message }`.

## What `check` receives

Archangel calls `check(view, options)`. This is a real call, from a repository with two features,
`cart` and `login`.

The config has this rule:

```ts
rule({ name: 'show', files: 'feature', plugin: 'dump/show', options: { max: 2 } })
```

The condition reads `imports`, `importers` and `exports`. Archangel calls `check` with these two
values:

```json
{
  "options": { "max": 2 },
  "view": {
    "schemaVersion": 1,
    "files": [
      {
        "path": "src/features/cart/cart.ts",
        "element": "feature",
        "captures": { "feature": "cart" },
        "selectors": [],
        "imports": [
          {
            "specifier": "react", "occurrence": 0, "line": 1, "column": 26,
            "kind": "runtime", "names": ["useState"], "file": null, "package": "react"
          },
          {
            "specifier": "../login", "occurrence": 0, "line": 2, "column": 25,
            "kind": "runtime", "names": ["session"], "file": "src/features/login/index.ts", "package": null
          }
        ],
        "importers": [],
        "exports": { "names": ["cart"], "default": false, "star": false }
      },
      {
        "path": "src/features/login/index.ts",
        "element": "feature",
        "captures": { "feature": "login" },
        "selectors": [],
        "imports": [],
        "importers": ["src/features/cart/cart.ts"],
        "exports": { "names": ["session"], "default": false, "star": false }
      }
    ]
  }
}
```

- `options` is the `options` object of the rule, without changes.
- `view.files` has one entry for each file in the rule's `files`, sorted by path.

### A file

| Field | Type | What it is |
| --- | --- | --- |
| `path` | `string` | The path from the config folder. |
| `element` | `string` or `null` | The element of the file. `null` if the file is in no element. |
| `captures` | object | The values of the element's captures. For `src/features/{feature}/**`, this is `{ "feature": "cart" }`. |
| `selectors` | `string[]` | The selectors that contain the file. |
| `imports` | list of imports | Only if the condition reads `'imports'`. |
| `importers` | `string[]` | Only if the condition reads `'importers'`. The paths of the files that import this file. |
| `exports` | object | Only if the condition reads `'exports'`. |

### An import

| Field | Type | What it is |
| --- | --- | --- |
| `specifier` | `string` | The text in the quotes, for example `'../login'`. |
| `occurrence` | `number` | 0 for the first import of this specifier in the file, 1 for the second, and so on. |
| `line`, `column` | `number` | The position of the specifier. Both start at 1. |
| `kind` | `'runtime'` or `'type'` | `'type'` for an import that the compiler removes, for example `import type`. |
| `names` | `string[]` | The names that the import brings in: a name, `default` or `*`. |
| `file` | `string` or `null` | The file that the import goes to. `null` for a package, or if the import does not resolve. |
| `package` | `string` or `null` | The package that the import goes to. `null` for a file in the repository. |

### The exports of a file

| Field | Type | What it is |
| --- | --- | --- |
| `names` | `string[]` | The names that the file exports, `default` included. |
| `default` | `boolean` | `true` if the file has a default export. |
| `star` | `boolean` | `true` if the file has an `export *`. |

### What `check` returns

`check` returns a list of problems. It can also return a promise of a list. An empty list means that
there are no problems.

```ts
[{ file: 'src/features/cart/cart.ts', message: 'cart.ts has 2 imports, and the limit is 1' }]
```

The TypeScript types of all these values are in `@enhance-eng/archangel`: `View`, `FileRecord`,
`ImportRecord`, `ExportsRecord` and `Violation`.

## A first plugin

This plugin reports a file that imports more than `max` other files.

**1. Write the plugin.** Put it in `plugins/team.ts`:

```ts
import { defineCondition, definePlugin } from '@enhance-eng/archangel';

const maxImports = defineCondition({
  name: 'max-imports',
  description: 'A file imports at most `max` other files.',
  reads: ['imports'],
  optionsSchema: {
    type: 'object',
    properties: { max: { type: 'integer', minimum: 1 } },
    required: ['max'],
  },
  check(view, options: { max: number }) {
    return view.files
      .filter((file) => file.imports.length > options.max)
      .map((file) => ({
        file: file.path,
        message: `${file.path} has ${file.imports.length} imports, and the limit is ${options.max}`,
      }));
  },
});

export default definePlugin({ name: 'team', schemaVersion: 1, conditions: [maxImports] });
```

**2. Use it in a rule.** In `archangel.config.ts`, load the plugin with a name. Then write a rule
with `plugin: '<plugin name>/<condition name>'`:

```ts
import { defineConfig } from '@enhance-eng/archangel';
import team from './plugins/team.ts';

export default defineConfig({
  elements: { feature: 'src/features/{feature}/**' },
  plugins: { team },
  rules: ({ rule }) => [
    rule({
      name: 'small-files',
      files: 'feature',
      plugin: 'team/max-imports',
      options: { max: 2 },
      examples: {
        accepted: [{ 'src/features/a/x.ts': "import './y';", 'src/features/a/y.ts': '' }],
        rejected: [
          {
            'src/features/a/x.ts': "import './y';\nimport './z';\nimport './w';",
            'src/features/a/y.ts': '',
            'src/features/a/z.ts': '',
            'src/features/a/w.ts': '',
          },
        ],
      },
    }),
  ],
});
```

**3. Prove and run it.** `npx archangel test` checks the examples. `npx archangel check` then
reports each file in a feature that has more than two imports.

## A larger example

This condition finds entities that have too many stores. A store is a file that ends in `.store.ts`.
It reads no facts, because it needs only the paths and the captures.

```ts
const oneStorePerEntity = defineCondition({
  name: 'one-store-per-entity',
  description: 'Each entity has at most `max` stores.',
  reads: [],
  optionsSchema: {
    type: 'object',
    properties: { max: { type: 'integer', minimum: 1 } },
    required: ['max'],
  },
  check(view, options: { max: number }) {
    // Make a list of the stores of each entity.
    const stores = new Map<string, string[]>();
    for (const file of view.files) {
      if (!file.path.endsWith('.store.ts')) continue;
      const entity = file.captures.entity ?? '';
      stores.set(entity, [...(stores.get(entity) ?? []), file.path]);
    }
    // Report each store of an entity that has too many.
    return [...stores.values()]
      .filter((files) => files.length > options.max)
      .flatMap((files) =>
        files.map((file) => ({ file, message: `${file} is one of ${files.length} stores in its entity` })),
      );
  },
});
```

## What a problem can say

A problem names a file of the view, or a folder that contains a file of the view:

```ts
{ file, message }
```

A problem can also name an import. An exception comment above that import can then permit it:

```ts
{ file, import: { specifier, occurrence }, message }
```

## When a plugin fails

A plugin rule never passes because its code did not run. Archangel shows `plugin-failed`, and the
check fails, when:

- a condition throws an error;
- a condition reports a file that is not in its view;
- the plugin process stops.

If a condition has a field that archangel does not know, archangel stops before the check and names
the field.

## What a plugin prints

Each line that a plugin prints goes to standard error, or to the editor's log, with the name of the
plugin.

## In the editor

Archangel shows its own problems immediately. It shows the plugin's problems when the plugin answers.
Plugins run in the Node process that loads your config, not in archangel's own process.

## Publish a plugin

- Name the package `archangel-plugin-<name>` or `@scope/archangel-plugin`.
- The types of the view are in `@enhance-eng/archangel`: `View`, `FileRecord`, `ImportRecord` and
  `ExportsRecord`.
- A new field in the view does not change `schemaVersion`. A plugin reads only the fields that it
  knows, so a new release of archangel does not break it.
