# Let a folder that must not exist pass when no folder matches

## Why

A `mustNotExist` rule passes when it finds nothing. For `files`, archangel does this. For `folders`,
a pattern that matches no folder gives `selector-empty`, so the rule fails in the one state that it
asks for. A developer who writes `{ folders: 'src/**/utils', mustNotExist: true }` in a repository
with no `utils` folder gets an error and must rewrite the rule with `files`.

## What changes

- A `folders` pattern of a `mustNotExist` sentence may match no folder. It is not `selector-empty`,
  as the `files` of a `mustNotExist` sentence already are not.
- A `folders` pattern of every other condition that matches no folder is still `selector-empty`.
- The docs say that both forms may match nothing. The example in `docs/rules.md` returns to the
  `folders` form that it had before 0.3.2.

## Capabilities

### Modified capabilities

- `structure-rules`: a `mustNotExist` sentence about folders passes when no folder matches.

## Impact

- `archangel_rules`: one more condition test for each folder sentence that covers no folder. The
  check does no more work for each file or folder.
- A misspelt folder pattern in a `mustNotExist` sentence now passes silently, as a misspelt files
  pattern there already does. Examples of the rule still prove it: a rejected example must fail.
- Release 0.3.3.
