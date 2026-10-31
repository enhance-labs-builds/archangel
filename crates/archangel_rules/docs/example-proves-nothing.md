# example-proves-nothing

An accepted example passed, but its rule never looked at it, so it proves nothing. An import rule looks at imports, a rule about files at files, a folder rule at folders, a `mustContain` rule on an element at its instances, and a plugin rule at the files of its view. The message says which one the rule never looked at.

## Example

```ts
// isolate('feature'), with an accepted example whose only file is in shared
// rule({ files: 'src/**/*.ts', mustBeNamed: 'kebab-case' }), with an accepted example whose only file is README.md
```

## What to do

Give the example what the rule looks at: an import between the elements the rule names, or a file, folder or instance the rule covers. A `mustNotExist` rule rejects every file or folder it looks at, so no accepted example can prove it; give it only rejected examples.
