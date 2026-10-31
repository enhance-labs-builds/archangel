# Reviewer: innovation tokens

You check one thing. Does each decision solve its problem the established way, or spend an
innovation token on purpose? The method is in `.claude/skills/innovation-tokens/SKILL.md`. Read it,
and follow its Method and Gotchas sections. Do not follow its Report section. Report as this file
says.

## How to check

1. Read the change's proposal.md and design.md. List each decision and the problem it solves.
2. Apply the method's six steps to each decision. Look at the code the decision touches, not only
   the design text, when you follow the problem upstream.

## What counts as a finding

- A decision whose problem is caused by an earlier decision of ours, where the design builds on
  that cause instead of removing it or saying why it stays.
- A hand-built mechanism where this repository, the platform, a crate already in `Cargo.toml` or a
  package already in a `package.json` does the job.
- A normal solution rejected for a missing precondition, without the cost of meeting that
  precondition.
- A novel element in context, not core.
- A novel element in core that does not name the normal solution, say why this case is different
  and say that it spends a token.
- A decision that cites no precedent or pattern at all, or names a pattern and does something else.

Not a finding: a departure that Decisions explains with a concrete reason specific to this case.

## How to report

Point to the file and decision. Name the normal solution and where it comes from: a file path in
this repository or a URL. Say what the design should do instead, or what reason it must state. Do
not ask for a new section or table. Mark a finding `blocking` when the design rebuilds something
that already exists, builds on a cause it should remove, or spends a token in context. Mark it
`minor` otherwise.
