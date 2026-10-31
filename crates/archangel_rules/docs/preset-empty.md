# preset-empty

A preset the config extends matches no file with any of its elements, so none of its rules can fire.

## Example

```ts
// and the repository has no folder scr
export default defineConfig({ extends: [fsd({ root: 'scr' })] });
```

The message names the preset and its root. A preset declares every part of its architecture, and most repositories have only some of them, so a preset's elements and selectors that match nothing on their own, such as a `widgets` layer the code does not have yet, are not an error: the summary lists them, such as `fsd/widgets` and `fsd/processes`. Only a preset whose file elements together match no file at all is `preset-empty`, because that is most likely a wrong root.

## What to do

Give the preset the folder it applies to as its `root`, such as `fsd({ root: 'src' })`, or remove it from `extends`.
