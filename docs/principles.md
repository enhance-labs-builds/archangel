# Principles

This page tells you what archangel believes about architecture, and why. Read it to understand the
decisions behind the tool. The [README](../README.md) tells you how to use it.

## What architecture is

Architecture is a set of decisions. They say which code may depend on which code, where code lives,
and what each part shows to the other parts. Some examples:

- Features do not import each other.
- The UI does not import the database client.
- Shared code does not know about the app.
- Each feature has one public file, `index.ts`, and other code imports only that file.

These decisions make a codebase easy to change. If features do not depend on each other, you can
change one feature without breaking a different feature. If the UI does not know about the
database, you can replace the database.

## Why architecture must be verifiable

An architecture decision is usually in a document, a diagram, or the memory of a senior engineer.
None of these stop the next import. Each small change can break a decision, and each break makes the
next break easier. After some months, the code no longer has the architecture that the diagram
shows.

A reviewer can find some breaks. But a reviewer reads the diff, not the whole graph of imports, and
a break is often one line in a large change.

Archangel turns each decision into a rule that a machine checks. A broken decision then fails like a
broken test. The architecture is in the code repository, it has a version, and each change is
checked against it.

## Why this is more important with coding agents

A coding agent writes code faster than a person can review it. It does what makes the task work: if
the fastest fix is an import from a different feature, it adds that import. It does not know your
architecture unless something tells it.

An agent needs three things to keep an architecture:

1. **A check that it can run itself.** Archangel runs from the command line, in watch mode, and as a
   language server.
2. **A fast answer.** If the answer comes in CI ten minutes later, the agent has already built more
   code on the break. Archangel answers in milliseconds.
3. **An answer that says what to do.** Archangel's messages name the rule and the reason. They also
   show the permitted alternative, for example "export it there, then import it through `../login`".

Tests verify behavior. The type checker verifies types. Archangel verifies architecture. With all
three, an agent can check its own work.

## Our opinions

### A rule that cannot fail is a defect

A rule that finds no files shows "no problems". It looks the same as a rule that works. This is the
most dangerous failure of an architecture tool, because the team believes that it is protected.

So archangel does not let a rule pass in silence:

- Each rule needs a **rejected example**: code that the rule must report. `archangel test` fails
  for a rule without one.
- An element or selector that finds no files is the error `selector-empty`.
- An import that archangel cannot resolve is an error, unless the config permits it.
- A file with a syntax error fails the check, because archangel cannot read its imports.

### There are no warnings

A rule is on or off. Teams learn to ignore warnings, and a long list of warnings hides the one that
matters. An agent cannot know which warnings are important. If a rule is not important enough to
fail the check, turn it off.

### Each problem says what to do

A message is an instruction for the next reader, a person or an agent. It names the rule and your
reason, and it shows the permitted alternative. If archangel can make a fix that it has checked, it
offers that fix.

### Exceptions are visible and have a reason

Sometimes a break is correct. An exception is a comment directly above the one import that it
permits, with a reason:

```ts
// archangel-allow isolate(feature): the checkout flow needs the cart total until #142 moves it
import { total } from '../cart/total';
```

For an old codebase, `archangel baseline add` records the problems of today. The check then fails
only for new problems. The baseline can only get smaller: when you fix a problem, you prune its
entry.

### Fast enough for each keystroke

A check that runs only in CI comes too late. By then, the code is written around the break. Archangel
reads each file one time and keeps a map of the imports in memory. After a change, it checks only
what the change can affect. So the editor, the watcher and the command line can all give the answer
while the code is written.

### The facts and the rules are separate

Which file imports which file is a fact about the code. Which imports are permitted is a decision
in the config. Archangel keeps the two separate. A change to the config does not read the code again,
and you can test the rules with small examples.

### Structure, not style

Archangel checks where code is, what it imports, what it exports, and what files and folders are
called. It does not check code style, the syntax inside a file, or types. ESLint and the TypeScript
compiler do that well. This limit is what lets archangel be fast enough for each keystroke: it does
not need a type checker.

## Where the ideas come from

Archangel adds speed, examples and agents to old ideas.

| Idea | Where it comes from |
| --- | --- |
| A module hides a decision, and other modules depend only on what it shows | David Parnas, "On the Criteria To Be Used in Decomposing Systems into Modules" (1972) |
| Layers, and no cycles between parts of the code | John Lakos, *Large-Scale C++ Software Design* (1996), which calls it levelization; Robert C. Martin's Acyclic Dependencies Principle |
| Dependencies point inward, toward the core of the application | Robert C. Martin's Dependency Rule, in *Clean Architecture* (2017); Alistair Cockburn's hexagonal architecture, or ports and adapters (2005) |
| Architecture rules as tests that run with the build | ArchUnit, for Java |
| An automated check that keeps an architecture from decay | "Fitness functions", in *Building Evolutionary Architectures* by Neal Ford, Rebecca Parsons and Patrick Kua (2017) |
| Import rules for JavaScript and TypeScript | dependency-cruiser, eslint-plugin-boundaries, the module boundaries of Nx, and Sheriff |
| Layers, slices and segments for front ends | Feature-Sliced Design, and its linter Steiger |
| A rule is proved by code it must accept and code it must reject | The `valid` and `invalid` cases of ESLint's `RuleTester`, and the idea from mutation testing that a check must be seen to fail |
| A baseline of old problems that only gets smaller | The baselines of linters such as Psalm and detekt |
| Fast tools for JavaScript, written in Rust | oxc, which archangel uses to read and resolve code, and Ruff and Biome |
| One analysis for the editor and the command line | The Language Server Protocol |

## What archangel is not

- **Not a linter for code style.** Use ESLint or Biome.
- **Not a type checker.** Rules that need types, for example "only the repository calls `save`",
  stay in ESLint with type information.
- **Not a diagram tool.** Archangel can explain a file and show a chain of imports, but it does not
  draw your architecture. It checks it.
