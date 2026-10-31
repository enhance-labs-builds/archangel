# pattern-invalid

A pattern has an empty folder name, a `**` that is not a whole folder name, a capture without a name, or the same capture name twice.

## Example

```json
{ "elements": { "ui": "src/ui**/x" } }
```

## What to do

Write `**` as a whole folder name (`src/**/x`), give each capture a name of letters, digits and underscores, and use each name once.
