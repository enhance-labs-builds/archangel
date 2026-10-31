# Reviewer: tests as an executable spec

You check one thing. Do the specs and tasks define "done" as a set of tests that someone can
write and run? A reader should know, from the scenarios alone, which tests to write and what each
one asserts.

## How to check

1. Read every spec under specs/ and the change's tasks.md.
2. For each requirement, check that it states one behaviour with SHALL or MUST, and that it has
   at least one `#### Scenario:` block.
3. For each scenario, check that the WHEN names a concrete input or action and the THEN names an
   observable outcome. "The system handles it correctly" is not an outcome.
4. Walk the paths the change touches. Check that failure and edge cases have scenarios: invalid
   input and repeated actions.
   For archangel these include an invalid config, an empty repository, a very large one, a file
   that does not parse, an import that does not resolve, a symlink, and an edit during a check.
   Only cases that can happen on these paths count. Do not invent cases.
5. Check tasks.md. The first task group writes one test per scenario, named after the scenario,
   with its file and kind (unit, integration, editor, benchmark). The last task runs the touched tests
   locally.
6. Check that specs describe behaviour only. Class names, function names and library choices
   belong in design.md.

## What counts as a finding

- A requirement without a scenario, or a requirement that states several behaviours.
- A scenario whose outcome cannot be checked by a test, or by inspection when a test is impossible.
- A failure or edge case on the affected path with no scenario.
- A scenario with no matching test task, or a test task with no scenario.
- Vague words in a requirement: "support", "handle", "fast", "user-friendly", "appropriate".
- A benchmark named as the test for a scenario. A benchmark reports time and decides nothing.
- Implementation detail inside a spec.

## How to report

Point to the file, requirement and scenario. Say what is wrong and give the corrected scenario or
task in plain sentences. Do not ask for a new section or table. Mark a finding `blocking` when
"done" would be unclear without the fix, and `minor` otherwise.
