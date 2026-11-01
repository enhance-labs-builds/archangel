# Design: scoped imports and import tables

## In short

A config written with archangel should read as the architecture, and a developer should know what
each rule means without guessing. Archangel 0.1 checks enhance3, a 9,800-file monorepo, with no
problems, but its config is an 828-line builder that reads a 2,896-line vocabulary and produces
about 380 rules with computed names. Two cold readers tested a proposal for a better interface. A
junior engineer understood the model and could write a config, and a senior staff engineer who knows
Nx, eslint-plugin-boundaries, ArchUnit and dependency-cruiser called the direction better than all
of them. Both rated it 2 of 5, because the proposal's own examples broke its own meaning. So this
design writes the meaning down as scenarios with tests, and makes every config example in the docs
a fixture that archangel itself checks.

With 0.2, a developer writes four new things:

- **A table**, `mayImport('<table>', { <row>: [<names it may import>] }, { <options> })`. Each row
  is a named group of files and the names it may import. Rows can be kinds of file, parts or groups,
  mixed in one table. Tables fail closed: a file with a row is judged on every import it makes into
  the repository, and an import that its row does not list is refused, with the row named. A row may
  import itself, in the same instance, unless `mayNotImportItself` lists it.
- **Kinds.** A kind is a name that a table's one pattern with `{kind}` gives a file, such as
  `**/*.{kind}.ts` (so `board.entity.ts` is an `entity`). A row is a kind, or a declared part or
  group; a part or group row is not a kind. Every table also makes the rule
  `<table>.every-file-has-a-kind`, which refuses a file in the table's files that has no row.
- **New spellings**: `parts` for elements, `groups` for selectors, `externals` for named sets of
  outside packages, `because` for the reason, and `rule('<name>', { ... })` with the name first.
- **`within`** on `mayOnlyImport` and `mayOnlyBeImportedBy`: the area whose imports the rule judges.

And archangel answers two questions it could not answer before. `archangel rules [path]` prints
every rule as the sentence it means, with its reason, the file where it is written, and its reach:
the files and imports it judged, its problems and its exceptions. A rule or row that covers no file
says that it checks nothing. `archangel explain <file>:<line>` shows, for one import, every rule
that judged it and every rule that skipped it, and why.

Every 0.1 config keeps working, with two exceptions that "Decisions that need you" names for a
decision: `mayOnlyImport` on a part with a capture now permits only its own instance (item 9, a
deliberate change of default, so that tables and rules agree), and type imports under
`verbatimModuleSyntax` are fixed (item 10, where 0.1 gives a plausible wrong answer).

This is the small app of the docs, written with 0.2. It is a fixture that a test checks, so it can
never disagree with what archangel does:

```ts
import { defineConfig } from '@enhance-eng/archangel';

export default defineConfig({
  parts: {
    feature: 'src/features/{feature}/**',   // each folder in src/features is one feature
    shared: 'src/shared/**',
  },
  rules: ({ mayImport }) => [
    mayImport('app', {
      feature: ['shared'],
      shared: [],
    }, {
      because: 'Features share code only through src/shared, and shared code never depends on a feature.',
    }),

    mayImport('feature-code', {
      views: ['models', 'shared'],
      models: ['api', 'shared'],
      api: ['shared'],
    }, {
      kinds: 'src/features/*/{kind}/**',     // src/features/cart/views/cart-page.tsx is in views
      because: 'A view shows a model. Only the api folder talks to the server.',
    }),
  ],
});
```

A view that fetches data gives this error, which names the row, what it may import and why:

```
error[feature-code.views]: views may not import api, through "../api/fetch-cart"
 --> src/features/cart/views/cart-page.tsx:2:27
  |
2 | import { fetchCart } from '../api/fetch-cart';
  |                           ^^^^^^^^^^^^^^^^^^^ views may import only: models, shared
  |
  = note: rule: feature-code.views, in archangel.config.ts:16
  = note: because: A view shows a model. Only the api folder talks to the server.

checked 7 files and 9 imports: 1 problem
```

Read the tables as sentences: "a feature may import shared code, and itself"; "a view may import
models and shared code". The first table also refuses an import from the cart feature into the
login feature, because a row permits itself only in the same instance. No rule grants anything: an
import must pass both tables.

Three ideas from 0.1 carry this design. A rule is made of **sentences**, each one condition, such as
`mayOnlyImport`, on a set of files; a shorthand stands for one or more sentences. Archangel checks an
import by the pair of **classes** at its ends: a class is the set of files that are in the same part
and exactly the same groups, and an outside package has a class too. A **pair entry** holds, for one
pair of classes, the sentences that pair can break. New sets, such as a row or an area, become part
of each class, so the new forms cost what the old ones cost.

**How it works:**

1. Archangel loads the config. Each table row becomes one rule named `<table>.<row>`, and every
   table, with or without a kind pattern, gets the rule `<table>.every-file-has-a-kind`; for a table
   of parts or groups with no `files` option it covers no file, and `archangel rules` prints it as
   checking nothing. From then on each is a rule like any other, so `off`, exceptions and the
   baseline name it.
2. Archangel classifies each file once: its part, its groups, and its kind in each table. Each
   outside package an import names is in the externals sets that list it.
3. Before it looks at any import, archangel fills one pair entry for each pair of classes that an
   import connects, with the sentences that pair can break. A table row's sentence, its area and its
   own-instance permission are decided there. The instance test of that permission is the one part
   that runs per import, and only for imports inside one row whose name has a capture (Cost).
4. After the check, the summary lists the rows that have no file. A table with no row, or whose rows
   together have no file, is `table-empty` (item 12 of "Decisions that need you").
5. `archangel test` runs the author's written examples. A table row needs none (item 5 of
   "Decisions that need you").
6. `archangel rules` and `archangel explain` read the same compiled rules, so what they say is what
   the check does.

Eleven terms are new or renamed, and one is widened. A **part** is what 0.1 calls an element: a named set of files, and a
file is in at most one of the config's parts. A **group** is what 0.1 calls a selector: a named set of
files, and a file can be in any number of groups. An **externals set** is a named list of outside
packages, such as `driver: ['pg', 'drizzle-orm']`. An **area** is the set of files and packages,
written with `within`, whose imports a rule judges. A **table** is a `mayImport` shorthand. A **row**
is one key of a table and the rule made from it. A **cell** is one entry in a row: a name or a path
pattern that the row's files may import. A **kind** is a name that a table's kind pattern gives a file; a part or group row is not a kind. A **kind
pattern** is a table's one pattern with the capture `{kind}`. The **reach** of a rule is what it
covered in one check: its files, the imports it judged, its problems, its exceptions and its baseline
matches. An **instance** (the glossary's "element instance", widened) is one value of a capture in a part's
pattern, or of a capture other than `{kind}` in a kind pattern: in `src/features/{feature}/**`, the
folder `cart` is one instance of `feature`. A group, an inline pattern and a name without a capture
have one instance, so a rule or row on them permits itself everywhere. An **unjudged** file or
import is one that no rule looks at.

### Decisions that need you

Each item gives the recommendation that the specs follow, and the other options. Items 1 to 8 are the
questions the owner listed; items 9 to 12 came up while the change was written.

1. **Outside packages are externals, not parts.** The config gets a third key, `externals`, whose
   values are lists of package patterns: `driver: ['pg', 'drizzle-orm', '@aws-sdk/*', 'node:fs']`. A
   pattern means what it means in a 0.1 package element, so a list moved from `elements` keeps its
   meaning: `react`, `@aws-sdk/*`, `node:*`, `*`, and a wildcard such as `knex*`. An import counts as
   the package it resolves to, so `pg` covers `pg/lib/client`. An import of a built-in module counts
   under its `node:` name, written with or without the prefix, so `node:fs` covers `fs`, and
   `fs/promises` is the separate module `node:fs/promises`. Four names could never match any import
   and are refused: a bare `fs`, with the fix `node:fs`; a name under `@types/`, because code never
   imports `@types/react`; a package of the workspace, because an import of it resolves to a
   repository file and that file's part classifies it; and a name with a subpath, such as
   `pg/lib/client`, with the fix `pg`. Which names are built-in comes from the same list that the
   resolver uses. A table judges an import of an outside package only when a cell, or
   `everyRowMayImport`, names an externals set that holds it; `react` stays the business of other
   rules. `mayOnlyImport` without `within` keeps its 0.1 meaning and judges every import, outside
   packages included. The other options: package parts in `parts`, as the interface proposal that
   the two readers tested did and as 0.1 allows in `elements`, which makes "a file is in one part"
   false and lets a table judge packages that no row thought about; `{ packages: [...] }` inline
   only, which repeats each list in every rule that names it; or matching subpaths of npm packages,
   which needs the graph to record the subpath of every package import, for a case nobody has
   shown. The 0.1 package elements keep working in `elements`.

2. **A name means the same files everywhere, and a table's `files` only choose whose imports it
   judges.** A kind applies in the whole repository: `apps/api/src/board/application/draft.entity.ts`
   is an `entity` for the domain table even though it is outside the table's files, so the cell
   `entity` permits it. A target in no row of the table is refused with "no row for" and the most
   exact name archangel has for it: its kind if the kind pattern gives it one, else its part, else
   its path with "which is in no part". So an entity that imports a use case reads "no row for
   use-case", and one that imports a contract file with no kind reads "no row for contract". The
   other option, kinds only inside the table's files, makes a cell mean different files depending on
   where the importer is, and makes every target outside the files read "no row for api", which
   tells the developer nothing.

3. **Every table makes the rule `<table>.every-file-has-a-kind`, and there is no option to leave it
   out.** Both readers want every file in a table to have a row. A team that cannot have that yet
   turns the rule off with `off: ['<table>.every-file-has-a-kind']`, as for any rule, and
   `archangel rules` then shows it as off. The rule refuses a file in the table's files that has a
   kind with no row: in the small app, `src/features/cart/hooks/use-cart.ts` is refused with "no
   row for hooks". Archangel exempts no file by itself: tests, `index.ts`, generated files and tool
   configs that should have no kind are left out of the table's `files` with `not`, for example
   `files: { all: ['domain', { not: ['tests', '**/index.ts', '**/*.d.ts'] }] }`; the enhance3 slice
   leaves out only `tests`, so an `index.ts` in a domain folder would be refused there. When a table
   has no `files`, its files are only those that its kind pattern matches with any kind, so a file
   with no kind, such as `src/features/cart/index.ts` in the small app or `board.ts` in a domain
   folder, is outside the table and judged by no row: the gap where a file escapes a table by
   dropping its suffix stays open. Writing `files`, as the enhance3 slice does, is what closes it,
   because then every file in `files` must have a kind with a row. The kind is what `{kind}` matches,
   as for any capture; in `**/*.{kind}.ts` the `*` before it takes the most it can, so the kind is
   the word just before the extension: `board.entity.test.ts` is the kind `test`, and
   `board.entity.ts` is `entity`. A declaration file counts `.d.ts` as its extension, so
   `board.entity.d.ts` is an `entity` and `env.d.ts` has no kind. In `src/features/*/{kind}/**` a
   folder `api.v2` is the kind `api.v2`, and it needs a row or gives a violation. The other option
   is an `everyFileHasAKind` option on the table, off by default with a summary count; it is gentler
   on a codebase without suffixes, but it is a second spelling of `off` with a second output form,
   for a user this change cannot name.

4. **One spelling per concept, chosen now and kept to 1.0: `parts`, `groups`, `because`, and
   `rule('<name>', { ... })`.** The docs, the README, `archangel init` and every example use only
   these. The 0.1 spellings, `elements`, `selectors`, `reason` and `rule({ name, ... })`, keep
   working through 0.x so that 0.1 configs load, and 1.0 removes them. A config that writes both
   spellings of one key, such as `parts` and `elements`, or `because` and `reason` on one rule, is
   refused with `config-invalid`, so a config never holds two spellings for one thing. The cost of
   aliases is then bounded to 0.x. The other options: keep the 0.1 spellings only, which leaves the
   docs saying "element" for what readers call a part; or keep both spellings forever, a cost that
   never goes away.

5. **A table row needs no written example, and archangel generates none.** `rule-unproven` does not
   apply to table rows or to `<table>.every-file-has-a-kind`. What shows that a row can fire is what
   boundary tools such as eslint-plugin-boundaries and dependency-cruiser give, which demand no
   per-rule proof: the summary lists each row with no file, `table-empty` fails a table that checks
   nothing, and `archangel rules` prints each row's reach. That archangel decides every cell as
   written is shown by the property tests, which compare each table with one hand-written rule per
   row. A written example still runs and names the row that rejected it; authors write one where
   intent is easy to get wrong. Rules that are not table rows keep 0.1's demand for a written
   rejected example, so 0.1 results do not change. The other options: one written rejected example
   per table, which the senior reader rejects, because enhance3 would need 61 hand-written examples
   for its rules and tables, and they will not get written; or a generated proof per row, a rejected
   example that `archangel test` builds from the row's first file and the first repository file the
   row refuses. The proof cannot fail on a correct build of archangel: the second file is chosen
   because the row refuses it. What it gives you is a check that archangel's own table logic works,
   which the property tests already give, and a list of rows that have no file or refuse nothing,
   which the summary and `archangel rules` already give. It never tells you that a row's cells are
   wrong, and it is a new gate in `archangel test` with its own outcome and its own cost.

6. **Keep the capture syntax `{name}` in 0.2.** Braces with a comma list alternatives, as in
   `{ts,tsx}`, and braces without one are a capture, as in `{feature}`. The clash is a brace with
   one word, such as `**/*.{ts}`, which is a capture named `ts`. `archangel rules` and
   `archangel explain` show every capture as an instance, so the mistake is visible on first use. A
   new syntax, such as `[feature]`, removes the clash but breaks every 0.1 config that has a
   capture. If it is wanted, 0.x can accept both and 1.0 can drop the old one, as item 4 does.

7. **In `parts`, two parts that match one file are an error, `part-overlap`.** The order of keys then
   cannot change a verdict. A preset's parts are a separate set: a file can be in the config's part
   `web` and in the preset's part `fsd-web/features`, and each rule sees the part it names. In 0.1
   `elements`, the first match still wins, so 0.1 configs keep their results; moving a config from
   `elements` to `parts` shows each overlap as an error, never as a changed verdict. Inside one table,
   a file in two rows is an error too, `table-row-overlap`, because one row must say what a file may
   import. The other options are the first match, which makes key order a hidden rule, or a file
   judged by each part it matches, which no rule author expects.

8. **Ship this as one release, 0.2.0.** The docs and the fixtures must be written once, in the final
   spellings, and `archangel explain` of one import and the unjudged report reuse the reach counts
   that `archangel rules` needs. The change is large: the tasks list about 150 new tests. The other
   option is two releases: 0.2.0 with the meaning, the spellings, tables, kinds, externals, `within`,
   `archangel rules` and the fixtures; then 0.3.0 with `archangel explain` of one import and the
   unjudged report. If you choose that, those two requirements move to a new change.

9. **A row and a `mayOnlyImport` rule both permit their own name in the same instance.** For a part
   with a capture, such as `feature`, "itself" is the same feature; a cell that names the part
   permits every instance. For a kind, "itself" is the same kind, in the same instance of any other
   capture in the kind pattern. A group, an inline pattern and a name without a capture have one
   instance, so a rule or row on them permits itself everywhere, as in 0.1; the only 0.1 rules that
   change are `mayOnlyImport` rules whose files are a part with a capture. One default for tables
   and rules is what the readers asked for, and
   it is the answer a reader expects from "features may import only shared code". In 0.1,
   `mayOnlyImport` permits the rule's own files in any instance, so a 0.1 rule on a part with a
   capture now refuses imports between instances that it permitted. That breaks the promise that
   every 0.1 config gives the same results, for those rules only, and the changelog lists it under
   "Changed". The other option keeps 0.1's any-instance permission for rules, which is additive, but
   then tables and rules have two defaults, and a table of features needs an `isolate` rule beside
   it.

10. **An import's runtime or type mark follows what TypeScript emits for the file's options.** 0.1
    decides it from syntax alone: a statement whose names all carry `type` is a type import. That
    builds TypeScript's default into archangel, and it is wrong under `verbatimModuleSyntax`, where
    TypeScript keeps `import { type A } from './a'` as `import {} from './a'` and
    `export { type A } from './a'` as `export {} from './a'`, so the module runs. A `noCycles` rule,
    which counts runtime imports only, then misses a cycle that runs, through an import or through
    a barrel's re-export. The flaw is one inference that covers both statement forms, so 0.2 fixes
    the inference: a statement is a type import when it has the `type` modifier, or when every name
    it imports or re-exports has the modifier and the file's tsconfig does not set
    `verbatimModuleSyntax`. The mark is set when the import is resolved, where the file's tsconfig
    is already in hand, so a change to a tsconfig re-marks every import without a parse, in
    `archangel watch` and in the editor as in a check. Without the option, nothing changes.
    `export type * from './x'` is a
    type import, and so is `import('./x').T` in a type, as in 0.1. Two other inputs give the same
    class of wrong answer, and both stay as Limits: without the option, TypeScript drops
    `import { A } from './a'` when `A` is used only as a type, which archangel still counts as a
    runtime import; and the options `importsNotUsedAsValues` and `preserveValueImports` are not
    read. This changes results for 0.1 configs on repositories with `verbatimModuleSyntax`, and the
    changelog lists it under "Fixed". The other option is to keep the 0.1 answer and document it,
    which leaves a plausible wrong answer.

11. **enhance3's config written by hand is about 1,150 to 1,350 lines** ("What enhance3 gains").
    Recommendation: ship 0.2 without one-line examples and measure the rewrite (task 9.1 records the
    slice). A one-line example is a rejected or accepted example written as one import string in
    the rule, such as `'entity -> use-case'`, in place of an example object with source files. An
    earlier interface sketch estimated about 700 lines; it assumed one-line examples (about 250
    lines) and left out the literal file lists in enhance3's own rules (about 175 lines). The other
    option is to add one-line examples to this change, which saves about 250 lines in enhance3 and
    adds a new form to specify and test.

12. **A table that checks nothing fails the check, `table-empty`.** A table with no row, or whose
    rows together have no file, checks nothing while the check stays green, and the usual cause is
    a misspelt kind pattern or row names. A single row with no file is only listed, because a kind
    that no file has yet is the team's declared intent; a whole table with no file is not. A
    preset's table is the exception: it gives one summary line, as a preset's empty parts do,
    because `preset-empty` covers the preset. The other option is a summary line for every table,
    as for a preset's table, which is gentler during adoption but lets a table that checks nothing
    pass.

13. **`within` is refused on `mayNotImport`, with the fix in the message.** A `mayNotImport` rule
    already names exactly what it refuses, so `mayNotImport: 'use-case', within: 'apps/api/src/**'`
    says nothing that `mayNotImport: { all: ['use-case', 'apps/api/src/**'] }` does not. The message
    gives that form, with the rule's own side and area, so there is one way to write it, as in item
    4. The other option accepts `within` on `mayNotImport` with that meaning, which lets a developer
    learn `within` once for all three import conditions, at the cost of a second way to write the
    same rule. `within` on a condition that checks no import, such as `mustBeNamed`, has no meaning
    and is refused either way.

## How it works

### Parts, groups and externals

The config names its parts once, and every rule after that talks about names, not paths:

```ts
parts: {
  api: 'apps/api/**',
  web: 'apps/web/**',
  contract: 'packages/contract/**',
},
groups: {
  tests: ['**/*.{test,spec}.{ts,tsx}', '**/__tests__/**'],
  domain: 'apps/api/src/*/domain/**',
},
externals: {
  driver: ['pg', 'drizzle-orm', '@aws-sdk/*', 'node:fs'],
},
```

A part is a set of files, and a file is in at most one of the config's parts; two parts that match
one file give `part-overlap` with both names, the file, and the way forward: narrow one pattern with
`not`, or make one of them a group. Every overlap is fixed in the config, because the baseline holds
violations of rules and this is not one. A group is a set of files, and a file can
be in any number of groups. An externals set is a list of outside packages, matched as item 1 of
"Decisions that need you" says. A name means one of the three, and a rule side, a table cell and an
area can use any of them. A string that names nothing is a path pattern when it holds `/` or `*`,
and is otherwise refused with the closest name, as in 0.1.

### Patterns and where they start

Every pattern in `parts`, `groups`, `files`, `within`, `kinds` and a cell starts at the folder of the
config, the repository root in a usual setup, and never gets an implicit `**/`. So
`src/domain/{kind}/**` matches only the root's `src`, and a monorepo writes `**/domain/{kind}/**`.
An `entry` pattern starts at the root of the instance, as in 0.1; for a part without a capture, that
root is the folders before its first wildcard, so `entry('canvas-document', 'src/*/index.ts')` on
`packages/canvas-document/**` means `packages/canvas-document/src/*/index.ts`.

A capture, such as `{feature}`, matches one or more characters of one folder or file name and makes
an instance. Braces with a comma list alternatives. The capture `{kind}` matches as any capture
does, and differs in two ways: it may appear more than once in a kind pattern, each place taking the
same name, and a declaration file counts `.d.ts`, `.d.mts` or `.d.cts` as its extension for it
(Decision 2).

### A table

```ts
mayImport('domain', {
  entity:           ['value-object', 'domain-event', 'specification', 'policy', 'error', 'types', 'consts'],
  'value-object':   ['error', 'types', 'consts'],
  // ...
}, {
  files: { all: ['domain', { not: 'tests' }] },
  kinds: '**/*.{kind}.ts',
  everyRowMayImport: ['apps/api/src/shared/result.ts'],
  mayNotImportItself: ['aggregate-root', 'domain-event', 'policy', 'port'],
  because: 'Each kind of domain file depends only on the kinds below it.',
})
```

The first argument is the table's name, the second the rows, and the third the options. Each row key
is a declared part or group, or, when it names neither and the table has a kind pattern, a kind. A
key that names an externals set is refused, because packages have no files that import. Each cell
names a row of the table, a declared part, group or externals set, or a path pattern; anything else
gives `rule-unknown-element` with the closest name, so a misspelt cell never passes. The options are:

- `files`: the files whose imports the table judges. By default, the files its kind pattern matches
  with any kind, together with the files of its part and group rows. A row's files are the files of
  its name that are in the table's files.
- `kinds`: the kind pattern. Without it, every row key must be a declared part or group.
- `within`: an area that narrows which targets the table judges. Without it, the area of each row
  is every repository file and the externals sets that the table names in a cell or in
  `everyRowMayImport`.
- `everyRowMayImport`: cells that every row also has, such as a shared Result type.
- `mayNotImportItself`: rows that do not permit their own name. A row in this list that also names
  itself in its cells is refused with `config-invalid`.
- `because`, `link`, `imports`, `names`, `specifier`, `dependency` and `instance`: they belong to
  the table and apply to each row, as on any rule. `instance` filters the imports a row judges, as
  in 0.1: with `instance: 'different'`, each row judges only imports between two instances of its
  name, and a row whose name has no capture gives `rule-needs-capture`.
- `examples`: written examples, run once against all the rows.

**How a table decides an import.** A file in the table's files that has a row is judged on each
import it makes whose target is a repository file, or an outside package in an externals set that
the table names. An import that does not resolve gives `import-unresolved` and is never a
violation. The import is permitted when the target is in one of the row's cells, in
`everyRowMayImport`, or in the row's own name and the same instance, unless `mayNotImportItself`
lists the row. Otherwise it is refused. The message says what the row may import, and names the
target: by its row when it has one, and otherwise with "no row for" and its kind, its part or its
path (item 2 of "Decisions that need you"). In the enhance3 slice of "What enhance3 gains", an
entity that imports a use case gives:

```
error[domain.entity]: entity may not import use-case, through "../application/save.use-case"
 --> apps/api/src/board/domain/board.entity.ts:3:22
  |
3 | import { save } from '../application/save.use-case';
  |                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ entity may import only: value-object, domain-event, specification, policy, error, types, consts, apps/api/src/shared/result.ts
  |
  = note: no row for use-case: the table domain has no row for this kind
  = note: rule: domain.entity, in archangel.config.ts:62
  = note: because: Each kind of domain file depends only on the kinds below it.
```

A file in the table's files with no row gives one violation of `<table>.every-file-has-a-kind`, a
rule that every table makes and that `off` can name. The message says whether the file has no kind,
and how to give it one, or has a kind with no row, and which row is closest:

```
error[domain.every-file-has-a-kind]: apps/api/src/board/domain/board.spec-helper.ts has the kind spec-helper, and the table domain has no row for it
  = help: add a row for spec-helper, rename the file, or leave it out of the table's files with not
```

A file in two rows of one table gives `table-row-overlap`, which names the file and both rows and
says to narrow one of the two with `not` or to leave the file out of the table's `files`; the
file is judged by both rows, so no import passes in silence while the error stands, and the overlap
is fixed in the config, because the baseline does not hold it. A row key that
names a declared part or group means it, even when the kind pattern also gives that name as a kind
to some file. `archangel explain` and `archangel rules` show that the key means the part or group,
and a file with that kind that is not in the part or group falls under `every-file-has-a-kind`, whose
message says that the row key of that name means the part or group, not the kind.

**Rows with no file, and a table with none.** A kind that no file has yet is the team's declared
intent, so its row gives no diagnostic and the kind pattern never gives `selector-empty`. The summary
lists the row, and `archangel rules` shows it as checking nothing. A row that names a part or group
is covered where that name is declared: a part or group that matches nothing gives `element-empty`
or `selector-empty` once, at its declaration, and the row adds nothing. A table with no row, or
whose rows together have no file, gives `table-empty` (item 12 of "Decisions that need you"); it is
not given when the table's `files` or area pattern is already
`selector-empty`. A preset's table never gives `table-empty`; it gives one summary line, as for a
preset's empty parts.

**The sentences of a row.** Each row compiles to one rule of one `mayOnlyImport` sentence, with the
row's cells, `everyRowMayImport`, the area, and the own-instance permission that every
`mayOnlyImport` has. When the table has no `within`, the area of each row is every repository file
and the externals sets that the table names in a cell or in `everyRowMayImport`; with `within`, it
is that area. A row in `mayNotImportItself` gets a second sentence, `mayNotImport` of its own name.
`archangel explain` and `archangel rules` show these sentences, and the docs page for tables shows
both shapes. Because each row is an ordinary rule, `off: ['domain.policy']` drops one row,
`// archangel-allow domain.entity: …` permits one import, and the baseline records the row's name.
A table's name in `off`, such as `off: ['domain']`, turns off every rule that the table makes: each
row and `domain.every-file-has-a-kind`, including rows added to the table later.

### A row's own name, and instances

A row, and a `mayOnlyImport` rule, permit an import of their own name in the same instance (item 9
of "Decisions that need you"). An instance comes from a capture: in a part such as
`src/features/{feature}/**`, and in a kind pattern's other captures, such as `{context}` in
`apps/api/src/{context}/domain/**/*.{kind}.ts`. A group, an inline pattern and a name without a
capture have one instance, so a rule or row on them permits itself everywhere. The values of a kind
pattern's other captures are recorded for each file when it is classified, beside the values of its
part's captures, so the instance comparison of a kind row reads stored values and matches no pattern
per import (Cost). A cell that names the row's own name permits every instance:
`feature: ['feature', 'shared']` lets one feature import another. The rule `isolate` stays for teams
that want it without a table.

### Outside packages

A table judges an import of an outside package only when one of its cells names an externals set
that holds the package. So a table that names `driver` in one row refuses `pg` in every row that
does not list `driver`, and a table that names no externals set never looks at a package import. An
import of a workspace package resolves to a repository file, and that file's part and kind decide
it, as for any file. On a rule, an externals set is a side like any other: `rule('core-not-to-driver',
{ files: ['domain', 'application'], mayNotImport: 'driver' })`. `mayOnlyImport` without `within`
judges every import, outside packages included, as in 0.1.

### An area on a rule

`within` takes what a rule side takes: names, path patterns, externals sets, and `all`, `any` and
`not`. On `mayOnlyImport`, it limits the imports the rule judges to those whose target is in the
area. On `mayOnlyBeImportedBy`, it limits the importers. On `mayNotImport` it is refused, with the
one-line fix `mayNotImport: { all: [side, area] }`: 0.1 already writes that meaning, and an option
that only forwards to it earns no place. Conditions that check no import refuse it too. An area pattern that matches no
file is `selector-empty`, a misspelt name is `rule-unknown-element`, and an empty area is
`config-invalid`. The violation message names the area.

```ts
rule('cli-uses-only-the-api-router', {
  files: 'cli',
  mayOnlyImport: 'apps/api/src/app.router.ts',
  within: 'api',
  because: 'The CLI derives its parser from the API router and may not import any other API code.',
})
```

### Type imports

An import is a type import when its statement has the `type` modifier, or when every name it
imports or re-exports has the modifier and the file's tsconfig does not set `verbatimModuleSyntax`
(item 10 of "Decisions that need you"). The mark is set when the import is resolved, where the
file's tsconfig is in hand, so a change to a tsconfig re-marks every import with no parse, in
`archangel watch` and in the editor as in a check. So under the option, `import { type A } from './a'` and
`export { type A } from './a'` are both runtime imports. `export type * from './x'`, an import type such as `import('./x').T`,
triple-slash references and every import in a declaration file are type imports. Every rule counts
type imports, except `noCycles` and a transitive rule, which count runtime imports only, and the
`imports` option changes that, as in 0.1.

### One import, several rules

Each rule judges on its own, and every rule must pass, so no rule can grant what another refuses. An
import that breaks a table row and a rule gives two violations, and the summary counts two problems.
An exception names one rule and permits only that rule's violation; a run of exception comments
above an import can name several rules, one each. The baseline records one entry per rule. An
exception that names a removed row gives `exception-unknown-rule`, one that permits nothing gives
`exception-unused`, and a baseline entry for a removed row, or for a violation that is gone, gives
`baseline-stale`. All three fail the check, as in 0.1.

### What `archangel rules` prints

```
$ npx archangel rules

app  2 rows                                                     archangel.config.ts:9
  because: Features share code only through src/shared, and shared code never depends on a feature.
  feature may import only shared, and itself in the same instance
                                          6 files · 7 imports judged · 0 problems · 0 exceptions
  shared may import only itself
                                          1 file  · 0 imports judged · 0 problems · 0 exceptions
  every file in the table's files has a kind with a row
                                          checks nothing: every file of app is in a row

feature-code  3 rows                                            archangel.config.ts:16
  because: A view shows a model. Only the api folder talks to the server.
  views may import only models, shared, and itself
                                          2 files · 3 imports judged · 1 problem · 0 exceptions
  models may import only api, shared, and itself
                                          2 files · 3 imports judged · 0 problems · 0 exceptions
  api may import only shared, and itself
                                          2 files · 1 import judged  · 0 problems · 0 exceptions
  every file in the table's files has a kind with a row
                                          6 files · 0 problems

Not judged by any rule: 0 files, 0 imports between repository files.
Outside packages that no rule names: react (2 imports).
```

The command runs a full check, then prints every rule and every row in config order, with nothing
left out: its sentence in words, its reason, the file and line where it is written, and its reach.
A rule written in a preset says so ("from the preset fsd-web"), and a config rule that replaces a
preset rule says which one it replaces. A rule or row whose files are empty says "checks nothing" and
why: no file is in <name>, or, for a table's `every-file-has-a-kind` rule, every file of the table
is in a row. The last lines list the files and imports between repository files that no rule
judges, and, by package, the outside packages that no rule names. With a path, the command prints
only the rules whose files include a file under that path, and the unjudged files under it; a path
with no source file under it is refused with status 2, as for any path argument. With
`--format json`, it prints the same content as data. The command exits with status 0 when it
prints, whatever problems the check finds, because its job is to describe, and with status 2 for
an invalid config. The file and line come from the config loader, which records where each builder
was called, in whichever file that is, so a rule built in a helper module names that module and its
line; a rule written as a plain object gives the config file only.

### What `archangel explain` prints for one import

```
$ npx archangel explain src/features/cart/views/cart-page.tsx:2

src/features/cart/views/cart-page.tsx:2  '../api/fetch-cart' -> src/features/cart/api/fetch-cart.ts
  file:    part feature (instance cart), kind views in feature-code
  target:  part feature (instance cart), kind api in feature-code
  judged by:
    app.feature           permits: the same instance of feature
    feature-code.views    refuses: api is not in its row (views may import only models, shared)
  skipped by:
    app.shared            the file is not in shared
    feature-code.models   the file has the kind views, not models
    feature-code.api      the file has the kind views, not api
```

`archangel explain <file>:<line>` lists every rule: those that judged the import, with the verdict,
and those that skipped it, with the reason: the file is not in the rule's files, the target is
outside the area, the import kind or an import limit does not match, the instance differs, the
import does not resolve, the rule is off, or the rule checks no import (a naming, `mustExport`,
`mustContain`, `noCycles` or transitive rule). An import that does not resolve is shown with the
reason of its `import-unresolved` diagnostic and no target. An exception or a baseline match is
shown beside the rule it permits. The line may be any line of the import statement, and a line with
two or more imports gets one block for each. A line with no import says so and exits with status 1,
and so does a file that does not parse, with its `parse-failed` diagnostic; a file that is not a
source file exits with status 2, as for `archangel explain <file>`. `archangel explain <file>` keeps
its 0.1 output, and gains the file's kind in each table. Both answer in JSON with `--format json`.

### Examples that a test checks

Every config example in the docs and in the README is a fixture: a folder with the config, the
source files and the expected output of `archangel check`, `archangel test` and, where the doc shows
it, `archangel rules`. A test runs archangel on each fixture and compares the output, and a second
test reads each config block in the docs and the README and checks that it is the text of a fixture,
so an example that contradicts archangel fails a test. Two fixtures carry the payoff: the small app
in "In short", and a slice of enhance3's config, below.

### What enhance3 gains

The slice of enhance3's config is its workspace table of apps and packages and its API domain table,
written with 0.2. It is the second checked fixture:

```ts
import { defineConfig } from '@enhance-eng/archangel';

export default defineConfig({
  parts: {
    api: 'apps/api/**',
    web: 'apps/web/**',
    extension: 'apps/extension/**',
    storybook: 'apps/storybook/**',
    cli: 'apps/cli/**',
    desktop: 'apps/desktop/**',
    docs: 'apps/docs/**',
    contract: 'packages/contract/**',
    ui: 'packages/ui/**',
    'canvas-document': 'packages/canvas-document/**',
    'canvas-format': 'packages/canvas-format/**',
    'artwork-paint': 'packages/artwork-paint/**',
    'figma-import': 'packages/figma-import/**',
    'capture-browser': 'packages/capture-browser/**',
    sdk: 'packages/sdk/**',
    'agent-plugins': 'packages/agent-plugins/**',
  },
  groups: {
    tests: ['**/*.{test,spec}.{ts,tsx}', '**/__tests__/**'],
    domain: 'apps/api/src/*/domain/**',
    application: 'apps/api/src/*/application/**',
  },
  externals: {
    driver: ['pg', 'drizzle-orm', '@aws-sdk/*', 'node:fs'],
  },
  rules: ({ mayImport, rule }) => [
    mayImport('workspace', {
      web:               ['contract', 'ui', 'canvas-document', 'artwork-paint', 'capture-browser', 'figma-import', 'api'],
      extension:         ['contract', 'ui', 'capture-browser'],
      storybook:         ['ui'],
      cli:               ['contract', 'canvas-document', 'sdk', 'api'],
      desktop:           ['contract'],
      api:               ['contract', 'artwork-paint', 'canvas-format'],
      docs:              ['cli'],
      contract:          ['artwork-paint', 'canvas-format', 'ui'],
      'canvas-document': ['contract', 'ui'],
      'figma-import':    ['canvas-document', 'contract'],
      sdk:               ['capture-browser', 'contract'],
      'capture-browser': ['contract'],
      'agent-plugins':   ['cli'],
      ui:                [],
      'artwork-paint':   [],
      'canvas-format':   [],
    }, {
      because: 'Apps share code only through packages, and a package never reaches into an app.',
    }),

    rule('web-uses-api-types-only', {
      files: 'web', mayNotImport: 'api', imports: 'runtime',
      because: 'The web app types its requests from the API router. No API code ships to the browser.',
      examples: { rejected: [{ 'apps/web/src/a.ts': "import { app } from '../../api/src/app';", 'apps/api/src/app.ts': '' }] },
    }),

    rule('core-not-to-driver', {
      files: ['domain', 'application'], mayNotImport: 'driver',
      because: 'The domain and application layers never touch an I/O driver. A use case that needs one declares a port.',
      examples: { rejected: [{ 'apps/api/src/board/domain/a.entity.ts': "import { Client } from 'pg';" }] },
    }),

    mayImport('domain', {
      entity:           ['value-object', 'domain-event', 'specification', 'policy', 'error', 'types', 'consts'],
      'aggregate-root': ['entity', 'value-object', 'domain-event', 'specification', 'policy', 'error', 'types', 'consts'],
      'value-object':   ['error', 'types', 'consts'],
      'domain-event':   ['value-object', 'types', 'consts'],
      specification:    ['entity', 'value-object', 'aggregate-root', 'types', 'consts'],
      policy:           ['entity', 'value-object', 'aggregate-root', 'specification', 'error', 'types', 'consts'],
      port:             ['entity', 'value-object', 'error', 'types', 'consts'],
      error:            ['value-object', 'entity', 'types', 'consts'],
      types:            ['consts'],
      consts:           [],
    }, {
      files: { all: ['domain', { not: 'tests' }] },
      kinds: '**/*.{kind}.ts',
      everyRowMayImport: ['apps/api/src/shared/result.ts'],
      mayNotImportItself: ['aggregate-root', 'domain-event', 'policy', 'port'],
      because: 'Each kind of domain file depends only on the kinds below it.',
    }),
  ],
});
```

The two rules that are not tables carry a written rejected example, because 0.1's `rule-unproven`
still applies to them. The domain table is the one enhance3 states, with one row added:
`specification`. The stated table
names `specification` as a cell but gives it no row, and 0.2 refuses a cell that is neither a row
nor a declared name, so that a misspelt cell can never pass and the types can check every cell. The
added row is the one enhance3's builder has today. review.md records this and two other notes about
the table for enhance3.

What this replaces in enhance3 today: the domain table lives in four places (a table of strings in
the vocabulary, a builder function per row, a glob at load time that drops rows no file has, and a
naming rule that repeats the kinds), and each package's dependencies are two rules. The workspace
table replaces the per-package "may only import" rules and the app-to-app rules, and the domain
table replaces its 11 generated rules and the naming rule `naming:api:domain`.

**An estimate of enhance3's config written by hand.** enhance3's builder produces about 380 rules:
about 200 from 9 kind tables (107 rows, 709 cells) applied 14 times, and about 180 others (61
dependency rules, 5 API layer rules, 107 naming rules and 9 overrides of the Feature-Sliced Design
preset). With the forms of this change:

| Part | Rules today | Lines by hand |
|---|---|---|
| Ignore list, presets and `off`, parts, groups and externals | | 80 |
| The workspace table and its 2 narrow rules | about 20 | 40 |
| 9 kind tables as shared `const` objects, with no self cells (the default permits them) | | 150 to 200 |
| 14 table uses, about 6 lines each, with no written example and the `every-file-has-a-kind` rule | up to 195 rows and about 20 naming rules | 90 |
| API layers and editor models as tables, and 4 `entry` rules | 9 | 50 |
| Naming rules for folders that no table covers, with brace alternatives | about 85 | 140 |
| Other dependency rules, name first, each with a reason and a written rejected example | about 34 | 330 to 380 |
| Overrides of the Feature-Sliced Design preset, for each of three apps | 9 | 120 |
| Literal file lists in those rules | | 150 to 200 |
| **Total** | about 380 | **about 1,150 to 1,350** |

The main savings over the builder's output are that table rows need no written example (about 80
lines), the workspace table and externals (about 90 lines of package rules), the `every-file-has-a-kind`
rule (about 10 naming rules), and the name-first form (about one line per rule). This is an
estimate from counts, not a measurement; the rewrite is a separate task, and task 9.1 measures the
slice.

## Decisions

**1. Compile each row to an ordinary rule, decided in the pair entry.** eslint-plugin-boundaries'
`element-types` rule (https://github.com/javierbrea/eslint-plugin-boundaries) is the normal solution
for an import table: element types found by path patterns with captures, `default: 'disallow'`, and
one policy per type. Nx's dependency constraints have the same shape over project tags. Archangel
follows them in the table form and in disallow by default, and departs in one place: each row is a
rule with its own name, because `off`, exceptions and the baseline refer to rules by name. A row's
sentence, its area and its own-instance permission are all facts about the classes at the two ends,
so they are decided when a pair entry is filled, as 0.1 decides a `mayNotImport` side and as
dependency-cruiser's `from` and `to` describe a rule. The per-import cost stays one look-up, plus
the instance comparison that Cost names for an import inside a captured row.

**2. Find kinds with one capture, `{kind}`.** The capture syntax already exists for instances, and a
capture value is the kind's name, so one pattern gives every kind, including the kinds that have no
row and need a "no row for" message. eslint-plugin-boundaries reads element types from captures the
same way, and its captures are ordinary path segments with no behaviour of their own, so `{kind}`
matches as every capture does, one name of one or more characters; in `**/*.{kind}.ts` the `*`
before it takes the most it can, which already makes `board.entity.test.ts` the kind `test` and
`board.entity.ts` the kind `entity`. The one case that gives a wrong answer is a declaration file,
where `env.d.ts` would be the kind `d`, so `.d.ts`, `.d.mts` and `.d.cts` count as one extension for
`{kind}`; the repeat allowance is the other difference, because each place takes the same name. Both
are scoped to `{kind}`, because changing every capture would change what a 0.1 pattern such as
`src/{feature}.d.ts` matches. This spends a token: one capture name with two rules of its own. A list of patterns per kind is what enhance3's builder computes today, and it cannot name
a kind it does not list.

**3. Keep the self permission inside the row's sentence.** The 0.1 `mayOnlyImport` already permits
its own files; 0.2 gives that permission the instance test that `instance: 'same'` already defines,
for every `mayOnlyImport`, so a row's sentence is an ordinary `mayOnlyImport` with nothing added.
`mayNotImportItself` adds a `mayNotImport` sentence, which 0.1 can already say. No hidden flag makes
a row mean something its sentences cannot show.

**4. Externals are package elements under a name.** 0.1 already classifies a package import by the
package elements and inline package sides that list it, and decides that per pair of classes. An
externals set is that mechanism with a name, and the same patterns; it adds no new kind of set and
no new store. One thing in that mechanism changes: 0.1 puts a package in the first package element
that lists it, and an outside package is in every externals set that lists it.

**5. Show that rows fire with reach, not with proofs.** The normal answer to "does this rule fire
at all" is an unused-rule or empty-match report, which eslint-plugin-boundaries and
dependency-cruiser give without per-rule proofs. Archangel has that report in this change: rows
with no file in the summary, `table-empty`, and the reach of `archangel rules`. A generated proof
would choose its refused file with the same pair entries that it then checks, so it could only fail
when the pair-entry fill disagrees with itself, which the property tests already cover. 0.1's
demand for a written rejected example stays for rules that are not table rows.

**6. `archangel rules` and `explain` read the compiled rules.** The sentence printer and the pair
entries already exist for `explain` in 0.1. Reach is a count per rule, gathered while the check
evaluates sentences, only when `archangel rules` asks for it. A separate description of the rules
would be a second source that could disagree with the check.

**7. Check examples by running them.** A docs example that a test runs cannot drift, which is what
the readers asked for. Rust's doc tests and the TypeScript handbook's checked samples are the
precedent.

**8. Name a row `<table>.<row>`.** A row's name is `<table>.<row>`, and a table or row name may
not hold a colon or a slash. An exception comment ends the rule name at the first colon, and the
slash is a preset's prefix, so a table `roles` in the preset `fsd-x` is `fsd-x/roles` and its rows
`fsd-x/roles.entity`. Fixing the exception parser for colons stays out of 0.2, because any reading
that lets a name hold a colon changes what some 0.1 comment means. A dot and white space are
accepted, as 0.1 accepts them in a rule name: nothing in archangel reads a rule name by its dots,
`off: ['domain']` turns off the table's rules by identity and not by a prefix, and the exception
parser keeps the spaces inside a name, so a row `api.v2`, the kind that a folder `api.v2` has (item
3 of "Decisions that need you"), is one rule name like any other. The one clash, a table `a.b`
beside a table `a` with a row `b`, is a duplicate name, `rule-name-duplicate`, as for any two
rules.

## Cost

An area and a row's sentence add no work per import beyond 0.1's. When a pair entry is filled,
archangel tests one more set membership per sentence with an area or a row; that grows with
connected class pairs times sentences, the cost that filling pair entries already has. The
own-instance permission has one cost that 0.1 does not have. In 0.1, an import between two files of
one `mayOnlyImport` rule costs no rule evaluation, because the sentence is left out of the pair entry
of its own files. A class carries no capture value, so to refuse an import between two instances the
sentence must go into that pair entry and compare the two instances per import. The cost is one more
rule evaluation for each import between two files of a row, or of a part under a `mayOnlyImport`
rule, whose name has an instance capture; it stays zero for a row or part without one. Work-count
tests hold these exact numbers: entity files with 10 package imports and with 1,000 give the same
rule evaluations; a table of 5 rows and the same table with 45 more rows that no import connects give
the same rule evaluations and pattern passes; N imports inside one captured part, or inside one
instance of a kind row whose pattern has another capture, cost N rule evaluations, and imports
inside an uncaptured kind row cost 0. A deliberately slowed build that tests
the area, or every row, on each import must fail these tests, so they cannot pass by accident.

A kind pattern is one pattern in the compiled set, so pattern passes stay one per file. For a file
that it matches, archangel reads the `{kind}` value, and the values of the pattern's other captures,
with one more match of that pattern when the file is classified, and stores them beside the values
of the file's part captures; so the work grows with files that have a kind, times tables with a kind
pattern, and the instance comparison of a kind row reads stored values and matches no pattern per
import. Each row, and each pattern cell or area, adds one set whose membership is tested for every
file, so classification grows with files times sets. Archangel's counters, which
`archangel check --stats` prints as `filesClassified`, `patternPasses`, `ruleEvaluations` and
`structureEvaluations`, do not see this work, which is why task 9.1 reports the number of compiled
sets beside the times. A class carries one bit per row, not per kind
value, so the number of classes grows with the rows that files really have; a kind with no row is
named only when a message needs it. A work-count test holds that the files classified and the
pattern passes of a check with a kind table equal those without it.

Externals add one membership per externals set for each distinct outside package, as 0.1's package
elements do. `part-overlap` reads the matches that the compiled set already returns for a file, and
`table-row-overlap` counts rows per class, so both grow with classes times rows, not with imports.
`<table>.every-file-has-a-kind` is one structure sentence per table, and a structure sentence is decided per
file: one bit test per table for each file, and one structure evaluation for each file with no row.
It adds no rule evaluation. A work-count test holds that `structureEvaluations` equals the number of
files with no row, and that `ruleEvaluations` does not change. The list of rows with no file and
`table-empty` read the count of files per class on each assembled report, in a check and after each
edit.

The type-import change reads one option per tsconfig, which the resolver already loads, and adds one
test per import when it is resolved; a change to a tsconfig re-marks every import inside the
re-resolution that it already causes, with no parse.

`archangel rules` adds one counter increment per rule evaluation, only for that command, and reads
classes times rules for the file counts. `archangel explain` of one import evaluates every rule on
that one import, on demand. Neither is on the path of `archangel check`.

At enhance3's size a fixture cannot show whether the compiled pattern set still compiles as one, so
before release the enhance3 slice is measured once on enhance3 itself (task 9.1): pattern passes,
files classified, rule evaluations and the number of compiled sets from `archangel check --stats`,
and the times.

## Risks

- **The self default changes some 0.1 rules.** A 0.1 `mayOnlyImport` rule on a part with a capture
  now refuses imports between instances (item 9 of "Decisions that need you"). The changelog names
  it, and `archangel rules` prints "and itself in the same instance" on every such rule.
- **A table row needs no written example, so a wrong cell is not caught by `archangel test`.** It
  passes until a violation or a written example shows it. The property tests show that every cell
  is decided as written, and `archangel rules` shows each row's reach.
- **The own-instance default costs one rule evaluation per import inside a captured row or part.**
  Cost states the number and a work-count test holds it; task 9.1 measures it on enhance3.
- **`<table>.every-file-has-a-kind` can give many violations on adoption.** The message says how to
  fix each file, and `archangel baseline add` records them, as for any new rule.
- **Two spellings live until 1.0.** A config cannot mix them, and the docs show one.
- **A one-word brace is a capture.** `archangel rules` and `explain` show the instance it makes.
- **A wide table adds sets to each class.** Rows of one table rarely overlap, and an overlap is an
  error, so classes grow with real combinations. The enhance3 measurement is the evidence at size.

## Compatibility and release

The release is 0.2.0. A 0.1 config loads and gives the same diagnostics, summary, JSON output and
exit status, except in the two cases that "Decisions that need you" names: a `mayOnlyImport` rule on
a part with a capture (item 9), and a repository whose tsconfig sets `verbatimModuleSyntax` (item
10). Snapshots of the output of `archangel check` and `archangel test` on the 0.1 test fixtures,
recorded at 0.1.1 before any other commit of this change, hold this; a fixture that one of the two
changes touches is named in the test with the reason. The config gains `parts`, `groups`,
`externals`, `because`, the name-first `rule`, `within` and the `mayImport` kind, in TypeScript and
in the JSON schema, where a table is `{ "kind": "mayImport", "name", "rows", ... }`. The JSON summary
gains `rowsWithNoFile` only when the config has a table, and `schemaVersion` stays. New diagnostic
codes, each with a page: `table-empty`, `table-row-overlap` and `part-overlap`. The command line
gains `archangel rules` and the `<file>:<line>` form of `explain`.

## Limits

- A file in a table's files that has no row is judged by no row as an importer; it is a violation
  of `<table>.every-file-has-a-kind`, unless `off` names that rule, which `archangel rules` then
  shows. A file with no kind is outside a table that has no `files`, so the suffix gap stays open
  until the author writes `files`, as the enhance3 slice does (item 3 of "Decisions that need
  you"); `archangel rules` lists such a file when no rule judges it.
- A table judges an outside package only when it names an externals set that holds it. `archangel
  rules` lists, by package, the outside packages that no rule names.
- A file that no rule judges, and an import between repository files that no rule judges, are listed
  by `archangel rules`, not by `archangel check`, because a check reports problems and an unjudged
  file is not one.
- A table row has no proof in `archangel test` unless the author writes one. What shows that a row
  can fire is the summary's list of rows with no file, `table-empty` and the reach that
  `archangel rules` prints (item 5 of "Decisions that need you").
- In a suffix pattern such as `**/*.{kind}.ts`, the `*` before `{kind}` takes the most it can, so the
  kind is the last dotted word before the extension, and the pattern cannot name a kind with a dot
  in it; a group can be a row instead. A folder pattern such as `src/features/*/{kind}/**` gives a
  folder named `api.v2` the kind `api.v2`.
- A `.js` or `.mjs` config needs the same Node as a `.ts` config, 22.18 or later: the Node gate runs
  before the loader for every config kind, and that is what makes `util.getCallSites()` available
  to the loader. The npm package's `engines` field says `>=18`, which disagrees with the gate; this
  change builds nothing around that, and a change to the gate is its own decision.
- The runtime or type mark of an import reads only `verbatimModuleSyntax`. Two other inputs give the
  same class of wrong answer and stay: without the option, TypeScript drops `import { A } from './a'`
  when `A` is used only as a type, which archangel still counts as a runtime import, because telling
  needs a semantic pass over every file; and the deprecated options `importsNotUsedAsValues` and
  `preserveValueImports` are not read (item 10 of "Decisions that need you").
- A preset rule is still replaced by a config rule with its prefixed name, with no explicit word;
  `archangel rules` shows each replacement. An explicit `replaces` would break 0.1 configs that
  replace by name, so it is deferred to a later change.
- `mayOnlyImportTypesFrom`, layers and isolate over groups, one-line examples, a map printed by
  `archangel init`, and typed builders for a config split across files are not in this change.

None of these drops work silently. Searched: the config loader and its builders, the rule loader and
its shorthand compilation, pattern parsing and its capture checks, element and package
classification, the resolver's tsconfig reading, the import reader's type marking, the graph's
re-resolution after a tsconfig change, the compiled
architecture and pair-entry filling, the structure conditions, the check's empty-set and
preset-empty reports, the example runner and the test command, exception parsing, the baseline, the
merge of presets and `off`, explain, the config types and the JSON schema.

## Where in the code

This section is for whoever builds the change.

| Part | Where |
|---|---|
| New config keys | `crates/archangel_config/src/parse.rs`: read `parts` beside `elements`, `groups` beside `selectors`, `externals`, and `because` beside `reason` (in `crates/archangel_config/src/rules.rs`); refuse both spellings of one key with `config-invalid`; `parts` refuses `{ packages }` values with the fix to move them to `externals` |
| Externals | No new model type: `crates/archangel_config/src/parse.rs` reads the `externals` key into `Config.elements` as `Element { kind: ElementKind::Packages(..) }` entries flagged as externals (`Element.externals: bool`), so the existing owner of named package sets holds them, the duplicate-name check in `crates/archangel_config/src/selectors.rs` ("is also an element") refuses an externals set with the name of a part or group, and a row key that names a flagged element is refused; matching stays `package_matches` in `crates/archangel_config/src/matcher.rs`. The one real change in the owner: `ElementMatcher::classify_package` in `crates/archangel_config/src/matcher.rs` returns the first package element today, and `ClassKey::Package { element, .. }` in `crates/archangel_rules/src/classes.rs` holds one element; both must record every set that lists a package. Refusal of a bare built-in name (the fix `node:x`), an `@types/` name, a name with a subpath (the fix is its package) and a workspace package's name in `parse.rs`, the last after manifests are read; the built-in names are `nodejs_built_in_modules::BUILTINS`, the list that oxc_resolver reads when `crates/archangel_graph/src/resolve.rs` sets `builtin_modules: true`, so the config and the resolver cannot disagree on what is a built-in |
| Part overlap | `crates/archangel_config/src/matcher.rs`: `ElementMatcher` already returns every matching index; for `parts`, more than one own-part match gives `ProblemCode::PartOverlap`; a preset's parts form their own namespace in `Classification` (one `ElementId` per namespace); `elements` keeps the first match |
| Pattern start | `crates/archangel_config/src/pattern.rs` (patterns start at the config folder, unchanged); entry roots for parts without a capture from `Classification.root` in `crates/archangel_config/src/matcher.rs` |
| `{kind}` | `crates/archangel_config/src/pattern.rs`: `Token::Capture("kind")` compiles to the same `(?P<name>[^/]+)` as every capture (line 113), so the suffix answer comes from the greedy `[^/]*` of the `*` before it, with no rule of its own. Two differences: it may repeat (the `RepeatedCapture` check skips `kind`, each place compiles to its own group, and the values are compared after the match, because the `regex` crate has no back-references), and `.d.ts`, `.d.mts` and `.d.cts` count as one extension for it; `PatternError::RepeatedCapture` still fires for any other name |
| The table kind | `crates/archangel_config/src/rules.rs`: `"mayImport"` in `SHORTHANDS`, a `table` method beside `isolate` and `entry` that returns several `Rule`s; `parse_rule` returns `Vec<Rule>`, and its two callers in `parse.rs` push every row; keys resolved through `Names` (`crates/archangel_config/src/names.rs`) as parts or groups first, then kinds; cells must be rows, declared names or patterns; `instance`, `names`, `specifier`, `imports` and `dependency` copied into each row's `ImportOptions`, with `rule-needs-capture` for `instance` on a row without a capture; precedent `layer_sentences` |
| Table data | `crates/archangel_config/src/model.rs`: `Config.tables: Vec<Table>` holds only what no row sentence holds: the name, the kind pattern (for classification), the files (the default and the rows' files), the examples and the preset; the area, `everyRowMayImport` and `mayNotImportItself` live only in the sentences they compile to, and no check result lives here, so `rules` and `explain` have one source; `RuleKind::TableRow { table: usize, row: Option<String> }` points a row at its table. No `own` flag: the row's own-instance permission is the changed own-files test of `mayOnlyImport` (next row) |
| `mayOnlyImport` self default | `crates/archangel_rules/src/architecture.rs`: in `applies`, a `MayOnlyImport` sentence whose files have an instance capture now enters the pair entry of its own files (`to_in_files`) with a `Test::Facts` instance comparison (`facts::instances_compare`), as `instance: 'same'` does; for a kind row, `instances_compare` reads the kind pattern's capture values that classification stored in each end's `Classification` (`crates/archangel_config/src/matcher.rs`, beside the part's captures), not `sentence.files.element()`, so no pattern is matched per import; a sentence without a capture stays out of that pair entry as in 0.1; recorded in `CHANGELOG.md` |
| `every-file-has-a-kind` | `crates/archangel_config/src/rules.rs`: one rule `<table>.every-file-has-a-kind` for every table, with one `MustNotExist` sentence on the table's files minus its rows; no option and no count; its message, with the file's kind or none and the closest row, in `crates/archangel_rules/src/messages.rs` |
| Area | `crates/archangel_config/src/model.rs`: `within: Option<Side>` on `ImportOptions`; `crates/archangel_rules/src/architecture.rs`: one compiled-side value for `side` and `area`, tested in `applies` and `considers`; `report_inline` reports area patterns |
| Type imports | The flaw is the `all_typed` inference in `StatementNames::of` in `crates/archangel_graph/src/imports.rs`, which marks a statement as typed when every import entry, or every re-export entry, carries the type mark, and so builds TypeScript's default elision into both statement forms. The mark is decided where the file's tsconfig is already in hand, not in the parser: `extract` stays syntactic and records the statement's `type` modifier and `all_typed` as facts; `resolve_file` in `crates/archangel_graph/src/graph.rs`, which already calls `resolver.tsconfig(file)` and holds the governing `TsConfig` (oxc_resolver 11.23 exposes `compiler_options.verbatim_module_syntax`, merged through `extends`), sets `ImportKind` from `all_typed && !verbatim_module_syntax`. So `resolve_all_again` in graph.rs, the path that `crates/archangel_engine/src/events.rs` takes for a tsconfig edit, re-derives every mark with zero parses, the language-server scenario "A change to a tsconfig" still holds, and an example's overlaid tsconfig (`crates/archangel_graph/src/overlay.rs`) is seen by `archangel test`; `export type *` through `request.is_type` (test added) |
| Overlap in a table | `crates/archangel_rules/src/check.rs`: in `Architecture::assemble`, `table-row-overlap` per class with two rows of one table, with the way forward in its message (`messages.rs`), `table-empty`, and `Summary.rows_with_no_file`; `crates/archangel/src/render/json.rs` writes `rowsWithNoFile` only when the config has a table. A group row key that is also a kind value needs no check: the key resolves as the group, and `every-file-has-a-kind` names the group in its message for a file with that kind outside the group (`messages.rs`) |
| Diagnostic codes | `crates/archangel_config/src/problem.rs` and `crates/archangel_rules/src/messages.rs`: `PartOverlap`, `TableRowOverlap`, `TableEmpty`; pages `crates/archangel_rules/docs/part-overlap.md`, `table-row-overlap.md`, `table-empty.md`, `may-import.md` |
| Table rows and `rule-unproven` | `crates/archangel_rules/src/examples.rs`: `unproven_rules` skips `TableRow` rules and the `every-file-has-a-kind` rule; a table's written examples run against all its rows; no generated proof (item 5 of "Decisions that need you") |
| `archangel rules` | `crates/archangel/src/commands/rules.rs` (new) and `crates/archangel/src/commands.rs`; reach counters per rule in `crates/archangel_rules/src/check.rs`, filled only when asked; the sentence printer shared with `crates/archangel/src/commands/explain.rs`; unjudged files and imports from pair entries with no sentence |
| Source locations | `crates/archangel_config/src/load.mjs`: each builder records the file and line of its caller with `util.getCallSites()` (Node 22.9 and later), never by parsing stack text and with no fallback: `NODE_MINIMUM` in `crates/archangel_config/src/load.rs` is 22.18, and `Process::start` in `crates/archangel_engine/src/plugins.rs` applies it to every config before the loader starts, so the API is present on every Node that runs the loader (the npm `engines` field's `>=18` disagrees with the gate; Limits names it). The recorded frame is the first call-site frame outside `load.mjs`, whichever file that is, so a rule built in a helper module names that module and its line; a plain object gives the config file only. The loader sends the locations beside the config, keyed by rule index; `load.rs` attaches them to `Rule` |
| Explain of one import | `crates/archangel/src/commands/explain.rs`: parse `<file>:<line>`, evaluate every rule on that import with a reason for each skip (files, area, import kind, limits, instance, an import that does not resolve, a rule that is off, a condition that judges no import); a file with `parse-failed` is reported with that diagnostic and status 1 |
| Rules that `off` names | `crates/archangel_config/src/merge.rs` stays the one place that decides which rules are live: `off` still removes the rules it names from `Config.rules`, so no other reader changes, and now returns them in their own list, `Config.off`, beside `presets`. Only `archangel rules` and `explain` read that list, to print a rule as off. A table's name in `off` removes every rule of the table |
| Stale entries | unchanged code paths: `exception-unknown-rule`, `exception-unused` and `baseline-stale` in `crates/archangel_rules/src/exceptions.rs` and `crates/archangel_rules/src/baseline.rs`; tests added for rows |
| Fault controls | `crates/archangel_rules/src/architecture.rs`: fail points `rules::area_per_import` and `rules::every_row`, beside `rules::search_every_rule` |
| Work counts | `crates/archangel_graph/src/work.rs`: no new counter; tests use `rule_evaluations`, `structure_evaluations`, `pattern_passes` and `files_classified` |
| Reference evaluator | `crates/archangel_rules/tests/rule_properties.rs`: extended with areas, tables of each row form, own-instance permission and externals |
| Checked examples | `docs/examples/small-app/` and `docs/examples/enhance3-slice/` (config, files, expected output), one folder per other docs example; `crates/archangel/tests/doc_examples.rs` runs each and compares, and checks each config block in `README.md` and `docs/*.md` against a fixture |
| Types | `npm/archangel/index.d.ts`: `parts`, `groups`, `externals`, `because`; `rule(name, body)` overload; `within?: Side` on the `mayOnlyImport` and `mayOnlyBeImportedBy` arms of `Sentence`; `mayImport<const N, const K extends string>(name, rows, options)` with cells typed as `K \| E \| S \| X \| PathPattern`, where `E`, `S` and `X` are the type parameters that hold the config's declared part, group and externals names (`E` and `S` as `index.d.ts` already uses for elements and selectors; `X` is new, for `externals`); `TableName<N>` refuses a colon or a slash |
| JSON schema | `npm/archangel/archangel.schema.json`: the new keys and the `mayImport` kind |
| New terms | `openspec/glossary.md`: Part, Group, Externals set, Area, Table, Row, Cell, Kind, Kind pattern, Reach, Unjudged; Element instance renamed to Instance and widened to a kind pattern's other captures and to a name without a capture, with Element instance pointing at it; Element and Selector point to Part and Group; Shorthand kind and Rule builder name four kinds |
| Docs | `crates/archangel_rules/docs/may-import.md` says that a table without `files` leaves a file with no kind outside the table, and that `files` closes that gap; `docs/rules.md`, `README.md`, `docs/commands.md` (`rules`, `explain <file>:<line>`), `docs/migrating.md` (the new spellings), `CHANGELOG.md` (0.2.0), versions in `Cargo.toml` and `npm/archangel/package.json` |
| Precedents | `layer_sentences` in `crates/archangel_config/src/rules.rs`; `unproven_rules` in `crates/archangel_rules/src/examples.rs`; enhance3's builder at `packages/architecture/archangel.mjs` and vocabulary at `packages/architecture/index.mjs` on the main branch of https://github.com/enhance-labs-builds/enhance3 |
