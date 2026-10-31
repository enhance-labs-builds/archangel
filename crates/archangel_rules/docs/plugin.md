# plugin

A file breaks a rule whose condition a plugin defines, such as `team/one-store-per-entity`. The plugin's condition wrote the message.

## Example

```ts
// plugins: { team: teamPlugin }
// rule({ name: 'one-store', files: 'entity', plugin: 'team/one-store-per-entity', options: { max: 1 } })
// src/entities/user/ has two stores
```

A violation names a file of the condition's view, or a folder that holds one, as the rules of the Feature-Sliced Design preset's plugin do, and may name one of the file's imports.

## What to do

Change the code as the message says, or read the condition's documentation, which the plugin links to. An exception above an import that the violation names permits it; a violation about a whole file can only be baselined.
