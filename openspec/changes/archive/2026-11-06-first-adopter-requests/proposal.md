# The rest of the first adopter's requests

## Why

The team that moved from ESLint to archangel filed sixteen issues. 0.3.4 answered six. These are
the other ten: five bugs that we reproduced on 0.3.3, and five requests.

- **#36.** A type import of a package that exists only as `@types/<name>` is unresolved.
- **#37.** Two parts whose patterns nest cannot be separated, and `part-overlap` suggests `not`,
  which a part does not accept.
- **#40.** When the config's `ignore` list removes a group's files, `selector-empty` blames the
  group's pattern.
- **#41.** A file at the root cannot be named: `app.config.ts` reads as a name, and
  `./app.config.ts` matches nothing.
- **#44.** `test --format json` lists its problems under other keys than `check`, and a config
  that stops a command writes its JSON document to standard error.
- **#34.** A package pattern cannot name a subpath, so deep imports cannot be banned.
- **#42.** Every rule fails the check, so a deprecation cannot warn while its call sites move.
- **#43.** The entry fix only shortens a specifier, so it misses an entry file in a sibling
  folder, such as `public/`.
- **#45.** The note about computed specifiers points to `archangel explain`, which needs a file,
  and no output names the file or the line.
- **#46.** An `import()` of a template literal is not followed, though bundlers treat it as a
  glob.

## What changes

- A type import that names no installed package falls back to its `@types` package.
- A `!` pattern in a part's list leaves those files out of the part.
- `./` names the config folder in a pattern; a bare file name's error suggests it.
- An empty group or part names the ignore line that left its files out.
- `test --format json` lists its problems under `diagnostics`; a stopped command's JSON and
  SARIF go to standard output.
- A package pattern with a subpath, such as `lodash-es/*`, names those subpaths only.
- `severity: 'warn'` makes a rule report and pass; `check --max-warnings <count>` fails past a
  count.
- The entry fix also reaches an entry file in a sibling folder.
- The summary lists each computed import's file and line.
- A template `import()` with a static relative prefix and a written extension is one import of
  each file that it matches.

## Capabilities

### Modified capabilities

- `configuration`, `import-graph`, `architecture-rules`, `diagnostics`, `command-line`.

## Impact

- Every crate. The JSON output gains `severity` on each diagnostic, `summary.warnings` and
  `summary.unfollowedImports`, and `test`'s `diagnostics`; nothing is removed.
- Release 0.3.5.
