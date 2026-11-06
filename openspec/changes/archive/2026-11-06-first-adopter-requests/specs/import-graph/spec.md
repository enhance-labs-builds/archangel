# import-graph

## ADDED Requirements

### Requirement: A type import falls back to its `@types` package

When a type import's bare specifier names no installed package, archangel MUST resolve it to `@types/<name>`, or `@types/<scope>__<name>` for a scoped package, as TypeScript does, and the resolution MUST name the package that the import names. A runtime import MUST NOT fall back.

#### Scenario: Types-only packages

- **GIVEN** `@types/geo` and `@types/acme__maps` installed, and neither `geo` nor `@acme/maps`
- **AND** a file with `import type { Point } from 'geo'`, `import type { Map } from '@acme/maps'` and `import { Point as P } from 'geo'`
- **WHEN** archangel builds the import graph
- **THEN** the type imports resolve to the packages `geo` and `@acme/maps`, and the runtime import is unresolved
- **TEST** `crates/archangel_graph/tests/templates_and_types.rs` › "a_type_import_of_a_types_only_package_resolves_to_the_package"

### Requirement: A template import is one import of each file it can load

An `import()` of a template literal whose static text starts with `./` or `../`, ends in a written extension and, in the importing file's own folder, writes part of the file name, MUST be one import of each walked file that its pattern matches, with each `${…}` one file or folder name. A template that matches no file MUST be `import-unresolved`. Every other computed specifier MUST stay unfollowed, and the summary MUST list each by file and line.

#### Scenario: Locales and pages

- **GIVEN** `import(\`../locales/${name}.json\`)` with `en.json`, `fr.json` and `nested/de.json` below `locales`, `import(\`./pages/${p}/index.ts\`)`, `import(\`./missing/${x}.ts\`)` and `import(url)`
- **WHEN** archangel builds the import graph
- **THEN** the first is an import of `en.json` and `fr.json`, the second of each page, the third is unresolved, and only `import(url)` is unfollowed
- **TEST** `crates/archangel_graph/tests/templates_and_types.rs` › "a_template_import_resolves_to_each_file_that_its_static_parts_match"
- **TEST** `crates/archangel_graph/tests/templates_and_types.rs` › "a_template_that_matches_no_file_is_unresolved_and_a_computed_specifier_unfollowed"

#### Scenario: A template that reaches past an entry

- **GIVEN** an entry rule on features, and `import(\`../features/billing/internal/${name}.ts\`)` with two files there
- **WHEN** archangel makes a check
- **THEN** archangel gives one entry violation for each file, each naming the file that the template can load
- **TEST** `crates/archangel_rules/tests/subpaths_and_templates.rs` › "a_template_import_is_judged_against_each_file_it_reaches"

### Requirement: An empty set names the ignore line that emptied it

When a group or a part matches no file, and the config's `ignore` list leaves out files that it would match, its diagnostic MUST name the first such line, how many files it leaves out, and one of them. A line with no `/` except at its end MUST also be said to match at any depth.

#### Scenario: An unanchored ignore line

- **GIVEN** `ignore: ['public/']`, a group `src/**/public/**`, and `src/feature/public/index.ts`
- **WHEN** archangel makes a check
- **THEN** `selector-empty` names the line `public/`, one file, `src/feature/public/index.ts`, and the fix `/public/`
- **TEST** `crates/archangel_rules/tests/selectors.rs` › "an_empty_group_names_the_ignore_line_that_left_its_files_out"
