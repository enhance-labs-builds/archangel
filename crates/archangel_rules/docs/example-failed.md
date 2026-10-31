# example-failed

A rule did not judge an example as the example says: it accepted a rejected example, or rejected an accepted one. When another rule rejected the example instead, the message names it.

## Example

```sh
example-failed: entry(entity) did not reject the example; layers(app, entity, shared) did
```

## What to do

Fix the example so it shows what the rule is about, or fix the rule.
