# Review: scoped-imports-and-role-matrices

## Open questions

Nobody could answer questions while this change was written. Each item below is an assumption
that the change makes, and what would change if the answer is different.

1. **The option is `within`, and its value is called an area.** The brief says "within a scope".
   In archangel, "scope" already means a package scope (`'@acme/*'`), and the glossary lists
   "scope" as a word not to use for a filter. If "scope" is preferred, the glossary must give it a
   second meaning, which its one-word-one-meaning rule forbids.
2. **A matrix row permits its own role only when it lists it.** This keeps enhance3's
   `domain-event` row, which does not list itself, meaning what it means today. `within` on a plain
   `mayOnlyImport` keeps 0.1's own-files permission. If rows should always permit their own role,
   the second sentence per row goes away, and the enhance3 rewrite would permit one domain event
   to import another, which its rules refuse today.
3. **Row names are `<matrix>.<role>`, and matrix and role names refuse a dot, a colon, a slash
   and white space.** The colon is refused because exception comments end the rule name at the
   first colon. A different separator changes only the naming requirement and its tests.
4. **One written rejected example proves a matrix and all its rows**, as one proves a `layers`
   rule. A row with files whose cells are wrong is not caught by that example. If per-row proof is
   wanted, an example that archangel derives for each row from repository files was designed and
   dropped in round 1 as a new mechanism that the 0.1 precedent does not need.
5. **Roles come from at most one role pattern with the capture `{role}`.** The capture is filled
   with each row's name, which gives one literal pattern per role. A matrix without a role pattern
   has declared elements and selectors as rows (round 3), which covers roles found by a directive
   or a folder.
6. **The area includes files that have no row, as targets.** A row refuses an import of an untyped
   file in the area unless a cell names it. As an importer, a file with no row is judged by no row;
   `everyFileHasARow` refuses it, and without the option the summary counts it. If rows should judge
   only imports of files that have a row, an untyped file escapes the table as a target.
7. **A matrix's `within` is optional; its `files` default to the area's files, or to every file
   without an area.** Without an area each row judges every import, packages included, as a 0.1
   `mayOnlyImport` rule does. Round 2 removed the refusal of a matrix without an area, because the
   config has a clear meaning.
8. **`within` on `mayNotImport` narrows the side** to `{ all: [side, area] }`. This gives two ways
   to write one rule; refusing it would refuse a config whose meaning is clear.
9. **`off` takes row names only.** `off: ['api-domain']` is refused with the closest row names,
   as any unknown name in `off` is today. Dropping a whole matrix is done by removing it.
10. **A matrix applies `names`, `specifier`, `instance` and `dependency` to each row**, as it
    applies `reason`, `link` and `imports`. No measured matrix uses them.
11. **A role shadows a declared name of the same name inside its own matrix.** `explain` shows
    which one a cell means. Refusing the collision would hit names such as `types` and `consts`,
    which enhance3 uses as roles.
12. **The glossary is not edited by this change folder.** The run that wrote this change could edit
    only `openspec/changes/scoped-imports-and-role-matrices`. The specs use six new terms (area,
    role, role pattern, matrix, matrix row, cell) and four shorthand kinds before the
    glossary has them; task 2.1 adds them first. This is a conflict with the spec rule "add a new
    term there before a spec uses it", named here and not resolved silently.
13. **Conflict found: rule names with a colon cannot take an exception.** The exception parser
    splits `archangel-allow <rule>: <reason>` at the first colon, and the exceptions spec puts no
    limit on rule names. enhance3's generated names, such as `role:api:domain:entity`, therefore
    cannot be named by an exception today. This change refuses the colon only in matrix and role
    names, because refusing it in every rule name would break 0.1 configs. Fixing the parser stays out of
    0.2, because any reading that lets a name hold a colon changes what some 0.1 comment means, such
    as `archangel-allow no-legacy:migration`. A follow-up could parse the rule name up to the last
    `: ` or refuse colons in 0.3 with a deprecation in 0.2.
14. **Conflict found: enhance3 has 105 naming-suffix rules, not about 45.** The request for round 3
    says about 45. enhance3's builder (`suffixScopes` in `packages/architecture/archangel.mjs` on its
    main branch) emits 4 API rules, 4 for the two local-agent apps, 75 for the three frontend apps
    (4 sliced layers and shared, times 5 segments, times 3 apps), 2 and 4 for the web and extension
    shared segments, 6 for `packages/ui`, 2 for ui geometry and native, 3 for the SDK, and one each
    for capture-browser, artwork-paint, canvas-document, figma-import and contract. Written by hand
    with brace alternatives they are about 26 distinct folder-and-role-list pairs. `everyFileHasARow`
    replaces 10 generator rules (7 of the 26 pairs); the design names them.
15. **The length of enhance3's rewritten config is an estimate** of about 1,400 to 1,600 lines,
    from the counts of tables, rows, cells and the other rules by kind. The previous estimate,
    1,000 to 1,200 lines, left out written examples, preset overrides and literal file lists. The
    rewrite will measure it (task 8.1 measures the API and package part).
16. **The JSON `schemaVersion` stays.** The summary and the test output gain fields; no field
    changes. If new fields need a new schema version under the project's reading of semver, the
    compatibility section and task 7.2 change.
17. **Config field names appear in requirement descriptions.** The spec rules ask to keep code
    identifiers out of them, but the existing specs name config fields such as `mayOnlyImport`,
    and this change follows the existing specs.
18. **The type test is called a unit test.** The task rules list no kind for
    `npm/archangel/test/types.test-d.ts`; unit is the closest.
19. **A preset's matrix never gives `matrix-empty`.** `preset-empty` covers a preset, as for its
    elements. A preset whose matrix is empty while its elements match files passes, and the summary
    gives one line that names the matrix, with the preset's prefix, and its role pattern.
20. **Conflict found: the scheduled benchmark does not check enhance3.** The first draft said it
    reports time on enhance3; `benches/repos.toml` lists vscode, excalidraw and a generated
    repository. Task 8.1 adds a one-time before-and-after run on enhance3 instead.
21. **Rows with no file get their own summary field.** The existing list of preset names that match
    nothing is written to JSON as `presetNamesMatchingNothing`, and renaming that key would change
    0.1 output. The new key, `rowsWithNoFile`, is left out when the config has no matrix.
22. **A matrix with no row, or whose rows have no file, fails the check with `matrix-empty`.** This
    change could not read the brief to confirm that it asks for this error. If it does not, the other
    option is one summary line, as for a preset's matrix; "Decisions that need you", item 11, states
    both. An empty table was `config-invalid` until round 3; it now shares this one decision.
23. **A role pattern takes `{role}` one or more times, and other captures act as wildcards.**
    `{role}` is filled with the role name as literal text after the pattern is read, so a role name
    never acts as a wildcard. Another capture matches as `*` does, never an empty part, and gives no
    instance. A repeated capture name other than `{role}` stays `pattern-invalid`.
24. **Conflict found: a repeated capture name in a role pattern.** openspec/specs/configuration/spec.md,
    requirement "Archangel refuses a pattern that cannot mean what it says", makes any capture name
    two times `pattern-invalid` (test `repeated_capture_name_is_pattern_invalid`), while this change
    lets `{role}` repeat in a role pattern. The change adds a MODIFIED copy of that requirement in
    specs/configuration/spec.md that allows the repetition for `{role}` in a role pattern only and
    keeps the existing scenario and test unchanged.
25. **A file that two rows have is judged by both.** Each row is an ordinary rule, so this needs no
    mechanism. If the first row in table order should win, or such a file should be refused, the
    requirement "A file that two rows have is judged by each row" and its test change ("Decisions
    that need you", item 6).
26. **`everyFileHasARow` is off by default, and the summary counts files with no row.** If it should
    be on by default, the option becomes `everyFileHasARow: false` to turn it off, the summary count
    goes away, and the default config of every matrix refuses unsuffixed files ("Decisions that need
    you", item 8). The name was chosen because it is true for both forms of matrix;
    `everyFileHasARole` is false for a matrix of packages.
27. **What still makes enhance3's config long is left for later changes.** One-line import examples,
    a naming condition that takes a list of roles, and a Feature-Sliced Design preset option would
    each remove one block of the estimate. They are not in this change ("Decisions that need you",
    item 13).
28. **Some enhance3 tables need index-file cells in their rows.** Its frontend and ui tables permit,
    in each row, the index files of the slices and segments that row may use, which its builder
    computes. The design writes the slice index files out of the area and each segment index file as
    a declared selector cell. If a derived cell per row is wanted, it is a later feature.

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
