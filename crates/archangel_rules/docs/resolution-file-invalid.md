# resolution-file-invalid

A tsconfig or package manifest that resolution needs is not valid: it does not parse, cannot be read, or extends a package that is not installed. Archangel reports it once, not once per import.

## Example

```json
{ "extends": "@tsconfig/node20/tsconfig.json" }
```

## What to do

Fix the file, or install the package it extends. When the package is missing on purpose, for
example in a sub-project that installs its own packages, add the package to the config's
`allowUnresolved` with the reason. The summary then counts it as a permitted unresolved import.
