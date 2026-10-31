# must-be-named

A file or folder in a `mustBeNamed` rule has a name that is not in the rule's style, or a full name that does not match the rule's name pattern.

## Example

```ts
// rule({ files: 'src/components/**/*.tsx', mustBeNamed: 'PascalCase' })
// src/components/icon-button.tsx, whose name icon-button is not PascalCase
```

## What to do

Rename the file or folder to the name the message suggests. The styles are `kebab-case`, `camelCase`, `PascalCase`, `snake_case` and `SCREAMING_SNAKE_CASE`. A style checks the name up to the first dot, so `Card.stories.tsx` is named `Card`. A name pattern checks the full name, the name without its last extension, and can use `{FileName}` or `{folder-name}` in any of the five styles.
