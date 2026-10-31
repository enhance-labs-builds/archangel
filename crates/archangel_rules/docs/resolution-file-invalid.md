# resolution-file-invalid

A tsconfig or package manifest that resolution needs is not valid: it does not parse, cannot be read, or extends a package that is not installed. Archangel reports it once, not once per import.

## Example

```json
{ "extends": "@tsconfig/node20/tsconfig.json" }
```

## What to do

Fix the file, or install the package it extends.
