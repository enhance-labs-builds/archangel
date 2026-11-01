# Presets

A preset gives you a set of rules in one line. It is a function that returns part of a config:
parts, groups, rules with their examples, and the plugins that its rules use.

## Use a preset

Put the preset in `extends`:

```ts
extends: [fsd({ root: 'src' }), nextjs({ root: '.' })],
```

Then run `npx archangel test`. Archangel checks the preset's examples in your repository, under its
`root`, with your tsconfig.

## The presets in archangel

### `fsd`: Feature-Sliced Design

`fsd({ root })` checks a front end that uses [Feature-Sliced Design](https://feature-sliced.design).
It has the rules of Steiger, the official linter for Feature-Sliced Design.

- By default, it uses the rules that Steiger recommends.
- `strict: true` adds the four rules that Steiger has as options.

### `nextjs`: Next.js

`nextjs({ root })` keeps server code out of client components. A file that starts with
`'use client'`, and each file that it reaches through runtime imports, may not import
`server-only` or a Node built-in module. A chain of imports stops at a file that starts with
`'use server'`.

## Change a preset

```ts
import { defineConfig, fsd } from '@enhance-eng/archangel';

export default defineConfig({
  extends: [
    fsd({ root: 'apps/web/src' }),
    fsd({ root: 'apps/admin/src', name: 'fsd-admin', strict: true }),
  ],
  off: ['fsd/insignificant-slice'],
  rules: ({ rule }) => [
    rule('fsd/public-api', {
      folders: 'apps/web/src/features/*',
      mustContain: ['index.ts', 'index.tsx'],
      examples: { rejected: ['apps/web/src/features/login/form.ts'] },
    }),
  ],
});
```

- **Names have a prefix.** Each name in a preset starts with the preset's name, for example
  `fsd/features` or `fsd/public-api`. Your own `shared` part does not replace the preset's.
- **A second use needs its own name.** To use a preset two times, give the second use a `name`, as
  `fsd-admin` above.
- **Replace a rule.** Write a rule with the same name as a preset rule. Your rule replaces it.
- **Remove a rule.** Put its name in `off`. `archangel rules` lists the rules that `off` removes.
- **Typing errors stop the check.** A name in `off` must match a preset rule, and so must a
  replacement rule. If it does not, archangel stops and shows the nearest correct name.
- **Your names cannot have a slash.** Only preset names have a prefix. With `defineConfig`, the
  editor already marks a rule name with a slash that does not start with a preset you extend.

## Empty parts of a preset

A preset declares each part of its architecture, and most repositories have only some parts. So a
preset part that finds no files is not an error. Archangel lists it in the summary.

If all parts of a preset find no files, the `root` is probably wrong. Archangel then shows
`preset-empty`.

## Write a preset

A preset is a function that returns part of a config. Publish it as a plain function, next to your
plugin if it has one. [docs/plugins.md](plugins.md#publish-a-plugin) tells you how to name the
package.
