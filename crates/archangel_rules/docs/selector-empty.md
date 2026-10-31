# selector-empty

A selector that the config declares, a path pattern written inline in a rule, or the `folders` of a folder rule matches nothing, so no rule about it can fire.

## Example

```json
{ "selectors": { "views": "**/*.viewz.tsx" } }
```

```ts
// rule({ name: 'legacy', files: 'src/legasy/**', mayNotImport: 'src/next/**' }), and no folder src/legasy
// rule({ name: 'readme', folders: 'modules/*', mustContain: 'README.md' }), and no folder modules
```

The message names the selector and its definition, or the rule and its pattern. A selector counts every file the walk visits, not only source files and not only files an import reaches, so `**/*.png` is not empty while the repository has an image. In the editor, a selector can become empty after an edit, such as `{ directive: 'use client' }` when the last client file loses its directive, and the server reports it then.

Three cases are exempt, because matching nothing is how they pass or changes nothing: the files of a `mustNotExist` rule, a pattern a rule writes inline only inside a `not`, such as an exemption list shared by rules whose globs match nothing in this repository yet, and everything a preset declares. A selector the config declares is not exempt inside a `not`, because declaring it says the repository has such files. A preset's selectors and elements that match nothing are listed in the summary instead; a preset that matches nothing at all is [`preset-empty`](preset-empty.md).

## What to do

Fix the pattern, or remove the selector or the rule. An element that matches nothing is [`element-empty`](element-empty.md).
