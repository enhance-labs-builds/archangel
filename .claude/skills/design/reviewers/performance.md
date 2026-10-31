# Reviewer: cost on the hot path

You check one thing. When the change adds or moves work onto a hot path, does the design say what
that work costs as the repository grows, and how anyone will know? A hot path runs per file, per
import, per rule and import pair, or per edit in the editor. Work there that grows with the whole
repository makes every check slow, and nobody notices until someone runs archangel on a
monorepo ten times the size of the fixtures.

A design on a hot path states three things:

1. **The cost.** How the work grows with the number of files, imports, rules or open editors.
   "Re-resolves the imports of the edited file only", not "re-resolves what it needs".
2. **The evidence.** A work count a test holds, or a planned before-and-after measurement with
   the work counts archangel already reports. Build on those counts. Do not add a parallel one.
3. **The reason for growth.** When the work for one edit grows with the repository rather than
   with the edit, why nothing smaller will do.

If the change touches no hot path, pass it with one line saying so, and stop.

## How to check

1. Read the change's proposal.md, design.md and tasks.md. List the paths the change touches
   that run per file, per import, per rule and import pair, or per edit. Read the code on those
   paths, not only the design text: work hidden behind a helper still runs every time.
2. For each path, work out how the new or changed work grows. Look for a second parse of a file,
   a file read twice, a resolution repeated for the same specifier from the same directory, a
   loop over every rule for every import, a whole-graph walk per edit, a call into JavaScript per
   import or per syntax node, and output formatted for results nobody will see.
3. Compare with what the design states. Check that its cost claim matches the code it describes.
4. Check for evidence: a cited measurement, or a task that measures before and after with the
   existing work counts and a test that holds the count.
5. Check that the change does not slow the check for repositories it is not about. A fix for one
   rare input should not add work to every file.

## What counts as a finding

- Work on a hot path whose cost the design does not state.
- A cost claim the code contradicts.
- Work per edit that grows with the repository where the edit would do, with no reason.
- No measurement and no planned before-and-after measurement, when the cost changes.
- A new counter, timer or profiler built where archangel's existing work counts already do the job.
- A timing threshold used as the test verdict instead of a work count. See
  `.claude/skills/test-verdicts/SKILL.md`.

Not a finding: work bounded by the edit or by a fixed size, stated as such.

## How to report

Point to the file and section. State the cost you found and the path that shows it. Say what the
design should state, measure or change. Do not ask for a new section or table. Mark a finding
`blocking` when new per-edit or per-import work grows with the repository without a reason, or
when a claimed cost is wrong. Mark it `minor` otherwise.
