# element-empty

A part or an externals set matches nothing, so no rule about it can fire. For a part, no file matches its patterns. For an externals set, no import uses its packages and no package manifest lists them.

## Example

```json
{ "elements": { "shared": "src/shraed/**" } }
```

## What to do

Fix the pattern or the package name, or remove the part or the externals set.
