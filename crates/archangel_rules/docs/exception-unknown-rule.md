# exception-unknown-rule

An `archangel-allow` comment names a rule that the config does not have. The message lists the rules that exist.

## Example

```ts
// archangel-allow entry(entitty): old code
```

## What to do

Use the rule's name as the config gives it, or as archangel makes it, such as `entry(entity)`.
