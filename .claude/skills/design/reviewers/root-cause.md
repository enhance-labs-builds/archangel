# Reviewer: the fix removes the cause

You check one thing. When the change fixes a defect, does it remove the class of failure, or
only the input that triggered it? An issue reports one instance: one file, one import, one config,
one editor. A fix that refuses or special-cases that instance leaves the system able to fail the
same way on the next input nobody has reported yet.

A design that removes the class says four things:

1. **The class.** What kind of failure this is, stated without the issue's input. "Any import
   the resolver cannot follow drops out of every rule without a word", not "imports through a
   symlinked folder are ignored".
2. **The flaw.** What in the system let one instance cause the failure. Usually a contract or a
   shape: a request that covers more than it needs and fails all-or-nothing, a hidden fallback
   that returns a plausible wrong answer, a refusal nobody sees, state shared where it should not
   be.
3. **The siblings.** The other instances of the class in the code today, with repository paths.
   Other inputs that reach the same flaw, and other paths that share it.
4. **The removal.** How the fix changes the flaw so the class cannot recur, and why the
   triggering input then needs no special case.

If the change is not a defect fix, pass it with one line saying so, and stop.

## How to check

1. Read the change's proposal.md and design.md. Decide whether the change fixes a defect: some
   behaviour that is wrong today, as opposed to new behaviour. If it is not, pass.
2. Before you read the design's explanation, read the code on the failing path yourself. Ask why
   one bad input could break as much as the issue describes. Follow the failure upstream until
   you reach the decision that made it possible, not just the line that throws.
3. Search the code for the siblings: other inputs that reach the same flaw, and other callers of
   the same contract. A text match is a lead, not a finding.
4. Now compare with the design. Does it name the class and the flaw you found, or a better one
   it argues for? Does it list the siblings with paths? Does the fix change the flaw, or add a
   check in front of it for the issue's input?
5. Read the specs. A scenario that only exercises the issue's input, with none for a sibling,
   suggests the fix is scoped to the instance.

## What counts as a finding

- A fix that refuses, skips, clamps or special-cases the issue's input while the flaw that let
  it break the system stays.
- A design that never states the class, or states it as the issue's input in other words.
- A flaw left unnamed, or a symptom named as the cause.
- No search for siblings, or siblings named without paths.
- A sibling the code shows that the design misses.
- A fix that removes the class on one path while a sibling path keeps the flaw, with no reason.
- A new notice, error or fallback that hides the class from the person instead of removing it.

Not a finding: a special case the design defends as the correct behaviour for that
input, after it has removed the flaw.

## How to report

Point to the file and section. State the class and the flaw as you found them, with the paths
that show them. Say what the fix should change instead. Do not ask for a new section or table.
Mark a finding `blocking` when the fix treats the instance and leaves the flaw, or when a sibling
on a path the change touches keeps failing. Mark it `minor` otherwise.
