# rule-unproven

A rule has no rejected example, so nothing proves that it can find a violation.

## Example

```ts
rule({ files: 'entity', mayNotImport: 'react' })
```

## What to do

Add a rejected example: a small set of files that the rule must reject, such as `{ rejected: [{ 'src/entities/a/x.ts': "import 'react'" }] }`.
