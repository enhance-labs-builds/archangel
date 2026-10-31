# may-not-export

A file in a `mayNotExport` rule has an export of the kind the rule refuses: a default export, an `export *` or `export * as`, or a named export whose name matches the rule's pattern.

## Example

```ts
// rule({ files: 'src/**', mayNotExport: 'default' })
export default function App() {}
```

## What to do

Export the value by name, or pass on the names one by one instead of `export *`. The violation is at the line of the export. A violation about a file is permitted only through the baseline, not an exception comment.
