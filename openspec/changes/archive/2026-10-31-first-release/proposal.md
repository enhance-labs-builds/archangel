# Proposal: the first release of archangel

## Why

The brief in `docs/briefs/first-release.md` asks for one tool that checks a TypeScript
repository's architecture rules: where code lives and what it may import. Teams enforce these
rules today with several tools that each parse the whole repository again. On one
monorepo of 9,824 source files and 1.34 million lines, one dependency-cruiser run takes 24 seconds and one
ESLint run takes about 20 seconds per file. So the checks run late, in CI, and a rule that is
configured wrong reports success while it checks nothing.

The brief's requirements, in short:

- One config in TypeScript or JSON. It declares elements by path patterns with captures, and six
  kinds of rules between them: layers, isolate, entry, only, forbid and no cycles.
- Rules that prove themselves. Examples of accepted and rejected code are checked in memory, an
  example must pass for the right reason, and an element that matches nothing is an error.
- Exceptions that need a reason, and a baseline that can only shrink, with the number of each per rule
  in every result.
- A diagnostic for every violation that says what is wrong, what to do instead and where to read
  more, with fixes that archangel has checked. Readable output for people, JSON for tools, and the
  same results in the same order on every machine. Nothing is skipped silently.
- The commands check, test, explain, why, watch, baseline add and baseline prune, a language server
  that the editor starts with `archangel lsp`, and a report of the work a check did.
- A full check of the monorepo above in under one second, and work per edit that does not grow
  with the repository, except where nothing smaller is correct, such as the search for cycles.
  Tests count work; benchmarks report time.
- A Rust core on established crates, published to npm with prebuilt binaries and to crates.io.

## What Changes

A first build of the original six rule kinds exists and ran on a private monorepo, the dogfood; this change finishes the
first release. It differs from the brief in several deliberate ways, each listed in the design's
"Decisions that need you": the largest is bringing presets and plugins, which the brief puts later,
into this release; others include that a check reports a fixed baseline entry instead of removing
it, and that creating, deleting or renaming a file resolves every import again. Every
command, output format and config field is new, so nothing breaks.

The release also grows from six rule kinds into a rule language that covers what ArchUnit checks
without a type checker. Haziq Nordin asked for this on 2026-10-30, for the first release, after the
first build ran on the dogfood and showed that one element per file could not
say its rules. It adds: overlapping selectors, rules
written as one sentence each, conditions on imports (directly or transitively, by imported names and
by specifier style), on file and folder names, on where files live and which must or must not exist,
and on exports; presets composed with `extends`, with Feature-Sliced Design and the Next.js
server and client boundary built in; and plugins, packages whose conditions are TypeScript functions
over a view of the graph, in the command line and in the editor. Layers, isolate and entry stay as
shorthands; forbid, only and noCycles become the conditions `mayNotImport`, `mayOnlyBeImportedBy` and
`noCycles`, which is not breaking because nothing is published.

Out of scope: rules about code inside a file (globals, properties, syntax, and declarations a file
does not export), rules written as code patterns, extractors for other languages such as CSS,
unused files and exports, importing other tools' configs, a cache that persists between command-line
runs, generated documentation and diagram conformance, and rules that need the type checker.

## Capabilities

### New Capabilities

- `configuration`: how archangel finds, loads and validates the config, and what elements,
  patterns and rule declarations mean.
- `import-graph`: which files archangel reads, which imports it finds, and how it resolves them.
- `architecture-rules`: what each import condition, the no-cycles condition and the three shorthand
  kinds permit and forbid.
- `selectors`: how named, overlapping sets of files are declared and matched, and how rule sides name
  packages.
- `structure-rules`: conditions on file and folder names, where files live, and which files must or
  must not exist.
- `export-rules`: conditions on what a file exports.
- `presets`: how shareable presets compose into a config, and the two built-in presets.
- `plugins`: how a plugin's conditions are declared, run and reported, on the command line and in the
  editor.
- `rule-examples`: how the test command proves each rule with its examples.
- `exceptions-and-baselines`: how a team permits one import, or all existing violations, without
  weakening the rule for new code.
- `diagnostics`: what every result contains, its formats, its order, and the fixes archangel offers.
- `command-line`: the commands, their output and their exit status, including the work report.
- `language-server`: diagnostics and fixes in the editor while the developer types.
- `distribution`: how developers install archangel from npm and crates.io.

### Modified Capabilities

None. There are no specs yet.

## Impact

- New Cargo workspace under `crates/`: `archangel_config`, `archangel_graph`, `archangel_rules`,
  `archangel_engine`, `archangel_lsp`, the `archangel` binary, which replaces the placeholder
  `src/lib.rs`, and `archangel_testkit` for tests.
- New npm packages under `npm/`: `archangel` and one package per platform binary.
- New dependencies: `oxc_parser`, `oxc_allocator`, `oxc_ast`, `oxc_span`, `oxc_resolver`,
  `ignore`, `regex`, `memchr`, `smallvec`, `petgraph`, `rayon`, `serde`, `serde_json`, `thiserror`,
  `fail`, `lsp-server`, `lsp-types`, `notify`, `notify-debouncer-full`, `annotate-snippets` and
  `clap`. For tests: `proptest`, `proptest-state-machine`, `assert_cmd`, `insta`, `tempfile`,
  `cargo-fuzz`, `cargo-mutants`, `hyperfine`, and the npm packages `typescript` and `tsd`.
- CI gains release builds for seven platforms and scheduled jobs for fuzzing, mutation testing and
  benchmarks.
- For the rule language: the crates `fixedbitset` (already in the tree through petgraph) for
  signatures, `jsonschema` for plugin options, and `heck` for tests only; the npm package gains `src/presets/`, `src/plugin.ts`, and a build step that
  compiles them; `steiger` joins the pinned test dependencies for the differential test.
