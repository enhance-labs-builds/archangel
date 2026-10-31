# rule-name-duplicate

Two rules have the same name, often the default name of two rules about the same files, such as `mayNotImport(entity, react)`. Exceptions and the baseline refer to rules by name, so each needs its own.

## Example

```ts
rule({ files: 'entity', mayNotImport: 'react' }),
rule({ files: 'entity', mayNotImport: 'react', imports: 'runtime' }),
```

## What to do

Give each rule its own `name` option.
