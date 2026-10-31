# Design: the first release of archangel

## In short

A TypeScript team that writes down its architecture today needs several tools to enforce it: one
for import rules, one or two lint plugins for folders and boundaries, and scripts for the rest. Each
tool reads the whole repository again. On one monorepo of 9,824 source files, one import-rule run
takes 24 seconds and one lint run takes about 20 seconds for a single file. So the rules run in CI,
minutes after the import was written, and a rule that was configured wrong passes without a word.

With this release, the team writes one config that names the kinds of files in the repository and
the rules between them. On that monorepo, archangel checks the 400 rules that dependency-cruiser,
two lint plugins and Steiger enforce there in 1.9 seconds, where those tools take about 68 seconds
between them. Most of that time is classifying files, so the Cost section measures each stage. In
the editor, archangel shows the diagnostics for an edit 13 milliseconds after it.
Every result says what is wrong, what to do instead and where to read more, and every rule proves in
seconds that it rejects what it is meant to reject.

**How it works:**

1. The developer writes a config. It declares elements, which are named kinds of files such as
   "feature" or "entity", by path patterns, and it declares rules between them.
2. Archangel finds every source file that the repository does not ignore, reads each one once, and
   finds its imports.
3. Archangel resolves each import to the file or package it points to, with the same rules as the
   TypeScript compiler, and builds a graph of imports in memory.
4. Archangel puts each file into its element and checks each import against each rule. It removes
   the violations that an exception (a comment above one import that permits it for one rule, with
   a reason) or the baseline (a file of old violations that can only shrink) permits, and reports
   the rest.
5. In the editor and in watch mode, archangel keeps the graph in memory. After an edit to a file's
   text it parses only that file, checks its imports, and searches for cycles again. Creating,
   deleting or renaming a file, or changing a tsconfig or a package manifest, resolves every import
   again, once for all the changes that are waiting.

Archangel is meant to be what ArchUnit is for Java: the one place a TypeScript team writes every rule
about where code lives, what it is called, what it may import and what it exports, checked fast
enough for every keystroke. Every rule is one sentence: *these files* *should* *do this*. "These
files" is a **selector**, a named set of files that may overlap with other selectors, such as "test
files" or "files whose name ends in `.view.tsx`". "Should do this" is one **condition**: may not
import, may only import, may only be imported by, must be named, must contain, must not exist, must
export, may not export, must only re-export, and no cycles. An import condition can follow imports
transitively, look at the names an import brings in, at how its specifier is written, or at whether
the package is declared in the nearest `package.json`.

Teams share rules as **presets**: functions that return elements, selectors, rules and their examples,
used with `extends`. Archangel ships two. One is for Feature-Sliced Design, a published convention
that splits a front end into layers, slices and segments, and replaces Steiger, its official linter.
The other keeps server-only code out of Next.js client components. A team can also write its own
conditions in TypeScript, in a **plugin**, for what the built-in conditions cannot say. A plugin's
condition receives the graph of files, imports and exports, not a syntax tree, because it checks
architecture, not code style.

Three rule kinds stay as shorthands for patterns that take several sentences to say. **Layers** lists
elements in order; an import from one listed element into an earlier listed element is a violation,
and the rule does not look at imports to or from elements outside its list. **Isolate** keeps the
instances of one element, such as two features, from importing each other. **Entry** lets code
outside an instance import it only through its entry files, such as `index.ts`; a sibling instance
counts as outside.

A small config looks like this:

```ts
export default defineConfig({
  elements: {
    feature: 'src/features/{feature}/**',
    shared: 'src/shared/**',
    react: { packages: ['react', 'react-dom'] },
  },
  selectors: {
    view: '**/*.view.tsx',
    test: ['**/*.test.ts', '**/*.test.tsx'],
  },
  rules: ({ layers, isolate, entry, rule }) => [
    layers(['feature', 'shared'], { reason: 'Shared code must not know about features' }),
    isolate('feature'),
    entry('feature', 'index.ts'),
    rule({ name: 'views-stay-thin', files: 'view', mayOnlyImport: ['shared', 'react', '**/*.css'] }),
    rule({ name: 'views-are-named-for-their-file', files: 'view', mustExport: '{FileName}' }),
    rule({ name: 'no-tests-in-production', files: { not: 'test' }, mayNotImport: 'test' }),
    rule({ name: 'folders-are-kebab-case', folders: 'src/**', mustBeNamed: 'kebab-case' }),
    rule({ name: 'no-cycles', files: 'src/**', noCycles: true }),
  ],
})
```

A team on Feature-Sliced Design writes `extends: [fsd({ root: 'src' })]` and gets Steiger's rules,
each with the examples that prove it.

The developer runs `archangel check` in CI, `archangel test` to prove the rules, `archangel explain`
on a file to see its element, its rules and its imports, `archangel why` to see the shortest chain of
imports from one file to another, `archangel watch` beside a coding agent, `archangel baseline add`
and `archangel baseline prune` to manage old violations, and `archangel lsp`, which the editor starts.
Given file paths, `archangel check` prints only their diagnostics but still checks the whole
repository, because a cycle or an empty element depends on every file. `--format json` gives JSON
output and `--config` names a config file. Old violations live in `archangel.baseline.json` beside
the config. Every command exits with 0 when all is well, 1 when it found something, and 2 when the
config, the baseline file or the command line stops it.

Every check ends with a summary: the files and imports it checked, what it did not follow (symbolic links,
nested repositories and computed specifiers), and, for each rule, the number of exceptions and of violations that the baseline
matched, so that CI can hold those numbers down.

One idea carries the rest of the design: **the graph of imports does not depend on the config.**
Which files import which is a fact about the code. Which imports are allowed is a fact about the
config. Archangel keeps the two apart, so a change to the config checks the rules again without a
second read of any file, and the part that applies the rules can be tested with small graphs written
by hand.

### Decisions that need you

1. **Rules are written as a function that receives the rule builders.** The config looks like
   `rules: ({ layers, isolate, entry }) => [layers([...]), ...]`. The function is what lets the
   TypeScript types know the element names. So a misspelt element, or an isolate rule on an element
   whose pattern has no capture (a named part of the path, such as `{feature}`), is a red line in the
   editor before archangel runs. The other option, imported builder functions, reads a little more
   like other tools but cannot catch either mistake, because an imported function does not know which
   elements this config declares.

2. **A TypeScript config needs Node 22.18 or later; a JSON config needs nothing.** Archangel runs
   Node once to load a TypeScript config, because Node has run TypeScript files without a build step
   since that version. When Node is missing or older, archangel says which version it found, which it
   needs, and that a JSON config needs none. Node only strips types: a config cannot use `enum`,
   `namespace` or parameter properties, must write `.ts` on its relative imports, and cannot use
   tsconfig path aliases. When a config breaks one of these, archangel names the construct and what
   to write instead. The other option is to bundle a TypeScript transform, as Vite and ESLint do, or a
   JavaScript engine inside archangel. Either adds weight to every install to lift limits that a
   config file rarely meets.

3. **Type imports count for every rule except no cycles and transitive chains.** An `import type` from the UI into the
   database layer still couples the two, so layers, isolate, entry and the import conditions check it unless
   the rule says `imports: 'runtime'`. Type imports disappear when TypeScript compiles, so a cycle made only
   of type imports never happens when the code runs, and the no-cycles condition and transitive rules ignore them unless
   they say `imports: 'all'`. The other option, runtime imports for every rule, would let the most common kind of
   coupling through.

4. **An import that archangel cannot resolve fails the check, unless the config names it with a
   reason.** On the measured monorepo, exactly one of 52,196 imports did not resolve: a virtual module
   that the Astro framework makes at build time. The team writes
   `allowUnresolved: { 'astro:*': 'Astro makes these at build time' }`. The other option is a warning.
   It is visible, but CI passes with it and people learn to ignore it, so an import that the graph does
   not contain could hide a violation behind a green check. The brief asks for no plausible wrong
   answers, and a pass that leaves out one import is one.

5. **The test command fails for a rule that has no rejected example.** A rule without one has no proof
   that it can find a violation, which is the failure this tool exists to prevent. This makes a new
   rule slightly more work to add: one small example of what it forbids.

6. **A fixed violation in the baseline fails the check until the developer prunes the baseline.** The
   brief says that archangel removes a baseline entry when its violation is fixed. If a check rewrote
   the baseline, every check in CI would change a file in the repository, and two branches would fight
   over the same lines. So a check only reports the stale entry, and one command,
   `archangel baseline prune`, removes it. ESLint's bulk suppressions work the same way. This differs
   from the brief's words but keeps its intent: the baseline can only shrink, and nothing adds to it
   during a check.

7. **An exception is a comment directly above the import: `// archangel-allow <rule>: <reason>`.** It
   names exactly one rule and must give a reason. Several exception comments on consecutive lines all
   apply to the import below them, so one import can carry exceptions for two rules. An exception that
   no longer permits anything is an error. The form is Biome's suppression comment, which also needs
   a reason, so it reads as familiar.

8. **A file with a syntax error fails the check and contributes no imports.** A parser that recovers
   from an error can invent or lose imports, which would be a plausible wrong answer. The developer sees
   one clear diagnostic for the file instead.

9. **Two rules with the same name are refused, and the developer names them.** A rule without a name
   gets one from its kind or condition and its sides, such as `entry(entity)` or
   `mayNotImport(entity, react)`. Two unnamed rules on the same
   sides then share a name, even if their options differ, and archangel refuses the config with a
   message that says to name each rule. The other option is a longer generated name that includes the
   options. That would be unique more often, but a reason or an option change would rename the rule,
   and every exception and baseline entry that uses the old name would break without a word.

10. **Archangel checks only the TypeScript in its own repository; Cargo checks the Rust.** The brief
    asks that archangel's repository checks its own architecture with archangel. Archangel reads
    TypeScript, and its core is Rust. So the boundaries between the Rust crates enforce the core's
    layers (Cargo refuses any dependency that a crate does not declare, and any cycle between crates),
    and archangel checks the npm launcher and its types. The other option is to leave the Rust layering
    to review alone.

11. **Creating, deleting or renaming a file resolves every import again.** That takes 37 to 52 ms on the
    measured monorepo and grows with the repository. The brief permits growth only where nothing smaller
    is correct, and something smaller exists: the resolver can record every path that a failed
    resolution tried, so archangel could resolve again only the imports that tried the new path. That
    is how webpack and the TypeScript language service work. It costs memory for every import's tried
    paths, which nobody has measured yet, and it is a second mechanism to keep correct. So this release
    does the simple, measured thing, and the precise one waits until a repository makes it slow.

12. **The platform packages on npm need the scope `@archangel`.** Each platform's binary goes in its
    own package, such as `@archangel/darwin-arm64`. Someone has to register the npm organisation
    "archangel" before the first release. If the name is taken, the packages can be unscoped, such as
    `archangel-darwin-arm64`, which works the same way and reads slightly worse in a lockfile.

13. **The scope is everything ArchUnit checks that needs no type checker.** That is: what files may
    import and be imported by, directly or transitively; which names an import brings in; how the
    specifier is written; whether an imported package is declared; where files live and what files
    and folders are called; which files must exist and which must not; what a file exports; and
    cycles. It leaves out four things, each with the tool that keeps it:
    - Rules that need the type checker, such as "only the repository calls `save`" on a method of an
      object, or a class that inherits an interface through another class. ESLint with type
      information keeps these.
    - Rules about code inside a file: which globals, properties or syntax a file may use, such as
      "only `shared/api` calls `fetch`" or "only the config module reads `process.env`", and the kinds
      and order of declarations that a file does not export. ESLint's restricted-syntax rules keep
      these, and the brief's later structural pattern rules are their home in archangel.
    - Unused files and exports, which are dead code rather than architecture. Knip keeps these.
    - Checking the code against a diagram, generated documentation, and importing other tools'
      configs. The brief lists these for later releases.

    The dogfood is a private monorepo of 9,824 files that the Cost section measures, and the first
    repository archangel ran on. On it, this scope moves dependency-cruiser,
    eslint-plugin-boundaries, eslint-plugin-check-file, the folder rules of
    eslint-plugin-project-structure, the architecture rules of import-x, and Steiger into archangel:
    about 25 of the 32 kinds of architecture rule it has. Its 26 blocks of global, property and syntax
    rules stay in ESLint.

14. **A rule is one sentence, written as an object: which files, and one condition.** For example
    `rule({ files: 'view', mayOnlyImport: ['shared', 'react'] })`. Each condition is one field, so a
    rule reads aloud and a diagnostic names exactly what was broken. Builders keep the TypeScript
    types that catch a misspelt selector before archangel runs (Decisions that need you, 1). The other
    option is ArchUnit's fluent chain, `files('view').should().onlyImport(...)`. It reads well, but a
    chain is harder to type-check for one condition per rule, it hides that a rule is data, and
    dependency-cruiser, eslint-plugin-boundaries and Sheriff, which TypeScript teams already know,
    all write rules as objects.

15. **Selectors overlap; elements do not.** An element is still the one place a file belongs, with
    its captures, which isolate, entry and `archangel explain` need. A selector is a set of files
    that any number of others can overlap, like a tag in Nx or Sheriff. Without overlap, the
    dogfood needed a 373-line translator that made one generated element for every
    combination of rules a file fell under, and it caused two bugs on the way: a pattern set too large
    for the regular expression engine, which then matched nothing, and a one-second compile of 360
    elements. With overlap, those rules are written as they are meant.

16. **Forbid, only and the no-cycles kind become conditions.** `forbid(a, b)` was exactly
    `rule({ files: a, mayNotImport: b })`, `only(a, [b])` was `rule({ files: a, mayOnlyBeImportedBy: b })`,
    and `noCycles()` is `rule({ files, noCycles: true })`. Keeping both spellings would give every
    reader two ways to say one thing, and on the dogfood `only('db', ['api'])` was read as "db may only
    import api", the opposite of its meaning. Layers, isolate and entry stay, because each stands for
    more than one condition. Nothing is published, so no one migrates.

17. **An empty selector is an error only where emptiness means a mistake.** An element or selector the
    config declares, or an inline pattern in its rules, that matches nothing is an error, as an empty
    element already is, because a misspelt pattern must not pass. Two cases are exempt, because
    nothing is their passing state: the files of a `mustNotExist` rule, and anything a preset declares.
    A preset declares every layer of its architecture, and most repositories use only some, so the
    preset's empty elements and selectors are listed in the summary instead; a preset whose elements
    together match no file at all is an error, which is what a wrong `root` looks like. An inline package
side of a `mayNotImport` rule may also name packages nobody has installed, because forbidding a package
before anyone adds it is that rule's passing state. An inline package side of `mayOnlyImport` or
`mayOnlyBeImportedBy` is not exempt: it permits the package, so a misspelt name there refuses imports the
team meant to permit, and it is checked as a package element is. A team's own directive selector is not exempt:
declaring it says the team has such files, and a misspelt directive must not pass.

18. **Presets and plugins need Node, like a TypeScript config.** A preset is a function, and a
    plugin's condition is code, so archangel runs them in the Node process that loads the config. A
    JSON config can use every built-in condition but no preset or plugin. The other option, a
    JavaScript engine inside archangel, adds weight to every install (Decisions that need you, 2).

19. **Plugin rules run in the editor too, a moment after archangel's own, and nothing times out.** In
    watch mode and the language server, the Node process stays alive with the plugins loaded. After
    each change, archangel publishes its own diagnostics at once and the plugin diagnostics when Node
    answers; while it waits, the editor shows that the plugin is running. A plugin that throws, and a
    Node process that stops, each give one diagnostic that says so, so a plugin rule never passes
    because its code did not run. There is no deadline: like a config that is slow to load, a slow
    plugin keeps archangel waiting where the developer can see it, and a deadline would fail a slow
    but correct plugin on a large repository. The other option is plugin rules in `check` and CI only,
    which is how oxlint shipped its JavaScript plugins. It is simpler, but a developer would then learn
    about a broken rule from CI, which is the delay archangel exists to remove.

20. **Rules are on or off; there is no warning level.** A preset rule a team does not want is turned
    off by name: `off: ['fsd/insignificant-slice']`, and a name in `off` that matches no rule is an
    error, so a renamed rule never stops being off without a word. A violation the team accepts for
    now goes in the baseline, which only shrinks. ESLint, Steiger and dependency-cruiser all have
    warnings, and teams learn to ignore them, which is the silent pass this tool exists to prevent.

21. **The first release ships two presets: Feature-Sliced Design and the Next.js server and client
    boundary.** The brief names both among its examples of later work. The Feature-Sliced Design
    preset matches Steiger's recommended rules by default and all 21 of its rules with
    `fsd({ strict: true })`, and a differential test runs both tools on the same repositories. Seven
    of Steiger's rules are guesses about names, such as a layer name one typo away from a real one, or
    counts, such as a slice that only one other slice uses, so the preset writes those as plugin
    conditions. That makes the preset the first user of the plugin interface, so the interface is
    proven before anyone else writes a plugin.

22. **This makes the first release later.** Selectors, the new conditions, presets and plugins are a
    second large piece of work on top of the one already built: roughly as much again as the rules,
    examples and engine took. Everything built so far stays, except that forbid, only and noCycles
    become conditions and the rule table changes its key. The other option is to publish what exists
    as 0.1 and add this as 0.2. That would mean publishing a config format that this design changes,
    and every early user would then need to migrate.

## How it works

### Loading the config

Archangel looks for the config in the current folder and then in each parent folder, and uses the
first one it finds. All paths in archangel's output are relative to that folder.

A JSON config is read directly. For a TypeScript or JavaScript config, archangel first asks Node for
its version, and then starts Node once with a small loader that archangel carries inside itself. The
loader imports the config, calls the rules function with the rule builders, and writes the result as
JSON to a separate channel that config code does not write to, so a `console.log` in the config cannot
break the load. From then on the config is data. No config code runs per file or per import, and the builders
cannot disagree with the binary, because the binary carries them. The binary does this itself, and
not the npm launcher, so that the same path serves an install from crates.io and an editor that
starts the binary directly, and so that watch mode can load the config again after a change.

Archangel then validates the config and reports every problem it finds, not only the first: fields it
does not know, values of the wrong type, patterns that cannot mean what they say, rules that name
undeclared elements or selectors, isolate, `instance` and `mustContain` on elements without a capture,
plugin options that break their schema, and two rules with the same name. A rule without a name gets
one from its kind or condition and its sides, written as in the config, such as `entry(entity)` or
`mustBeNamed(folders: src/**)`, because the name appears in every diagnostic, exception and baseline
entry.

### Finding and reading source files

Archangel walks the folders below the config and reads every JavaScript and TypeScript file,
declaration files and files in hidden folders included. It records the path of every other file it
visits without reading it, for the rules about names and structure. The repository is the nearest folder at or
above the config that contains `.git`, as a folder or, in a Git worktree, as a file, or the config folder
when there is none. Archangel obeys the
`.gitignore` files from that folder down, and the config's ignore list, which uses the same syntax
because it extends them. It uses no ignore rules from outside the repository: not a global Git
ignore file, not Git's local exclude file, not `.ignore` files, and not a `.gitignore` in a folder
above the repository. Those differ between machines, and the same commit would then give different
results on two laptops or in a Docker build that leaves out `.git`. A nested repository below the config
folder, such as a submodule, still obeys the outer `.gitignore` files, which Git would not apply there;
the summary lists each nested repository so the difference is visible.

Archangel does not follow symbolic links, to folders or to files, because the same file would then
appear at two paths and every rule would see it twice. Each link it does not follow appears in the
summary. A folder or file that it cannot open, or a file that is not UTF-8 or UTF-16 text, is a
diagnostic that fails the check.

Archangel reads the files in parallel and then sorts them by path, so that the result does not depend
on which thread finished first.

### Finding imports

Archangel parses each file with the oxc parser, with JSX permitted in `.js` files too, because many
React projects write JSX there. Most imports come from the parser's record of the file's imports and
exports: import statements, export-from statements, and dynamic imports with a
literal specifier. Three forms are not in that record: require calls, TypeScript's import-equals
declarations, and import types such as `import('./store').Store`. For those, archangel walks the
syntax tree, but only in files whose text contains `require` or `import(`, so most files skip the
walk. Triple-slash references sit in comments at the top of a file, and archangel reads them there. A file
with syntax errors gives one diagnostic that lists every error the parser found, so the developer can fix
them all in one pass.

An import is a type import when its statement says `import type`, or when every name it imports says
`type`. Import types, triple-slash references and every import in a declaration file are type
imports. A dynamic import or require call whose specifier is computed, such as a template string with
a variable, cannot be followed; archangel counts each one in the summary and lists it in
`archangel explain`, so the developer can see what the graph does not contain.

From each file, archangel keeps its imports with their lines and specifiers, the names that each
import statement imports, the names that the file exports, and the text of any comment that contains
`archangel-allow`. For the export conditions and directive selectors it also keeps three small facts
that the parse already has: which exports are a default export, an `export *` or a re-export of
another file; whether the file has any top-level statement other than imports and re-exports; and the
directive of its first statement, such as `'use client'`. It drops the syntax tree and the rest of the
text. Import names serve fixes and the `names` option of import rules; export names serve fixes and
the export conditions.

### Resolving imports

Archangel resolves each import with the oxc resolver, the same resolver that the Rolldown bundler
uses. For each importing file, it finds the nearest tsconfig and follows its `extends`, base URL and
path aliases. It reads `exports` and `imports` in package manifests, finds the TypeScript file behind
a `.js` specifier, and follows symbolic links to the real file. So a workspace package that a package
manager links into `node_modules` resolves to its source in the repository.

A resolution is one of three things: a repository file, a package, or nothing. A file inside
`node_modules` becomes its package, and a Node built-in such as `fs` becomes the package `node:fs`. A
repository file does not have to be a source file: an import of a CSS file resolves to the CSS file,
so rules can cover it.

Archangel finds the tsconfig of each folder itself and hands it to the resolver, because the resolver
does not report which tsconfigs it read. While resolving, archangel keeps two sets of files, which
together are the files that the resolutions depend on. The first holds the files whose contents
decide resolutions: each tsconfig, every tsconfig that its `references` name, every file in the
`extends` chain of each of those, every place where a tsconfig was looked for, the package manifests the resolver read, and Yarn's Plug'n'Play file. The
second holds the files whose existence decides resolutions: every file that an import points to and
every path that the resolver tried and did not find. Both sets only grow between full resolutions,
because the resolver reports a manifest only the first time it reads it; a full resolution rebuilds
them. A path that stays in a set after the import that needed it is gone can cost, at most, one full
resolution that was not needed. Watch mode and the language server watch both sets. A change to the contents of a file in the first set means every import is resolved again; a
change to the contents of a file in the second set, such as a saved CSS file, changes nothing.

The resolver can also read two environment variables: one adds folders to search for packages, and
one turns on Plug'n'Play. Archangel turns both off, because they differ between machines. It turns
Plug'n'Play on only when the repository has the Plug'n'Play file, and it resolves from the repository
folder, not from the folder the process started in. A tsconfig that exists but cannot be read is a
diagnostic that fails the check, because the resolver would otherwise skip it in silence and use a
parent's settings.

When an import has no resolution, archangel says why in plain words: no file at the path, no TypeScript
file behind a `.js` specifier, no package of that name, a package whose `exports` field does not offer
that path, or an alias or `exports` chain that loops or runs longer than the resolver follows. Every
kind of failure the resolver can report today has its own plain message; a kind added by a later
version of the resolver gets a plain message that names the specifier and says how to report it, never
the resolver's own text. The graph records the reason as a fact. The
config's `allowUnresolved` field can permit specifiers, which may end in `*`, each with a reason; the
rules apply it, and the summary counts the import instead. A problem in a tsconfig or package manifest,
such as a file that does not parse, an `extends` that names a package that is not installed, or an
`extends` chain that loops, gives one diagnostic on that file instead of one for each import that it
affects. Repositories that use Yarn's Plug'n'Play, which have no `node_modules` folder, resolve too,
because the resolver supports them.

### Putting files into elements

Each element has patterns. A pattern can capture part of a path, as `{feature}` does in
`src/features/{feature}/**`, and each set of capture values is one instance of the element. The
instance root is the matched path up to the first wildcard, with the captures filled in: for a file in
the "login" feature, the root is `src/features/login`. Entry rules give their entry files relative to
that root, in the same pattern language. Inline captures follow Sheriff, a module-boundary tool for
TypeScript. Archangel writes them in braces, as URI templates and OpenAPI paths do. Braces that contain a
comma list alternatives, as in globset and micromatch, and braces without a comma name a capture.

Each file goes into the first element, in declaration order, whose pattern matches it, so no file is
in two elements. Archangel tests all the element patterns against a path in one pass, so the cost of
classification grows with the number of files, not with the number of patterns. A file that no pattern
matches is in no element. Rules about elements do not apply to it, except a `mayOnlyBeImportedBy` rule, which refuses
imports from anything outside its list. Packages go into package elements by name, and a name can end
in `/*` to cover a whole scope.

After this step archangel knows, for every import, the element instance at each end.

### Selectors

A selector is a named set of files. Its definition is a pattern, a list of patterns, or a
combination: `{ all: [...] }`, `{ any: [...] }` and `{ not: ... }` of patterns, element names,
other selectors, `{ element: 'feature', capture: { feature: 'login' } }` for one instance,
`{ element: '*' }` for a file in any element (so `{ not: { element: '*' } }` is a file in none), and
`{ directive: 'use client' }`, which matches files whose first statement is that directive, in
either kind of quotes. Selectors select files only. Packages are what package elements already name,
by name or by scope, with `node:*` for Node's built-in modules, which the resolver always gives the
`node:` prefix, and `*` for every package, built-in modules included. A rule side can also write
`{ packages: [...] }` inline. A package is in one package element, the first that names it, as a file is
in one element; but an inline package side matches every package it names, whichever package element
it is in, as a selector matches files whatever their element.

A string on a rule side is first looked up as the name of a declared element or selector, a preset's
names included, such as `fsd/shared`. A string that names nothing is a path pattern when it contains
`/` or `*`, and otherwise a config error that lists the declared names closest to it. So `react` the
element and `react` the package cannot be confused: the package is `{ packages: ['react'] }`.

Selectors overlap, so a file can be in many. Archangel groups the files that are in the same element
and the same selectors, and checks an import by the pair of groups at its ends, so the cost of an
import does not grow with the number of selectors or rules. Each group is a **class**. Packages have
classes too: the packages in the same package element and the same inline package sides.

A selector is about files, not only source files. Archangel's walk already visits every file that
the repository does not ignore, and it records the path of each one, though it reads only source
files. So a folder that holds only images is still named and checked, and `mustContain: 'README.md'`
finds the file.

Most selectors depend only on the path. A directive selector also depends on the file's text, so an
edit can move a file to another class. When it does, archangel checks the imports into that file
again, from its list of importers, and the structure and export rules of that file, because a rule
about them may now give another answer. A file that does not parse has no directive, and its parse
diagnostic fails the check.

### Rules as sentences

A rule written with `rule()` has `files` (a selector, or `folders` for a rule about folders), one
condition, and the options every rule has: `name`, `reason`, `link`, `imports` (`all`, `runtime` or
`types`) and `examples`. The conditions are:

| Condition | It refuses |
| --- | --- |
| `mayNotImport: X` | an import from these files into X |
| `mayOnlyImport: X` | an import from these files into anything that is neither X nor these files |
| `mayOnlyBeImportedBy: X` | an import into these files from anything that is neither X nor these files, a file in no element included |
| `mustBeNamed: style or pattern` | a file or folder whose name does not match |
| `mustContain: pattern` | an instance, or a folder, without a file that matches |
| `mustNotExist: true` | every file in the selector, such as files in no element or in a reserved folder |
| `mustExport: name pattern` | a file that does not export a name that matches |
| `mayNotExport: 'default', 'star' or a name pattern` | a file that has such an export |
| `mustOnlyReExport: true` | a file that declares or runs anything of its own |
| `noCycles: true` | a group of files in the selector that all import each other |

An import into a non-source file, such as a CSS file, is an import of that file like any other, so a
`mayOnlyImport` rule lists what it permits, such as `'**/*.css'`. An import that does not resolve is
never a violation of a condition: it has its own diagnostic, which fails the check unless the config
permits it (Decisions that need you, 4).

The import conditions take six more options; `transitively` and `stopAt` apply to `mayNotImport`
only:

- `names` limits the rule to imports that bring in those names, such as
  `{ mayNotImport: { packages: ['node:process'] }, names: ['env'] }`. A default import brings in the
  name `default`. An import whose names archangel cannot see, a namespace import, an `export *`, a
  dynamic import or a `require`, counts as bringing in every name, so the rule never passes because a
  name was hidden. A side-effect import, `import 'x'`, brings in no names, so a rule with `names` does
  not apply to it. It sees only imports: the global `process.env` is code inside a file and stays
  with ESLint.
- `specifier` limits it to imports written a certain way: `relative` (starting with `./` or `../`),
  `parent` (starting with `../`), `alias` (through a tsconfig path or a package `imports` field) or
  `package`.
- `instance: 'same'` or `'different'` compares the captures at both ends. It needs `files` to name an
  element with a capture, and it is how isolate is said as a sentence.
- `dependency: 'dev'` or `'undeclared'` limits it to packages that the nearest `package.json` of the
  importing file lists only as development dependencies, or does not list at all. This is a fact about
  the import, not the package, because two files can have different nearest manifests. Node's
  built-in modules are never declared, and `dependency` never applies to them.
- `transitively: true`, on `mayNotImport` only, refuses a chain of imports of any length. A chain
  follows runtime imports unless the rule says `imports: 'all'`, because a type import disappears when
  TypeScript compiles, and it stops at packages, whose own imports archangel does not read. The
  diagnostic shows the shortest chain, and between chains of the same length the one whose files come
  first in path order. `imports: 'types'` is refused on a transitive rule, because a chain of type
  imports alone never runs.
- `stopAt: X` ends a chain at the files in X without following their imports. The Next.js preset
  stops at `'use server'` files, because Next.js turns an import of one from a client component into a
  network call, so the server code behind it never reaches the client.

`mustContain` needs `files` to name an element with a capture, and then looks in each instance, or
`folders`, and then looks in each folder. A `folders` pattern matches the folders below its fixed
part, so `folders: 'src/**'` checks every folder inside `src` and not `src` itself.

A file's **name** is its name up to the first dot, so `button.view.tsx` is `button`; its **full
name** is its name without the last extension, so `auth.test.ts` is `auth.test`. A leading dot starts
no name, so `.eslintrc.js` is named `eslintrc`. A folder's name is all of its last part. A style checks the
name, and a name is in a style when writing it in that style changes nothing, so `HTTPServer` is not
PascalCase, because the reference conversion writes it `HttpServer`. Styles are `kebab-case`, `camelCase`, `PascalCase`, `snake_case` and `SCREAMING_SNAKE_CASE`. A
name pattern is matched against the full name, and can use `{FileName}`, `{fileName}`, `{file-name}`,
`{file_name}` and `{FILE_NAME}`, the file's name in each style, and the same five spellings of
`{FolderName}` for the folder that holds it. So `mustExport: '{FileName}'` asks `button.view.tsx` to
export `Button`, and `mustBeNamed: '{folder-name}.test'` asks a test in `login` to be `login.test.ts`.
These are the spellings eslint-plugin-check-file and eslint-plugin-project-structure use, which
their users know.

`mustExport` counts names a file passes on: a re-export, and every name an `export *` brings in,
following the chain through the graph as the entry fix already does. When the chain ends in a package
or an import that does not resolve, archangel cannot know the names, and says so in a diagnostic of
its own rather than reporting a violation or passing. `mustOnlyReExport` permits imports, re-exports,
directives, and `export { a }` of names the file imported; any other top-level statement, a type
declaration included, is the file's own, and the violation points at the first one. `mayNotExport`
looks at the file's own export statements, at the line of each, and not at the names an `export *`
brings in, which are another file's exports. `imports` chooses the imports a rule checks, so it is
refused on a rule that checks none, such as `mustBeNamed` or `mustExport`. Every violation that is not
about an import, of a file, a folder or an instance, has the rule and that path as its identity, so a
file added to a folder leaves the folder's violation and its baseline entry as they were.

The three shorthand kinds compile to sentences when the config is validated. `isolate('feature')`
is `rule({ files: 'feature', mayNotImport: 'feature', instance: 'different' })`.
`layers(['app', 'feature', 'shared'])` is one `mayNotImport` sentence for each element after the
first, into every element before it. `entry('entity', 'index.ts')` is one `mayOnlyBeImportedBy`
sentence for each instance: the instance's files other than its entry files may be imported only by
the instance itself. Entry files are relative to the instance root, which no single selector can
say, so archangel keeps that sentence as one internal condition rather than one sentence per
instance. From then on, a rule is one form, whichever way it was written, and `archangel explain`
shows each rule as its sentence. A transitive rule follows chains, so `names`, `specifier`,
`instance` and `dependency`, which limit one import, are refused beside `transitively`; the limited
rule is a rule of its own.

### Applying the rules

Before it looks at any import, archangel compiles the import rules into a table with one cell for each
pair of classes that some import connects. It finds those pairs in one pass over the imports, fills
each cell by testing every import rule against the pair once, and only then evaluates the imports, in
parallel, against a table that no longer changes. So the cost of the table is the number of connected
pairs times the number of rules, once per check and again only when the config or a class changes.
The work count of rule evaluations counts the checks of imports against their cells, not the
filling of the table, because the table grows with the rules and the count must not: five rules
and five hundred give the same count when the extra rules cannot apply. Most rules are decided by the pair alone, so their
cells already hold the answer. Rules with `instance`, `names`, `specifier` or `dependency` also need
the import's own facts, so their cells hold that last test; `dependency` looks up the importing file's
nearest manifest, which archangel finds once per folder. Entry rules need the path inside the instance
too. Checking an import is then one look-up and, sometimes, one comparison, however many rules there
are. A work-count test holds this: five rules and five hundred give the same number of rule
evaluations per import when the extra rules cannot apply.

The other conditions do not depend on imports. Naming and `mustNotExist` look at each file once, and
folder naming at each folder once. `mustContain` looks at each instance or folder once, through a set
of its file names. The export conditions read the export names and statement kinds that the parse
kept. An edit to a file's text checks the export rules of that file and of the files that re-export
from it through `export *`, and checks no naming or structure rule unless the file's class changed.
Creating, deleting or renaming files checks the naming and structure rules again for the folders and
instances the change touched.

A transitive rule is one breadth-first search backwards from the forbidden targets, over each file's
importers, following only the imports the rule counts and not passing through `stopAt` files. It visits
each file at most once and records each file's distance from the nearest target. A file's next step is
then the import, among those one step nearer, whose target comes first in path order, so each file's
chain is its shortest, and the first in path order among the shortest. The search and the choice of
steps each look at every import once, so a rule costs time in proportion to the files and imports.
Every file in the rule's selector that the search reached is a violation, reported at the first import
of its chain. So each transitive
rule costs one walk of the graph, about the 1.4 ms of a cycle search on the measured monorepo, and it
runs again after every edit, because an edit anywhere can complete a chain. A team with ten transitive
rules pays tens of milliseconds per edit, and the work-count tests count the files each walk visits.

The no-cycles condition works on the whole graph instead. Archangel finds each group of files that can all
reach each other, and reports one violation per group. The violation lists every file in the group and
shows the shortest cycle through its first file as the example. Its identity is the sorted list of the
group's files, so a group that gains a file is a new violation, and a baseline entry for the smaller
group no longer hides it. The search walks the graph with an explicit stack instead of recursion, so
a chain of a hundred thousand files cannot overflow the thread's stack. On the measured monorepo, a
search of this kind took about one and a half milliseconds.

Archangel also reports an element that can match nothing: a file element that matched no file, and a
package element whose packages no import uses and no package manifest lists. The second test is what
catches a misspelt package name, such as `recat` for `react`, while it lets a team forbid a package
that is installed but not imported yet. A selector that the config declares, or an inline pattern in
one of its rules, that matches no file is the same error, so a pattern that is wrong never passes in
silence. The files of a `mustNotExist` rule are exempt, because matching nothing is how that rule
passes, and so is everything a preset declares (Decisions that need you, 17). Archangel keeps a count
of the files in each class, so after an edit it knows which selectors became empty without looking at
every file.

### Presets

A preset is a function that returns part of a config: elements, selectors, rules with their examples,
and the plugin conditions its rules use. `extends: [fsd({ root: 'apps/web/src' })]` puts it under
the config. Its options say where it applies, so one monorepo can use it once per app; a second use
takes its own `name`, such as `fsd({ root: 'apps/admin/src', name: 'fsd-admin' })`.

Everything a preset declares is named with its prefix: the preset `fsd` declares the elements
`fsd/shared` and `fsd/features` and the rule `fsd/public-api`. So two uses of one preset never
collide, and a team's own `shared` element never replaces the preset's. The config's own rules come
after the presets: a config rule with the same name as a preset rule replaces it, and `off` lists
rules to drop after the merge, usually a preset's. A config rule whose name starts with a preset's prefix must match a rule of that
preset, as an `off` name must, so a misspelt replacement is refused rather than added as a new rule.
The config's own names cannot contain a slash. Within one preset, or within the config, two rules with one name are still
refused. Every preset rule must have a name, because the name is what `off`, exceptions and the
baseline refer to; a generated name would change whenever the preset changed its elements.

The loader sends archangel each preset's result unmerged, with the preset's name, followed by the
config's own fields and `off`. Archangel merges them and validates the result, so the checks that
need to know which rule came from which preset, the prefix, a preset used twice under one name, and
a name in `off` that matches nothing, all happen in one place, beside the near-name suggestions
every other config problem gets. A preset rule without a name is a problem the developer using the
preset cannot fix, so its message names the preset's package and says to report it to its author.

A preset's examples are written relative to its root, and `archangel test` checks them in the team's
own repository like any other. So `archangel test` proves that the preset's rules work on this
repository, with this repository's tsconfig and paths, not only on the preset author's fixtures.

The Feature-Sliced Design preset declares the layers as elements, slices and segments as captures,
and Steiger's 21 rules as sentences. By default it turns on the rules Steiger recommends; with
`strict: true` it turns on Steiger's four opt-in rules too. Fourteen rules are built-in conditions:
imports between layers and slices, the public API of each slice, how imports inside and between
slices are written, wildcard exports, the structure of layers, slices and segments, and the reserved
names. The other seven are plugin conditions, in a plugin named `fsd` that the `archangel` package
ships beside the preset. Four are guesses about names: a layer name one typo from a real one, a slice
name that is also a segment name, entity names that mix plural and singular, and a word that repeats
across slice names. Three count things: a slice that at most one other slice uses, more than 20
slices in a layer, and more than 15 modules in `shared/lib`. Counts and guesses each serve one
preset, so they stay out of the built-in conditions.

Building the preset showed that one sentence per Steiger rule is not enough. `forbidden-imports`
is the layer order and the isolation of each sliced layer's slices together, and
`no-public-api-sidestep` is an entry rule for each sliced layer and a rule each for Shared and App.
So a rule may list `sentences`, each a sentence or a shorthand, under one name, and is broken when
any of them is. Three more small forms came from the same work: a folder rule may say
`mustNotExist`, so `processes` is one violation and not one per file; `mustContain` takes a list
of patterns, so a slice's segment may be a file, `model.ts`, or a folder, `model/`; and a folder
pattern that starts with `!` leaves folders out, as in `.gitignore`, so Shared's segments need an
index but `shared/ui` and `shared/lib` do not. A plugin violation may name a folder that holds a
file of its view, because the seven plugin rules report folders, as Steiger does.

The built-in rules find slices by path: every folder directly in a sliced layer is a slice. Steiger
finds a slice by the segments inside it, and that would make a file's element depend on its
siblings, so one edit could change the class of other files. The plugin rules see the whole tree
and find slices and slice groups as Steiger does. This and the other differences from Steiger are
requirements in the presets spec, and the differential test names each one where a fixture shows
it. The plugin's `inconsistent-naming` uses `pluralize`, as Steiger's does, so the `archangel`
package depends on it.

The examples of the count and naming rules run in the team's repository, so an accepted example of
`excessive-slicing`, `repetitive-naming`, `inconsistent-naming` or `shared-lib-grouping` fails
where the repository already breaks that rule in the same folder; the check reports the real
violation beside it.

The Next.js preset says that files with `'use client'`, and every file they reach through runtime
imports, may not import `server-only` or a Node built-in module: one transitive rule from a
directive selector.

### Plugins

By convention a plugin is published as `archangel-plugin-<name>` or `@scope/archangel-plugin`, and
archangel loads whatever object the config passes. The package exports an
object made with `definePlugin`: its name, its conditions, and the view's schema version it was
written for; a plugin written for a schema version archangel does not send is a config error that
names both versions. This is the shape of an ESLint plugin.
A package that offers presets exports them as plain functions beside it, as the built-in presets
are. A condition made with `defineCondition` has a name, a description, a documentation link, an
options schema written as JSON Schema, which archangel validates, the facts it reads, and a `check`
function.

A config loads plugins as ESLint's flat config does, `plugins: { team: teamPlugin }`, and a preset
brings its own. A rule uses a condition by name: `rule({ name: 'one-store', files: 'entity',
plugin: 'team/one-store-per-entity', options: { max: 1 } })`. Archangel checks `options` against the
condition's schema when it loads the config, so a wrong option is a config error, not a crash.

The condition's **filter** is the rule's `files` together with the facts the condition reads: the
files' imports, their importers, or their exports. Archangel evaluates the filter itself, so a file
outside it is never sent to Node. The bundler Rolldown filters its JavaScript hooks in Rust this
way, and measured ten plugins without filters turning a 0.75 s build into 2.8 s.

`check` receives a **view** of the graph: for each file in the filter, its path, element, captures
and selectors, and the facts the condition reads, with paths and names as plain strings. A condition
that reads importers also sees each importer's path. It reports violations with a file in its view,
an optional import, and a message. Archangel turns each into a diagnostic like any other, so
exceptions, the baseline, sorting and every output format apply. A violation's identity is the rule,
the file and, when it names one, the import's specifier and occurrence; the message is not part of
it, so a message that carries a count does not break a baseline entry when the count changes. The
view carries a schema version, and adding a field is not a breaking change; a plugin reads only the
fields it knows. The compiler SWC broke every plugin when it added a field to its plugin interface,
and moved to this rule afterwards.

A plugin rule needs a rejected example like every other rule. `archangel test` sends the view once
and then each example as a batch of changed file records, which it takes back after the example, so
an example costs one run of each condition that covers it, not a second view.

**Where the code runs.** Archangel's engine starts the Node process that loads the config, and owns
it. The process sends the config as its first answer. When no rule uses a plugin condition it then
exits; otherwise it stays and hosts the plugins, for one command in `check`, `test` and CI, and for
the session in watch mode and the language server. It speaks one protocol either way: archangel
sends the view as one message per file record, so no message grows with the repository, and after
each batch of changes it sends the records that changed. A file's record changes when its own facts
change, or, for a condition that reads importers, when the files importing it change. Node runs each
condition whose filter covers a changed record, over its copy of the view, and answers with all its
violations. Archangel does not record which facts each condition read; it would add that only if a
measurement showed the full rerun too slow for the editor. A resolution of every import sends the
records whose resolved imports changed. Node's standard error is read on its own thread and shown
with the plugin's name, so a plugin that logs a lot cannot block the protocol. Nx runs its
project-graph plugins in a long-lived process in the same way, and tsserver is the same idea for an
editor.

In the editor, archangel publishes its own diagnostics at once and the plugin diagnostics when Node
answers, for the same document version, as rust-analyzer publishes its own diagnostics first and the
compiler's later; while Node works, the editor shows a progress message that names the plugin. A
plugin that throws, a violation for a file outside the condition's view, and a Node process that
stops each give one `plugin-failed` diagnostic that names the plugin and what happened, says the rule
can be turned off with `off` or reported to the plugin's author, and fails the check; in the editor
the server starts Node again on the next change. Plugin rules never pass because their code did not
run.

Archangel never runs plugin code inside its own process, and a config without plugin conditions
leaves no Node process running.

### Exceptions and the baseline

Only a violation of an import can take an exception, because the comment sits above the import. A
transitive violation is reported at the first import of its chain, so it can take one there. A
violation about a file, a folder or an instance, and a plugin violation that names no import, can be
permitted only through the baseline, whose entry is then the rule and that path; a folder of images has
no line to put a comment on, and the baseline is the one place every kind of violation fits.

The graph gives the rules the text of each comment that contains `archangel-allow`, with its line. An
exception, or a run of exceptions on consecutive lines, applies to the import on the next line, and
each exception applies to exactly one rule. After the rules run, archangel removes each violation that
an exception permits and reports each exception that permitted nothing.

Each violation of an import has an identity made from the rule, the file, the specifier, and which
occurrence of that specifier in the file it is, so the identity survives an edit that moves lines. A
cycle group's identity is the rule and its sorted files. A baseline entry
is that identity without the occurrence, with a count. The baseline file is sorted JSON. A check uses
each entry up to its count, reports the violations that are left, and reports an entry that matched
fewer violations than its count, or that names a rule the config no longer has, as stale. Only
`archangel baseline add` adds entries, and only `archangel baseline prune` lowers them. A baseline file
that does not parse stops the check with its line and column.

### Reporting

Every diagnostic is data first: a code from a fixed list (a violation's code is its shorthand kind or
its condition, such as `isolate` or `mayOnlyImport`, or `plugin` for a plugin rule), a location, the
rule, the element instance of each file that has one,
the reason and documentation link from the config, and a sentence about what the code may do instead.
The command line turns the data into text with the source line and a mark under the specifier, or into
one JSON document. The language server turns it into editor diagnostics. Nothing that applies rules
knows about terminals or editors, and nothing is ever cut short: every diagnostic and every step of
every cycle is printed.

Diagnostics are sorted by file, line, column and rule, so two checks of the same code give the same
bytes on any machine.

For an entry violation, archangel tries one fix. It shortens the specifier so that it points at the
instance's entry file, and resolves the new specifier. It checks that the entry file exports every name
the import uses, following `export *` chains through the graph, and it checks the new import against
every rule. Only a fix that passes all of that is offered. All of this uses data the graph already
holds, so a fix costs a resolution or two and no parse. When an `export *` chain ends in a package,
whose exports archangel does not know, the violation says so instead of claiming the name is missing.
Archangel tries fixes only for violations it reports, not for those that an exception or the baseline
permits.

### Examples

Each rule in the config can carry examples. Each example is a small set of files given as path and
text, such as `rejected: [{ 'src/shared/a.ts': "import '../features/login'" }]`.

`archangel test` loads the repository once. For each example, it puts the example's files over the
repository in memory: the resolver sees them as files, and the rules see the JavaScript and
TypeScript files among them as source files and the rest, such as a `README.md`, by path only. An
example file at the path of a real file replaces that file for this example only. Archangel parses only
the example files and resolves only their imports, so a hundred examples cost a hundred small parses,
not a hundred reads of the repository. Nothing is written to disk.

A rejected example passes only when its own rule reports a violation that involves an example file. If
another rule rejects it instead, the example fails and the output names that rule. An accepted example
passes only when its rule reports nothing for the example files and its rule evaluated at least one of
them: one of their imports for an import rule, or one of the files, folders or instances they make for
a naming, structure or export rule. An accepted example that its rule never evaluated proves nothing
and fails. An example
file with a syntax error fails its example. A rule with no rejected example fails as unproven.

A repository file never imports an example file at a new path, because its imports were resolved
before the example existed. But a repository file that imports a path that an example replaces does
import the example's file. So the cycle search for an example covers the example files and the
repository files that can reach a replaced file, and nothing else. For an example that replaces a widely
imported file, that can be most of the repository: one walk backwards through the importers and one
search, about the 1.4 ms that a full search takes. Nothing smaller is correct, because any of those files
can close a cycle through the replaced one. A work count holds the number of files the search visits.
For each example, the resolutions and rule evaluations are those of the example files' imports only.

### Watch mode and the language server

Both use one engine, the same one that `archangel check` uses. The engine keeps the graph, the compiled
rules and the results in memory, and owns one file watcher that both use. It takes the file events that
are waiting, compares them with the paths it already knows, and only then decides what changed. So a save
by a write to a temporary file and a rename over the original, which Vim, JetBrains' safe write and the
helpers that coding agents use all do, is a change to the text of one file, not a new file. Events for
files that the check does not depend on, such as an editor's temporary files, are ignored. There are
five kinds of change:

- **The text of one file**, saved or not yet saved: the engine parses that file, resolves its imports,
  updates the importer lists of the files those imports point to, classifies that one file, checks its
  imports against the rules, checks the export rules of the file and of the files that re-export from it
  through `export *`, matches that file's exceptions and baseline entries again, and adjusts the counts
  of which packages are in use and of the files in each class. If the file's class changed, because
  its directive did, it also checks the imports into the file and the file's naming and structure
  rules. Then it searches for cycles and walks the transitive rules again, and sends the changed
  records to the plugin process if one runs. Nothing else is read, parsed or classified.
- **Source files created, deleted or renamed, a `.gitignore` changed, or the config's ignore list
  changed**: the engine walks the folders again if an ignore rule changed, resolves every import again, one time for all the changes that are
  waiting, because a new file can change where any import points, and parses only the files that are new
  to it. Then it checks the naming and structure rules of the new files and of the folders and
  instances that gained or lost a file, and the export rules of the files whose imports now resolve
  elsewhere. Files that are not source, such as an image or a README, or a folder of them, cost less:
  the engine adds or removes their paths, without a walk or a parse, and without a resolution unless
  a resolution found one of them, as one finds a CSS file that an import points to, and checks the
  naming and structure rules of those files and of the folders and instances that hold them. A change
  to the contents of such a file costs nothing.
- **A file whose contents decide resolutions**, such as a tsconfig, a tsconfig that another one
  extends, a package manifest, or Yarn's Plug'n'Play file: the engine resolves every import again, one
  time for all the waiting changes, without a parse. A file whose existence decides resolutions, such
  as a CSS file that an import points to, counts here only when it appears or disappears; a change to its
  contents costs nothing. Nothing smaller is correct, because the resolver's cache must be
  cleared, and the `extends` chains and path aliases of one tsconfig can reach files anywhere.
- **The config**, or a preset or plugin file it imports: the engine loads it, compiles the rules,
  starts the plugin process again, and checks again, without a parse or a resolution, unless the
  ignore list changed, which counts as an ignore-rule change above. An invalid config replaces all diagnostics with its own, so the developer is never shown
  results from rules that no longer exist.
- **The baseline, or a package manifest that no resolution read**: the engine reads it and applies the
  baseline, or the manifest's dependency names, to the current results, with no parse or resolution.
  A manifest also decides the `dependency` option, so when a rule uses it the engine checks again the
  imports of the files whose nearest manifest it is, and no others.

The language server publishes diagnostics only for files whose diagnostics changed, each with the
version of the document it checked. When several changes to one document wait in the queue, it checks
only the newest, because the older versions no longer exist in the editor. While a document is open, the
server ignores changes to its file on disk, because the editor owns its text; when the editor closes a
document whose changes were not saved, the server goes back to the text on disk. It asks the editor to report
changes to every file below the repository and lets the engine ignore the ones the check does not depend
on. When the editor cannot report them, the server uses the engine's own watcher, with the same full
check after lost events and the same polling, which it tells the developer about in a message.

Watch mode uses the operating system's file events and prints only the diagnostics that are new or
gone, with a summary line, or one JSON line per check when asked, so a test watcher beside a coding agent
can pass the result into the agent's turn. When the operating system reports that it lost events, watch
mode makes a full check from the start. When it cannot watch the files at all, for example because
Linux's limit on file watches is too low for the repository, it checks for changes every two seconds,
and every summary line says so and names the command that raises the limit. Each of those checks reads
the metadata of every watched file, so it grows with the repository; it is a fallback for a system that
offers no watches, not the normal path.

### Counting work

Every stage counts its work: files found, files read, bytes read, files parsed, imports found,
resolutions, files classified, pattern passes, rule evaluations, files visited by cycle searches and
by transitive walks, looks of the naming and structure rules at files and instances, of the folder rules
at folders and of the export rules at files, examples checked and files written. The counts are part of the result:
`--stats` prints them with the duration of each stage, and the JSON output carries them. Tests assert
counts. A full check parses each file once, an edit parses one file, a config change parses none, a
burst of file events resolves everything once, a fix parses nothing, an example parses only its own
files, and the test command writes nothing.

Each of those tests has a fault control: a build of archangel with one deliberate defect, such as a
second parse of each file, which the test must reject. The faults are named points in the code that do
nothing in a normal build. A fault is switched on for one test at a time, because the switch is
shared by every test in the same process. Durations appear only in `--stats` and in benchmarks, and no test passes or
fails because of one.

### How archangel itself is tested

The brief asks for test layers whose gaps do not line up. Each layer catches a different kind of
mistake:

- **Scenario tests**, one for each scenario in the specs. Many run the archangel binary on a small
  repository that the test writes to a temporary folder.
- **Property tests** compare the pattern matcher, the cycle search and the rule table with slow,
  obviously correct reference versions, on random inputs.
- **Differential tests** compare archangel's resolution of every import in a set of fixture
  repositories with the TypeScript compiler's own resolution. A disagreement is a bug in one of them.
- **Fuzzing** feeds random text to the three parsers that read what people write: the config, the
  patterns and the exception comments. Each must give a result or a diagnostic, never a crash.
- **Simulated editor sessions**, a kind of stateful property test, send random sequences of edits,
  creations, deletions and renames to the language server, and after every step compare its diagnostics with a full check from the start. This
  is the test that proves the shortcuts in the previous sections are correct.
- **Mutation testing** changes archangel's code in small ways and checks that some test fails. Every
  pull request must leave no surviving change in the code it touches, and a weekly run covers all the
  code. A deliberate exclusion carries its reason. The brief asks that the score never goes down; no
  surviving change is the strictest form of that.
- **Benchmarks** check fixed versions of public repositories and a generated repository of one million
  lines, and report durations. They never decide a pass or a fail.

## Decisions

**1. Build on the oxc parser and resolver, not on tree-sitter, swc or our own.** oxc parses TypeScript
quickly, gives a record of a file's imports without a walk of the tree, and its resolver already
implements tsconfig paths, package exports and symbolic links. The oxlint linter and the Rolldown and
Rspack bundlers depend on both. Tree-sitter, which ast-grep uses, has no resolver and gives a concrete
syntax tree that is slower to query for imports.

**2. Parse every file in full, not with a separate import scanner.** A scanner that reads only import
statements, as es-module-lexer does, would be a second parser to keep correct. The spike showed it
would save almost nothing: parsing 52 MB of source took about 70 ms on 14 threads, while listing and
reading the files took about 400 ms.

**3. Keep the graph of imports apart from the config.** The graph is built from the files alone, and the
rules are applied to it afterwards. So a config change costs a new classification and a new
evaluation, not a new parse, and the rules can be tested on hand-written graphs. The TypeScript language
service keeps its program apart from the checks that use it in the same way.

**4. Store each file's imports and importers in plain lists, not in one compressed array.** A compressed
array is faster to walk, but an edit must replace one file's imports, and a compressed array would have
to be rebuilt for that. The list of each file's importers serves `archangel explain` and lets the
language server find the files whose diagnostics an edit can change. At about 52,000 imports, the
spike's search for cycles took about 1.4 ms with plain lists.

**5. Do not cache resolutions on top of the resolver's own cache.** The spike resolved all 52,196 imports
again with a warm resolver in 12 to 15 ms. Only 11,111 of them are different pairs of folder and
specifier, so a second cache could save at most about 10 ms, and it would need its own rules for when to
forget an entry.

**6. Resolve everything again, once, after files are created, deleted or renamed.** A new file can
change where any import points, for example when a file named `button.ts` appears beside a folder
`button` with an index file. The TypeScript language service and webpack handle this by recording every
place that a failed resolution looked and resolving again only the imports that looked at the new path;
the oxc resolver can record those places too. That is correct but costs memory for every import, and it
is a second mechanism to keep correct (Decisions that need you, 11). The spike measured a
full new resolution, after the resolver's cache was cleared, at 37 to 52 ms. Because all waiting file
events share one resolution, a branch switch of a thousand files costs one resolution, not a thousand.
If a larger repository makes it slow, recording the failed places is the known next step.

**7. Compile the rules into a table of class pairs, not a list that each import searches.** Lint
plugins for boundaries check each import against each rule in turn, so their cost grows with the number
of rules. The table makes the cost of an import the same for five rules or five hundred. It is the usual
way to compile a set of conditions on a small key: a decision table. The key is the pair of classes, the
atomic-predicates partition of the selectors (Yang and Lam, 2013), which regular expression engines also
use to group bytes that every transition treats alike. The table holds only the pairs that some import
connects, found in one pass over the imports and filled before evaluation starts, so evaluation reads a
table that does not change.

**8. Find cycles with petgraph's Kosaraju search, again after each edit.** petgraph is the standard graph
library for Rust, and its Kosaraju search walks with an explicit stack, so a long chain cannot overflow
the stack. Each no-cycles rule counts its own kinds of import (all, runtime or types), so archangel
copies the edges that rule counts into a petgraph graph indexed by file, one pass over the imports per
rule. Traits on the import graph itself would need a filtered view per rule for the same result.
An algorithm that keeps cycles up to date edit by edit exists, but it is complex, and a full search took
1.4 ms on the measured monorepo.

**9. Use one pattern language for elements and entry files, compiled to regular expressions, with
inline captures as Sheriff has them.** Braces with a comma list alternatives, as they do in globset and
micromatch, and braces without one name a capture, so both familiar meanings work.
Archangel translates each pattern to a regular expression with a named group for each capture, which is
how the globset crate implements globs, and tests all element patterns at once with a set of regular
expressions. The ignore list uses `.gitignore` syntax instead, because it extends the `.gitignore`
files, and developers already know that syntax from them.

**10. Load a TypeScript config with Node's own type stripping, once, from the binary.** Vite, ESLint and
Vitest all load their configs with Node, after a transform of their own; ESLint also offers Node's
native type stripping as an option, which is the mechanism archangel uses. Only archangel's loader runs inside Node, and its output is plain JSON, so no config
code runs during a check. The binary starts Node itself, rather than the npm launcher, so that one path
serves installs from both registries, editors that start the binary directly, and reloads in watch mode.

**11. Make exceptions comments that name one rule and need a reason, and fail when they are unused.**
Biome's suppression comment has this form and also needs a reason. ESLint's report of unused directives,
and Rust's `expect` attribute, are the precedent for an exception that fails when it no longer permits
anything.

**12. Key the baseline by the violation's identity without its occurrence, with a count, and prune it
with a command.** ESLint's bulk suppressions file has the same shape, rule and file with a count, and the
same command to prune it. Adding the specifier makes an entry more exact, and leaving out the line makes
it survive edits. Using the violation's own identity means there is one identity for a violation, not two.

**13. Put examples over the repository in memory, through the resolver's file system interface.** The oxc
resolver reads files through an interface that archangel can implement, so example files exist for the
resolver without being on disk. The alternative, writing each example into the source tree and running the real tool, is how the
team in the brief checks its 400 rules today, and that takes about 30 minutes. Each example gets a fresh
resolver, because a shared one would remember that the example's paths did not exist. The spike measured
this on 200 real files of the monorepo, as if each were one example: a fresh resolver cost 0.53 ms per
example and a shared one 0.07 ms. So 400 examples cost about 0.2 s, and the simpler, always-correct
choice is cheap enough.

**14. Keep the rules apart from all input and output, and give one engine the whole check.** The parts that
build the graph read files only through the resolver's file system interface and the file walker; the
part that applies rules reads nothing and prints nothing. One engine owns a check from start to end:
loading the config, building the graph, applying the rules, reading and writing the baseline, and keeping
all of it up to date after changes. The command line, watch mode and the language server each turn the
engine's results into their own output. rust-analyzer is built the same way: its analysis is apart from
the protocol, and its main loop applies changes to one state.

**15. Watch what the resolutions depend on, as bundlers do.** Webpack and rspack watch exactly the
files that their resolver reported reading or finding. Archangel does the same with the oxc resolver's
record, and adds the tsconfigs and the Plug'n'Play file itself, because the resolver does not report
those. So a deleted CSS file or a changed tsconfig that another one extends is never missed, a saved CSS
file costs nothing, and a change to an unrelated file costs nothing.

**16. Write the five kinds of change by hand, not with salsa.** salsa, the incremental computation
library behind rust-analyzer, records which result depends on which input and recomputes only what an
input change affects. Archangel's inputs fall into five coarse kinds, each with a fixed and measured
response, and the simulated editor sessions prove each response against a full check. salsa would add a
framework that every contributor must learn, to save work that is already a few milliseconds.

**17. Use the lsp-server crate, not an asynchronous server framework.** rust-analyzer's own crate handles
messages on a plain thread. Archangel's work per message is milliseconds, so an asynchronous runtime
would add a large dependency and nothing else.

**18. Render text with annotate-snippets, rustc's own renderer.** It draws the source line and the mark
under the specifier the way Rust's compiler does, and ruff uses it too, so the output looks like
something developers already trust.

**19. Split the code into crates along the same lines as the stages.** Cargo refuses a dependency
between crates that a crate's manifest does not declare, and refuses cycles between crates. So the rule
that the rules part reads no files is enforced by the compiler, not by a review. oxc, ruff and
rust-analyzer are each split in this way.

**20. Ship to npm as one main package with one optional package per platform, released by a workflow
written after Biome's.** The main package holds the types and a small launcher; each platform package
holds one binary, and npm installs only the one that matches the machine. esbuild, Biome and oxlint
ship this way. cargo-dist can write a release workflow, but its npm installer downloads the binary from
GitHub when the package is installed, which fails behind a registry proxy and on machines without
internet access. Cargo publishes the crates in dependency order with one command. Both registries publish through
trusted publishing from GitHub Actions, so the release stores no registry token, and npm adds
provenance itself.

**21. Make the fault controls with the fail crate.** TiKV's fail crate puts named fail points in the code,
which compile to nothing unless a test build turns them on, and a test turns each one on by name. This is
the established way to inject a fault in Rust, and it keeps the faults out of release builds. Its
switches are shared by the whole test process, so fault-control tests take the crate's scenario lock,
which runs them one at a time.

**22. Hold mutation testing at no surviving change, with cargo-mutants.** cargo-mutants exits with an
error when a change to the code survives every test, and can test only the code a pull request changes.
Its config file holds each deliberate exclusion with a reason. That needs no score file and no script.

**23. Keep elements and add selectors, as Nx has projects and tags and Sheriff has modules and tags.**
An element answers "where does this file belong", with captures; a selector answers "is this file one
of these", any number of times. Merging the two would make isolate and entry ambiguous for a file in
two elements, and leaving out selectors forced the translator that the dogfood needed (Decisions that
need you, 15). The two are kept apart in the code too, so a later change does not merge them by accident.

**24. Write rules as objects with one condition each, built by typed functions.** dependency-cruiser,
eslint-plugin-boundaries, Sheriff and Nx all write rules as data, and ArchUnit's own rules compile to
data before they run. One condition per rule gives one meaning per diagnostic. The condition names read
as English (`mayOnlyImport`), as ArchUnit's do, because ArchUnit is what this tool is compared with.

**25. Compose presets as ESLint's flat config does, with every name prefixed.** A preset returns
config; the config's own rules replace preset rules of the same name; `off` drops a preset rule by
name, and a name that matches nothing is an error. Everything a preset declares carries its prefix,
as ESLint plugin rules do (`fsd/public-api`), so one preset used twice, or a team element with a
common name, never collides with it. Archangel merges the presets itself rather than the loader, so
the checks that need to know where a rule came from live beside the rest of config validation.

**26. Make plugins ESLint-shaped packages whose conditions receive a graph view, not a syntax tree.**
The package name, `definePlugin`, and a condition's description, options schema and `check` follow
ESLint, which every TypeScript developer has met. The view follows Nx's project-graph plugins: data
in, findings out, once per batch. A filter that archangel evaluates follows Rolldown's hook filters.
The view's schema version, with new fields never breaking a plugin, follows what SWC learnt when every
plugin broke on one added field.

**27. Host plugins in the Node process that loads the config, owned by the engine, with one protocol
for a check and for an editor session.** oxlint hosts its JavaScript plugins the other way round:
Node runs and loads the Rust core as a native module. That suits a linter that passes syntax trees
without copying them. Archangel passes a filtered graph, far smaller than syntax trees, and its binary
must run on its own for installs from crates.io and for editors that start it directly (decision 10).
So the engine starts Node, receives the config as the first answer, and keeps the same process to run
the plugins: for one command in CI and for the session in the editor, with one protocol for both. It
reruns every affected condition over its copy of the view after each batch, as a language server with
full document sync does, rather than tracking which facts each condition read.

**28. Prove the Feature-Sliced Design preset against Steiger.** Steiger is the reference implementation
of those rules, so a differential test runs Steiger 0.8.1 with its Feature-Sliced Design plugin, both pinned,
and archangel with the preset on the same fixture repositories, as the resolution tests compare
archangel with the TypeScript compiler. A disagreement is a bug in one of them, and a fixture first,
unless the presets spec lists it as intended, with its reason; the test then names it by rule and
path, so a new difference fails and so does one that goes away.

**29. Walk a transitive rule backwards from its targets.** A forward search from each file in the
selector would cost one walk per file. One breadth-first search backwards from the forbidden targets,
over the importer lists the graph already keeps, reaches every file that has a chain and gives each
its shortest chain, in one walk per rule. It is the usual way to answer "which nodes can reach this
set" for many sources at once: reverse reachability, as a build tool finds every target that depends on
a changed file.

## Cost

We measured the stages that the design depends on with the spike, run twice on a monorepo of 9,824
source files, 1.34 million lines and 52 MB, on a 14-core laptop. The second run had the operating
system's file cache warm. The spike left out the 6 declaration files, the walk for require calls and
import types, and the scan of comments.

| Stage | First run | Second run | Grows with |
| --- | --- | --- | --- |
| List the source files | 291 ms | 149 ms | files and folders |
| Read the source files | 329 ms | 240 ms | bytes |
| Parse and find 52,196 imports | 66 ms | 72 ms | bytes; 300 ms on one thread |
| Resolve every import | 117 ms | 39 ms | imports |
| Search for cycles in 31,504 runtime imports between files (Tarjan, no cycles present) | 1.3 ms | 1.4 ms | files and imports |
| **These stages together** | **about 0.8 s** | **about 0.5 s** | |
| One example's imports with a fresh resolver (200 files, 951 imports) | 0.53 ms each | | imports per example |

So reading, parsing and resolving the monorepo from cold meets the brief's target of under one second,
and listing and reading files take about three quarters of it. These costs were not measured, and a
benchmark measures each before the release:

- putting files into elements and applying the rules: one pass of the element patterns per file and one
  table look-up per import, estimated at a few milliseconds;
- the walk for require calls and import types, in files whose text contains `require` or `import(`, and
  the scan of comments for exceptions: estimated at a fraction of the parse time;
- starting Node to load a TypeScript config: tens of milliseconds, once per load;
- memory, which holds paths, imports and names but no file text or syntax trees: estimated at tens of
  megabytes for this monorepo;
- the cost of recording the files that each resolution depends on, which the spike's resolution did not
  do, and the size of those sets on the monorepo, which also bounds the memory of the precise method in
  Decisions that need you 11;
- the cycle search that the design chooses, a Kosaraju search and a shortest-cycle search per group, on a
  graph with large cycle groups; the spike measured only Tarjan's search on a graph without cycles.

The rule language, presets and plugins were estimated before they were built: classification at
one pass of all patterns per file plus grouping, the naming, structure and export conditions at a
few milliseconds, each transitive rule at the 1.4 ms of a cycle search, the plugin view at a size
small enough to send once per check, and the Feature-Sliced Design plugin conditions after an edit
within about a second, the time a developer waits for a second set of diagnostics. They were then
measured on the dogfood monorepo, with its rules written by hand as selectors and sentences in a
config that stays with that repository: 356 rules of its own, which hold the
rules of dependency-cruiser, eslint-plugin-boundaries and eslint-plugin-check-file, and the
Feature-Sliced Design preset used three times, once per front end, for Steiger's; 400 rules after the
merge. Release build, Apple M4 Pro with 14 cores and 48 GB, macOS 26.5, Node 24.18, other work
loading the machine at the same time; each figure is the median of at least five runs.

| What | Estimate | Measured on the dogfood |
| --- | --- | --- |
| Files and imports | | 9,765 source files and 51,612 imports; 50,680 paths visited under `apps` and `packages` |
| Pattern set | one compiled set | compiled as one: 50,680 pattern passes for 50,680 paths |
| Classes and connected pairs | small | 957 sets of files; 680 classes, 22 of them packages; 4,185 connected class pairs |
| Filling the class-pair table | pairs times rules | 5.1 ms for the 4,185 cells; every import rule, the fills included, 23 ms |
| Classification | a few milliseconds | 790 ms: 957 set tests for each of 50,680 paths, about 16 ns each |
| Naming, structure and export conditions | a few milliseconds | about 140 ms for 109 rules about files and folders, every file and folder looked at |
| Transitive and export rules | 1.4 ms per transitive rule | two transitive rules over the two larger front ends, 5,718 files walked, and 107 export evaluations, together about 45 ms more, with the classification of their three sets |
| Plugin view | small enough to send once | 4,011 file records, 2.1 MB; sent in 52 ms, and Node answered the 20 plugin conditions 26 ms later |
| A TypeScript config with three presets | 80 to 110 ms | 210 ms: Node loads the presets and plugin and sends a 285 KB config |
| Whole check | | 1.90 s, against about 68 s for dependency-cruiser, the two ESLint plugins and Steiger on the same rules |

Classification is the estimate that missed. Its cost is the number of paths the walk visits times the
number of sets of files the rules name, and the dogfood names many sets because each role matrix row is
one. Each set test allocated its own stack of answers, and the rule stage took 2.1 s; one stack lent to
every test of a path brought the stage to 1.25 s. The structure stage looked up each file's class once
per rule about files; reading it once per file took the stage to 1.04 s, and structure from about 350
to about 140 ms. Neither change alters what
the work counts count. Classifying a path once per distinct set of pattern matches, rather than once
per path, is the next step if a check on a repository of this size must come under a second.

After one edit in the editor, measured by driving `archangel lsp` over standard input with an edit
that adds and removes an import from one feature into another, eight edits in a row: archangel
published its own diagnostics a median of 13 ms after the change, sent the one changed file record to
Node, and the plugin's progress message ended a median of 29 ms after the change, of which Node took
18 ms. The second set of diagnostics is far inside its budget of about a second.

Measured for the release with `scripts/bench`, release build, on a 14-core Apple M4 Pro that other
work was loading at the same time, so single runs vary; the median of 10 runs after 2 warm-ups:

| Repository | Files | Imports | Whole check, median (fastest) | Peak memory |
| --- | --- | --- | --- | --- |
| VS Code 1.144.0, `src/vs` | 7,652 | 89,390 | 361 ms (350 ms) | 130 MB |
| Excalidraw 0.18.1 | 436 | 3,396 | 33 ms (32 ms) | 31 MB |
| Generated, one million lines | 10,000 | 29,982 | 312 ms (281 ms) | 121 MB |

The stages of one VS Code check: listing the files 40 ms; reading and parsing them, with the walk for
require calls and import types and the scan of comments, 159 ms; resolving every import and recording
what each resolution read, 105 ms; and classifying the files, applying the rules and searching for
cycles, 35 ms. The first measurement found 165 ms more in the graph stage: merging each file's
resolution inputs into ordered sets, about a million paths compared component by component to keep
eleven thousand, and the real path of every file the resolver read. Hash sets, and real paths only
under `node_modules`, where a workspace link can be, brought the check from 654 ms to 361 ms.
A TypeScript config costs 80 to 110 ms to load through Node; a JSON config, under 1 ms. Memory is
higher than the estimate of tens of megabytes: about 130 MB for VS Code. The work-count tests hold the
cost of an edit; on the dogfood, one edit took 13 ms in the editor, as above.

After the first check, the cost of a change is:

- **An edit to one file**: one parse and the resolution of that file's imports. The spike parsed the
  largest file in the monorepo, a one-megabyte bundled script with no imports, in 9 to 12 ms; the
  benchmark measures an ordinary file with imports. The search for cycles, about 1.4 ms, and the walk
  of each transitive rule are the parts that grow with the repository.
- **Files created, deleted or renamed, or a change to a tsconfig or manifest**: one full resolution for
  all waiting changes, measured at 37 to 52 ms, plus the rules.
- **A config change**: a new classification and evaluation, with no read, parse or resolution.
- **A `.gitignore` change, or a change to the config's ignore list**: a new walk of the folders (149 to
  291 ms in the spike) and one full resolution, which is rare enough to accept.
- **A save to a file that an import points to but that is not a source file**, such as a CSS module:
  nothing.
- **An example that replaces a widely imported file**: one backward walk and one cycle search over the
  files that reach it, up to the whole repository, about 1.4 ms.
- **Any edit, with transitive rules**: one walk of the graph per transitive rule.
- **Any edit, with plugin conditions**: the changed file records sent to Node, and every plugin
  condition whose filter covers them run again in Node, after archangel's own diagnostics are out.
- **An edit that changes a file's class**: the imports into that file, through its importer list, and
  that file's naming and structure rules.
- **An edit to a file that others re-export through `export *`**: the export rules of those files.
- **A manifest change, with a `dependency` rule**: the imports of the files whose nearest manifest it
  is.

The work-count tests in the specs hold these as counts, each with a fault control.

## Risks

- **oxc changes its interfaces often.** Archangel pins exact versions, Dependabot proposes each update as
  a pull request, and the scenario and differential tests decide whether it can merge.
- **The resolver can disagree with TypeScript.** The differential tests compare the two on fixture
  repositories with path aliases, package exports, workspace links and `.js` specifiers. A disagreement
  found later becomes a new fixture first.
- **Node is required for a TypeScript config.** The diagnostic for a missing or old Node says that a JSON
  config needs none, so a developer is never stuck.
- **The repository's own strict checks.** This repository runs zanity, a checker for code written by
  people and agents, with its strict rules. Some of them come from TigerStyle, a coding style that, among
  other things, forbids memory allocation after start-up, which does not fit a tool that builds a graph
  from a repository. When such a rule fires on correct code, it is switched off in zanity's config with
  its reason, as the repository's rules require.
- **Listing and reading files is three quarters of a cold check.** That is the operating system's cost,
  not archangel's. The cache that persists between command-line runs, in a later release, is the answer if
  teams need a faster cold start.

## Compatibility and release

Everything is new, so nothing breaks. The release is version 0.1.0 of the crates and the npm packages.
From then on the config format, the JSON output, the diagnostic codes, the exception comment and the
baseline file follow semantic versioning: a change that makes a valid config invalid, or that changes the
meaning of an output field, is a major version. The JSON output carries a schema version for programs
that read it.

A release starts from a version tag. CI builds the binaries for seven platforms, runs every test on each,
publishes the crates to crates.io, publishes the npm packages with provenance, attaches the binaries and
their checksums to a GitHub release, and installs the result from both registries on a clean machine. A
bad release is deprecated on npm and yanked on crates.io, and the next patch version replaces it; neither
registry lets a version be replaced in place.

## Limits

Archangel itself sets no limit on the size of a file, the number of files, imports, rules, examples,
exceptions or baseline entries, the length of a cycle, or the number of diagnostics it prints. Watch mode
and the language server never drop a result; the language server skips only document versions that a
newer version of the same document has replaced, because the editor no longer has them. Loading the
config has no time limit: a config that never finishes keeps archangel waiting, and the developer sees it
waiting, which is better than a limit that would fail a slow but valid config.

Archangel sets no limit on the number of selectors, classes, presets, plugins or plugin violations
either, and no time limit on a plugin: like a slow config, a slow plugin keeps archangel waiting
where the developer can see it, in the terminal or in the editor's progress message.

These places could look like silent limits, and each one is reported:

- Symbolic links are not followed, and each one is listed in the summary.
- A nested repository, such as a submodule, obeys the outer `.gitignore` files, and each one is listed
  in the summary.
- Computed specifiers are not followed, and each one is counted and listed.
- Unresolved imports that the config permits are counted in the summary.
- A file that Git tracks but a `.gitignore` pattern matches is not read, because archangel follows the
  `.gitignore` files, as ripgrep does, and not Git's index. The documentation says so, and a team that
  wants such a file checked removes it from the pattern.

The dependencies bring these limits, each with the result the developer sees:

- The oxc parser refuses a source file larger than 4 GiB. Archangel gives a parse diagnostic that says
  the file is larger than the parser accepts.
- The oxc resolver reads the `NODE_PATH` and `OXC_RESOLVER_YARN_PNP` environment variables by default
  and resolves from the process's current folder. Archangel turns both variables off, turns
  Plug'n'Play on only when the repository has the file, and sets the folder to the repository.
- The oxc resolver skips a tsconfig that it cannot read and uses a parent's. Archangel checks that each
  tsconfig it finds can be read, and gives a diagnostic when it cannot.
- The oxc resolver stops after 64 steps through aliases and `exports` fields. Archangel gives an
  unresolved-import diagnostic that says the chain loops or is too long.
- The ignore crate skips hidden files and reads `.ignore` files, the global Git ignore file and Git's
  local exclude file by default. Archangel turns all four off, so the source files are the same on every
  machine.
- The notify crate can lose events when the operating system's event queue overflows, and on Linux it
  needs one watch per folder within a system limit. Archangel makes a full check after lost events and
  polls, with a note in every summary line, when it cannot watch.
- An editor may not let the language server register file watches. The server then watches the files
  itself.
- The regex crate refuses a set of patterns larger than its size limit, which archangel sets to 256 MB.
  Archangel then matches each pattern on its own: slower, never wrong, and counted as one pass per
  pattern in the work counts.
- V8 limits one string to about 512 MB, and a pipe holds 64 KiB. Archangel sends the view as one
  message per file record, so no message grows with the repository, and reads Node's standard error on
  its own thread, so a plugin that logs a lot cannot stop the protocol.
- The oxc parser does not read Flow type syntax. A Flow file gives a parse diagnostic that says Flow is
  not supported and that the config's ignore list can leave the file out.
- The oxc parser descends into nested code with recursion. Archangel gives each parse thread a 64 MB
  stack: code nested 10,000 levels deep needed between 32 and 64 MB in a debug build. A file deeper
  than that would stop the process, and a test of 10,000 levels guards the figure.
- Node's type stripping refuses some TypeScript in a config. Archangel names the construct and the
  alternative.

The code searched for limits: the sources of oxc_parser 0.133, oxc_span 0.133, oxc_resolver 11.23,
ignore 0.4, notify and lsp-server 0.7, regex 1, and Node 22's limits on strings and pipes.

## Where in the code

This section is for whoever builds the change.

| Part | Where |
|---|---|
| Workspace (decision 19) | `Cargo.toml` becomes a workspace with members under `crates/`. Shared lints move to `[workspace.lints]`, and each crate has `[lints] workspace = true`. `src/lib.rs` is removed. Dependency direction: `archangel_testkit` depends on nothing of ours and every crate takes it as a dev-dependency only; `archangel_graph` and `archangel_config` depend on neither each other nor anything of ours; `archangel_rules` depends on both; `archangel_engine` on all three; `archangel_lsp` and `archangel` on the engine. Precedent: the oxc, ruff and rust-analyzer workspaces. |
| Config (decisions 9, 10) | `crates/archangel_config`. `Config`, `ElementSpec`, `Rule { name, files, condition, filter, shorthand, reason, link, examples }` with `Shorthand::{Layers, Isolate, Entry}` (the kind a rule was written as, if any) and `ImportKinds::{All, Runtime, Types}`, as `serde` types with `deny_unknown_fields`. `ElementSpec` is a pattern string, a list of them, or `{ packages: [...] }`. `archangel_config` parses and validates a JSON config, and the JSON the loader sends; it starts no process. The loader is `crates/archangel_config/src/load.mjs`, embedded with `include_str!` and started by the engine's `plugins.rs` as `node --input-type=module -e` after a `node --version` check. Because its standard output carries the protocol, the loader moves the config's and the plugins' `console.log` to standard error before it imports the config. (A separate file descriptor would need `unsafe` code in Rust, which the workspace forbids.) Node's `ERR_UNSUPPORTED_TYPESCRIPT_SYNTAX` and missing-extension errors map to `ConfigProblem` messages that name the construct. `Pattern` in `pattern.rs` compiles to a `regex::Regex` with named groups and gives `PatternMatch { captures, root }`; `ElementMatcher` holds a `regex::RegexSet` of all element patterns. `validate.rs` returns every problem as `Vec<ConfigProblem>`. |
| Graph (decisions 2, 4, 5, 6, 13, 15) | `crates/archangel_graph`. `SourceSet::discover(config_dir, ignore_lines: &[String])` finds the repository folder (the nearest ancestor with `.git` as a folder or a file, else `config_dir`), records each nested `.git` below `config_dir` for the summary, and uses `ignore::WalkBuilder` with `hidden(false)`, `ignore(false)`, `git_global(false)`, `git_exclude(false)`, `parents(false)`, `git_ignore(true)`, `require_git(false)` and `follow_links(false)`; the `.gitignore` files between the repository folder and `config_dir`, and the config's ignore lines, are added through `ignore::gitignore::GitignoreBuilder`. It records skipped links, turns walk errors into `FileProblem`, checks file size against the parser's 4 GiB limit before reading, and records the dependency names of every package manifest it finds as `ManifestDependencies`. The graph crate does not depend on `archangel_config`: it takes plain strings. `text::decode` handles UTF-8 and UTF-16 with a byte order mark. Parse threads run in a `rayon` pool with a 64 MB stack. `SourceType::from_path(path)` gets `.with_jsx(true)` for `.js`, `.mjs` and `.cjs`. `imports::extract(&Allocator, path, text) -> Result<FileFacts, ParseFailure>` reads `ParserReturn::module_record` (requested modules, import entries, local, indirect and star export entries, dynamic imports), walks with `oxc_ast::Visit` only when `memchr::memmem` finds `require` or `import(`, reads triple-slash directives from the leading comments, and keeps comments whose text contains `archangel-allow`. `FileFacts { imports: Vec<Import>, exports: ExportNames, allow_comments: Vec<CommentText>, unfollowed: Vec<Span> }`. `Import { specifier, kind: ImportKind::{Runtime, Type}, form, span, line, column, names }`. `resolve.rs` wraps `oxc_resolver::ResolverGeneric<Overlay>` with `cwd` set to the repository folder, `node_path: false`, `yarn_pnp` set to whether `.pnp.cjs` exists at the repository folder (overriding the environment variable that `ResolveOptions::default()` reads), `extensions` `.ts .tsx .mts .cts .js .jsx .mjs .cjs .json .d.ts`, `extension_alias` in TypeScript's order, `.js` → `.ts .tsx .d.ts .js .jsx`, `.jsx` → `.tsx .ts .d.ts .jsx .js`, `.mjs` → `.mts .d.mts .mjs`, `.cjs` → `.cts .d.cts .cjs`, `condition_names` `types import module node default require`, `main_fields` `types module main`, `builtin_modules: true`, and the crate's `yarn_pnp` feature on. The options set `tsconfig: Some(TsconfigDiscovery::Auto)`. Archangel calls `find_tsconfig` for each importing file (once per folder when no tsconfig on the walk has `include`, `exclude`, `files` or `references`), records every `tsconfig.json` path the walk checked, the `references_resolved` of the result, and every file the resolver read through the `Overlay` file system, which holds each config an `extends` chain loads and each manifest; the simulated sessions edit an extended config to prove the record complete. It checks that each existing tsconfig can be read, and passes the result to `resolve_with_context`. `ResolutionInputs { contents: Set<Path>, existence: Set<Path> }` is a field of `ImportGraph`, which `replace_file` and `apply_file_events` only add to and `resolve_all_again` rebuilds after the cache is cleared: `contents` holds the tsconfigs, the manifests the resolver read and `.pnp.cjs`; `existence` holds each `ResolveContext`'s other `file_dependencies` and all its `missing_dependencies`. `ResolveError` is `#[non_exhaustive]`, so its `match` has named arms for the 23 variants of 11.23 and a wildcard arm that gives a plain reason naming the specifier and where to report it, never the error's `Display` text; a unit test lists the 23 known variants so that an oxc update with a new one fails a test. The named arms: `NotFound`, `MatchedAliasNotFound`, `ExtensionAlias`, `PackagePathNotExported`, `PackageImportNotDefined`, `InvalidPackageTarget`, `InvalidModuleSpecifier`, `Specifier`, `PathNotSupported`, `Ignored`, `Unimplemented`, `Recursion` and `IOError` become an `UnresolvedReason` with plain words and the path; `Builtin { resolved, .. }` becomes `Resolution::Package` named by `resolved` with the `node:` prefix; `Json`, `TsconfigNotFound`, `TsconfigCircularExtend`, `TsconfigSelfReference`, `InvalidPackageConfig`, `InvalidPackageConfigDefault`, `InvalidPackageConfigDirectory`, `FailedToFindYarnPnpManifest` and `YarnPnpError` become one `ResolutionFileProblem` per file, deduplicated by path.  `Resolution::{File(FileId), Package(PackageName), Unresolved(UnresolvedReason)}`. `ImportGraph { files, facts, resolutions: Vec<Vec<Resolution>>, importers: Vec<Vec<FileId>> }` indexed by `FileId(u32)`, with `replace_file`, `apply_file_events` and `resolve_all_again`, and petgraph's `IntoNeighborsDirected`, `IntoNodeIdentifiers` and `Visitable` traits. `Overlay` implements `oxc_resolver::FileSystem` over `FileSystemOs` with in-memory files for open documents and examples. `Work` and its `Stage` timings live here because this is the lowest crate that every counting stage depends on; each crate adds to the counters that belong to its stage. |
| Rules (decisions 7, 8, 11, 12) | `crates/archangel_rules`, pure: no `std::fs`, no printing. `Architecture::compile(&Config) -> Result<Architecture, Vec<ConfigProblem>>` builds the element table and `RuleTable` (cells indexed by class pairs, each a `SmallVec<RuleCheck>`; see the Selectors and classes row). `classify.rs` maps `FileId` and packages to `Instance { element, captures, root }`. `evaluate.rs` gives `Vec<Diagnostic>` for one file or all files, applies `allowUnresolved` to `Resolution::Unresolved`, and uses `ManifestDependencies` for package elements in `element-empty`. `cycles.rs` calls `petgraph::algo::kosaraju_scc`, lists every file of each group, finds the shortest cycle with a breadth-first search that visits neighbours in path order, which settles ties, and counts the files it visits in `Work`. A group's `ViolationId` is the rule and the sorted file paths. `exceptions.rs` parses the `archangel-allow` grammar from `CommentText` and matches runs to imports. `baseline.rs` holds `BaselineEntry`, `Baseline::matches`, `add` and `prune` over data only. `examples.rs` builds `ExampleView`, which overlays the example's `FileFacts` on the graph without changing it, and limits the cycle search to the example files and the repository files that reach a replaced file (a reverse walk over `importers`). `diagnostic.rs` holds `Diagnostic`, `Code` and `ViolationId`; `ViolationId::baseline_key` drops the occurrence. |
| Engine (decisions 3, 6, 13, 14, 16) | `crates/archangel_engine`. `Session::open(root, config_path)` runs a full check; every command and the language server start through it. `Session` owns the `ImportGraph`, `Architecture`, open documents, package use counts and the last diagnostics; `Session::apply(Change)` takes `Change::{Text, FileEvents, ResolutionInputs, Config, Baseline}` and returns the files whose diagnostics changed. `events.rs` turns a batch of raw path events into changes, in this order: it nets creations, deletions and renames against the paths known before the batch; a source path that existed before and after is `Change::Text`, and one that appeared or disappeared is `Change::FileEvents`; the config path is `Change::Config`, which walks again when the ignore list changed; the baseline path is `Change::Baseline`; a `.gitignore` path walks again; a path in `ResolutionInputs::contents` is `Change::ResolutionInputs`; a path in `ResolutionInputs::existence` is `Change::ResolutionInputs` only when it appeared or disappeared; a manifest recorded in `ManifestDependencies` re-reads its dependency names; any other path, and any path of an open document while it is open, is dropped. `watcher.rs` owns the `notify` watcher with `notify-debouncer-full`, the full check after `need_rescan()` or an overflow, and the switch to `notify::PollWatcher` with a note when the system limit is reached; watch mode and the language server both use it. `baseline_file.rs` reads and writes `archangel.baseline.json` with stable sorting and formatting. `fixes.rs` builds and verifies entry fixes from `FileFacts` names and `ImportGraph` resolutions, following `export *` through star export entries; its tests are in `crates/archangel_engine/tests/fixes.rs`. `test_command.rs` runs examples with a fresh `Overlay` and resolver each. The work-count tests for `Session::apply` are in `crates/archangel_engine/tests/work.rs`; `crates/archangel_lsp/tests` keeps only what the protocol mapping owns. Closing an unsaved document is `Change::Text` with the text on disk. |
| Work counts and faults (decision 21) | `archangel_graph::Work`, a struct of `u64` counters passed by `&mut` and merged after parallel stages. Fail points with the `fail` crate's `fail_point!`, compiled in only with each crate's `failpoints` feature, which release builds never enable. `archangel_lsp` and `archangel` have a `failpoints` feature too, which turns on the features of the crates below them, because their tests use engine fail points. `scripts/check` runs `cargo test --workspace --features failpoints` after the plain test run, so CI runs every fault control. The fail points: `graph::parse_twice`, `rules::search_every_rule`, `engine::reread_per_example`, `engine::evaluate_graph_per_example`, `engine::write_examples`, `engine::reparse_per_fix`, `engine::full_check_per_edit`, `engine::evaluate_every_file_per_edit`, `engine::full_check_per_event` (file events in the engine and in watch mode), `engine::resolve_per_event` (file events and manifest events), `engine::full_check_per_config`, `engine::full_check_per_baseline`, `engine::skip_netting`, `engine::resolve_per_found_file`, `rules::search_whole_graph_per_example`, `rules::match_per_selector`, `rules::walk_per_file`, `engine::classify_every_file_per_edit`, `engine::evaluate_every_folder_per_event`, `engine::evaluate_exports_of_every_file`, `engine::send_every_file`, `engine::view_gains_a_field`, and three that simulate what tests cannot cause: `engine::watcher_reports_lost_events`, `engine::watcher_has_no_watches` and `engine::plugin_dies`. Fault-control tests hold `fail::FailScenario::setup()` for their duration, which runs them one at a time; binary tests set `FAILPOINTS` on a build with the feature. |
| Language server (decision 17) | `crates/archangel_lsp`: only the mapping between protocol messages and `Session` changes, and from `Diagnostic` to `lsp_types::Diagnostic` and code actions. Built on `lsp-server` and `lsp-types`. At initialize it reads `workspace.didChangeWatchedFiles.dynamicRegistration`; when it is true it registers one watch for `**/*` below the repository folder and passes every event to the engine's `events.rs`, which drops what the check does not depend on; otherwise it starts the engine's `watcher.rs` and sends `window/showMessage` when that watcher falls back to polling. Queued `didChange` notifications for one document are collapsed to the newest before `Session::apply`. Precedent: rust-analyzer's main loop. |
| Command line (decision 18) | `crates/archangel`, the published binary. `clap` derive for `check`, `test`, `explain`, `why`, `watch`, `baseline add`, `baseline prune` and `lsp`; `--format text|json`, `--stats`, `--config`. `render/text.rs` uses `annotate-snippets`; `render/json.rs` uses `serde_json` with `schemaVersion: 1`. `watch.rs` starts the engine's `watcher.rs`, applies each batch through `Session::apply`, and prints the polling note in each summary line when the watcher reports one. `why` is a breadth-first search over `ImportGraph`. Exit statuses 0, 1 and 2 come from one `Outcome` enum in `main.rs`, the only place that calls `std::process::exit`. |
| Test support | `crates/archangel_testkit` (`publish = false`): `FixtureRepo` writes a temporary repository from paths and texts, and generates large synthetic repositories for benchmarks. Used by the tests of every crate. CLI tests use `assert_cmd` and `insta` snapshots. LSP tests use `lsp_server::Connection::memory`. Property tests use `proptest`; the simulated sessions in `crates/archangel_engine/tests/sessions.rs` use `proptest-state-machine` to drive `Session::apply` against `Session::open`, so a failing session shrinks to the shortest sequence that fails, and watch mode and the language server share the proof. |
| Differential resolution tests | `crates/archangel_graph/tests/resolve_differential.rs` runs `scripts/resolve-with-typescript.mjs`, which calls `ts.resolveModuleName` with each fixture's tsconfig. `typescript` is pinned in the root `package.json`. |
| Fuzzing | `fuzz/` with `cargo-fuzz` targets `config`, `pattern` and `exception`. Runs on nightly Rust in a scheduled CI job, because `cargo-fuzz` needs it; the hand-written seeds, and each input that once crashed a target, live in `fuzz/seeds/`, and the corpus each run grows is kept in the Actions cache. |
| Mutation testing (decision 22) | `git diff origin/main... > pr.diff && cargo mutants --in-diff pr.diff` on each pull request and a full `cargo mutants` run weekly in CI. Exclusions in `.cargo/mutants.toml`, each with a comment that gives its reason. |
| Benchmarks | `scripts/bench` checks out the public repositories pinned by commit in `benches/repos.toml` and generates a synthetic repository with `archangel_testkit`, then runs `hyperfine --export-json` on `archangel check` for durations and collects the work counts from `--stats --format json`, in a scheduled job that uploads the results. Precedent: ruff and Biome benchmark with hyperfine. |
| npm packages (decision 20) | `npm/archangel/` with `package.json` (`optionalDependencies` on each platform package), `index.js` (`defineConfig` returns its argument), `index.d.ts` (the config types, with template literal types that extract capture names), and `bin/archangel.js`, which finds the platform package and runs its binary, or prints the platform and the `cargo install` alternative. `npm/platforms/<target>/package.json` for each of the seven targets. Precedent: esbuild, Biome, oxlint. Type tests in `npm/archangel/test/types.test-d.ts` run with `tsd`. |
| Release (decision 20) | `.github/workflows/release.yml`, modelled on Biome's release workflow: build matrix for `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` and `x86_64-pc-windows-msvc`; `cargo publish --workspace` with `rust-lang/crates-io-auth-action` for trusted publishing; `npm publish` with npm trusted publishing, which adds provenance; no stored registry tokens; checksums attached to the GitHub release; jobs `install-from-npm`, `install-from-crates-io`, `checksums-match` and `provenance-verified`. |
| Diagnostic documentation | `crates/archangel_rules/docs/<code>.md`, one per `Code`, built into the binary for `archangel docs`; a test checks each page names its code and says what it means. |
| Selectors and classes (decisions 7, 23) | `archangel_config`: `SelectorSpec::{Patterns, All, Any, Not, Ref, Element { name, capture }, AnyElement, Directive}`; a rule side is `Side::{Name, Pattern, Packages}`, and an inline `{ packages }` becomes an unnamed package element. Validation refuses a cycle between selectors and resolves each string as a declared name first. Shorthand kinds expand into `Rule { files, condition, filter, shorthand }` in `parse.rs`, so `Architecture::compile` reads one form; `shorthand` keeps the kind for `Code` and `explain`. `archangel_rules`: `Selectors::compile` puts every selector pattern into the `RegexSet` that elements already use; `classify.rs` gives each file a signature of its element and selectors as a `fixedbitset::FixedBitSet` and interns it to a `ClassId`; packages get a `ClassId` per package element. `RuleTable` becomes a `HashMap<(ClassId, ClassId), SmallVec<RuleCheck>>` filled for the connected pairs before evaluation. `Session` keeps each file's `ClassId` and a file count per class, classifies only the edited file, and re-checks its importers when its `ClassId` changes; `ImportGraph::replace_file` updates the importer lists of the old and new targets only, instead of `refresh()`. Precedent for the partition: regex-automata's byte classes. |
| Conditions | `archangel_config::Condition::{MayNotImport, MayOnlyImport, MayOnlyBeImportedBy, MustBeNamed, MustContain, MustNotExist, MustExport, MayNotExport, MustOnlyReExport, NoCycles, Plugin { condition, options }}` with `ImportFilter { imports, names, specifier, instance, dependency, transitively }`. `archangel_rules`: `imports.rs` (the table, with `dependency` answered from `ManifestDependencies` by the importing file's nearest manifest, found once per folder), `structure.rs` (naming per file and per distinct folder, over every visited path, `mustContain` per instance or folder from a set of names, `mustNotExist`), `exports.rs` (with the `export *` chain walk that `fixes.rs` already has, moved here so both use it), `reach.rs` (one backward breadth-first search per transitive rule from its targets over `importers`, keeping each visited file's next step, and counting visited files in `Work`). `naming.rs` holds the five styles and the `{FileName}` spellings; a property test compares it with the `heck` crate's conversions on names with acronyms and digits, such as `HTTPServer` and `v2Api`. |
| New file facts | `archangel_graph::FileFacts` gains `exports: ExportNames` with `default`, `star` and re-export marks from the module record's `local_export_entries`, `indirect_export_entries` and `star_export_entries`; `own_statement: Option<OwnStatement>`, the line, column and declared name of the first top-level statement that is not an import, an export-from, a directive, or an `export { … }` of imported bindings only, because a `mustOnlyReExport` violation points at that statement; and `directive: Option<CompactString>` from `Program::directives`. None needs a walk of the tree. `SourceSet::discover` also records the paths of the files it does not read, as `OtherFile`, for the structure rules. |
| Presets | `npm/archangel/presets/fsd.js` and `nextjs.js`, with the `fsd` plugin's seven conditions in `npm/archangel/presets/fsd-conditions.js`, written in JavaScript, with `Preset`, `fsd` and `nextjs` typed by hand in `index.d.ts`, because the package has no build step, as `plugin.js` has none. `load.mjs` calls each `extends` entry and sends the results unmerged, each with its preset name, then the config's own fields and `off`. `archangel_config::merge.rs` prefixes each preset's names, merges in order, applies `off`, and reports the prefix, duplicate-use and unknown-`off` problems with near-name suggestions. Empty preset elements and selectors go to the summary. |
| Plugin host (decisions 26, 27) | `npm/archangel/plugin.d.ts`: the view types and the types of `definePlugin` and `defineCondition`, written by hand, with the two helpers in `index.js`, because the package has no build step; a test compares the view types with the `serde` types in `crates/archangel_engine/src/view.rs`, which carry `schemaVersion: 1`. `definePlugin` takes a name, the schema version and a list of conditions, each with its own name. `crates/archangel_engine/src/plugins.rs` starts `load.mjs`, owns the child process and the newline-delimited JSON protocol on standard input and output: Node answers first with `config` (or `error`), and the engine then sends `rules` (each plugin rule with its condition and options), `file` (one record, with the plugin rules whose `files` hold it), `remove` (a path) and `check` (the rules to run), which Node answers with one `result`; closing standard input ends the process. A message from Node is read on its own thread and one to Node is written on another, so neither side waits for a busy other. It reads standard error on a third thread, gives `plugin-failed`, and starts the process again on the next change after it stopped; `archangel_config` only parses and validates the JSON it receives, through `from_loader`. The loader also sends each plugin without its code, `{ schemaVersion, conditions: { name: { description, docs, reads, options } } }`, which `archangel_config` validates: the schema version, each rule's condition name, and its options with the `jsonschema` crate. A JSON config cannot load a plugin, because a plugin is code, so the parser and the JSON schema refuse `plugins` in one. A plugin violation has the code `plugin`, and the work counts gain `pluginRecordsSent` and `pluginChecks`. The editor shows waiting through LSP work-done progress. The fail points `engine::plugin_dies` and `engine::send_every_file` stand in for a stopped process and a bad filter, and `engine::view_gains_a_field` for a later archangel whose view has a field no plugin knows; a fixture plugin that waits on a gate the test opens stands in for a slow one. The loader records the files the config imported, through a Node module resolution hook, so a change to a preset or plugin file reloads the config. |
| Feature-Sliced Design differential | `crates/archangel/tests/fsd_differential.rs` runs `steiger` (pinned in the root `package.json`) and `archangel check` with the preset on the fixtures in `tests/fixtures/fsd/`, and compares the files and rules each reports, through a table that maps each Steiger rule name to its preset rule. |
| New abstractions and their uses | `Pattern` and `ElementMatcher` (classification, entry files, explain); `ImportGraph` (check, test, explain, why, watch, lsp); `FileFacts` (rules, exceptions, fixes); `Overlay` (open documents, examples, fix resolution); `Architecture` and `RuleTable` (every evaluation); `Diagnostic`, `Code` and `ViolationId` (every output, the baseline key); `Work` (every stage, every work-count test, `--stats`); `ExampleView` (examples only, because an example must not change the graph that the next example sees); `Session` (check, test, watch and lsp); `Change` (watch and lsp); `events.rs` (watch and lsp's registered watches); `watcher.rs` (watch mode and lsp's fallback); `ResolutionInputs` (event routing and the watched set); `ManifestDependencies` (package elements in element-empty); `FixtureRepo` (tests of every crate). |
| Conflicts with the brief and the code | The brief puts plugins and presets in a later release, and calls rules about ownership part of plugins; this design brings presets, plugins and the sentence form into the first release at Haziq Nordin's request on 2026-10-30, before anything is published, so the config format changes only once (Decisions that need you, 22). The built code classifies every file and rebuilds every importer list after each edit, which the per-edit costs in this design already ruled out; this change wins over that code, and a work count now holds it (Cost). The brief's later "structural pattern rules" for code inside a file, and its "extractors that add new kinds of facts" such as CSS, stay later: plugin conditions read the graph, not syntax trees or other languages. The brief says archangel removes a fixed baseline entry; this design reports it and removes it only with `baseline prune` (Decisions that need you, 6). The brief says archangel's repository checks its own architecture with archangel; archangel reads TypeScript, so its Rust layering is enforced by the crate graph and archangel checks only the TypeScript in `npm/` (Decisions that need you, 10). The brief asks for a mutation score that only goes up; this design holds it at no surviving change, which is stricter (decision 22). The brief permits per-edit work that grows with the repository only where nothing smaller is correct; resolving every import after a file is created is simpler than the correct smaller method, and the design raises it (Decisions that need you, 11). |
