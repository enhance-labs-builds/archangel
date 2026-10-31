---
name: implement
description: Build an approved OpenSpec design from its draft PR in a worktree rebased on main, archive the change, and mark the PR ready.
argument-hint: <PR number, or a change name for a local-only design>
disable-model-invocation: true
---

# Implement

Build the OpenSpec change in draft PR $ARGUMENTS. The design, the code, the tests and the archived
specs all ship in this one PR.

Started without an argument, there is no approved design to build: ask the person for a brief,
follow the `design` skill for it, and stop at its draft PR.

## 1. Find the change

```bash
gh pr view $ARGUMENTS --json headRefName,isDraft,title,files
```

The change is the one folder under `openspec/changes/` (not `archive/`) in the PR's files. If
there is none, or more than one, stop and ask the person. For a local-only design, the argument
is the change name and its branch is `design/<change>`.

## 2. Work in the PR's worktree, rebased on main

Run `git worktree list`. If a worktree already has the PR's branch, use it. Otherwise create one
under the main checkout, which is the first path `git worktree list` prints. Write that path out
in full, not in a shell variable:

```bash
git fetch origin <branch>
git worktree add <main checkout>/.claude/worktrees/<change> <branch>
```

Move this session into that worktree with the EnterWorktree tool. Then rebase on the fetched
remote main, or on local `main` when there is no remote:

```bash
git fetch origin main
git rebase origin/main
```

Resolve conflicts without dropping any design commit.

## 3. Apply review feedback on the design first

Read the PR's comments and review comments (`gh pr view $ARGUMENTS --comments`, and
`gh api repos/{owner}/{repo}/pulls/$ARGUMENTS/comments`). If a comment asks for a design change,
update the change's artifacts first. Run `openspec validate <change> --strict` and commit.

## 4. Build it, tests first

Follow `.claude/skills/openspec-apply-change/SKILL.md` for the change, in this order:

1. Write one test for each scenario in the change's specs, named after the scenario. Run each new
   test and confirm it fails for the right reason.
2. Implement the tasks in `tasks.md`. Tick each task when it is done.
3. Run the tests the change touches, locally, as AGENTS.md requires. Every scenario's test passes.

If the code shows that the design is wrong, change the spec and design files in the same PR and
say why in the commit message. Do not build something the specs do not describe.

## 5. Archive the change

Archiving is the last commit. It merges the change's spec deltas into `openspec/specs/`.

```bash
openspec validate <change> --strict
openspec archive <change> --yes
git add openspec
git commit -m "Archive <change>"
```

The OpenSpec CI check fails while the change is still active, so do not skip this step.

## 6. Hand it over

```bash
git push --force-with-lease
gh pr ready $ARGUMENTS
```

Update the PR body. Add the tests written, the tests run and their results, and any change to
the design. Report the PR link.

## After review

If a review fix changes behaviour, edit the spec under `openspec/specs/` in the same commit as
the code. The change is already archived, so the specs are now the record.
