# Plugins

A plugin adds conditions that you write in TypeScript. Use a plugin when the built-in conditions in
[docs/rules.md](rules.md) cannot say what you want.

## How a plugin works

- A condition receives a **view** of the files that its rule is about. Each file in the view has its
  path, its element, its captures and its selectors. It also has the facts that the condition asks
  for: imports, importers or exports.
- The condition returns **violations**. Archangel shows each violation like any other problem. Your
  exceptions, your baseline and each output format apply to it.
- Plugins run in the Node process that loads your config. They never run in archangel's own process.

## Write a plugin

This plugin limits the number of stores in each entity. A store is a file that ends in `.store.ts`.
Put it in `plugins/team.ts`:

```ts
import { defineCondition, definePlugin } from '@enhance-eng/archangel';

const oneStorePerEntity = defineCondition({
  name: 'one-store-per-entity',
  description: 'Each entity has at most `max` stores.',
  docs: 'https://example.com/team/one-store-per-entity',
  // A JSON Schema. Archangel refuses a rule whose options do not match it.
  options: {
    type: 'object',
    properties: { max: { type: 'integer', minimum: 1 } },
    required: ['max'],
    additionalProperties: false,
  },
  // The facts that the condition reads. This one reads only paths and captures.
  reads: [],
  check(view, options: { max: number }) {
    const stores = new Map<string, string[]>();
    for (const file of view.files) {
      if (!file.path.endsWith('.store.ts')) continue;
      const entity = file.captures.entity ?? '';
      stores.set(entity, [...(stores.get(entity) ?? []), file.path]);
    }
    return [...stores.values()]
      .filter((files) => files.length > options.max)
      .flatMap((files) =>
        files.map((file) => ({
          file,
          message: `${file} is one of ${files.length} stores in its entity, and an entity has at most ${options.max}`,
        })),
      );
  },
});

export default definePlugin({ name: 'team', schemaVersion: 1, conditions: [oneStorePerEntity] });
```

## Use the plugin

Load the plugin in your config with a name. A rule then uses the condition as
`<name>/<condition>`, with its `options` and its examples:

```ts
import { defineConfig } from '@enhance-eng/archangel';
import team from './plugins/team.ts';

export default defineConfig({
  elements: { entity: 'src/entities/{entity}/**' },
  plugins: { team },
  rules: ({ rule }) => [
    rule({
      name: 'one-store',
      files: 'entity',
      plugin: 'team/one-store-per-entity',
      options: { max: 1 },
      reason: 'An entity keeps its state in one place',
      examples: {
        accepted: [{ 'src/entities/example/user.store.ts': 'export const user = 1;\n' }],
        rejected: [
          {
            'src/entities/example/user.store.ts': 'export const user = 1;\n',
            'src/entities/example/session.store.ts': 'export const session = 1;\n',
          },
        ],
      },
    }),
  ],
});
```

Run `npx archangel test` to prove the rule. Then run `npx archangel check`. It reports each entity
that has two stores.

## What a violation can say

A violation names a file of the view, or a folder that contains a file of the view:

```ts
{ file, message }
```

It can also name an import. An exception comment above that import can then permit it:

```ts
{ file, import: { specifier, occurrence }, message }
```

## When a plugin fails

A plugin rule never passes because its code did not run. Archangel shows `plugin-failed`, and the
check fails, when:

- a condition throws an error;
- a condition reports a file that is not in its view;
- the plugin process stops.

## What a plugin prints

Each line that a plugin prints goes to standard error, or to the editor's log, with the name of the
plugin.

## In the editor

Archangel shows its own problems immediately. It shows the plugin's problems when Node answers.

## Publish a plugin

- Name the package `archangel-plugin-<name>` or `@scope/archangel-plugin`.
- The types of the view are in `@enhance-eng/archangel`: `View`, `FileRecord`, `ImportRecord` and
  `ExportsRecord`.
- A new field in the view does not change `schemaVersion`. So a plugin reads only the fields that it
  knows, and a new release of archangel does not break it.
