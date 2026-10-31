---
name: test-verdicts
description: Rules for test verdicts that do not depend on timing. Use when writing, fixing or reviewing a test that waits on the language server or a watcher, is flaky, or asserts performance or amounts of work.
---

# Test verdicts

A test passes or fails because of what archangel did, never because of how fast the machine ran
it. So a test never sleeps, never compares a duration against a threshold, and never sets its own
timeout to make a slow case pass.

## Wait on the operation's result, not on the desired answer

A test that waits until the desired answer appears can wait forever after the operation has
already finished with a different answer. Wait for the operation's own terminal result instead,
then assert the output directly. The terminal result names the request it answers and covers
failure and supersession as well as success, so a wrong answer fails at once.

For the language server, that result is the diagnostics it publishes for the exact document
version the test sent. A test that waits for "some diagnostics" passes on a stale set. For a
watcher, the result is the check that reports the changed file, not the next check to finish.

## Flaky tests

A passing rerun is not evidence of repair. The person fixing a flaky test reproduces the ordering
or state dependency locally, repairs the shared owner or the defective test, and adds a controlled
counterexample that fails without the repair.

When a test passes locally and fails in CI, the person investigating reproduces the discrepancy
locally. They compare the toolchain and dependency versions, the command, the selected tests, the
effective configuration and the fixtures between the two runs. Nobody reruns CI to search for a
lucky pass; a new CI run follows an explained local repair.

## Performance and amounts of work

A performance test asserts how much work archangel does, not how long it took. It counts parses,
file reads, resolutions, rule evaluations and calls into JavaScript. Replacing an arbitrary
duration threshold with an equally arbitrary count does not help. The expected count comes from
the scenario: an edit to one file parses one file, and adding unrelated files leaves that count
unchanged.

Every work count needs a fault control: a test that restores the old defect, such as a second
parse of each file, and shows the count rejects it. A count also sits beside an independent
assertion on the result, because a count alone does not prove the result is right.

Benchmarks report time and decide nothing. They run on fixed repositories so that two runs can be
compared, and a regression they show becomes a work-count test before it is fixed.
