# must-not-exist

A file is in the selector of a `mustNotExist` rule, such as a file in no element or in a reserved folder.

## Example

```ts
// rule({ files: { all: ['src/**', { not: { element: '*' } }] }, mustNotExist: true })
// src/stray.ts, which no element's pattern matches
```

With `folders` instead of `files`, the rule gives one violation for each folder it covers, not one for each file in it:

```ts
rule({ folders: 'src/processes', mustNotExist: true })
```

## What to do

Move the file to where the architecture puts it, or delete it. A selector of a `mustNotExist` rule may match nothing, because that is how the rule passes.
