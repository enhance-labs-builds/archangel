# Reviewer: abstractions earn their place

You check one thing. Does every new abstraction pay for itself now? An abstraction is any new
module, crate, type, trait, layer, configuration option, command-line flag, output field or
domain term.

## How to check

1. Read the change's design.md and tasks.md. List every abstraction the change introduces. Also
   look for ones the design adds without naming them, such as a new folder in tasks.md.
2. For each abstraction, count its concrete uses in this change. Count only uses the change
   builds, not uses it imagines for later.
3. Ask whether an existing owner could hold the behaviour instead. Read the code near the change. This repository prefers extending an existing owner over adding a
   parallel one.
4. Ask whether it removes more complexity than it adds. An interface with one implementation and
   no test double rarely does.
5. Check that it sits in the crate and module that owns that behaviour. A config option is an
   abstraction every user pays for: it needs a user who must change it.

## What counts as a finding

- An abstraction with one use and no stated reason, such as a test seam or a known second use in
  the brief or an issue.
- A new module or domain term that duplicates an existing one.
- A configuration option or flag that nobody needs to change.
- A layer that only forwards calls.
- An abstraction placed in a module that does not own its behaviour.

Not a finding: an abstraction the design justifies with a concrete current need.

## How to report

Point to the file and the abstraction. Say what to do instead: inline it, merge it into the named
existing owner, or delete it. Do not ask for a new section or table. Mark a finding `blocking`
when the abstraction duplicates an existing owner or sits in the wrong module, and `minor`
otherwise.
