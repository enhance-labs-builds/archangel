# baseline-stale

A baseline entry permits more violations than the code has, because they were fixed, or it names a rule that no longer exists. A check never rewrites the baseline.

## Example

```sh
baseline-stale: the baseline permits 1 more violation(s) of "layers(feature, shared)" in src/shared/a.ts
```

## What to do

Run `archangel baseline prune`, which lowers or removes those entries and never adds one.
