# Commands

## The commands

| Command | What it does |
| --- | --- |
| `archangel check` | Checks the repository, and shows each problem and a summary. |
| `archangel check src/a.ts` | Shows only the problems of this file. Archangel still checks the whole repository, because a cycle or an empty element depends on all files. |
| `archangel test` | Checks the examples of each rule. Examples prove that a rule can find a problem. |
| `archangel explain src/a.ts` | Shows the file's element, its rules, its imports and the files that import it. |
| `archangel why src/a.ts react` | Shows the shortest chain of imports from the file to the other file or package. |
| `archangel watch` | Checks again after each change, and shows what is new and what is fixed. |
| `archangel baseline add` | Records the problems of today. Then the check fails only for new problems. |
| `archangel baseline prune` | Removes the baseline entries that the code does not need now. |
| `archangel lsp` | Starts the language server. Your editor starts it for you. |
| `archangel init` | Writes a starter config. Add `--typescript` for `archangel.config.ts`. |
| `archangel docs layers` | Shows what a problem code means, and what to do about it. |
| `archangel completions zsh` | Writes the completion script for bash, zsh, fish, elvish or PowerShell. |

## Options

| Option | What it does |
| --- | --- |
| `--format json` | `check`, `test`, `explain` and `why` write one JSON document. `watch` writes one JSON line for each check. |
| `--format github` | Writes each problem as a GitHub Actions annotation, on its line in the pull request. |
| `--format sarif` | Writes a SARIF 2.1.0 log, for GitHub code scanning or a different tool that reads SARIF. |
| `--stats` | Adds the work that archangel did, and the time of each stage. |
| `--color` | `auto`, `always` or `never`. |
| `--config` | Uses this config. Without it, archangel uses the nearest config in the current folder or above it. |

In SARIF, each result has the identity of its problem. If a line moves, the alert stays the same.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | No problems. |
| `1` | Archangel found one or more problems. |
| `2` | Archangel could not run: the config, the baseline or the command line has an error. |

## In CI

```yaml
- run: npx archangel check --format github
- run: npx archangel test --format github
```

## The config file

Archangel looks for the config in the current folder, and then in each folder above it. The config
can be TypeScript or JSON:

- **`archangel.config.ts`** needs Node 22.18 or later, which runs it without a build step. With
  `defineConfig`, the TypeScript compiler finds some errors before archangel runs. For example, it
  finds a rule that names an element that does not exist.
- **`archangel.config.json`** does not need Node. Each rule is the object that you give to
  `rule(...)`. Add `"$schema": "./node_modules/@enhance-eng/archangel/archangel.schema.json"` to get
  completion and checks in your editor. A JSON config cannot use presets or plugins, because they are
  code.
