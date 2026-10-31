# Brief: the first release of archangel

This brief is the input to the first design. It says what the first release must do for the
developers who use it, and why, and which questions the design must settle. It does not say how
the code is organised; the design decides that.

## The problem

TypeScript teams that care about their architecture write it down as rules: the UI may not import
the database, a feature may not reach inside another feature, an entity is used only through its
public entry file, nothing imports in a cycle. Today those rules are enforced by several tools at
once. A typical large repository uses dependency-cruiser for import rules, an ESLint plugin for
folder and file naming, another ESLint plugin for boundaries, Steiger for Feature-Sliced Design
slices, and custom scripts for the rest.

This has three costs.

**The checks are slow.** Each tool parses the whole repository again. Measured on one 9,600-file,
1.3-million-line TypeScript monorepo, one dependency-cruiser run over the repository took 24
seconds. One ESLint run on a single file took about 20 seconds, of which the rules themselves
took about 0.15 seconds; the rest was start-up: loading plugins and building a typed program.

**The checks run late.** Because they are slow, they run in CI or in a pre-push hook. A developer
or a coding agent learns that a rule is broken minutes after writing the import, not while
writing it.

**A broken check looks like a passing one.** A glob that matches no files, a rule scoped to the
wrong folder, or a linter that classifies every file as unknown all report success while they
check nothing. The same monorepo guards against this with a test suite that writes a known
violation into the source tree for each of about 400 rules, runs the real tool, and checks that
the tool rejects it for the expected reason. Because each case starts the tools again, that suite
takes about 30 minutes.

## What archangel is

Archangel is one tool for the rules about where code lives, what it may depend on, and who owns
what. It is not a general linter, a formatter or a type checker; ESLint, oxlint, Biome and the
TypeScript compiler already do those jobs well.

The developer writes one config. The config names the kinds of files in the repository, called
elements, by path patterns, and states rules between elements. Archangel reads every file once,
builds a graph of imports in memory, and checks every rule against that graph. The same engine
serves the command line, a watch mode and a language server, so an editor or a coding agent sees
a broken rule as the code is written.

## Who it is for

- **A developer in an editor** who imports something they should not, and sees why at once,
  with the allowed alternative.
- **A coding agent** that changes code and needs machine-readable results with stable codes and
  the allowed alternatives, so it can fix a violation in one attempt.
- **The person who owns the architecture**, who writes the rules, needs to know each rule works,
  and wants the rules documented without writing the documentation twice.
- **CI**, which needs a fast, deterministic pass or fail.

## What the first release must do

### Config

1. The developer writes the config in TypeScript, with types that catch mistakes as they type: a
   rule that names an element the config does not declare is a type error, and so is a rule that
   needs an element with a captured name, such as one feature per folder, when the element has
   none. A JSON config with the same content also works.
2. An element is declared by one or more path patterns. A pattern can capture a part of the path,
   as in `src/features/{feature}/**`, so that each feature folder is its own instance of the
   element. An element can also name external packages, such as `react` or `@scope/*`.
3. The config is data after it is loaded. Archangel never runs config code per file or per import.

### Rules

The first release has these rule kinds. Each accepts a human reason that appears in every
diagnostic, a link to where a reader can learn more, and a choice of which imports it covers: all,
only imports that survive compilation, or only type imports.

1. **Layers.** An ordered list of elements in which each may import only the elements after it.
2. **Isolate.** Instances of one element may not import each other, such as one feature from
   another.
3. **Entry.** Code outside an instance of an element imports it only through its entry files.
4. **Only.** An element may be imported only by the named elements.
5. **Forbid.** The named elements may not import the other named elements.
6. **No cycles.** No import cycle, with type-only imports left out unless the rule says otherwise.

### Rules that prove themselves

1. Any rule can carry examples: small virtual file trees it must accept and file trees it must
   reject. Archangel checks the examples in memory, against the real repository, without writing
   any file to disk.
2. An example passes only for the right reason. A rejected example must be rejected by its own
   rule, not by another one. An accepted example must actually exercise its rule; an example the
   rule never looked at is reported, not passed.
3. An element whose patterns match no files is an error, because a rule about it checks nothing.

### Exceptions and baselines that only shrink

A team adopting archangel on an existing repository needs to turn a rule on before the code obeys
it, and occasionally needs one deliberate exception. Both are escape hatches, and an escape hatch
that can grow quietly undoes the rule.

1. An exception for one import is written next to that import and must give a reason. An exception
   without a reason is an error.
2. An exception that no longer matches a violation is an error, so exceptions disappear when the
   code is fixed instead of accumulating.
3. A baseline records the violations that exist when a rule is turned on. A check then fails only
   on violations that are not in the baseline. Archangel removes an entry when its violation is
   fixed and never adds one during a check, so the baseline can only shrink. Adding to it is a
   separate, explicit command.
4. The results always report how many exceptions and baseline entries there are, per rule, so the
   number is visible and can be held down in CI.

### Results

1. Every violation names the file, the line and the import, the rule, the reason, what to do
   instead, and where to read more. A violation is a prompt that arrives exactly when the mistake
   is made, so it must be enough on its own for a person or an agent to fix the code. Where archangel can offer a concrete fix, such as the entry file to import instead, it
   checks that the fix resolves and satisfies the rule before it offers it.
2. A file archangel cannot parse and an import it cannot resolve are reported, never silently
   skipped. Output is never truncated.
3. The command line prints readable results for people and JSON for tools and agents, with a
   stable code per rule kind and a stable identity per violation.
4. Results are deterministic: the same repository and config give the same results in the same
   order, on every machine.

### Commands

1. **check**: check the repository against the rules; exit with a non-zero status on a violation.
2. **test**: check every rule's examples.
3. **explain** a file: its element, its captured names, which rules apply to it, what it imports
   and whether each import is allowed, and what imports it.
4. **why** a file reaches another file or a package: the shortest import chain between them.
5. A **language server**: diagnostics in the editor as the developer types, for files they have
   not saved yet, and quick fixes for the fixes archangel has checked.
6. A **watch** mode that re-checks as files change and reports only what changed, in a form that
   a test watcher running beside a coding agent can relay into the agent's turn.
7. A way to see where a check spent its work: files read, files parsed, imports resolved, rules
   evaluated, and time per stage.

### Speed

1. Speed is the reason archangel exists, so each part of the design states its cost as work that
   grows with files, imports, rules or edits. Tests hold those counts. Tests never assert time.
2. An edit in the editor re-parses and re-resolves only the edited file. Work for one edit does
   not grow with the size of the repository, except where the design shows that nothing smaller
   is correct, such as a change to a configuration file that affects how every import resolves.
3. Targets to measure, on the 1.3-million-line monorepo above, on a laptop: a full check from cold
   in under one second, and the 400-rule self-test in seconds rather than minutes. These are
   targets for benchmarks to report, not test verdicts. If an early spike shows a target is wrong,
   the design says so.

### How archangel itself is tested

Archangel tells other teams whether their rules hold, so a wrong answer from it is worse than a
slow one. Each test layer has gaps, so the design stacks layers whose gaps do not line up:

1. **Unit and integration tests** for every requirement, as the specs' scenarios. Integration
   tests run the archangel binary on small fixture repositories.
2. **Property tests** for the parts with simple laws, such as path patterns and graph algorithms,
   checked against a slow reference implementation.
3. **Differential tests** against an oracle: import resolution compared with the TypeScript
   compiler's own resolution on real repositories, so a disagreement is a found bug.
4. **Fuzzing** for every input archangel parses from a person: the config, path patterns and
   inline exceptions.
5. **Simulated editor sessions**: random sequences of edits, creations, deletions and renames,
   driven through the language server, after each of which the results must equal a full check
   from cold. This is what proves that re-checking only the edited file is correct.
6. **Mutation testing**, whose score is recorded and may only go up.

As soon as the check command exists, archangel's own repository checks its architecture with
archangel.

### Distribution

1. The core is written in Rust. It builds on established crates for parsing, import resolution
   and the language server protocol rather than writing its own, unless the design shows a
   measured reason to.
2. TypeScript developers install it from npm, with prebuilt binaries, and it is also published as
   a Rust crate.

## Later releases

These shape the design, which must leave room for them, but the first release does not build
them:

- **Plugins.** Presets that are plain functions returning elements and rules, such as one for
  Feature-Sliced Design and one for the React Server Components boundary in Next.js. Rules written
  as queries over the graph instead of syntax-tree visitors. Extractors that add new kinds of
  facts, such as style declarations from CSS, so that ownership rules can cover them.
- **Structural pattern rules**, written as code patterns, for rules inside one file, such as "only
  the transport layer may throw".
- **Adoption tools**: generating a starting config from the folder structure, and importing
  configs from dependency-cruiser, eslint-plugin-boundaries, Steiger and ts-arch.
- **A persistent cache** across command-line runs and a background process shared by the command
  line and the editor.
- **Generated documentation** from rules and their reasons, and a local page that draws the graph
  of elements.

## Out of scope

- Type-aware rules, which need the TypeScript type checker.
- General code-quality lint rules and formatting.
- CommonJS `require` calls and other import forms that are not ES module syntax, unless the
  design finds them cheap to cover. If they are not covered, the results say how many files use
  them, so the gap is visible.

## Prior art

The design compares itself with these, and borrows from them where they already solve a problem:

- **dependency-cruiser**: import rules over a dependency graph, configured with regular
  expressions; slow on large repositories.
- **eslint-plugin-boundaries**, **Sheriff** and **Nx's module-boundary rule**: boundary rules
  inside ESLint, which pay ESLint's start-up and per-file cost.
- **Steiger**: Feature-Sliced Design rules.
- **ts-arch**: architecture tests written as unit tests with a fluent API; the API cannot be
  inspected, so the tool cannot explain or document the rules.
- **ast-grep**: structural search and lint in Rust, with a language server and rule tests written
  as accepted and rejected code; it reads one file at a time and does not resolve imports.
- **oxlint**: a Rust linter with an ESLint-compatible plugin API and a language server.
- **tach**: module boundaries for Python with a Rust core. It is the closest existing tool to this
  idea, in another language.
- **zanity**: deterministic sanity checks for code written by people and agents, in one pass per
  file, with a language server, an agent-oriented output, stable finding identities, baselines,
  and automatic fixes limited to changes that cannot alter what the code does. It checks code
  inside files rather than architecture across them, and its shape is a model for archangel's.
- **overwatch**: runs a project's tests and linters in watch mode beside a coding agent, records
  the failures that already exist, and returns only new ones into the agent's turn. Archangel's
  watch mode is meant to be one of the checks a tool like this runs.

## Questions the design must settle

1. How rules are written in the TypeScript config: as a function that receives the rule builders,
   which is what lets the types know the element names, or as imported functions.
2. Whether a TypeScript config needs Node at load time, and what happens on a machine without it.
3. Whether type-only imports count by default for each rule kind.
4. What counts as an element's instance root, and therefore which path an entry rule's entry files
   are relative to.
5. How the language server learns that files were created, deleted or renamed, and what it
   re-checks when that happens.
6. How examples see the rest of the repository: whether a planted file can import real files,
   and how a planted file interacts with a real file at the same path.
