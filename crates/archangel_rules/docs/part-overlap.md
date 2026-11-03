# part-overlap

A file is in two of the config's own parts. A file can be in one part at most, so a rule about parts cannot know which part judges the file.

## Example

```json
{ "parts": { "model": "src/**/*.model.ts", "feature": "src/features/{feature}/**" } }
```

The file `src/features/login/session.model.ts` is in `model` and in `feature`.

## What to do

Narrow one of the two patterns with `not`, or make one of the two parts a group. A file can be in many groups. Fix this problem in the config, because the baseline does not hold it. A part of a preset is in a separate set, so it does not cause this problem.
