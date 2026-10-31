# plugin-failed

A plugin could not give its condition's result: the condition threw, it reported a file that was not in its view, or the plugin's Node process stopped.

## Example

```sh
plugin-failed: the plugin "team" failed in its condition `team/one-store-per-entity` for the rule "one-store": the condition threw `boom`
```

Each of these gives one `plugin-failed` that names the plugin, its condition and the rule, and says what happened:

- the condition's `check` threw, or its promise was rejected;
- it reported a violation without a file, for a file that is not in its view and not a folder that holds a file of its view, with an import on a folder, or with an import the file does not have;
- the Node process stopped, with what it printed before it stopped.

The check fails, because a rule whose code did not run proves nothing. In the editor, the language server starts the plugin process again on the next change. A plugin written for a view schema version archangel does not send is refused earlier, as [`config-invalid`](config-invalid.md) naming both versions.

## What to do

Read what the plugin printed, which archangel shows with the plugin's name, and report the failure to the plugin's author. Until it is fixed, turn the rule off with `off`, which takes any rule's name after the presets are merged.
