---
name: innovation-tokens
description: Check a technical choice against the established way to solve its problem before the choice is made. Use when choosing an approach, an architecture, a data structure, a library, a script, a test mechanism or a fix, and above all when the proposal is a new mechanism such as a ledger, graph, registry, state machine, queue, cache, retry layer, custom script, gate or counter. Also use when a fix works around an earlier fix, when nobody can name who else solves this problem this way, and when reviewing a design. Pass the problem, the proposed approach and the files involved.
context: fork
background: false
argument-hint: <the problem, the proposed approach, and the files it touches>
---

# Innovation tokens

Check one technical choice: $ARGUMENTS

Most problems an engineer meets are not new. Someone has solved them before, and the solution has
a name. A novel solution costs more than its own code. It costs the tests, the documentation, the
debugging nobody else has done, and the next novel thing built to fix it. Dan McKinley calls this
budget *innovation tokens*: a team gets about three ("Choose Boring Technology", 2015). This
check makes sure a choice spends a token only on purpose, and only where archangel differs from
every other tool.

You report. You do not change code or design files.

## Inputs

The arguments must name the problem and the proposed approach. If they name files, read them. If
you cannot state the problem and the approach after that, stop and return
`Verdict: need input` with what is missing. Do not guess the proposal.

## Method

Do these steps in order. Each step can end the check. Write down what each step found.

1. **State the problem without our words.** Write one sentence a stranger could search for, such
   as "resolve a TypeScript import specifier to a file" or "find cycles in a directed graph".
   Remove project names, issue names and our own terms. If you cannot write that
   sentence, the problem is not understood yet. That is the finding.

2. **Ask whether we should have this problem at all.** Follow the problem upstream: what in our
   code or our earlier decisions causes it? Read `git log` and `git blame` on the code involved.
   Check for these two shapes, from Peter Senge's systems archetypes (*The Fifth Discipline*, 1990):
   - *Fixes that fail.* The problem is a side effect of an earlier fix. The proposal is the next
     floor of a tower. Name each floor.
   - *Shifting the burden.* The proposal treats a symptom. A fundamental fix exists, costs more
     and would remove the symptom and others with it.
   If the cause is our own accidental complexity (Fred Brooks, "No Silver Bullet", 1986) and not
   part of the problem itself, the default recommendation is to remove the cause.

3. **Find how this is normally solved.** Look in this order, and stop at the first good answer:
   - this repository: code with the same shape;
   - the platform: the Rust standard library, Node, the operating system, or the Language Server
     Protocol;
   - a crate already in `Cargo.toml`, or a package already in a `package.json`;
   - a tool that solves the same problem: oxc, Biome, ESLint, ast-grep, dependency-cruiser,
     rust-analyzer, ripgrep;
   - a named pattern or a well-known library in the field.
   Use web search. If you cannot search, say that the reference is from memory. Name the solution
   and its source: a file path or a URL. "Nothing like this exists" is a claim. Support it with
   what you searched.

4. **List the preconditions of the normal solution.** It may not fit because we lack something it
   assumes: a schema, a stable identifier, an event, a single owner, a real API instead of a
   scraped one. Price the work to meet each precondition. Then list the other problems that work
   would also solve. The precondition work is often larger and still the better choice.

5. **Count the tokens.** List every novel element the proposal adds: each script, data structure,
   file format, protocol, state machine, registry, ledger, gate, counter, convention or term that
   has no established name outside this repository. For each one, say whether it is **core**,
   meaning what developers choose archangel for (checking architecture rules quickly and
   explaining each result, on the command line and in the editor), or **context**, meaning everything else (Geoffrey
   Moore's split). Simon Wardley's evolution axis gives the same test: building a custom version of
   something that is already a product or a commodity is waste. A novel element in context is a
   finding.

6. **Decide.** Choose one:
   - **Conventional.** Use the normal solution. Say what replaces the proposal.
   - **Fix upstream.** Remove the cause from step 2, or meet the preconditions from step 4.
   - **Spend a token.** Allow the novel element only when all of these are true: it is core, the
     design names the normal solution, and it says why this case is different, with evidence such
     as a measurement, a spike or a constraint the normal solution breaks. The decision must say a
     token is spent, so the spending is a choice and not an accident.

## Gotchas in this repository

- Rules already settle some choices. Do not reopen them: errors are `Result` values until the
  command line or the language server turns them into output, and tests count work instead of
  time (see the `test-verdicts` skill).
- A gate, counter or check that guards another mechanism is still a mechanism. Count it in step 5.
- A known library used for an unusual purpose is novel. Count the purpose, not the dependency.
- Speed is core, so a faster novel element can be worth a token. It still needs a measurement
  that shows the normal solution is too slow, not a belief that it is.

## Report

Return exactly this, short and in plain words. The caller reads it without your notes.

```
Verdict: conventional | fix upstream | spend a token | need input

Problem: <step 1 sentence>
Upstream cause: <what creates the problem, the tower floors if any, or "inherent to the problem">
Normal solution: <name> — <file path or URL>
Preconditions: <each one, what meeting it costs, what else it fixes; or "met">
Tokens: <each novel element — core or context>; or "none"
Recommendation: <what to do instead, or the justification the design must state to spend the token>
```
