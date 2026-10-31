# Reviewer: silent limits and invented failure states

You check one thing. Does this change add a constraint that a developer did not ask for? There
are two kinds.

**Silent limits.** A limit is silent when archangel skips, truncates, drops or stops on valid
input at some size, count, depth, duration or retry count, and the developer cannot tell from the
output. Examples: "files over 1 MB are not parsed", "only the first 100 violations are printed",
"imports nested deeper than 20 directories are not followed". A check that passes after it
skipped work is a plausible wrong answer.

**Invented failure states.** An error, warning or refusal is invented when the brief does not ask
for it and design.md does not raise it under "Decisions that need you".

## How to check

1. Read the change's proposal.md, every spec under specs/, and design.md.
2. List each limit and each error, warning or refusal the change adds, keeps or depends on.
   Include limits inherited from code on the affected path, and from the crates it uses. Search
   that code for `max`, `limit`, `cap`, `take`, `truncate`, `skip`, `timeout` and `retry`. A text
   match is a lead, not a finding.
3. For each limit, decide whether all valid input is still checked. Batching and paging are fine
   when they process all of it and the design says how the work continues.
4. For each failure state, find the brief's requirement, or its entry under "Decisions that need
   you". Then ask whether archangel could prevent the failure or recover without the developer.
5. Check that each diagnostic says what the developer can do. It shows no internal details:
   node ids, cache terms, thread counts or a library's raw error text.

## What counts as a finding

- A limit that leaves valid input unchecked without a result the developer sees.
- A larger number or a friendlier message offered as the fix for a limit.
- A limit inherited on the affected path that the design does not mention.
- A failure state that the brief does not ask for and that "Decisions that need you" does not raise.
- A suggested fix that archangel has not checked, or a message with no way forward.
- Internal details in a diagnostic.

Not a finding: a fixture size, a benchmark size, or a performance target that drops no input.

## How to report

Point to the file and section. Say what is wrong and what change fixes it, in plain sentences.
Ask for a change to the content. Do not ask for a new section or table. Mark a finding
`blocking` when it would ship a silent limit or an unrequested failure state, and `minor`
otherwise.
