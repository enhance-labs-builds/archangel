# Review

## Open questions

- The `@archangel` npm organisation may already be taken. The design's fallback is unscoped
  platform packages.
- The three costs that the spike did not measure (classification and rule evaluation, one example,
  memory) are estimates until task 11.3 measures them.
- The number of classes and connected class pairs on a hand-written config for the dogfood, and the size of
  the plugin view, decide whether the class table and one view per check stay cheap; task 14.12
  measures both.

## Review log

The design was drafted in the session that wrote the brief, by the same writer, instead of by the
design-review workflow's cold writer, because that session held the brief's reasoning and a spike's
measurements. The eight reviewers below then reviewed it independently, each with only the
repository.

### Round 1

Seven of eight reviewers failed the draft; root-cause passed because the change adds behaviour and
fixes no defect.

**cold-read**
- The weakened self-check requirement sat only in the code table: fixed, now Decisions that need you 10.
- Decision 4 cited a "nothing warns" rule that the brief does not have: fixed, the real reason replaces it.
- The reader sections named the rule kinds without defining them: fixed, In short defines all six.
- The reader could not see where examples live: fixed, Examples opens with the config shape.
- Commands and exit statuses were not listed together: fixed, In short lists them; the proposal lists every command.
- "Who owns what" suggested ownership rules: fixed, the proposal puts ownership rules out of scope.
- "Which we measured" overstated the spike: fixed, In short now separates measured from estimated, with one file count.
- Undefined terms (the spike, innovation token, recat, zanity, TigerStyle): fixed, each is defined or removed.
- Capture used before its definition, and type-import erasure unclear: fixed in Decisions that need you 1 and 3.
- Identifiers above "Where in the code": fixed in part. Diagnostic codes are now words; config fields, commands and example paths stay, because the reader writes and runs them.

**limits**
- The ignore crate's defaults skip hidden files and use machine-local ignore files: fixed, the walker settings and a scenario for each.
- Import types and triple-slash references were not collected: fixed, both are type imports with scenarios; computed require calls are counted.
- notify can lose events or run out of watches: fixed, full check after lost events and polling with a note.
- Editors without file watch registration: fixed, the server watches files itself.
- Two exceptions could not apply to one import: fixed, a run of consecutive exceptions applies to the next import.
- rule-name-duplicate was an unraised refusal: fixed, now Decisions that need you 9, with the reason to keep it.
- Inherited limits (4 GiB parse, 64 resolver steps, crate sources) were missing: fixed in Limits.
- Resolver errors had no mapping and a bad tsconfig gave one error per import: fixed, plain reasons and one `config-file-invalid`.
- An old Node gave Node's own error: fixed, a version check and a scenario.
- Fixes could not see names behind `export *`: fixed, the graph keeps export names and fixes follow the chain.
- `file-unreadable` did not say it fails the check, and UTF-16 was refused: fixed, both stated with scenarios.
- Tracked files that a `.gitignore` pattern matches: rejected as a silent gap, kept as documented behaviour in Limits, because reading Git's index would make the source set depend on Git being present.

**executable-spec**
- Exit statuses were proven by library tests that cannot see them: fixed, a binary test is added to each such scenario.
- Fuzz targets and release jobs were not in group 1, and test kinds were undefined: fixed, tasks 1.9 and 1.10, and the config's test kinds now include unit, differential, fuzz and release.
- Watch JSON lines, config changes and unchanged saves had no scenario: fixed.
- "Config code runs once" and "JSON needs no Node" had no scenario: fixed.
- Fixes that would break another rule or cannot resolve had no scenario: fixed.
- A baseline that does not parse, and an entry for a removed rule: fixed, `baseline-invalid` and a stale entry.
- "Never truncated" had no requirement: fixed, with a 5,000-violation scenario.
- Queued edits during a check: fixed.
- Missing scenarios for check paths with cycles, explain, JSON work counts, wrong types, invalid patterns, package element order, only and forbid edges, type cycles with all imports, links, snapshots, NO_COLOR, order, block comments, blank lines, repeated baseline add, an empty repository, example syntax errors and tsconfig changes: fixed, one scenario each.
- Task 6.1 named four exception diagnostics: fixed, it names the three.

**innovation-tokens**
- Hand-built fault injection: fixed, the fail crate (decision 20).
- Hand-built mutation score ratchet: fixed, cargo-mutants with no surviving change (decision 21).
- Tarjan without naming petgraph: fixed, petgraph's Kosaraju search with an explicit stack (decision 8).
- The pattern matcher had no precedent: fixed, translation to regular expressions as globset does, and eslint-plugin-boundaries' capture syntax (decision 9).
- The exception comment cited the wrong precedent: fixed, Biome's suppression comment (decision 11).
- salsa was not named: fixed, named and rejected with its reason (decision 15).
- Manual per-crate publishing and no release precedent: fixed, `cargo publish --workspace` and Biome's workflow; cargo-dist named and rejected (decision 19).
- Why the binary, not the npm launcher, starts Node: fixed, the reason is stated (decision 10).
- A fresh resolver per example was unmeasured: fixed in part. The spike measurement was blocked by the machine's memory limit, so task 11.3 measures it and switches to a shared cache above 5 ms.

**abstractions**
- Session sat in the language server crate: fixed, a new engine crate owns the full check and `Session`.
- Baseline file input and output sat in the pure rules crate: fixed, the file is in the engine and matching stays pure.
- Exception parsing needed the parser's output in the wrong crate: fixed, the graph keeps comment text and the rules parse it.
- The no-cycles `within` option had no user: fixed, removed.
- Two pattern languages: fixed in part. Elements and entry files share one; the ignore list keeps `.gitignore` syntax because it extends `.gitignore` files.
- Two identities for a violation: fixed, the baseline key is the violation identity without its occurrence.
- Work and Fault ownership was unclear: fixed, Work stays in the graph crate with the reason, and each fail point is named in its crate.

**standalone**
- The design parsed created files while the spec forbade it: fixed, the spec now parses only created and renamed files.
- Task 6.1's count: fixed.
- Resolver settings existed only in the spike: fixed, written in the Graph row.
- Missing dependencies in the proposal: fixed.
- Baseline commands missing from the proposal: fixed.
- The exit status rule did not cover `why` and failed examples: fixed.
- No requirement for an old Node: fixed.
- Undefined terms (innovation token, zanity, rule builders, fault control, summary, package manifest): fixed in the glossary and the design.
- Glossary disagreements (type import, import forms, violation, pattern, "check" for a rule, "module" for a file): fixed.

**performance**
- Fix verification would re-parse: fixed, the graph keeps names and fixes parse nothing, with a fault control.
- A burst of file events would resolve once per event: fixed, waiting events share one resolution, with a fault control.
- Per-edit package use, baseline and publishing were unstated: fixed, each is adjusted by the edited file only; edit timing for an ordinary file is in task 11.3.
- Examples' cycle search and reads: fixed, the cycle search is limited to example files; the resolver cost is measured in task 11.3.
- Unmeasured costs (declaration files, the walk, Node start-up, the comment scan): fixed, each listed and measured in task 11.3; the comment scan is behind a text search.
- The rule table claim had no work count: fixed, with a five-or-five-hundred scenario and a fault control.
- Three work counts had no fault control: fixed, each has one.

### Round 2

cold-read and innovation-tokens passed; five reviewers failed. All findings below were fixed unless
marked otherwise.

**abstractions**
- Fix tests named the rules crate though fixes moved to the engine: fixed, `crates/archangel_engine/tests/fixes.rs`.
- `allowUnresolved` was applied in the graph, against the graph-free-of-config idea: fixed, the rules apply it and the graph drops its config dependency.
- `Engine` beside `Session` was one type too many: fixed, `Session::open` is the one entry point.
- Session work tests sat in the language server crate: fixed, they are in `crates/archangel_engine/tests/work.rs`.

**performance**
- Watch mode had no work count: fixed, a one-save scenario with a fault control.
- Tsconfig, manifest and baseline changes had no stated cost or count: fixed, one resolution for all waiting events with the reason, and a zero-work baseline change, each with a fault control.
- Edit evaluations and publishing were not counted: fixed, rule evaluations and the published documents are asserted, with a fault control.
- Example resolutions and evaluations were not counted: fixed, with a fault control.
- The branch switch used one notification: fixed, five notifications.
- The cycle cost was Tarjan on an acyclic graph: fixed, stated as such, and the chosen search is measured before release.

**standalone**
- Fix tests in the rules crate: fixed, as above.
- No fail point for a config change: fixed, `engine::full_check_per_config`.
- `config-file-invalid` read as archangel's config and had no exit status: fixed, renamed `resolution-file-invalid`, exit status 1.
- The glossary's pattern did not cover entry files or specifiers: fixed in the specs' wording ("specifiers, which may end in `*`").
- Exit status 2 left out the baseline in In short: fixed.
- Two file counts: fixed, 9,824 in both files.
- The 400-rule self-test was unexplained: fixed in decision 13.
- The example threshold was vague: fixed, then made moot by measurement (0.53 ms per example).
- No owner for manifest dependency names: fixed, `ManifestDependencies` in the graph.
- A baseline-bytes outcome tested in the pure crate: fixed, a binary test.
- Test kinds undefined in tasks.md: fixed in the group 1 introduction.

**executable-spec**
- Fix tests in the rules crate: fixed, as above.
- No scenario for a config that does not parse or has no default export: fixed, plus an enum and a `console.log` scenario.
- No scenario for a path argument that is not a source file: fixed, exit status 2 for check, explain and why.
- The built-in module test had no task: fixed in task 1.1.
- The invalid tsconfig scenario had no exit status: fixed.
- The baseline-bytes outcome: fixed, as above.
- The 4 GiB file had no test: fixed, a sparse file and a size check before reading.
- Ties between shortest cycles: fixed, path order decides, and the scenario shows the cycle.
- Two imports of one specifier: fixed.
- Publishing only changed files: fixed.
- Closing an unsaved document: fixed.
- Watch baseline changes and saves during a check: fixed.
- Provenance had no scenario: fixed.

**limits**
- Parent `.gitignore` files applied without `.git`: fixed, the repository is the nearest folder with `.git`, `parents` is off, with a scenario.
- A replaced file could close a cycle through a repository file: fixed, the search covers the repository files that reach a replaced file, with a scenario.
- `.js` files were parsed without JSX: fixed, with a scenario.
- Flow files and parser recursion: fixed, a Flow message and a 16 MB parse stack, in Limits.
- More per-file resolver errors and raw IO text: fixed, mapped to one diagnostic per file and plain reasons.
- Yarn Plug'n'Play: fixed, the resolver's support is turned on, with a scenario.
- Node type stripping and config output on standard output: fixed, named constructs and a separate channel, with scenarios.
- `.ignore` and Git's local exclude had no scenario: fixed.

**innovation-tokens** (passed; minor findings taken)
- The capture precedent was wrong: fixed, Sheriff, with braces kept as URI templates use them and a message for `{a,b}`.
- Node's type stripping, not a transform, is the mechanism: fixed in decision 10 and Decisions that need you 2.
- `--in-diff` takes a diff file: fixed.
- The fail crate's process-wide switches: fixed, a scenario lock.
- Simulated sessions named no library: fixed, `proptest-state-machine`.
- A custom benchmark runner: fixed, `hyperfine`.
- Stored registry tokens: fixed, trusted publishing on both registries.

**cold-read** (passed; minor findings taken)
- Exceptions and the baseline used before definition, no config sketch, no summary definition, no path arguments, pointers to task numbers in reader sections, and the brief's hedge on per-edit work: fixed.

The spike's per-example measurement, blocked in round 1 by the machine's memory limit, has since run: a
fresh resolver costs 0.53 ms per example on 200 real files, so decision 13 no longer has a fallback.

### Final check

The final pass re-checked each reviewer whose blocking finding revision 2 answered, plus the cold read.
No revision follows it. standalone passed. Five reviewers still had a blocking finding:

- **cold-read**: the plain-words definition of layers in In short reads as if a listed element may import
  nothing outside the list; the spec checks only imports between listed elements.
- **limits**: watch mode and the language server miss changes to non-source files that imports resolve
  to, to tsconfigs reached through `extends` under other names, to `.pnp.cjs` and to `.gitignore`, and
  the server lacks watch mode's lost-event and polling fallback; a cycle group shows one cycle and has no
  group identity, so a baselined group can grow without a new violation; braces with a comma are refused
  although they can only mean alternatives.
- **performance**: a save by temporary file and rename looks like a file event and resolves every import
  again; it should count as a text change when the path existed before and still exists.
- **executable-spec** and **abstractions**: five language-server scenarios still assert engine work counts
  from the language server crate's tests, and two engine fail points are controlled only there.

Minor findings from the final pass are in the reviewers' verdicts and are not yet applied: the
`--config` folder, layer wording in the spec, the nested-input fuzz target, resolver error variants,
`.git` files in worktrees, all parse errors in one diagnostic, the second argument of `why`, test
placement for the equal-configs and package-element scenarios, files visited by an example's cycle
search, and disk changes to open documents.

### Round 3

The person asked for a third revision after the final check, followed by a re-check of the five
reviewers that still blocked. Every blocking and minor finding of the final check was fixed:

- **cold-read**: layers now says that only imports between listed elements are checked; the sample config
  shows an entry rule; "symbolic links"; re-resolving after a file is created is raised as Decisions that
  need you 11, and in the conflicts row.
- **limits**: the engine owns one watcher for watch mode and the language server; the resolver's record of
  every file it read or found decides what is watched, so non-source files, extended tsconfigs and
  Plug'n'Play files are covered; a `.gitignore` change walks again; cycle groups list every file and are
  identified by their sorted files, with a baseline scenario; braces with a comma are alternatives; every
  resolver error has a plain message through an exhaustive match; a `.git` file marks a worktree and
  nested repositories are listed; parse errors are all reported; the second argument of `why` may be any
  file in the graph.
- **performance**: event batches are netted against known paths, so a save by rename is a text change,
  with scenarios in watch mode and the engine; disk changes to open documents are ignored; the example
  cycle search counts the files it visits and states its cost, with a fault control.
- **executable-spec** and **abstractions**: the five language-server scenarios now assert published
  diagnostics in the server's tests and work counts in the engine's; example work counts are in the
  engine; the equal-configs and package-element scenarios assert what their crates can see.
- **standalone** (minor): `--config` makes the config file's folder the root; "layer" is no longer used for
  one element; the nesting guard is a unit test.

### Round 3 re-check and revision 4

cold-read and abstractions passed the re-check; limits, performance and executable-spec each found new
blocking findings, all fixed in revision 4:

- **limits**: a leftover sentence still refused braces with a comma (fixed); the resolver does not report
  the tsconfigs, their `extends` chains or `.pnp.cjs` it reads, so archangel now finds each tsconfig
  itself and records those files (fixed); `NODE_PATH` and `OXC_RESOLVER_YARN_PNP` made results
  machine-dependent and forced Plug'n'Play broke ordinary repositories (fixed: both off, Plug'n'Play
  only with `.pnp.cjs`, resolver folder set to the repository); the resolver silently skips an
  unreadable tsconfig (fixed: a diagnostic); a config ignore-list change needs a new walk (fixed);
  `Builtin` was missing from the exhaustive match, the watch requirement under-listed its inputs, and
  manifests outside any resolution were unwatched (fixed).
- **performance**: a save to an imported CSS file re-resolved everything (fixed: files whose existence
  decides resolutions are netted, and only files whose contents decide them trigger a resolution, with a
  scenario and a fault control); the rename-save count had no fault control (fixed); context recording
  and the size of the recorded sets are unmeasured (now listed for the benchmark); `--stats` lacked the
  cycle-visit count (fixed).
- **executable-spec**: the lost-events and no-watches tests could not create their conditions (fixed: two
  fail points that simulate the operating system); the baseline-change scenario had no server test, and
  the polling message had no scenario (fixed).
- **abstractions** (minor): `ResolutionInputs` now lives on the graph; `events.rs` routes the config and
  the baseline; the simulated sessions live in the engine; the abstraction list is complete.
- **cold-read** (minor): the proposal names the two departures from the brief; step 5 of In short states
  the cost of each kind of change.

### Confirmation and revision 5

performance passed the confirmation pass. limits and executable-spec each found blocking findings,
fixed in revision 5 without a further review:

- **limits**: tsconfigs named by project `references`, and their `extends` chains, were not recorded,
  so a solution-style monorepo's `tsconfig.app.json` changes went unseen (fixed, with a scenario);
  `ResolveError` is non-exhaustive, so its match has a wildcard arm with a plain message and a test
  lists the known variants (fixed); tsconfig discovery is per importing file with
  `TsconfigDiscovery::Auto`, and archangel resolves `extends` chains itself and compares them with what
  the resolver loaded (fixed); the recorded sets only grow between full resolutions, because the
  resolver reports a manifest only once (fixed, with a scenario).
- **executable-spec**: the language server and the binary had no `failpoints` feature and the checks
  never ran with it, so every fault control could be skipped while CI stayed green (fixed: every crate
  has the feature and `scripts/check` runs the tests with it); the built-in module scenario lacked its
  element test (fixed); the test-support crate had no place in the dependency order (fixed).
- **performance** (minor): config changes now also assert zero resolutions; `.gitignore` changes and
  unread manifests have work counts and a fault control; the polling interval and its per-poll cost are
  stated.

The simulated editor sessions now edit tsconfigs with `extends` and `references`, manifests,
`.gitignore` and the config at random, and compare every step with a full check. So a gap in how
archangel tracks what resolutions depend on, the class behind most late findings, fails a test.

### Found while building

- The scenario "A path alias that loops" could not be built: the oxc resolver reaches its 64-step
  loop limit only through alias fields such as `browser`, which archangel does not turn on, and a
  tsconfig path alias cannot chain. The scenario now tests the mapping of the resolver's loop error to
  plain words directly.
- The Node loader returns its result through a temporary file, not a separate file descriptor,
  because the descriptor needs `unsafe` code, which the workspace forbids. design.md says so.
- The cycle search copies each no-cycles rule's edges into a petgraph graph instead of implementing
  petgraph's traits on `ImportGraph`, because each rule filters by import kind. The copy is one pass
  over the imports per no-cycles rule. design.md says so; the cost is measured with the benchmarks.
- `Architecture::compile` takes a validated `Config` and cannot fail: validation in `archangel_config`
  already reports every problem, so a second error path would be one that no input reaches.
- An accepted example proves something when its rule's definition looks at one of its imports, not
  when the rule table lists the rule for it: the table lists only the rules an import can break, so
  an allowed layers import never appears there. Layers looks at imports between two listed elements,
  isolate between two files of its element, entry and only at imports of their element, and forbid at
  imports from its `from` elements, each limited by the rule's `imports` option.
- An example fails when its files give any diagnostic that is not a rule violation, such as
  `parse-failed` or `import-unresolved`, because the rules cannot judge an example they cannot read.
  The spec states this for syntax errors; the same reason covers unresolved imports.
- The resolution inputs record every file the resolver reads, through the overlay file system,
  instead of resolving each tsconfig's `extends` chain a second time with the resolver's rules.
  The resolver reads every extended config through that file system, so the record is exact, and a
  later resolver that reads a new kind of file is covered too. The simulated sessions found the gap:
  an edit to an extended `tsconfig.base.json` left resolutions stale.
- The simulated sessions also found that a walk which brings back an open document's file read
  the text on disk. The session now keeps each open document's text and gives it back after a walk.
- When the system has no file watches left, watch mode polls with notify's poll watcher, which
  compares modification times in whole seconds. A save in the same second as the last poll would
  never show, so the poller compares file contents, which reads every watched file each second. That
  is correct but its cost grows with the repository; a poller over the walked files and the
  resolution inputs alone is the next step if it is too slow. The watch tests found the gap.
- The graph keeps a hash of each file's text, and a reported change that leaves the text as it was
  costs a read and no parse. macOS reports writes made just before a watcher started, and editors
  rewrite files they did not change; the watch tests met the first.
- Yarn's generated `.pnp.cjs` and `.pnp.loader.mjs` have source extensions but are not source: the
  walk leaves them out, and resolution records `.pnp.cjs` as an input. A package that Plug'n'Play
  does not list is an `import-unresolved` with its own reason, not a broken resolution file. The
  language server tests found all three.
- The language server answers `archangel/stats` with the work counts so far, which is how its tests
  check the spec's work counts through the protocol, and how an editor extension can show them.
- A document the editor opened at a path that does not exist on disk yet is not checked until it is
  saved: the graph holds the files the walk finds.
- The release workflow is written and passes actionlint and zizmor, but it has not run: it needs
  the GitHub repository, the npm organisation `archangel` and trusted publishing set up on both
  registries. Before the first release, the crates also need `publish = true` and the repository
  URL, which does not exist yet. Task 12.2 stays open until a dry run on a tag passes.
- The config fuzz target found a crash in its first minute: a config whose `elements` field is
  misspelled made the default name of each rule read an element that does not exist. A rule's
  default name is now made only after the rule parsed without a problem, a test in `validate.rs`
  covers the case, and the input is in the corpus.
- The committed seeds, in `fuzz/seeds/`, are the hand-written inputs and that input, not the 8,000
  files that a minute of fuzzing grows; the scheduled job keeps the corpus it grows in the Actions
  cache.
- Locally, without nightly Rust, the targets run with `RUSTC_BOOTSTRAP=1 cargo fuzz run <target> -s
  none`. The scheduled job uses nightly and the address sanitizer, as cargo-fuzz expects.
- `docs/editors.md` gives a Neovim config, which a headless Neovim ran against a fixture and showed
  the diagnostic. VS Code and Zed start a language server only through an extension, and this
  release ships none, so the page says so and points to watch mode; an extension for each is the
  next step.

### Found by the dogfood

- The element matcher compiled every file pattern into one regex set and, when the set was too
  large to compile, used an empty one: every file was in no element, every rule passed, and the check
  reported success. A config generated from the dogfood's 257 dependency-cruiser rules has 9,700
  exact-path patterns and hit it at once. Exact paths now go in a map, globs in a set with a larger
  limit, and a set that still does not compile becomes each glob tried alone, which is slower and
  never wrong. Tests in `pattern.rs` cover 10,000 exact paths and 3,000 globs.
- A package element of Node built-ins was `element-empty`, because no manifest lists `node:fs`.
- The rule table tried every rule on every pair of elements: about a second for 360 elements and
  209 rules. It now places each rule only in the pairs it can apply to.
- Design: a file is in one element, and the dogfood has two dimensions on the same files, layer and
  role. A config written from data can group files by the rules they fall under, which works, but a
  person writing a config by hand cannot. Letting a file be in more than one element, or adding tags,
  is the next design question.
- Design: `element-empty` is right for a misspelt pattern, and wrong for a preventive rule against a
  package nobody has installed yet, such as `pg` in a codebase that must never use it. The dogfood's
  config names only installed packages, which is exact because it runs on every check; a hand-written
  config has no such way out.


### The rule language, presets and plugins

After the dogfood, the person asked for the first release to cover what ArchUnit checks
without a type checker, with presets and plugins. Four research reports came first: the ArchUnit family
of tools and dependency-cruiser, Nx and Sheriff; the structure and naming lint tools and Steiger;
plugin hosting in Rust-cored JavaScript tools; and an inventory of the dogfood's 32 kinds of architecture
rule. The innovation-tokens check found overlapping selectors and the plugin process conventional, and
replaced two proposed mechanisms with plain ones: one protocol for a check and an editor session, and a
full rerun of affected conditions instead of tracking what each read.

Round 1 ran six reviewers independently on the draft: cold read, abstractions, performance, executable
spec, limits and stands-alone. Root cause did not apply, and innovation tokens had already run. Every
reviewer had blocking findings. The revision:

- removed the forbid, only and noCycles kinds, `mayNotBeImportedBy`, `mustBeIn`, package selectors and
  the package groups, each a second way to say something the change already says;
- kept selectors to files, moved `dev` and `undeclared` to the `dependency` option of an import, and
  wrote built-in modules as `node:*`;
- prefixed everything a preset declares, gave the merge and the Node process one owner each, and
  exempted presets and `mustNotExist` from the empty-selector error;
- made structure rules see every visited file, defined a file's name and full name, followed
  `export *` for `mustExport`, and described the transitive walk as one backward search;
- removed the plugin deadline in favour of a visible wait;
- named a defect in the built code: every edit classifies every file and rebuilds every importer list,
  which a new work count and fault control now hold.

The final check was a fresh cold read and a check of each blocking finding against the revised files.
It found accepted examples that could not pass for non-import rules, package sides that missed packages
in a declared package element, exceptions that cannot fit violations without an import, the Next.js
preset refusing server actions, and stale text in the configuration spec, the glossary and two rows of
the code table. Revision 2 fixed each, added `stopAt` for transitive chains, and passed
`openspec validate --strict`.
