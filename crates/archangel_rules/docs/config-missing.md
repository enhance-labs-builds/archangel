# config-missing

Archangel found no config in the current folder or any folder above it.

## Example

```sh
$ archangel check
config-missing: no config in /repo/packages/web or any folder above it
```

## What to do

Create `archangel.config.ts` or `archangel.config.json` at the top of the repository, or pass `--config <file>`.
