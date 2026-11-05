# Judge subpath imports by their exports, let an exception sit above a statement, and permit a missing tsconfig base

## Why

A team that moved from ESLint to archangel 0.3.3 reported three problems, each with a reproduction
that we confirmed:

- **#35.** A rule with `names` counts a namespace or dynamic import as bringing in every name. So
  `import * as loc from 'dates/locale/fr'` breaks a rule that bans `format`, although
  `dates/locale/fr` exports only `fr`. A file that loads locales one by one gives one false
  positive for each locale.
- **#38.** An `archangel-allow` comment above a multi-line import is reported as unused, and the
  violation stays. It works only on the line above the specifier, which is inside the braces. Each
  ESLint disable comment, which goes above the `import` keyword, must move into the braces.
- **#39.** A nested `tsconfig.json` whose `extends` names a package that is not installed fails
  the whole check. A sub-project that installs its own packages, or a CI job that runs the check
  before it installs them, cannot pass. Nothing in the config can permit it.

## What changes

- **Names.** When an import brings in every name of a package module, archangel reads the
  exports of the file that the import resolves to. The import then brings in those names only.
  When archangel cannot see every name that the module exports, the import still brings in every
  name, as it does today.
- **Exceptions.** An exception applies to the import whose statement starts on the line after the
  run of exception comments, and also, as now, to an import whose specifier is on that line.
- **Missing tsconfig base.** An `allowUnresolved` entry that matches the package that a tsconfig's
  `extends` names permits the missing base. Archangel then gives no `resolution-file-invalid`
  diagnostic for it, and the summary counts it as a permitted unresolved import.

## Capabilities

### Modified capabilities

- `architecture-rules`: an import that brings in every name of a package module brings in the
  names that the module exports, when archangel can see all of them.
- `exceptions-and-baselines`: an exception applies to the import statement on the next line.
- `import-graph`: `allowUnresolved` can permit a tsconfig base that is not installed.

## Impact

- `archangel_graph`: the resolver reads the export names of a package file that an import brings
  in every name of, and of the files that its relative `export * from` statements reach.
  `Import` gets the names it found. `GraphProblem::ResolutionFileInvalid` gets the package that a
  missing `extends` names.
- `archangel_rules`: `names` uses the module's names; exceptions take the imports of the file;
  the check permits a missing base through `allowUnresolved`.
- Docs: the `names`, exceptions and `allowUnresolved` sections, and the
  `resolution-file-invalid` page.
- Release 0.3.4.
