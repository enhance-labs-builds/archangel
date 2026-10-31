# file-unreadable

Archangel cannot open a folder or a file, or a source file is not UTF-8 or UTF-16 text.

## Example

```sh
file-unreadable: src/private: archangel cannot open it: permission was refused
```

## What to do

Fix the permissions or the encoding, or add the path to the config's `ignore` list.
