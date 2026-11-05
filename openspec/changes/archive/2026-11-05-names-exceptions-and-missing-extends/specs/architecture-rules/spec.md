# architecture-rules

## MODIFIED Requirements

### Requirement: An import rule can be limited to the names an import brings in

An import rule with `names` MUST apply only to imports that bring in at least one of those names. A default import MUST bring in the name `default`. A namespace import, an `export *`, a dynamic import and a `require` MUST count as bringing in every name of the module, because archangel cannot see which they use. When such an import resolves to a file of a package whose exports archangel can see, every name of the module MUST be the names that file exports, with the names of the files that its relative `export * from` statements reach. When archangel cannot see them all (a CommonJS module, an `export =`, an `export *` from a bare specifier, a Node built-in), the import MUST count as bringing in every name.

#### Scenario: Only one name of a module is forbidden

- **GIVEN** a rule `{ files: 'src/**', mayNotImport: { packages: ['node:process'] }, names: ['env'] }`
- **AND** a file with `import { env } from 'node:process'` and a file with `import { argv } from 'node:process'`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for the import of `env`
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "names_limits_a_rule_to_the_names_imported"

#### Scenario: Imports whose names archangel cannot see

- **GIVEN** the same rule, and four files with `import * as p from 'node:process'`, `export * from 'node:process'`, `await import('node:process')` and `require('node:process')`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation for each of the four imports
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "an_import_with_unseen_names_brings_in_every_name"

#### Scenario: A namespace or dynamic import of a subpath that does not export the name

- **GIVEN** a rule `{ files: 'src', mayNotImport: { packages: ['dates'] }, names: ['format'] }`, a package `dates` whose root exports `format` and whose `locale/fr.js` exports only `fr`
- **AND** files with `import * as loc from 'dates/locale/fr'`, `import('dates/locale/fr')` and `import * as d from 'dates'`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for the namespace import of `dates`
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "an_import_of_every_name_of_a_package_module_brings_in_its_exports"

#### Scenario: A subpath that re-exports the name

- **GIVEN** the same rule, and `dates/fp.js` with `export * from './format.js'`, whose file exports `format`
- **AND** a file with `import * as fp from 'dates/fp'`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for that import
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "a_module_that_re_exports_a_name_brings_it_in"

#### Scenario: A module whose exports archangel cannot see

- **GIVEN** the same rule, and `dates/cjs.js` with `exports.fr = {}`
- **AND** a file with `import * as c from 'dates/cjs'`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for that import
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "a_module_whose_exports_archangel_cannot_see_brings_in_every_name"

#### Scenario: A default import

- **GIVEN** a rule `{ files: 'src/**', mayNotImport: { packages: ['lodash'] }, names: ['default'] }`, a file with `import _ from 'lodash'` and a file with `import { map } from 'lodash'`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for the default import
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "a_default_import_brings_in_default"

## ADDED Requirements

### Requirement: Archangel parses each package module once

In one resolution of the repository, archangel MUST read and parse each package file whose names it needs one time, however many imports bring it in.

#### Scenario: Two files import one subpath

- **GIVEN** two source files with `import * as loc from 'dates/locale/fr'`
- **WHEN** archangel builds the import graph
- **THEN** the work counts one package module parsed
- **TEST** `crates/archangel_graph/tests/resolve.rs` › "a_package_module_is_parsed_once"
