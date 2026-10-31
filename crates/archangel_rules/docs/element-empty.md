# element-empty

An element matches nothing, so no rule about it can fire. For a file element, no file matches its patterns. For a package element, no import uses its packages and no package manifest lists them.

## Example

```json
{ "elements": { "shared": "src/shraed/**" } }
```

## What to do

Fix the pattern or the package name, or remove the element.
