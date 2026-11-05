# Design

## Decisions that need you

All three were decided before the design was written (2026-11-05):

1. **#35: judge by the module's own exports, not by "names applies only to the package root".**
   A root-only rule gives a wrong answer when a subpath re-exports a banned name:
   `import * as fp from 'dates/fp'` brings in `format` when `dates/fp` re-exports it. AGENTS.md rule
   7 forbids that. The comment on #35 says why.
2. **#39: `allowUnresolved` permits the missing base.** This is not a warning, because nothing
   warns (AGENTS.md rule 2), and a tsconfig that is broken by mistake must still fail.

## 1. Names of a package module (#35)

An import *brings in every name* when it is a namespace import, an `export *` or `export * as`, a
dynamic import, a `require`, an `import x = require()`, an `import()` type or a triple-slash
reference: today's list. For each such import that resolves to a file in a package, the resolver
finds the module's names:

1. Parse the file with the parser that reads source files. A file whose ESM exports archangel can
   see has at least one export entry and no `export =`. Any other file (CommonJS, a declaration file
   with `export =`, a file with no exports) gives **no names**: the import brings in every name.
2. Each `export * from '<relative>'` in the file adds the names of the file it resolves to, by the
   same steps. A file that the walk already read adds nothing, so a loop ends. An `export *` from
   a bare specifier, an `export *` that does not resolve, or more than 64 files for one module
   gives **no names**.
3. The names are kept on the import (`Import::module_names`). The `names` test then asks whether
   the module's names include one of the rule's names.

A Node built-in has no file, so it keeps every name. A repository file is out of scope: its facts
already list its exports, but no report asks for it, and it would change which imports a rule
judges in every repository that uses `names` with files.

Every file that this reads is added to the resolution inputs as a file whose contents decide
resolutions, so watch mode and the language server resolve again when it changes, as they do for a
package manifest.

**Cost.** Nothing for an import that names what it brings in, which is most imports. For an import
that brings in every name of a package: one read and one parse for each distinct module, cached in
the resolver for its life, which ends at the next full resolution. `Work::package_modules_parsed`
counts them, and a test holds the count: two files that import the same subpath parse it once.

## 2. An exception above a statement (#38)

`exceptions::read` takes the file's imports. After it finds the line after a run of comments, it
looks for an import whose statement starts on that line, and when the specifier of that import is on
a later line, the exception's target is the specifier's line. A violation of an import is reported
on the specifier's line, so the code that permits violations stays the same. One pass over the
imports for each exception comment: a file has few of either.

## 3. A missing tsconfig base (#39)

`GraphProblem::ResolutionFileInvalid` gets `missing_package: Option<String>`, set when a tsconfig
`extends` names a package that is not installed. When an `allowUnresolved` entry matches that name,
the check gives no `resolution-file-invalid` and counts one permitted unresolved import. The
diagnostic's help names the entry to add. The files below that tsconfig already resolve without it,
and an import that needed it (a `paths` alias) gives `import-unresolved`, so permitting the base
hides no import.

## Where in the code

- `crates/archangel_graph/src/imports.rs`: `Import::module_names`, `Import::brings_in_every_name`,
  `FileFacts::mark_imports` sets the names.
- `crates/archangel_graph/src/resolve.rs`: `ImportResolver::module_names`, its cache, and
  `missing_package` on the file problem.
- `crates/archangel_graph/src/graph.rs`: `Resolved::module_names`, the problem's new field.
- `crates/archangel_graph/src/work.rs`: `package_modules_parsed`.
- `crates/archangel_rules/src/facts.rs`: `brings_in`.
- `crates/archangel_rules/src/exceptions.rs`, `check.rs`, `judgement.rs`: exceptions take the
  imports; `problems` permits a missing base.
