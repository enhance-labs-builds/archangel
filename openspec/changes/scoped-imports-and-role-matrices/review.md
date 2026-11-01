# Review: scoped-imports-and-role-matrices

## Open questions

Nobody could answer questions while this change was written. Each item below is an assumption
that the change makes, and what would change if the answer is different. Items 1 to 12 come from the
rebuild of round 4; items 13 to 26 are earlier assumptions that still hold, renamed to the new terms.

1. **The specs follow the recommendation of each item of "Decisions that need you".** If the owner
   picks another option, the requirement named in that item and its tests change. Items 5
   (generated proof), 7 (part overlap), 9 (self default) and 10 (type imports) change the most.
2. **Conflict found: the owner asked for every change to be additive to 0.1 configs, and two
   recommendations are not.** The own-instance default of `mayOnlyImport` (item 9) and type imports
   under `verbatimModuleSyntax` (item 10) change 0.1 results, each because the 0.1 answer is a
   plausible wrong answer. The design names both, the compatibility test names each fixture they
   change, and the additive alternative is stated in each item. This is not resolved silently.
3. **Kinds apply in the whole repository, and a table's `files` choose only the importers.** This is
   needed for the proposal's own message "no row for use-case", whose target is outside the domain
   files. If kinds applied only inside `files`, that message would read "no row for api".
4. **A cell must name a row, a declared name or a path pattern.** So a misspelt cell can never pass,
   and the types can check every cell. A cell that names a kind with no row is refused, which means
   enhance3's stated domain table needs a `specification` row (note A below). The other option is to
   accept any kind name as a cell and list unused ones in the summary, which lets a misspelt cell
   pass until a file of that kind appears.
5. **Declared names win over kinds as row keys, and a clash is `table-name-ambiguous`.** The earlier
   change let a role shadow a declared name. With rows that mix kinds, parts and groups, a declared
   name must be reachable, so a key that names a part or group means it; when the kind pattern also
   gives that name to a file, the check refuses the clash instead of choosing.
6. **Captures other than `{kind}` in a kind pattern make instances.** The earlier change made them
   wildcards with no instance. One rule, "a capture always names an instance", makes "itself in the
   same instance" mean the same for parts and kinds.
7. **A file in two rows of one table is an error, `table-row-overlap`.** The earlier change judged it
   by both rows. The proposal says one row must say what a file may import; the file is still
   judged by both rows while the error stands, so no import passes in silence.
8. **`{kind}` never holds a dot, and a declaration file counts `.d.ts` as its extension.** This makes
   `board.entity.test.ts` the kind `test` and `board.entity.d.ts` an `entity`. A kind with a dot in
   it must be written as a group.
9. **`parts` carries the new meaning and `elements` keeps the old one.** Overlap is an error and
   presets are separate namespaces only under `parts`, so a 0.1 config keeps its results. A config
   that moves from `elements` to `parts` sees each overlap as an error, never as a changed verdict.
10. **The externals rules are archangel's own.** Matching subpaths, refusing a bare built-in name,
    an `@types/` name and a workspace package's name follow from the 0.1 resolver, which resolves
    `fs` to `node:fs` and a workspace package to a file. No other tool was copied.
11. **Generated proof uses the first files in path order.** Path order makes the proof the same on
    every machine. A row that refuses no existing file proves nothing yet and does not fail
    `archangel test`. If the owner wants such a row to fail, its requirement changes.
12. **Deferred, with reasons:** an explicit word to replace a preset rule (it breaks 0.1 configs that
    replace by name; `archangel rules` shows each replacement), `mayOnlyImportTypesFrom` (the
    `imports: 'runtime'` option says the same), layers and isolate over groups, one-line examples,
    the map from `archangel init`, a new capture syntax, and typed builders for a config split
    across files. The fixtures are one file each for that reason.
13. **The option is `within`, and its value is called an area.** "Scope" already means a package
    scope in archangel, and the glossary keeps one word for one meaning.
14. **Row names are `<table>.<row>`, and table and row names refuse a dot, a colon, a slash and white
    space.** The colon is refused because exception comments end the rule name at the first colon.
15. **`within` on `mayNotImport` narrows the side** to `{ all: [side, area] }`.
16. **`off` takes row names only.** A table name in `off` is refused with the closest row names.
17. **The options of a table apply to each row**, except `instance`, which a table refuses because
    each row already permits its own instance.
18. **The glossary is not edited by this change folder.** The run could edit only this folder. The
    specs use the new terms (part, group, externals set, area, table, row, cell, kind, kind pattern,
    reach, generated proof, unjudged) before the glossary has them; task 2.1 adds them first. This
    is a conflict with the spec rule "add a new term there before a spec uses it", named here.
19. **Conflict found: rule names with a colon cannot take an exception.** The exception parser splits
    at the first colon. This change refuses the colon only in table and row names, and fixing the
    parser stays out of 0.2.
20. **The JSON `schemaVersion` stays.** The summary gains `rowsWithNoFile` only when the config has a
    table.
21. **Config field names appear in requirement descriptions.** The spec rules ask to keep code
    identifiers out of them, but the existing specs name config fields such as `mayOnlyImport`, and
    this change follows the existing specs.
22. **The type test is called a unit test.** The task rules list no kind for
    `npm/archangel/test/types.test-d.ts`; unit is the closest.
23. **A preset's table never gives `table-empty`**; `preset-empty` covers the preset.
24. **Conflict found: a repeated capture name in a kind pattern.** openspec/specs/configuration/spec.md
    makes any capture name two times `pattern-invalid`; the change's MODIFIED copy allows `{kind}`
    to repeat in a kind pattern only, and keeps the existing scenario and test.
25. **Conflict found: the scheduled benchmark does not check enhance3.** Task 9.1 adds a one-time
    run on enhance3 instead.
26. **The length of enhance3's rewritten config is an estimate**, about 1,150 to 1,350 lines, from
    counts of tables, rows, cells and the other rules by kind. The rewrite will measure it.
27. **The fixture names.** The fixtures are `docs/examples/small-app` and
    `docs/examples/enhance3-slice`. If enhance3 should not be named in archangel's public docs, the
    second can be renamed without any other change.
28. **Line numbers in the design's example output come from the design's own configs.** The
    fixtures' expected output is the source of truth; the README copy is the one under test.

### Notes for enhance3, not for archangel

A. **The stated domain table names `specification` as a cell and has no row for it.** With
   `everyFileHasAKind` on, a specification file in the domain folder is refused, and 0.2 refuses a
   cell that names no row. The fixture adds `specification: ['entity', 'value-object',
   'aggregate-root', 'types', 'consts']`, the row enhance3's builder has today. enhance3's builder
   also has `domain-service`, which the stated table leaves out; it needs a row, or its files must
   leave the table's files.
B. **The table allows a cycle.** `value-object` may import `error`, and `error` may import
   `value-object`. A cycle between those kinds passes the table; only a `noCycles` rule catches it.
C. **A port may not name an aggregate root.** The row `port` lists `entity` but not
   `aggregate-root`, so a repository port cannot take or return an aggregate root by type. This may
   be intended; it is recorded so enhance3 can decide.

## Review log

- 2026-11-01: First draft of proposal, specs (role-matrices new; architecture-rules, rule-examples,
  selectors and distribution modified), design and tasks, written from the brief and from
  enhance3's main branch (`archangel.config.mjs`, `packages/architecture/archangel.mjs` and
  `index.mjs`). `openspec validate scoped-imports-and-role-matrices --strict` passes.

### Round 1

1. cold-read: the heading of decision 2 read as if every rule name refused a colon. fixed
2. cold-read: the single-rule `within` example looked like an exact replacement for every double negative. fixed
3. cold-read: "sentence", "class" and "cell" were used before they were defined, and "cell" had two meanings. fixed
4. cold-read: nothing said that a role cell means every file of the role in the area. fixed
5. cold-read: the design did not say how an accepted example of a matrix passes. fixed
6. cold-read: `off` on a whole matrix and a preset's empty matrix were open questions outside "Decisions that need you". fixed
7. cold-read: the estimate sentence read as rows times uses. fixed
8. cold-read: the proposal's Impact listed source files instead of what the user sees. fixed
9. limits: refusing `within` on `mayNotImport` was a failure state with a clear meaning to accept. fixed
10. limits: a role with the name of a declared selector was refused although the look-up order gives one meaning. fixed
11. limits: a declared role without a row was refused. fixed (the roles object form is removed, so a role always comes from a row)
12. limits: a matrix refused `names`, `specifier`, `instance` and `dependency` for no reason but non-use. fixed
13. limits: a derived example tried only one target before `rule-unproven`. rejected: derived examples are removed (finding 29)
14. limits: the `matrix-empty` message gave no next step. fixed
15. limits: a preset's matrix that matches no file was invisible apart from per-row lines. fixed
16. limits: a row whose only refused imports are packages was listed as having nothing to refuse. rejected: derived examples are removed (finding 29)
17. executable-spec: a matrix without an area or a name had no scenario. fixed
18. executable-spec: a row without a role had no loader scenario. rejected: the roles object form is removed (finding 32), and with a role pattern each row names its own role, so the case cannot be written
19. executable-spec: a row with files and nothing to refuse had no scenario. rejected: derived examples are removed (finding 29)
20. executable-spec: no scenario fixed which files a derived example takes. rejected: derived examples are removed (finding 29)
21. executable-spec: the JSON list of rows with no file had no assertion. fixed
22. executable-spec: only a colon in a matrix name had a scenario, not the other characters or role names. fixed
23. executable-spec: reason, link and the refused options had no scenario. fixed (the options now apply to each row, finding 12, so the scenarios show them applied)
24. executable-spec: a row that lists its own role, a packages cell and the look-up order had no scenario. fixed
25. executable-spec: a misspelt selector in an area had no loader scenario. fixed
26. executable-spec: the last file of a role deleted in the editor had no scenario. fixed
27. executable-spec: the cost requirements stated the mechanism, not the observable count. fixed
28. executable-spec: two requirements packed several behaviours each. fixed
29. innovation-tokens: derived examples were a new mechanism where 0.1 proves a shorthand with one written example. fixed (derived examples are dropped; one written rejected example proves a matrix and its rows; precedents in decisions 2 and 6 corrected)
30. innovation-tokens: the colon refusal works around the exception parser instead of fixing it. fixed (decision 4 says why the parser fix stays out of 0.2)
31. innovation-tokens: the design did not name eslint-plugin-boundaries' `element-types` as the normal solution. fixed
32. abstractions: the roles object form duplicated selectors with no current user. fixed
33. abstractions: the list of role patterns had no user. fixed
34. abstractions: the compiled area copied the side's three fields. fixed
35. abstractions: `Summary` gained a parallel list beside the preset names that match nothing. rejected: that list's JSON key is `presetNamesMatchingNothing`, and renaming it would change 0.1 output; the design now gives this reason and keeps the same text line
36. standalone: "cell" had two meanings. fixed
37. standalone: the "except a role declared as that same name" clause was unexplained. rejected: the clause is gone with the collision refusal (finding 10) and the roles object (finding 32)
38. standalone: "the first class that the row refuses" had no defined order. rejected: derived examples are removed (finding 29)
39. standalone: the spec defined accepted matrix examples that the design and tasks did not build. fixed
40. standalone: the Limits bullet disagreed with the specs on where rows are listed. fixed
41. standalone: the Types bullet left out the slash. fixed
42. standalone: the never-refuse fail point had no name or place. rejected: it belonged to derived examples, which are removed (finding 29)
43. standalone: the Steiger precedent named no place in the code. fixed (decision 3 names layers and the FSD preset; the path is in "Where in the code")
44. standalone: the proposal's Impact did not match "Where in the code". rejected: conflicts with finding 8, which removes file paths from the proposal; "Where in the code" now lists `may-only-be-imported-by.md` and `may-not-import.md`
45. standalone: the scheduled benchmark had no path. fixed (named in "Where in the code"; it does not check enhance3, open question 20)
46. standalone: "the brief" refers to a document the reader does not have. fixed
47. standalone: the explain requirement's title promised roles that the text did not show. fixed
48. performance: the design said the empty-row list runs only when a count crosses zero, but it runs on every assemble. fixed
49. performance: derived examples' trial resolutions were not counted. rejected: derived examples are removed (finding 29)
50. performance: per-file classification grows with the number of sets, and no measurement at enhance3's size was planned. fixed (Cost states it; task 8.2 adds the enhance3 run)

### Round 2

1. cold-read: two numbered lists were both called decisions, and the cross-references pointed to the wrong items. fixed (every reference names its list)
2. cold-read: Rust and `--stats` identifiers and the term "fault control" appeared above "Where in the code". fixed
3. cold-read: "Four ideas" named three. fixed (three ideas from 0.1, then the new part)
4. cold-read: "the 0.1 table" read as a matrix table. fixed
5. cold-read: the reason a role shadows a declared name was only in review.md. fixed
6. cold-read: the design did not show the new summary lines or the JSON key. fixed
7. limits: a matrix without `within` was refused although the config has a clear meaning. fixed (accepted; every row judges every import, and `files` defaults to every file)
8. limits: the `example-proves-nothing` message for a matrix, and for a rule with an area, was not stated. fixed
9. limits: a misspelt `files` pattern gave both `selector-empty` and `matrix-empty`. fixed
10. limits: `matrix-empty` was a failure state that "Decisions that need you" did not offer as a choice. fixed (item 9, with the summary-line option; open question 22)
11. limits: the design did not say how `{role}` is filled, whether it may repeat, or whether other captures are allowed. fixed (filled as literal text after tokenizing, may repeat, other captures refused with advice; open question 23)
12. executable-spec: the 0.1 compatibility scenario rested on snapshots that do not exist and on a JSON key that 0.1 output lacks. fixed (snapshots recorded in the first commit; `rowsWithNoFile` left out without a matrix)
13. executable-spec: an accepted matrix example that no row judges had no scenario. fixed (plus the scenario for a rule with an area)
14. executable-spec: a matrix rejected example that no row rejects had no scenario. fixed
15. executable-spec: no explain scenario showed a shadowed cell or a row that does not list its own role. fixed
16. executable-spec: a matrix with no row had no requirement or scenario. fixed (refused with `config-invalid`)
17. executable-spec: the default of a matrix's `files` had no scenario. fixed
18. executable-spec: an unresolved import on a rule with an area had no scenario. fixed
19. executable-spec: the last task was the enhance3 measurement, not the local test run. fixed (swapped)
20. abstractions: matrix-level data (examples, role pattern, area) had no named owner. fixed (`Config.matrices`, referred to by `RuleKind::MatrixRow`)
21. abstractions: `parse_rule` offered a `Parsed` variant, but `Parsed` is a tuple alias. fixed
22. abstractions: `MatrixName<N, X>` carried a preset parameter it never uses. fixed (`MatrixName<N>`; it cannot merge into `OwnRuleName`, which allows a slash after a preset prefix)
23. standalone: cross-references said "decision N" without naming the list. fixed (same fix as round 2, finding 1)
24. standalone: the design said four terms are new, the other files six, and "role pattern" was undefined. fixed
25. standalone: the definition of a class covered files only, so a package end of an import had no class. fixed
26. standalone: "about 195" and "up to 195" disagreed. fixed
27. standalone: the reference evaluator had no path or description. fixed
28. performance: the pair-entry fill cost of a matrix's sentences was left out of Cost, and no work count sees it. fixed

### Round 3 (interface review)

1. A, every file in a matrix's files has a row: added `everyFileHasARow` (off by default, summary count when off, files left out with `not`, rule `<matrix>.every-file-has-a-row`, message names the roles or rows); 10 enhance3 naming rules named as replaced. fixed
2. A, limits: Limits and "Decisions that need you" item 7 (was 6) claimed that a file with no row could not pass in silence; they now say it is judged by no row as an importer, and how the option and the summary close that. fixed
3. B, a matrix whose rows are declared elements or selectors: one builder, keys read as declared names when there is no role pattern; names, files, area, cells, types, examples and `matrix-empty` behave as for roles; shown with enhance3's package dependencies, the API layers and `entry`. fixed
4. C, re-estimate of enhance3's config: about 1,400 to 1,600 lines with a breakdown; the remaining bulk is item 13 of "Decisions that need you". fixed
5. D1, executable-spec: `{role}` twice conflicted with the configuration spec; added the MODIFIED requirement, the scenario and test for a role pattern that cannot mean what it says, task 1.6, and open question 24. fixed
6. D2, executable-spec: a file in two roles had no specified outcome; it is judged by both rows, with a scenario, test and task 1.3, and Decisions item 7 no longer claims one answer per file. fixed
7. D3, limits: another capture in a role pattern was refused without being asked for; it is now accepted and matches as `*` does. fixed
8. D4, limits: an empty table gave `config-invalid` while a table with no file gave `matrix-empty`; both now give `matrix-empty` under one decision. fixed
9. D5, executable-spec: a preset's matrix was named `fsd-x.roles`; it is `fsd-x/roles`, its rows `fsd-x/roles.entity`, and a config rule of that name replaces the row, with a scenario and test. fixed
10. D5, executable-spec (minor): two matrices with one name had no outcome; they give `rule-name-duplicate`. fixed
11. D5, executable-spec (minor): an empty area had no outcome; it is `config-invalid`. fixed
12. D5, cold-read: the frontend and ui tables' per-row index-file cells were left out of the claims and the estimate; the design now says how they are written and counts them, and the API example's reason is explained. fixed
13. D5, cold-read (minor): the proposal said enhance3 has five matrices; it now says 9 tables applied 14 times. fixed
14. D5, cold-read (minor): `matrix-empty` came after the preset exception to it; the two items are swapped (now 11 and 12) and the references updated. fixed
15. E, kept: `within`, row names `<matrix>.<row>`, one written rejected example per matrix, a row that permits exactly what it lists, and the types; design.md keeps its template shape and proposal.md stays near one page. fixed

### Round 4 (one-line examples)

1. Rebuilt the change around the tested interface proposal: `matrix` becomes `mayImport('<table>', rows, options)`, with the table first and options second. fixed
2. Renamed role to kind, role pattern to kind pattern (`{kind}`), matrix row to row, `everyFileHasARow` to `everyFileHasAKind`, and `matrix-empty` to `table-empty`; the capability `role-matrices` becomes `import-tables`. fixed
3. Tables fail closed: a file with a row is judged on every import into the repository, and a target with no row reads "no row for <kind, part or path>". fixed
4. A row now permits its own name in the same instance unless `mayNotImportItself` lists it, which reverses "a row permits exactly what it lists"; `everyRowMayImport` added. fixed
5. Rows may mix kinds, parts and groups; declared names win over kinds as keys, and a clash is `table-name-ambiguous`. fixed
6. Added `parts`, `groups`, `because` and `rule('<name>', { ... })` as the one spelling to 1.0, with mixing refused. fixed
7. Added `externals` with exact matching of subpaths, scopes and `node:` names, and refusals for bare built-ins, `@types/` and workspace names (B1). fixed
8. Defined where kinds apply: everywhere as targets, only `files` as importers (B2). fixed
9. Defined instances for parts and kind patterns, and one self default for tables and `mayOnlyImport`, named as non-additive (B3). fixed
10. `everyFileHasAKind` on by default; `{kind}` never holds a dot; `.d.ts` counts as one extension; tests and index files leave through `files` (B4). fixed
11. Made kind rows with no file and part or group rows with no file consistent with `selector-empty` and `element-empty` (B5). fixed
12. Two parts on one file are `part-overlap` under `parts`; a preset's parts are their own set; two rows on one file are `table-row-overlap` (B6). fixed
13. Type imports under `verbatimModuleSyntax` and `export type *` specified, named as non-additive (B7). fixed
14. One import that breaks two rules: two problems, one exception per rule, one baseline entry per rule (B8). fixed
15. Every pattern starts at the config folder; entry starts at the instance root, defined for parts without a capture (B9). fixed
16. Kept `{name}` captures, with a scenario for a one-word brace and the clash in "Decisions that need you" (B10). fixed
17. Added `archangel rules [path]` with reach, no elision, JSON, preset and replacement notes, and the unjudged report (A6, D). fixed
18. Added `archangel explain <file>:<line>` with judged and skipped rules and reasons (D). fixed
19. Stale exceptions and baseline entries for removed rows specified as failures; explicit preset replacement deferred with a reason (D). fixed
20. Generated proof of each table row from two repository files replaces the one written example per table, as the recommendation of item 5. fixed
21. Every config example in the docs and README becomes a checked fixture; the small app and the enhance3 slice are the payoff fixtures (E). fixed
22. Showed the small app's config in full with its error and `archangel rules` output, and re-estimated enhance3 at 1,150 to 1,350 lines (F). fixed
23. "Decisions that need you" rewritten as eleven items, eight from the owner and three found while writing. fixed
24. Recorded the enhance3 notes: the missing `specification` row, the value-object and error cycle, and a port that cannot name an aggregate root. fixed
25. Split six requirements over 500 characters so that `openspec validate --strict` passes. fixed

