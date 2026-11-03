# Let mustBeNamed take a list of names

## Why

A rule that permits several kinds of file name has no direct form today. An author must write a
`mustNotExist` rule with a negated set of files, which is a double negative. It is hard to read, and
its message cannot say which names are correct.

## What changes

- `mustBeNamed` takes a list of styles and name patterns. A file or folder passes when one entry
  matches. The single-string form does not change.
- A list with one entry gives the same result and the same message as the single string.
- An empty list and an entry that is not a string are config errors. Each entry follows the rules of
  a single value, so a folder rule refuses a pattern that uses the file name.
- The message of a failing file names the file, its full name and each permitted entry.
- The name pattern `*` already exists. It matches any text, and the empty text too. This change does
  not change it. `*.view` matches `cart.view`, and it does not match `view` or `cart.view.extra`.

## Capabilities

### Modified capabilities

- `structure-rules`: a `mustBeNamed` rule can list the names it permits.

## Impact

- `archangel_config`: `Condition::MustBeNamed` holds a list of `NameRule`, so a library user must
  change a match on it. The library crates have no stability guarantee.
- `archangel_rules`: the verdict, the messages and the sentence in words.
- The npm types and the JSON schema accept a list. The types refuse an empty list.
- The check path does one more name match for each extra entry, for each file that the rule selects.
  It reads no more files, and it builds a message only for a file that fails.
- Release 0.3.0.
