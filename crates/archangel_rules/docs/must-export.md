# must-export

A file in a `mustExport` rule exports no name that matches the rule's name pattern.

## Example

```ts
// rule({ files: '**/*.view.tsx', mustExport: '{FileName}' })
// src/button.view.tsx, which exports only ButtonView, not Button
```

## What to do

Export the name the message gives. A re-export counts, and so does every name an `export *` chain brings in, except `default`, which `export *` never passes on.
