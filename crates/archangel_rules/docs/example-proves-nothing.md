# example-proves-nothing

An accepted example passed, but its rule never checked any of its imports, so it proves nothing.

## Example

```ts
// isolate('feature'), with an accepted example whose only file is in shared
```

## What to do

Give the example a file and an import that the rule checks.
