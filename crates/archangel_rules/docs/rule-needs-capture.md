# rule-needs-capture

An isolate rule names an element whose pattern captures nothing, so it has only one instance.

## Example

```ts
// elements: { shared: 'src/shared/**' }
isolate('shared')
```

## What to do

Give the element a capture, such as `src/features/{feature}/**`, or remove the rule.
