# rule-unknown-element

A rule or a group names a part, a group or an externals set that the config does not declare. A string without a `/` or a `*` is a name, not a path pattern. The message lists the declared names.

## Example

```ts
rule({ files: 'entity', mayNotImport: 'reactt' })
```

## What to do

Use one of the declared names, or write a package as `{ packages: ['reactt'] }`. With the TypeScript config, the editor shows a misspelt name in a rule as a type error.
