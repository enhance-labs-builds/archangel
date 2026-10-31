# parse-failed

A source file has syntax errors, uses Flow syntax, or is larger than the parser accepts. Archangel uses none of its imports, because a parser that recovers from errors can invent or lose imports.

## Example

```ts
const = ;
```

## What to do

Fix the errors the message lists. For a Flow file, or a generated file you do not want checked, add it to the config's `ignore` list.
