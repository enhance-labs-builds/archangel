# Design: scoped imports and role matrices

## In short

A team that writes its architecture in archangel 0.1 can say "these files may not import that" and
"these files may import only this", but it cannot say "these files may import only this, within this
part of the code". `mayOnlyImport` judges every import, so a rule about the API's domain files also
refuses `zod` and every file of the web app. Teams work around it with a double negative:
`mayNotImport: { all: ['apps/api/src/**', { not: allowed }] }`. They also cannot write a table of
who may import whom, so they generate one rule per row, and one more rule per folder to say that
every file there has one of the table's kinds. On enhance3, a 9,800-file monorepo, that took an
828-line builder that reads a 2,896-line vocabulary, globs the repository at load time to drop rows
that no file has yet, and produces about 380 rules with computed names that no type can check. The
check passes, but nobody can read the config as an architecture.

With 0.2, an import rule takes `within`: the area whose imports it judges. A **matrix** is a fourth
shorthand, beside `layers`, `isolate` and `entry`, written as the table itself. Its rows are either
kinds of file found by name, such as every `.entity.ts` file, or parts of the code the config
already names, such as the package `canvas-document`. Each row is a rule with a readable name, such
as `api-domain.entity`, that `off`, exceptions and the baseline can use. With `everyFileHasARow`, a
matrix also refuses a file in its files that is in no row, so the table states its own naming rule.
One rejected example that the author writes proves a matrix, as one proves a `layers` rule.
`defineConfig` checks the row names, the cells and the area as the developer types them. Every 0.1
config gives the same results.

Three ideas from 0.1 carry this design. A rule is made of **sentences**, each one condition, such
as `mayOnlyImport`, on a set of files; a shorthand stands for one or more sentences. Archangel checks
an import by the pair of **classes** at its ends: a class is the set of files that are in the same
element and exactly the same selectors. A package at the other end of an import has a class too,
found from its name: which of the package names that the rules write match it. Before it looks at
any import, archangel fills one **pair entry** for each pair of classes that some import connects,
with the sentences that pair can break. The new part is that an area and a row are both sets of
files and packages, so they become part of each class, and both new forms are decided when a pair
entry is filled, not per import.

**How it works:**

1. The developer writes `within` on a `mayNotImport`, `mayOnlyImport` or `mayOnlyBeImportedBy`
   rule, or writes a matrix with a name, its files, usually an area, and one row per role or per
   declared element. A matrix with a role pattern, such as `**/*.{role}.ts`, has roles as rows; a
   matrix without one has declared elements and selectors as rows.
2. When archangel loads the config, it turns each row into one rule named `<matrix>.<row>`, made of
   the sentences the row stands for. With `everyFileHasARow`, it adds one rule,
   `<matrix>.every-file-has-a-row`, that refuses a file in the matrix's files that no row has. From
   then on each is a rule like any other.
3. When archangel compiles the rules, it puts a sentence with an area into the pair entry for a pair
   of classes only when the other end's class is in the area. So an import outside the area never
   reaches the rule, and an import costs the same look-up as before.
4. After the check, the summary lists the rows that have no file, and counts, for each matrix
   without `everyFileHasARow`, the files in its files that are in no row. A matrix with no row, or
   whose rows together have no file, fails the check with `matrix-empty`.
5. `archangel test` runs the matrix's written examples against all its rows, and a matrix with no
   written rejected example is `rule-unproven`.

Six terms are new. An **area** is the set of files and packages, written with `within`, whose
imports a rule judges. A **role** is a kind of file that a matrix names, found by the file's path,
such as a name that ends in `.entity.ts`. A **role pattern** is the one path pattern of a matrix
that contains the capture `{role}`; a file is in a role when the pattern matches it with that role's
name in place of the capture. A **matrix** is a table that says, for each role or each declared
element or selector it lists, what the files of that row may import. A **matrix row** is the rule
that says what the files of one row may import. A **cell** is one entry in a matrix row: a role, a
declared name, a pattern or packages that the row's files may import.

### Decisions that need you

1. **A row permits exactly the cells it lists, its own row included.** In enhance3's domain table
   the row `domain-event` lists `value-object`, `types` and `consts`, and not `domain-event`, so today
   one domain event may not import another. `mayOnlyImport` in 0.1 always permits a file to import
   the other files of its own rule. If a matrix row behaved the same, the rewrite of enhance3 would
   quietly permit what the table forbids. So a row that does not list itself refuses its own files.
   The other option, rows that always permit themselves, reads like `mayOnlyImport` but makes an
   empty cell in the table mean "yes" on one diagonal only. `within` on a plain `mayOnlyImport` rule
   keeps the 0.1 behaviour, so that adding `within` never changes what a rule permits inside its area.

2. **A row's name is `<matrix>.<row>`. The matrix name and each row name may not hold a dot, a
   colon, a slash or a space.** Other rule names keep the 0.1 rules, so enhance3's colon names, such
   as `role:api:domain:entity`, still load. They still cannot take an exception, and this change
   leaves that problem alone (Decisions, item 4, says why the exception parser stays unchanged). An
   exception comment, `// archangel-allow <rule>: <reason>`, ends the rule name at the first colon,
   so a row name must have none. A dot reads well and is free. The slash stays reserved for a
   preset's prefix: a matrix `roles` in the preset `fsd-x` is named `fsd-x/roles`, and its rows
   `fsd-x/roles.entity`. Two matrices with one name are refused, because the matrix name is what
   `rule-unproven` and `matrix-empty` name. The other option, a name per row written by the author,
   gives them freedom they do not need and a second name to keep in step with the row.

3. **One written rejected example proves a matrix and all its rows.** In 0.1 a `layers` rule stands
   for many sentences, and one rejected example proves it. A matrix is proven the same way: each row
   counts the matrix's written rejected examples as its own, and a matrix with none is
   `rule-unproven`. On enhance3 this means 17 written examples for about 200 rows, where today there
   are about 400 generated ones that prove only that the generator agrees with itself. What one
   example cannot catch is a row with files whose cells are wrong; the property tests prove that
   archangel decides every cell as written, and an author who fears a row is wrong writes an example
   for it. The other option, an example that archangel derives for each row from two repository
   files, was dropped: it is a new mechanism whose only job is to meet a per-row demand that a matrix
   need not make.

4. **The option is `within`, and the specs call its value an area.** The request for this change
   asked for rules "within a scope". The word scope already means a package scope in archangel, as in
   `'@acme/*'`, and the glossary keeps one word for one meaning. `within` reads as English in the
   rule: `mayOnlyImport: ['value-object'], within: 'apps/api/src/**'`.

5. **A role pattern uses the capture `{role}`, filled with each row's name; other captures act as
   wildcards.** `roles: '**/*.{role}.{ts,tsx}'` gives each row's role without a list of patterns,
   and the row names are the role names. `{role}` may appear more than once, as in
   `**/{role}s/*.{role}.ts`, and each place takes the same name. Another capture, such as `{app}` in
   `apps/{app}/**/*.{role}.ts`, matches as `*` does, except that it never matches an empty part, and
   the matrix makes no instance from it. The other option is to refuse another capture with advice
   to write `*`; it refuses a pattern whose meaning is clear.

6. **A file that two rows have is judged by both.** Some role patterns put one file in two roles:
   `**/{role}/**` puts `apps/api/domain/application/x.ts` in both `domain` and `application`, and
   `**/*.{role}.*` puts `a.entity.test.ts` in both `entity` and `test`. In the other form, two
   overlapping selectors can be two rows. Each row is an ordinary rule, so the file must keep both
   rows, and an import that one row does not list is a violation of that row; `archangel explain`
   lists both rows. This needs no new mechanism and never permits more than either row. The other
   options are that the first row in the table wins, which makes the order of the table a hidden
   rule, or that a file in two rows is refused, which adds a check that the two violations already
   show.

7. **The area includes files that have no row, but a file with no row is judged by no row.** A
   domain entity that imports `index.ts` in its own folder breaks its row unless a cell names that
   file, because the import's other end is in the area. So a file cannot become an unlisted target
   by dropping its suffix. As an importer, though, a file in the matrix's files that has no row is
   judged by no row: it may import anything. Item 8 of this list closes that gap. enhance3's API tables
   permit no index file today, so this matches them. Its frontend and ui tables permit, in each row,
   the index files of the slices and of the segments whose roles that row lists, which its builder
   computes. With 0.2 these are written by hand: the slice index files, which every row permits,
   are left out of the area, and each segment's index file is a declared selector that the rows
   which may import it list as a cell. The other option, judging only imports into files that have a
   row, would let any file escape the table as a target by dropping its suffix.

8. **`everyFileHasARow: true` refuses a file in the matrix's files that no row has. It is off by
   default, and the summary counts such files when it is off.** With it, each such file is one
   violation of the rule `<matrix>.every-file-has-a-row`, and the message names the roles, or the
   rows, the file could have. A team leaves files out of the requirement, such as tests, index files
   and `.d.ts` files, by leaving them out of the matrix's files with `not`, usually through one
   declared selector. The name says what it checks in both forms of a matrix; `everyFileHasARole`
   reads well for a role matrix but is false for a matrix of packages. It is off by default because
   a matrix states imports, and a team that adds one to a codebase whose files have no suffixes would
   otherwise get one violation per file from a rule it did not write. Off is not silent: the summary
   gives, for each such matrix, the count of files in its files that no row has. The other option is
   on by default, with `everyFileHasARow: false` to turn it off. It is the stricter default, and a
   team with a baseline can adopt it, but it puts a naming decision inside every import table.

9. **A matrix without a role pattern has declared elements and selectors as rows.** The developer
   writes `mayOnlyImport: { 'canvas-document': ['canvas-document', 'contract'] }`, and each key is an
   element or selector the config declares. This is the shape Nx calls dependency constraints and
   eslint-plugin-boundaries calls element types: what each part of the code may import. It is one
   builder with one shape; the role pattern decides how archangel reads the keys. A row key that
   names nothing is `rule-unknown-element` with the closest name. The other options are a second
   builder, which would repeat every option and every diagnostic, or rows keyed by inline patterns,
   which have no name to build the row's name from.

10. **`off` takes row names only, so you drop a whole matrix by removing it.** A matrix name in
    `off` matches no rule and is refused with the closest row names, as any unknown name is in 0.1.

11. **A matrix with no row, or whose rows together have no file, fails the check with
    `matrix-empty`.** Such a matrix checks nothing while the check stays green. A row with no file
    is declared intent, but a whole matrix with none is almost always a misspelt role pattern, such
    as `.tz` for `.ts`, and an empty table is an unfinished one. This is what 0.1 does for a preset
    with no file (`preset-empty`). The other option is one summary line, as for a preset's matrix
    (item 12): the check passes, and the developer must read the summary to see that the matrix
    checks nothing.

12. **A matrix in a preset never gives `matrix-empty`.** As for everything else a preset declares,
    `preset-empty` covers the preset. When a preset's matrix has no row with a file, the summary
    gives one line that names the matrix and its role pattern, so a preset with a misspelt role
    pattern still passes, but the developer can see that the matrix checks nothing.

13. **enhance3's config written by hand is still long: about 1,400 to 1,600 lines.** "What enhance3
    gains" gives the parts. This change removes the restated tables, the restated role lists and
    the per-package double negatives, about 165 lines. What remains is mostly one-off decisions: 46
    dependency rules, each with a reason and a written rejected example (about 460 lines), the
    overrides of the Feature-Sliced Design preset written again for each of three apps (about 150),
    about 20 naming rules for folders whose role list is not a matrix's (about 160), and the literal
    file lists in enhance3's own policies (about 175). The options are to ship 0.2 as designed and
    measure the rewrite, or to add to this change one of three later features that each remove one
    block: an example written as one line, "this file imports that file" (about 250 lines); a
    naming condition that takes a list of roles (about 100 lines); or a preset option for the
    Feature-Sliced Design rule that enhance3 overrides (about 120 lines).

## How it works

### An area on a rule

`within` takes what a rule side takes: element and selector names, path patterns, packages, and
`all`, `any` and `not`. On `mayOnlyImport`, it limits which imports the rule judges: an import from
the rule's files whose other end is outside the area is not the rule's business. Inside the area the
rule is the rule of 0.1: an import of something that is neither on the side nor among the rule's own
files is a violation.

```ts
rule({
  name: 'entities-stay-in-the-domain',
  files: 'entity',
  mayOnlyImport: ['entity', 'value-object'],
  within: 'apps/api/src/**',
})
```

This rule does not judge `import { z } from 'zod'` or an import of the web app. An area may name
packages too, so `within: ['apps/api/src/**', { packages: ['*'] }]` judges every package import
and the side must then list the permitted packages. The import limits, `names`, `specifier`,
`instance` and `dependency`, work as in 0.1 and only on imports that the area lets through. An
import that does not resolve is still never a violation of a condition; it has its own diagnostic.

On `mayOnlyBeImportedBy`, the area limits the importers: `{ files: 'db', mayOnlyBeImportedBy:
'api', within: 'apps/**' }` judges imports from the apps, and leaves a seed script in `scripts` to
other rules. On `mayNotImport` an area narrows the side: `{ mayNotImport: 'use-case', within:
'apps/api/src/**' }` means `mayNotImport: { all: ['use-case', 'apps/api/src/**'] }`, and archangel
accepts either. Conditions that check no import refuse `within`, as they refuse every import option.

An inline pattern in an area that matches no file is `selector-empty`, as every other inline
pattern in a rule is, because a misspelt area makes a rule that never fires. A misspelt selector
name in an area is `rule-unknown-element` with the closest name, as on a side. An empty area, such as
`within: []`, is refused with `config-invalid`, because it would turn the rule off without a word.

The violation message names the area beside what the file may import: "`canvas.entity.ts` may
import only entity and value-object within `apps/api/src/**`". `archangel explain` shows the rule
as its sentence with `within`.

### A matrix of roles

A matrix is written with the `matrix` builder, or in JSON with `"kind": "matrix"`:

```ts
matrix({
  name: 'api-domain',
  files: { all: ['apps/api/src/*/domain/**', { not: 'role-exempt' }] },
  within: 'apps/api/src/**',
  roles: '**/*.{role}.{ts,tsx}',
  everyFileHasARow: true,
  mayOnlyImport: {
    entity: ['entity', 'value-object', 'error'],
    'value-object': ['value-object', 'error'],
    error: ['error'],
  },
})
```

- `name` is required, because every row's name starts with it.
- `files` are where the rows' files are. They default to the files of the area.
- `within` is the area, as on a rule. It is optional. Without it, each row judges every import, as
  a `mayOnlyImport` rule without an area does in 0.1, packages included, and `files` defaults to
  every file.
- `roles` is one role pattern with the capture `{role}`. The keys of the table are the roles. The
  pattern is read once, and each role fills the capture with its name as literal text, so a
  character in a role name never acts as a wildcard. A role pattern that breaks the rules of every
  pattern, such as `**//*.{role}.ts`, is `pattern-invalid` and names the matrix.
- `everyFileHasARow` is optional and off by default ("Decisions that need you", item 8).
- `mayOnlyImport` is the table. In it, each key is a row and each value lists the cells, which
  are roles of this matrix, declared elements and selectors, path patterns or packages. A cell that
  names a role means every file of that role, not only the role's files inside `files`. The cell
  `entity` permits any `*.entity.ts` file under `apps/api/src`.
- `reason`, `link`, `imports`, `names`, `specifier`, `instance`, `dependency` and `examples` belong
  to the matrix. Each row takes all of them except `examples`, which archangel runs once against all
  the rows.

Archangel reads a cell name first as a role of the matrix and then as a declared name. A role with
the name of a declared element or selector shadows that name inside its own matrix only, and
`archangel explain` shows which one each cell means. The other option, refusing a role that has a
declared name, would refuse common role names such as `types` and `consts`, which enhance3 uses as
both. A misspelt cell is `rule-unknown-element` with the closest role or declared name.

### A matrix of elements

Without `roles`, the keys of the table are elements and selectors that the config declares, and a
row's files are that element's or selector's files inside the matrix's files:

```ts
matrix({
  name: 'packages',
  files: { not: 'test' },
  mayOnlyImport: {
    'canvas-format': ['canvas-format'],
    'canvas-document': ['canvas-document', 'contract', 'ui-geometry', { packages: ['*'] }],
  },
})
```

Everything else is the same as for a matrix of roles: the area and its default, `files` and its
default, the cells, the options, the names `packages.canvas-document`, the rows with no file,
`matrix-empty`, the examples and `everyFileHasARow`. A cell that names a row key is that declared
name, as on any rule side. A row key that names no declared element or selector is
`rule-unknown-element` with the closest name. A row key that names a package element is refused with
`config-invalid`, because packages have no files that could import. A row key must also be a name
that a row name can hold, so a declared name with a dot, colon, slash or space cannot be a key.

### The sentences of a row

When it loads the config, archangel turns each row into one rule, named `<matrix>.<row>`, with the
matrix's options. The rule has one sentence when the row lists itself:

```ts
rule({
  files: { all: [{ all: ['apps/api/src/*/domain/**', { not: 'role-exempt' }] }, '**/*.entity.{ts,tsx}'] },
  mayOnlyImport: ['**/*.entity.{ts,tsx}', '**/*.value-object.{ts,tsx}', '**/*.error.{ts,tsx}'],
  within: 'apps/api/src/**',
})
```

A row that does not list itself gets a second sentence, `mayNotImport` of its own files, because
`mayOnlyImport` permits those files. A rule of several sentences is broken when any of them is, as
in 0.1. A file that two rows have is in the files of both rules, and each judges it. Because each
row is an ordinary rule, `off: ['api-domain.policy']` drops one row,
`// archangel-allow api-domain.entity: …` permits one import, and the baseline records the row's
name. `archangel explain` lists the rows whose files include a file and shows each as its sentences,
and the documentation page for matrix shows both shapes.

With `everyFileHasARow`, archangel adds one more rule, `<matrix>.every-file-has-a-row`, of one
sentence: the matrix's files that are in no row must not exist. Its violation names the file and
says what it could be:

```
apps/api/src/canvas/domain/canvas.ts  api-domain.every-file-has-a-row
  This file is in the files of api-domain, and no row has it. Give it a role: the role pattern
  **/*.{role}.{ts,tsx} with one of entity, value-object, error.
```

In a matrix of elements the message lists the rows instead: "it is in none of the rows of
api-layers: api-domain, api-application, api-infrastructure, api-shared".

### Rows with no file, files with no row, and a matrix with none

A matrix declares a team's intent, and a role that no file has yet is part of that intent: a domain
with no aggregate root today may have one next week. So a row with no file gives no diagnostic, and
the role pattern never gives `selector-empty`. The summary lists such rows, in text and in JSON, as
a preset's unused layers are listed:

```
  api-domain.aggregate-root has no file in this repository
  api-application has 3 files that no row has; everyFileHasARow: true refuses them
```

The JSON summary lists the same rows under `rowsWithNoFile`, and the counts under `filesWithNoRow`,
by matrix name. A matrix with `everyFileHasARow` is not counted, because its files with no row are
violations. A config without a matrix has neither key, so its JSON output is the output of 0.1. When
the first file of a role appears, in a check or in the editor, the row applies at once, because the
classes already carry the role. When the last file of a role goes, the row is listed again.

A matrix with no row, or whose rows together have no file, gives `matrix-empty` and fails the check.
For an empty table the message says that the matrix has no row, and tells the developer to add a
row or remove the matrix. Otherwise it says that no file in the matrix's files matches the role
pattern with any row name in place of `{role}`, or is in any of its rows, and tells the developer to
check the pattern's extension and the row names, or to remove the matrix. When the area itself
matches nothing, the area's own `selector-empty` names the misspelt pattern, and archangel does not
add `matrix-empty` for the same mistake. The same holds when the matrix's `files` pattern matches
nothing. A matrix in a preset gives one summary line instead ("Decisions that need you", item 12):

```
  fsd-x/roles matches no file: no file matches its role pattern **/*.{role}.ts
```

and its name joins the JSON list `presetNamesMatchingNothing`, which a 0.1 config never fills with a
matrix.

### How a matrix is proven

0.1 asks every rule for a rejected example, because a rule that cannot fire looks the same as one
that works. On enhance3, the builder met that demand by generating one rejected and one accepted
example per row from the same table that generated the rule, with invented paths such as
`apps/api/src/example/domain/a.entity.ts`. Such an example proves that the generator agrees with
itself: the path was made to match the pattern that the rule then matches.

In 0.2 a matrix is proven as a `layers` rule is: by examples its author writes. The matrix needs at
least one rejected example, and each of its rules, the rows and the rule of `everyFileHasARow`,
counts the matrix's rejected examples as its own, so a matrix with one gives no `rule-unproven`. A
matrix with none gives one `rule-unproven` that names the matrix, not one per row. A written
rejected example of a matrix passes when any of its rules rejects it, and archangel names that rule.
When none rejects it, the example fails, and the message says that no row of the matrix rejected it.
A matrix may also have accepted examples. One passes when no rule of the matrix rejects it and at
least one row judged one of its imports. When no row judged an import, the example fails with
`example-proves-nothing`, as in 0.1, and the message says what to give it: "no row of the matrix
"api-domain" looked at an import of this example, so accepting it proves nothing; give the example an
import from a file of a row into `apps/api/src/**`". Without an area the message ends at "from a
file of a row". An accepted example of a plain rule with an area fails the same way when its imports
are outside the area, and the message names the area: "give the example an import from the rule's
files into `apps/api/src/**`". The examples run through the same in-memory check as every other
example: parse, resolve with the repository's tsconfig, classify, evaluate.

A written example shows intent: that a role pattern catches the files the author means, and that a
cell is the right policy. Authors should write more than one where a row's intent is easy to get
wrong, such as a file with two role suffixes.

### Types

`defineConfig` types the new forms with the names the config writes:

- `within` is typed as a rule side, so a misspelt selector in an area is a red line. It is allowed on
  `mayNotImport`, `mayOnlyImport` and `mayOnlyBeImportedBy` sentences only.
- With `roles`, the `matrix` builder infers its role names from the keys of the table as literal
  types, and a cell must be one of those roles or a side item. Without `roles`, a key must be a
  declared element or selector name, and a cell must be a side item.
- The matrix name and the row names are checked by one type, and a name with a dot, a colon, a
  slash or a space becomes the text of the type error.
- `everyFileHasARow` is a boolean.

A table that several matrices share is a plain `const` object in the config, and its keys stay
literal types, so sharing a table needs no computed name.

### What enhance3 gains

Today the domain layer's matrix lives in four places: a table of strings in the vocabulary module, a
function in the builder that turns each row into a rule, a glob at load time that drops rows for
roles no file has, and a separate naming rule that repeats the table's roles. For one row, the
builder produces this:

```js
{
  name: 'role:api:domain:entity',
  reason: 'A .entity file may import only: .entity, .value-object, .domain-event, .specification, .policy, .error, .types, .consts, and the barrels of what it may import.',
  files: { all: ['apps/api/src/*/domain/**', '**/*.entity.{ts,tsx}'] },
  mayNotImport: {
    all: [
      'apps/api/src/**',
      { not: ['**/*.{entity,value-object,domain-event,specification,policy,error,types,consts}.{ts,tsx}'] },
    ],
  },
  examples: {
    rejected: [{ 'apps/api/src/example/domain/a.entity.ts': "import './b.use-case.ts';\n", 'apps/api/src/example/domain/b.use-case.ts': '' }],
    accepted: [{ 'apps/api/src/example/domain/a.entity.ts': "import './b.entity.ts';\n", 'apps/api/src/example/domain/b.entity.ts': '' }],
  },
}
```

and ten more like it. The reason's last words come from a template that every table shares; for
the API tables the builder adds no index file. The naming rule for the same folder says again which
roles exist:

```js
{
  name: 'naming:api:domain',
  files: { all: ['apps/api/src/*/domain/**/*.ts', { not: ROLE_EXEMPT }, { not: '**/*.{entity,value-object,aggregate-root,domain-event,domain-service,policy,specification,port,error,types,consts}.{ts,tsx}' }] },
  mustNotExist: true,
}
```

With 0.2 the whole layer, naming rule included, is one table in the config:

```ts
matrix({
  name: 'api-domain',
  reason: 'Each kind of domain file depends only on the kinds of domain file below it.',
  files: { all: ['apps/api/src/*/domain/**', { not: 'role-exempt' }] },
  within: 'apps/api/src/**',
  roles: '**/*.{role}.{ts,tsx}',
  everyFileHasARow: true,
  mayOnlyImport: {
    entity:           ['entity', 'value-object', 'domain-event', 'specification', 'policy', 'error', 'types', 'consts'],
    'value-object':   ['value-object', 'error', 'types', 'consts'],
    'aggregate-root': ['entity', 'value-object', 'domain-event', 'specification', 'policy', 'error', 'types', 'consts'],
    'domain-event':   ['value-object', 'types', 'consts'],
    'domain-service': ['entity', 'value-object', 'aggregate-root', 'domain-event', 'specification', 'policy', 'port', 'error', 'types', 'consts'],
    policy:           ['entity', 'value-object', 'aggregate-root', 'specification', 'error', 'types', 'consts'],
    specification:    ['entity', 'value-object', 'aggregate-root', 'types', 'consts'],
    port:             ['entity', 'value-object', 'error', 'types', 'consts'],
    error:            ['value-object', 'entity', 'error', 'types', 'consts'],
    types:            ['types', 'consts'],
    consts:           ['consts'],
  },
  examples: {
    rejected: [{
      'apps/api/src/board/domain/board.entity.ts': "import { save } from '../application/save.use-case';\n",
      'apps/api/src/board/application/save.use-case.ts': 'export const save = () => {};\n',
    }],
  },
}),
```

`role-exempt` is one selector, declared once, for the files that carry no role: index files, tests,
type tests, `.d.ts` files, tool configs and composition roots. The rows keep their meaning, the
`domain-event` row included, because a row permits only what it lists. The rows for roles that no
file has yet stay in the table and show in the summary. Their names become `api-domain.entity` and
so on, which exception comments can use.

`everyFileHasARow` replaces the 10 naming rules whose folder and roles are exactly a matrix's files
and rows: `naming:api:domain`, `naming:api:application`, `naming:api:infrastructure`,
`naming:api:shared`, the application and infrastructure rules of `cli` and of `desktop`,
`naming:contract` and `naming:capture-browser`. It does not replace the per-segment naming rules of
the frontend apps and of `packages/ui`: those give each segment, such as `ui` or `model`, its own
list of roles, while the frontend matrix covers a whole app with all 23 roles.

**Package dependencies.** enhance3 writes each package's dependencies as two rules: the package's
source may import only some packages and its own files, and code outside it may import it only
through its public entry files. For `canvas-document`:

```js
{
  name: 'canvas-document-is-pure',
  files: { all: ['packages/canvas-document/src/**', { not: '**/*.test.ts' }] },
  mayOnlyImport: ['packages/canvas-document/src/**', 'packages/contract/**', 'packages/ui/src/geometry/**', { packages: ['*'] }],
},
{
  name: 'canvas-document-public-entry',
  files: { not: 'packages/canvas-document/**' },
  mayNotImport: { all: ['packages/canvas-document/src/**', { not: 'packages/canvas-document/src/{document,flex-grid,typography,paint-projection,gradient,construction}/index.ts' }] },
},
```

Seven packages have the first rule, and four have the second. With 0.2, each package's source is a
declared element, the first rules become one matrix of elements, and the second become `entry`
rules:

```ts
matrix({
  name: 'packages',
  reason: 'A package depends only on the packages its row lists, so each one stays usable on its own.',
  files: { not: 'test' },
  mayOnlyImport: {
    'artwork-paint':   ['artwork-paint'],
    'canvas-format':   ['canvas-format'],
    'ui-geometry':     ['ui-geometry'],
    'canvas-document': ['canvas-document', 'contract', 'ui-geometry', { packages: ['*'] }],
    'figma-import':    ['figma-import', 'canvas-document', 'contract', { packages: ['*'] }],
    'capture-browser': ['capture-browser', 'contract', { packages: ['rrweb-snapshot', '@rrweb/types'] }],
    'sdk-core':        ['sdk-core', 'contract'],
  },
  examples: {
    rejected: [{
      'packages/canvas-format/src/version.lib.ts': "import '../../contract/src/canvas.schema';\n",
      'packages/contract/src/canvas.schema.ts': '',
    }],
  },
}),
entry('canvas-document', '{document,flex-grid,typography,paint-projection,gradient,construction}/index.ts', {
  reason: 'Consumers reach the canvas document code through one area entry at a time.',
}),
entry('figma-import', 'index.ts', { reason: 'Consumers reach Figma conversion through its one public entry.' }),
entry('artwork-paint', '**/index.ts', { reason: 'Consumers share paint semantics through the public entry.' }),
entry('capture-browser', '**/index.ts', { reason: 'Consumers share the capture kernel through its public entry.' }),
```

The matrix has no area, so each row judges every import, packages included, as the 0.1
`mayOnlyImport` rules do; that is why `artwork-paint` may import no npm package and
`canvas-document` lists `{ packages: ['*'] }`. Each row lists its own package, so a row means what
the rule it replaces means.

The same form writes enhance3's API layers, which are four `api-closed-world` rules and one more that
says every file in `apps/api/src` is in a layer:

```ts
matrix({
  name: 'api-layers',
  reason: 'Dependencies point inward: infrastructure, then application, then domain.',
  files: { all: ['apps/api/src/**', { not: 'api-exempt' }] },
  within: '{apps,packages}/**',
  everyFileHasARow: true,
  mayOnlyImport: {
    'api-domain':         ['api-domain', 'api-packages'],
    'api-application':    ['api-application', 'api-domain', 'api-shared', 'api-packages'],
    'api-infrastructure': ['api-infrastructure', 'api-application', 'api-domain', 'api-shared', 'api-packages'],
    'api-shared':         ['api-shared', 'api-packages'],
  },
  examples: { rejected: [/* a domain file that imports an application file */] },
}),
```

With an area of the repository's own files, a package import is no row's business, so the entry
`{ packages: ['*', 'node:*'] }` that each layer rule lists today goes away. A third matrix of
selectors writes the four web-app rules that say what an editor model may import.

A team with a single role rule rather than a table writes it with `within` and no double negative:

```ts
// Today
{
  name: 'role:api:domain:entity',
  files: { all: ['apps/api/src/*/domain/**', '**/*.entity.{ts,tsx}'] },
  mayNotImport: { all: ['apps/api/src/**', { not: ['**/*.{entity,value-object,error,types,consts}.{ts,tsx}'] }] },
}

// With 0.2
rule({
  name: 'api-domain-entity',
  files: { all: ['apps/api/src/*/domain/**', '**/*.entity.{ts,tsx}'] },
  mayOnlyImport: '**/*.{entity,value-object,error,types,consts}.{ts,tsx}',
  within: 'apps/api/src/**',
})
```

This is the same rule only because its list includes its own role, `entity`. `within` keeps
`mayOnlyImport`'s permission for a rule's own files. So a rule whose list leaves out its own files
also needs `mayNotImport` of those files, or should be a matrix row.

**An estimate of enhance3's config written by hand.** enhance3's builder produces about 380 rules
from 9 distinct role tables (107 rows, 709 cells), which it applies 14 times to give up to 195 rows,
and about 180 other rules: 61 dependency rules, 5 API layer rules, 107 naming rules and 9 overrides
of the Feature-Sliced Design preset. With `within`, both matrix forms and `everyFileHasARow`, one
typed config would take about this much:

| Part | Rules today | Lines by hand |
|---|---|---|
| Ignore list, presets and `off`, plugins, elements and selectors, with an element per package, `role-exempt` and nine segment index selectors | | 90 |
| 9 role tables as shared `const` objects, one line or two per row, with the segment index cells of the frontend and ui rows | | 170 to 220 |
| 14 role matrices, each about 12 lines with its written example; 10 of them with `everyFileHasARow` | up to 195 rows and 10 naming rules | 170 |
| 3 matrices of elements (API layers, packages, editor models) and 4 `entry` rules | 20 | 75 |
| Naming rules for folders whose roles are not a matrix's, written once for all apps with brace alternatives, plus the file-stem and folder-style rules | 97 | 160 |
| Other dependency rules, one by one, each with a reason and a written rejected example | 46 | 420 to 500 |
| Overrides of the Feature-Sliced Design preset, for each of three apps | 9 | 150 |
| Literal file lists in those rules, such as the web app's 110 files that integration tests may import | | 150 to 200 |
| **Total** | about 380 | **about 1,400 to 1,600** |

Without the matrix of elements and `everyFileHasARow`, the same method gives about 165 more lines:
the 10 naming rules, the 5 API layer rules and the 15 package and editor-model rules written one by
one. The previous estimate in this design, 1,000 to 1,200 lines, left out the written examples, the
preset overrides and the literal lists. The 95 per-segment naming rules shrink to about 20 because a
hand-written rule covers all apps and layers with brace alternatives, not because of this change.
This is an estimate from those counts, not a measurement; the rewrite is a separate task after this
change ships. What still makes the config long is "Decisions that need you", item 13.

## Decisions

**1. Decide the area when a pair entry is filled, not when an import is checked.** An area is a set
of files and packages, so whether a class is in it is known before any import is looked at. The pair
entry for a pair whose other end is outside the area simply does not hold the sentence. This is how
0.1 already decides the side of a `mayNotImport` rule, and dependency-cruiser's `from` and `to`
filters describe a rule by its two ends in the same way. The per-import cost stays one look-up.

**2. Write a matrix as a table, and compile each row to an ordinary rule.** The normal solution for
an import table in the JavaScript ecosystem is eslint-plugin-boundaries' `element-types` rule
(https://github.com/javierbrea/eslint-plugin-boundaries): element types found by path patterns with
captures, `default: 'disallow'`, and one `{ from, allow }` policy per type, where a type may not
import its own type unless the policy lists it. Nx's dependency constraints have the same shape,
keyed by project tags. Archangel follows them in the table form, in disallow by default and in the
own row permitted only when listed. It departs in one place: each row becomes a rule with its own
name, because `off`, exceptions and the baseline refer to rules by name. 0.1's `layers` compiles to
one rule of several sentences; a matrix cannot be one rule, because one row must be turned off or
excepted without the others. Each row becomes one rule of one or two sentences, so exceptions, the
baseline, `off`, explain, examples, presets and the editor need no new code paths. The rejected
option, a matrix condition evaluated as a table, would need its own rule identity per row anyway.

**3. Keep `mayOnlyImport`'s own-files permission, and add a sentence for a row that does not list
itself.** Changing `mayOnlyImport` would break 0.1 configs. Adding a hidden flag would make a row
mean something no sentence can say, and the 0.1 promise is that every shorthand is shown as its
sentences. A rule of several sentences already exists: `layers` compiles to one, and the
Feature-Sliced Design preset writes several of Steiger's rules (Steiger is the FSD linter) as one
rule with several sentences. The row reuses it.

**4. Name rows from the matrix and the row, and refuse names that exception comments cannot carry.**
A stable name is what `off`, exceptions and the baseline refer to, and 0.1 already refuses unnamed
preset rules for this reason. Deriving the name from the row keeps it stable when cells change,
which is what a baseline needs. The cause of the colon problem is the exception parser, which ends
the rule name at the first colon. Fixing the parser stays out of 0.2: any reading that lets a name
hold a colon changes what some 0.1 comment means, such as `archangel-allow no-legacy:migration`,
which today names the rule `no-legacy`, and 0.2 promises that every 0.1 config gives the same
results. Refusing the colon only in the new names costs nothing.

**5. Treat an empty row as declared intent, and an empty matrix as a mistake.** This is the rule 0.1
gives presets: a preset's empty layer is listed, and a preset with no file at all is `preset-empty`.
A matrix is the same kind of declaration, so it gets the same treatment, and no author has to glob
the repository to keep a check green. An empty table is the plainest case of a matrix with no file,
so it gets the same diagnostic.

**6. Prove a matrix with written examples, as a `layers` rule is proven.** In 0.1, `archangel test`
asks for one rejected example of each rule, not one per sentence, so one example proves a shorthand
that stands for many sentences. A matrix compiles to several rules only so that each row has a name;
its rows still share one table and one author, so they share its examples. No new mechanism is
needed. An example that archangel derives for each row from repository files would be one, and
proving rules is not archangel's core, so it does not earn that cost here.

**7. Find roles with a capture filled from the row names.** The capture syntax already exists for
instances, and filling it with known names gives one literal pattern per role, with no list to keep
in step with the table. A file may still match the patterns of two roles; it is then in both rows
("Decisions that need you", item 6). The rejected options are a list of patterns per role, which is
what enhance3's builder computes today, and a regular expression, which archangel's patterns do not
use anywhere.

**8. Write the requirement that every file has a row as one more sentence of the matrix.** A
`mustNotExist` sentence on the matrix's files minus the files of its rows says exactly what
`everyFileHasARow` means, and 0.1 already checks such sentences, reports them per file and exempts
them from `selector-empty`. eslint-plugin-boundaries has the same option, `boundaries/no-unknown-files`,
as a separate rule. Making it a rule of the matrix keeps the list of roles in one place, which is the
point.

**9. Read the keys as declared names when there is no role pattern.** A declared element or selector
already is a named set of files, so a row keyed by it needs no new way to find files, and its name
gives the row's name. The rows, the names, the examples and the diagnostics are the same code for
both forms.

## Cost

An area adds no work per import. When archangel fills a pair entry, it tests one more set membership
for each sentence with an area: work that grows with connected class pairs times sentences, which is
the cost that filling the pair entries already has in 0.1. An import whose other end is outside the
area finds no sentence of that rule in its pair entry and counts no rule evaluation for it. A test of
the work counts holds this: entity files with 10 package imports and with 1,000 give the same count
of rule evaluations. Its fault control, a deliberately slow build that tests the area on every
import, must make that test fail.

A matrix of roles with R rows adds R rules of one or two sentences, R sets for the rows' files, one
set for the area, and one set per distinct cell pattern, which rows share. Each set is one more
membership test for each file classified and one more bit in each class key, so the work to classify
a file grows with the number of sets: rows plus area plus distinct cell patterns. That depends on
the config, not on the number of files, and it applies in a check and to each file reclassified
after an edit. Each row adds one glob to the one compiled set of patterns, so pattern passes stay
one per file as long as that set compiles as one. A matrix of elements adds no set for its rows,
because each row key is an element or selector that classes already carry; it adds only its cell
patterns. An import costs one look-up and one evaluation for each row whose files contain the
importing file and whose pair entry holds the row's sentence, which is usually zero or one, and two
for a file that two rows have. The work-count test holds this: a matrix of 5 rows and the same matrix
with 45 more rows that no import connects give the same rule evaluations and the same pattern
passes, and a fault control that tests every row per import fails it.

`everyFileHasARow` adds one structure sentence per matrix. Archangel decides it once per class, as
it decides every `mustNotExist` sentence, from sets the classes already carry, so its work grows with
classes and not with imports, and it adds no rule evaluation for any import. A work-count test holds
that a check with the option and the same check without it give the same rule evaluations.

Each row's one or two sentences are also added to every pair-entry fill, as every sentence is: the
fill looks at each sentence for each new pair of classes, including pairs the matrix has nothing to
do with, and the row sets split classes, so the matrix adds pairs as well. That work grows with
connected class pairs times all sentences, the cost that filling the pair entries has in 0.1, and it
is paid once per pair in a check and once per new pair after an edit. No work count holds it, because
rule evaluations count only the sentences an import finds in its pair entry. So at enhance3's size
the recorded times are the evidence. enhance3's generated config already has the same rows as
sentences, so the before-and-after run should show no rise.

The list of rows with no file, the count of files with no row and the `matrix-empty` test run in
each assembled report, as the test for empty selectors does: in a check, and in the editor after
every edit. Each reads the count of files in each class, so the work grows with classes times matrix
rows and not with files.

Proving a matrix adds no work beyond its written examples, which cost what any example costs.

At enhance3's size a 50-row fixture cannot show whether the compiled pattern set still compiles as
one. So the change is measured once on enhance3 before it ships: the 0.1 generated config against
the hand-written matrix config, comparing the pattern passes, files classified and rule evaluations
that `archangel check --stats` reports, and the times.

## Risks

- **A row that permits itself only when listed surprises a reader who knows `mayOnlyImport`.**
  The matrix page shows both sentence shapes, the violation names the row and its cells, and the
  choice is item 1 of "Decisions that need you".
- **One written example per matrix does not prove every row.** A row with files whose cells are
  wrong passes until an example or a violation shows it. The property tests, which compare the
  compiled check with a slow reference that applies each condition to each import directly, prove
  that every cell is decided as written for random configs, and rows that cannot fire for lack of
  files are listed.
- **A role name that is also a declared name.** The role shadows the declared name inside its
  matrix. A reader who expects the selector sees which one a cell means in `archangel explain`.
- **A file in two rows gets two violations for one import.** Both name their row, and `explain`
  shows both rows, so the reader sees why. A role pattern that does this by mistake, such as
  `**/{role}/**`, shows on the first such import.
- **`everyFileHasARow` is off by default, so a team can miss it.** The summary counts each matrix's
  files with no row, the matrix page shows the option beside the table, and `archangel init` is not
  changed.
- **A wide matrix adds many sets to each class.** Rows of one matrix rarely overlap, so the number of
  distinct classes grows with real combinations, not with rows. The work-count tests watch pattern
  passes and evaluations. The scheduled benchmark does not check enhance3, so the before-and-after
  run in Cost is the evidence at that size.

## Compatibility and release

Everything is additive, so the release is 0.2.0. A 0.1 config loads unchanged and gives the same
diagnostics, summary, JSON output and exit status. Snapshots of the text and JSON output of
`archangel check` and `archangel test`, recorded at 0.1.1 before any other commit of this change,
hold this. The config gains `within` on three conditions and the `matrix` kind, with
`everyFileHasARow`, in TypeScript and in the JSON schema. The text and JSON summaries of a config
with a matrix gain the rows with no file and the count of files with no row. The JSON keys
`rowsWithNoFile` and `filesWithNoRow` are left out when the config has no matrix, so 0.1 output does
not change; the JSON `schemaVersion` stays, and a reader that ignores unknown fields is unaffected.
One rule of 0.1 changes for role patterns only: `{role}` may appear twice in a role pattern, where
any repeated capture name is `pattern-invalid` today. No 0.1 config has a role pattern. One new
diagnostic code, `matrix-empty`, gets its page. The command line gains no command and no option. The
glossary gains area, role, role pattern, matrix, matrix row and cell, and the shorthand kinds become
four.

## Limits

- A file in a matrix's files that no row has is judged by no row as an importer. With
  `everyFileHasARow` it is a violation; without it, the summary counts it under its matrix, in text
  and in JSON. So it never passes in silence.
- A file in the area that no row has is still a target that rows judge: a row refuses an import of
  it unless a cell names it, and the violation names it.
- A role pattern names only the roles that the table names. A file whose name ends in a role the
  table does not have is in no role, and the two limits above apply to it.
- A file that two rows have is judged by each, so it can give two violations for one import.
- A matrix takes one role pattern, or none. Roles that a directive finds are written as a matrix of
  selectors, because a selector can be a row key.
- A capture other than `{role}` in a role pattern matches as `*` does, never an empty part, and gives
  no instance; a rule that needs instances uses an element with a capture.
- A row with no file is listed in the check summary. It is proven by the matrix's examples like any
  other row.
- `off` takes row names, not a matrix name ("Decisions that need you", item 10).

None of these drops work silently. Searched: the rule loader and its shorthand compilation, pattern
parsing and its repeated-capture check, the compiled architecture and pair-entry filling, the
structure conditions, the check's empty-set and preset-empty reports, the example runner and the test
command, exception parsing, the baseline file, the merge of presets and `off`, the config types and
the JSON schema.

## Where in the code

This section is for whoever builds the change.

| Part | Where |
|---|---|
| `within` in the config model | `crates/archangel_config/src/model.rs`: a new `within: Option<Side>` on `ImportOptions` beside `stop_at`; `Condition::side()` unchanged |
| Reading `within` | `crates/archangel_config/src/rules.rs`: add `"within"` to `OPTIONS`, parse it as a side in `options`/`read_options`, and refuse an empty side there; `options_belong` already refuses import options on conditions that check no import, and refuses `within` there with them |
| The matrix kind (Decisions, items 2, 3, 4, 9) | `crates/archangel_config/src/rules.rs`: `"matrix"` in `SHORTHANDS`, a `matrix` method beside `isolate` and `entry` that returns several `Rule`s; `parse_rule` returns `Vec<Rule>`, and its two callers in `parse.rs` push every row; each row copies `reason`, `link`, `imports`, `names`, `specifier`, `instance` and `dependency` and has no examples of its own; without `roles`, each key is resolved through `Names` as a declared element or selector, a package element is refused; precedent `layer_sentences` |
| `everyFileHasARow` (Decisions, item 8) | `crates/archangel_config/src/rules.rs`: the `matrix` method adds one rule `<matrix>.every-file-has-a-row` with one `MustNotExist` sentence on `{ all: [files, { not: [row sets] }] }`, `RuleKind::MatrixRow` with no row; its message, which lists the roles or rows, in `crates/archangel_rules/src/messages.rs`; `MustNotExist` is already exempt from `selector-empty` |
| The matrix's own data | `crates/archangel_config/src/model.rs`: a new `Config.matrices: Vec<Matrix>`, as `Config.presets` keeps what a preset declared, holding each matrix's name, role pattern (if any), files, area, examples, `every_file_has_a_row` and preset; a new `RuleKind::MatrixRow { matrix: usize, row: Option<String> }` refers to it by index; `parse.rs` roots the matrix's examples there once, not on each row |
| Role patterns with `{role}` (Decisions, item 7) | `crates/archangel_config/src/pattern.rs`: tokenize the role pattern once, then for each role replace every `Token::Capture("role")` with `Token::Literal(role)`, which the expression builder escapes, producing one `Pattern` per role; this runs before the repeated-capture check, so only `{role}` may repeat (`PatternError::RepeatedCapture` still fires for any other name); other captures stay captures whose values no row reads; a `PatternError` in a role pattern is `ProblemCode::PatternInvalid` naming the matrix |
| Cell name look-up | `crates/archangel_config/src/selectors.rs` (`Names`, `resolve` in `rules.rs`): look up the matrix's roles before declared names; `rule-unknown-element` suggestions include roles; explain records which one a cell resolved to |
| Row names and the refused characters | `crates/archangel_config/src/merge.rs` (`preset_rule_name`, the slash check, the prefix `<preset>/` on a preset's matrix and its rows, and a config rule replacing a preset row of the same prefixed name) and `crates/archangel_rules/src/exceptions.rs` (`parse` splits at the first `:`, the reason for refusing colons; unchanged); two matrices with one name give `rule-name-duplicate` in `crates/archangel_config/src/parse.rs` |
| Area in pair-entry filling (Decisions, item 1) | `crates/archangel_rules/src/architecture.rs`: move `side_files`, `side_elements` and `side_packages` of `Compiled` into one compiled-side value, used for `side` and for a new `area: Option<_>`, with one membership test; `applies` adds the area test for `MayNotImport` and `MayOnlyImport` (to end) and `MayOnlyBeImportedBy` (from end); `considers` adds it too, so accepted examples outside the area prove nothing; `report_inline` reports area patterns |
| Fault controls | `crates/archangel_rules/src/architecture.rs`: fail points `rules::area_per_import` and `rules::every_row`, beside `rules::search_every_rule` |
| Rows with no file, files with no row and `matrix-empty` (Decisions, item 5) | `crates/archangel_rules/src/check.rs`: in `Architecture::assemble`, beside `empty_selectors` and `empty_presets`, using `Classes::files_in` over each row's set and over the matrix's files minus its rows; `matrix-empty` for an empty table or rows with no file, not when the area's or the `files` pattern is `selector-empty`; the role pattern and area for the message come from `Config.matrices`; it runs on every assemble, so after each edit through `Session::finish` in `crates/archangel_engine/src/session.rs`; `Summary` gains `rows_with_no_file` and `files_with_no_row`, beside `preset_names_matching_nothing`, because the JSON key `presetNamesMatchingNothing` says preset and cannot be renamed without changing 0.1 output; `crates/archangel/src/render/json.rs` writes them as `rowsWithNoFile` and `filesWithNoRow` only when the config has a matrix, and `render/text.rs` prints the two lines; a preset's matrix with no file at all adds one text line and its name to `preset_names_matching_nothing`; selector-empty exemption for role sets in `report_inline` |
| Examples of a matrix (Decisions, item 6) | `crates/archangel_rules/src/examples.rs`: `unproven_rules` reads the examples in `Config.matrices`, counts them for each `MatrixRow` rule, and gives one `rule-unproven` per matrix without a rejected one; a matrix example runs once against all its rules and names the rule that rejected it, and `rejection_missing` gains the matrix case ("no row of `<matrix>` rejected the example"); `unchecked` gains a matrix case that names the matrix and its area, and its import-condition branch names the area of a rule with `within`; `crates/archangel_engine/src/test_command.rs` prints the row |
| Messages and pages | `crates/archangel_rules/src/messages.rs` (area in the `mayOnlyImport` and `mayOnlyBeImportedBy` messages, the `matrix-empty` message for both cases, the `every-file-has-a-row` message); `crates/archangel_rules/docs/matrix.md`, `matrix-empty.md`, and `may-only-import.md`, `may-only-be-imported-by.md` and `may-not-import.md` gain `within` |
| Explain | `crates/archangel/src/commands/` (explain) and the sentence printer in `crates/archangel_config` |
| Types | `npm/archangel/index.d.ts`: `within?: Side<E, S>` on the `mayNotImport`, `mayOnlyImport` and `mayOnlyBeImportedBy` arms of `Sentence`, `never` elsewhere; a `Matrix` type and a `matrix` builder in `RuleBuilders` with two overloads, one with `roles` (`<const R extends string, const N extends string>`, keys inferred) and one without (keys `E \| S`); `everyFileHasARow?: boolean`; `MatrixName<N>`, with no preset parameter because a matrix or row name refuses every slash, used for the matrix name and the keys |
| JSON schema | `npm/archangel/archangel.schema.json`: `within`, and the `matrix` kind with `roles` optional and `everyFileHasARow` |
| Work counts | `crates/archangel_graph/src/work.rs`: no new counter; the tests use `rule_evaluations` and `pattern_passes`; the enhance3 run compares `patternPasses`, `filesClassified` and `ruleEvaluations` from `--stats`; `Architecture::fill` in `crates/archangel_rules/src/architecture.rs` loops over every compiled sentence per new class pair and no counter sees it, so the enhance3 times are its evidence |
| Reference evaluator | `crates/archangel_rules/tests/rule_properties.rs`: the slow reference that reads each condition's definition per import (`violations_agree_with_the_reference_evaluator`), which task 1.11 extends with areas and both matrix forms |
| Precedents named above | `layer_sentences` in `crates/archangel_config/src/rules.rs`; the FSD preset's rules with several sentences in `npm/archangel/presets/fsd.js`; `unproven_rules` in `crates/archangel_rules/src/examples.rs`; the scheduled benchmark `.github/workflows/bench.yml`, which runs `scripts/bench` on the repositories in `benches/repos.toml` (vscode, excalidraw and a generated repository, not enhance3); enhance3's builder and vocabulary at `packages/architecture/archangel.mjs` and `packages/architecture/index.mjs` on its main branch (`roleRules`, `suffixRule`, `suffixScopes`, `apiLayerRules`, `CANVAS_DOCUMENT_DEPENDENCY_RULES` and the other `*_DEPENDENCY_RULES`) |
| New terms | `openspec/glossary.md`: Area, Role, Role pattern, Matrix, Matrix row, Cell; Shorthand kind and Rule builder name four kinds |
| Docs | `docs/rules.md` (conditions, options, shorthands with both matrix forms and `everyFileHasARow`, proving rules), `README.md` (the rule forms), `CHANGELOG.md` (0.2.0), version in `Cargo.toml` and `npm/archangel/package.json` |
