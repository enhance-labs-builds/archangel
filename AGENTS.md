# AGENTS.md

This file is the least certain check in the repository: an agent may skip it, and it costs
context in every session. So it stays short. When a tool can check a rule here, the rule moves
into the tool and its line leaves this file. A line that a tool already enforces names the tool.

1. A mistake that happens twice becomes a deterministic check: a lint, a type, a test or a CI
   step. Its error says what is wrong, what to do instead, and where to read more.
2. Everything is on and nothing warns. Every lint is at `deny`, locally and in CI, so a warning
   can never be tolerated. (`[lints]` in `Cargo.toml`.)
3. There are no exceptions at the call site. When a check fires on correct code, make the
   correctness explicit enough for the check to verify. When a check is wrong for this codebase,
   switch it off where it is configured (`Cargo.toml`, `zanity.toml`, `deny.toml`), with the reason
   on the same line, in a PR that says why. (zanity `relaxed-check`, clippy `allow_attributes`,
   `allow_attributes_without_reason`, `unfulfilled_lint_expectations`.)
4. No debt. No TODO, no skipped or ignored test, no temporary workaround. Do it now or don't do it.
   (`todo`, `unimplemented`.)
5. Tests are the executable spec: every feature and fix ships with tests that specify its
   behaviour. Stack test layers whose holes don't line up: unit, property, differential against
   an oracle, fuzz, and simulated editor sessions where the design calls for them.
6. Speed is what archangel is for. A change on the check path states the work it adds, and a test
   holds that work as a count. Tests count work and never time; benchmarks report time and decide
   nothing. See the `test-verdicts` skill.
7. No silent gaps and no plausible wrong answers. Archangel never skips a file, drops an import or
   truncates output without saying so, and it never guesses. A rule that cannot fire is a defect.
8. Errors are `Result` values until the command line or the language server turns them into
   output. (`unwrap_used`, `expect_used`, `panic`, `unreachable`, `exit`.)
9. Non-trivial work starts with an approved design: an OpenSpec change in `openspec/changes/`,
   written with `/design` and built with `/implement`. Archive the change
   (`openspec archive <change> --yes`) as the last commit of its PR. If review then changes
   behaviour, edit `openspec/specs/` in the same PR.
10. One task, one PR. Don't split work into small PRs, and don't debug on CI.
11. When sources disagree, an approved change in `openspec/changes/` wins over the code it changes.
    Otherwise the code and its tests win over `openspec/specs/` and other docs. Name every conflict
    you find in your final reply; never resolve one silently.
12. Before pushing, run `scripts/check`. CI runs the same script, so it passes there only if it
    passes here.
13. Comments earn their place. Follow the `comments` skill.
