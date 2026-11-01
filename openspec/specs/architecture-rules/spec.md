# architecture-rules Specification

## Purpose
The rules are the architecture. This capability states what each import condition, the no-cycles
condition and the three shorthand kinds permit and forbid, so that a team can predict every violation
from the config alone.

## Requirements

### Requirement: A rule applies to the import kinds that it names

Each import rule MUST have the option `imports`, with the value `all`, `runtime` or `types`, and MUST check only the imports of that kind. The default MUST be `all`, except for a `noCycles` rule and a transitive rule, whose default MUST be `runtime`.

#### Scenario: A rule that checks runtime imports only

- **GIVEN** a rule `{ files: 'web', mayNotImport: 'api', imports: 'runtime' }`
- **AND** a file in "web" with `import type { Router } from` a file in "api"
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation for that import
- **TEST** `crates/archangel_rules/tests/rules.rs` › "runtime_rule_ignores_type_imports"

#### Scenario: The default checks type imports too

- **GIVEN** a rule `{ files: 'web', mayNotImport: 'api' }` with no `imports` option
- **AND** a file in "web" with `import type { Router } from` a file in "api"
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of the rule for that import
- **TEST** `crates/archangel_rules/tests/rules.rs` › "rule_without_imports_option_checks_type_imports"

### Requirement: A layers rule permits imports only toward later elements in its list

A layers rule MUST have an ordered list of elements. An import from a file in a listed element into a file or package in an earlier listed element MUST be a violation. The rule MUST NOT check an import when either end is in no listed element. Imports within one element and toward later elements MUST be permitted.

#### Scenario: An import toward an earlier element in the list

- **GIVEN** a layers rule with the list "app", "feature", "shared"
- **AND** a file in "shared" that imports a file in "feature"
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of the layers rule for that import
- **TEST** `crates/archangel_rules/tests/rules.rs` › "layers_rejects_an_import_toward_an_earlier_layer"

#### Scenario: An import toward a later element and within an element

- **GIVEN** the same layers rule
- **AND** a file in "app" that imports a file in "shared", and a file in "feature" that imports another file in "feature"
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the layers rule
- **TEST** `crates/archangel_rules/tests/rules.rs` › "layers_permits_imports_toward_later_layers_and_within_a_layer"

#### Scenario: An element that is not in the list

- **GIVEN** the same layers rule and an element "scripts" that is not in its list
- **AND** a file in "scripts" that imports a file in "app", and a file in "shared" that imports a file in "scripts"
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the layers rule
- **TEST** `crates/archangel_rules/tests/rules.rs` › "layers_does_not_apply_to_elements_outside_its_list"

### Requirement: An isolate rule keeps instances of one element apart

An isolate rule MUST name one element that has a capture. A file in one instance of the element MUST NOT import a file in a different instance of the same element. Imports within one instance MUST be permitted.

#### Scenario: One feature imports another

- **GIVEN** an isolate rule on "feature", with the pattern `src/features/{feature}/**`
- **AND** the file `src/features/login/a.ts` that imports `src/features/billing/b.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of the isolate rule that names the instances "login" and "billing"
- **TEST** `crates/archangel_rules/tests/rules.rs` › "isolate_rejects_an_import_between_two_instances"

#### Scenario: A feature imports its own file

- **GIVEN** the same isolate rule
- **AND** the file `src/features/login/a.ts` that imports `src/features/login/model/b.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the isolate rule
- **TEST** `crates/archangel_rules/tests/rules.rs` › "isolate_permits_imports_within_one_instance"

### Requirement: An entry rule permits imports into an instance only through its entry files

An entry rule MUST name one element and a list of entry file patterns relative to the instance root. An import from outside an instance into a file of that instance MUST resolve to an entry file. Imports within one instance MUST be permitted.

#### Scenario: An import past the entry file

- **GIVEN** an entry rule on "entity" with the entry file `index.ts`, and the pattern `src/entities/{entity}/**`
- **AND** a file outside "entity" that imports `src/entities/canvas/model/store.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of the entry rule that names `index.ts` as the entry file
- **TEST** `crates/archangel_rules/tests/rules.rs` › "entry_rejects_an_import_past_the_entry_file"

#### Scenario: An import through the entry file

- **GIVEN** the same entry rule
- **AND** a file outside "entity" that imports `src/entities/canvas/index.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the entry rule
- **TEST** `crates/archangel_rules/tests/rules.rs` › "entry_permits_an_import_through_the_entry_file"

#### Scenario: One instance imports another past its entry file

- **GIVEN** the same entry rule
- **AND** the file `src/entities/user/a.ts` that imports `src/entities/canvas/model/store.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of the entry rule, because the two files are in different instances
- **TEST** `crates/archangel_rules/tests/rules.rs` › "entry_rejects_a_sibling_instance_past_the_entry_file"

### Requirement: A rule is one selector and one condition

A rule written with `rule()` MUST have `files`, or `folders` for a folder rule, and exactly one condition field. A rule with no condition, with two, or with neither `files` nor `folders` MUST be refused with a `config-invalid` diagnostic that names the fields it found.

#### Scenario: A rule with two conditions

- **GIVEN** a rule `{ files: 'view', mayOnlyImport: ['shared'], mustExport: '{FileName}' }`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at the rule, naming `mayOnlyImport` and `mustExport` and saying to write two rules
- **TEST** `crates/archangel_config/tests/validate.rs` › "a_rule_with_two_conditions_is_refused"

#### Scenario: A rule with no condition

- **GIVEN** a rule `{ name: 'views', files: 'view' }`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at the rule, saying it has no condition and listing the conditions
- **TEST** `crates/archangel_config/tests/validate.rs` › "a_rule_with_no_condition_is_refused"

#### Scenario: A rule with no files

- **GIVEN** a rule `{ name: 'views', mayNotImport: 'test' }`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at the rule, saying it needs `files` or `folders`
- **TEST** `crates/archangel_config/tests/validate.rs` › "a_rule_with_no_files_is_refused"

#### Scenario: A config that uses a removed kind

- **GIVEN** a JSON config with a rule `{ "kind": "only", "element": "db", "importers": ["api"] }`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at the rule, saying to write `{ "files": "db", "mayOnlyBeImportedBy": ["api"] }`
- **TEST** `crates/archangel_config/tests/validate.rs` › "a_removed_kind_is_refused_with_its_sentence"

### Requirement: A rule can join several sentences under one name

A rule MAY list `sentences`, each a sentence or a shorthand, and is then broken when any of them is. Such a rule MUST have a `name`, and its sentences MUST NOT have a name, reason, link, imports or examples of their own, which belong to the rule.

#### Scenario: A rule of several sentences without a name

- **GIVEN** a rule `{ sentences: [{ files: 'entity', mayNotImport: 'shared' }, { folders: 'src/*', mustBeNamed: 'kebab-case' }] }`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at the rule's `name`, because exceptions and the baseline refer to a rule by name
- **TEST** `crates/archangel_config/tests/sentences.rs` › "a_rule_of_several_sentences_needs_a_name"

#### Scenario: A sentence of a rule with a reason of its own

- **GIVEN** a rule named `two` whose one sentence has a `reason`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at that sentence's `reason`
- **TEST** `crates/archangel_config/tests/sentences.rs` › "a_sentence_inside_a_rule_takes_no_name_or_examples"

#### Scenario: Each sentence of one rule reports under the rule's name

- **GIVEN** a rule `no-ui-in-app` with the sentences `{ folders: 'src/app/ui', mustNotExist: true }` and `{ files: 'src/app/ui.*', mustNotExist: true }`, and a repository with `src/app/ui/root.ts` and `src/app/ui.tsx`
- **WHEN** archangel makes a check
- **THEN** archangel gives two violations of `no-ui-in-app`, one for the folder `src/app/ui` and one for the file `src/app/ui.tsx`
- **TEST** `crates/archangel_rules/tests/structure.rs` › "a_rule_of_several_sentences_is_broken_by_each_of_them"

### Requirement: A mayNotImport rule refuses imports from its files into a side

A `mayNotImport` rule MUST give a violation for each import from a file in its selector into a file or package on the other side, and MUST permit every other import.

#### Scenario: Production code imports a test helper

- **GIVEN** a rule `{ files: { not: 'test' }, mayNotImport: 'test' }`, and the file `src/a.ts` that imports `src/helpers.test.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of that rule for that import
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_not_import_rejects_an_import_into_its_side"

#### Scenario: An entity imports React

- **GIVEN** a rule `{ files: 'entity', mayNotImport: { packages: ['react'] } }`
- **AND** a file in "entity" that imports `react`, and a file in "feature" that imports `react`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for the import in "entity"
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_not_import_rejects_a_package_and_permits_other_importers"

### Requirement: A mayOnlyImport rule refuses imports from its files into anything else

A `mayOnlyImport` rule MUST give a violation for each import from a file in its selector into a file or package that is neither on the other side nor its own files in the same instance. Without an area, the rule MUST judge every import, outside packages included. When the selector is a part with a capture, its own files MUST be the files of the same instance; otherwise they MUST be all the files of the selector. An import that does not resolve MUST NOT be a violation of the rule.

#### Scenario: A view imports a store

- **GIVEN** a rule `{ files: 'view', mayOnlyImport: ['shared', { packages: ['react'] }] }`, and a view file that imports a file in "model"
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation that names "shared" and the package `react` as what the file may import
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_import_rejects_an_import_outside_its_list"

#### Scenario: A view imports another view and React

- **GIVEN** the same rule, and a view file that imports another view file and `react`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of that rule
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_import_permits_its_list_and_its_own_files"

#### Scenario: A view imports a stylesheet

- **GIVEN** the same rule, and a view file that imports `./button.css`, which exists
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation for the import of `button.css`, because the stylesheet is not in the rule's list
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_import_counts_imports_of_non_source_files"

#### Scenario: A view imports a file that does not exist

- **GIVEN** the same rule, and a view file that imports `./missing`
- **WHEN** archangel makes a check
- **THEN** archangel gives an `import-unresolved` diagnostic for that import and no violation of the rule
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "an_unresolved_import_is_not_a_condition_violation"

#### Scenario: Without an area, an outside package is judged

- **GIVEN** a rule `{ files: 'view', mayOnlyImport: ['shared'] }` with no area, and a view file that imports `zod`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation for the import of `zod`
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_import_without_an_area_judges_outside_packages"

#### Scenario: One feature imports another

- **GIVEN** the part `feature` as `src/features/{feature}/**`, and a rule `{ files: 'feature', mayOnlyImport: ['shared'] }`
- **AND** the file `src/features/cart/a.ts`, which imports `src/features/cart/b.ts` and `src/features/login/c.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for the import of `src/features/login/c.ts`
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_import_permits_its_own_files_in_the_same_instance"

### Requirement: A mayOnlyBeImportedBy rule permits imports of its files from a side

A `mayOnlyBeImportedBy` rule MUST give a violation for each import into a file in its selector from a file that is neither on the other side nor in its own selector. An import from a file in no element MUST be a violation.

#### Scenario: An import from an element that is not in the list

- **GIVEN** a rule `{ files: 'db', mayOnlyBeImportedBy: 'api' }`
- **AND** a file in "web" that imports a file in "db"
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of the rule that names "api" as the permitted importer
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_be_imported_by_rejects_an_importer_not_in_its_list"

#### Scenario: An import from a file in no element

- **GIVEN** the same rule
- **AND** the file `scripts/seed.ts`, in no element, that imports a file in "db"
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of the rule
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_be_imported_by_rejects_an_importer_in_no_element"

#### Scenario: The files import each other

- **GIVEN** the same rule
- **AND** a file in "db" that imports another file in "db"
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the rule
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_be_imported_by_permits_its_own_files"

### Requirement: An import rule can be limited to the names an import brings in

An import rule with `names` MUST apply only to imports that bring in at least one of those names. A default import MUST bring in the name `default`. A namespace import, an `export *`, a dynamic import and a `require` MUST count as bringing in every name, because archangel cannot see which they use.

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

#### Scenario: A default import

- **GIVEN** a rule `{ files: 'src/**', mayNotImport: { packages: ['lodash'] }, names: ['default'] }`, a file with `import _ from 'lodash'` and a file with `import { map } from 'lodash'`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for the default import
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "a_default_import_brings_in_default"

### Requirement: An import rule can be limited to how the specifier is written

An import rule with `specifier` MUST apply only to imports whose specifier is of that form: `relative` (starting with `./` or `../`), `parent` (starting with `../`), `alias` (resolved through a tsconfig path alias or a package `imports` field) or `package`.

#### Scenario: A relative import between slices

- **GIVEN** a rule `{ files: 'slice', mayNotImport: 'slice', instance: 'different', specifier: 'relative' }`
- **AND** a file in the slice "login" that imports a file in the slice "billing" with `../billing/index.ts`, and another that imports it with `@/features/billing`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for the relative import
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "specifier_limits_a_rule_to_one_form_of_specifier"

#### Scenario: The four forms

- **GIVEN** four rules, each `mayNotImport` into `src/**` with one `specifier` form, and a file that imports one file each as `./a`, `../b`, `@/c` (an alias) and `#d` (a package `imports` entry)
- **WHEN** archangel makes a check
- **THEN** the `relative` rule reports `./a` and `../b`, the `parent` rule `../b`, the `alias` rule `@/c` and `#d`, and the `package` rule nothing
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "specifier_forms_are_told_apart"

### Requirement: An import rule can compare the instances at both ends

An import rule with `instance: 'same'` MUST apply only to imports whose ends are in one instance of the element that `files` names; with `instance: 'different'`, only to imports between two instances of it. A rule with `instance` whose `files` is not an element with a capture MUST be refused with `rule-needs-capture`.

#### Scenario: Isolate written as a sentence

- **GIVEN** a rule `{ files: 'feature', mayNotImport: 'feature', instance: 'different' }`, an isolate rule on "feature" in a second config, and a repository where `src/features/login/a.ts` imports `src/features/billing/b.ts` and `src/features/login/c.ts`
- **WHEN** archangel checks the repository with each config
- **THEN** each check gives exactly one violation, for the import of `src/features/billing/b.ts`
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "isolate_and_its_sentence_give_the_same_violations"

#### Scenario: Within one instance only

- **GIVEN** a rule `{ files: 'feature', mayNotImport: '**/internal/**', instance: 'same' }`, and the same repository with `src/features/login/a.ts` that imports `src/features/login/internal/x.ts`, and `src/features/billing/b.ts` that imports it too
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for the import from `src/features/login/a.ts`
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "instance_same_limits_a_rule_to_one_instance"

#### Scenario: instance on a selector without a capture

- **GIVEN** a rule `{ files: 'src/**', mayNotImport: 'src/**', instance: 'different' }`
- **WHEN** archangel loads the config
- **THEN** archangel gives `rule-needs-capture` at the rule
- **TEST** `crates/archangel_config/tests/validate.rs` › "instance_without_a_captured_element_is_rule_needs_capture"

### Requirement: An import rule can be limited to how the package is declared

An import rule with `dependency: 'dev'` MUST apply only to imports of packages that the nearest package manifest above the importing file lists only as development dependencies; with `dependency: 'undeclared'`, only to packages that manifest does not list. An importing file with no manifest above it MUST treat every package as undeclared.

#### Scenario: A development dependency in production code

- **GIVEN** a manifest that lists `vitest` under `devDependencies` and `zod` under both `dependencies` and `devDependencies`, and a rule `{ files: 'src/**', mayNotImport: { packages: ['*'] }, dependency: 'dev' }`
- **AND** a file that imports `vitest` and `zod`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for `vitest`
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "dependency_dev_selects_packages_listed_only_for_development"

#### Scenario: A package that the nearest manifest does not list

- **GIVEN** a root manifest that lists `lodash`, the manifest `packages/a/package.json` that does not, and a rule `{ files: 'packages/**', mayNotImport: { packages: ['*'] }, dependency: 'undeclared' }`
- **AND** the file `packages/a/src/x.ts` that imports `lodash`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation that names `packages/a/package.json`
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "dependency_undeclared_uses_the_nearest_manifest"

#### Scenario: A manifest change in the editor

- **GIVEN** that rule and that violation in a session
- **WHEN** the developer adds `lodash` to `packages/a/package.json`
- **THEN** the violation is gone, the imports of files under `packages/a` are checked again, and no other file's imports are
- **TEST** `crates/archangel_engine/tests/work.rs` › "a_manifest_change_checks_only_its_packages_imports"

### Requirement: A transitive rule refuses a chain of imports of any length

A `mayNotImport` rule with `transitively: true` MUST give one violation for each file in its selector that reaches the other side through imports of the kinds it checks, with the shortest chain, the first in path order among chains of one length. A chain MUST stop at packages and at files in `stopAt`. `imports: 'types'` MUST be refused.

#### Scenario: A client component reaches server code through a helper

- **GIVEN** a rule `{ files: { directive: 'use client' }, mayNotImport: { packages: ['server-only'] }, transitively: true }`
- **AND** the client file `app/a.tsx` that imports `lib/b.ts`, which imports `server-only`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation for `app/a.tsx` that shows the chain `app/a.tsx`, `lib/b.ts`, `server-only`
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "a_transitive_rule_shows_the_shortest_chain"

#### Scenario: Two chains of the same length

- **GIVEN** the same rule, and `app/a.tsx` that imports `lib/c.ts` and `lib/b.ts`, both of which import `server-only`
- **WHEN** archangel makes a check
- **THEN** the violation shows the chain through `lib/b.ts`
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "a_tie_between_chains_goes_to_path_order"

#### Scenario: A chain through a type import

- **GIVEN** the same rule, and `app/a.tsx` that has `import type { T } from './lib/b'`, where `lib/b.ts` imports `server-only`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation, because the type import disappears when TypeScript compiles
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "a_transitive_rule_follows_runtime_imports_by_default"

#### Scenario: Reachability agrees with a reference search

- **GIVEN** random import graphs and random transitive rules
- **WHEN** archangel evaluates the rules
- **THEN** the files it reports are the files that a slow reference search reports, with the same chains
- **TEST** `crates/archangel_rules/tests/rule_properties.rs` › "transitive_violations_agree_with_the_reference_search"

#### Scenario: The walk visits each file once per rule

- **GIVEN** a graph of 10,000 files and one transitive rule whose selector covers every file
- **WHEN** archangel makes a check
- **THEN** the count of files visited by the transitive walk is at most 10,000
- **TEST** `crates/archangel_rules/tests/work.rs` › "a_transitive_walk_visits_each_file_once"

#### Scenario: The fault control for the transitive walk

- **GIVEN** a build of archangel with a fault that starts a new walk from each file in the selector
- **WHEN** the transitive walk work test runs against that build
- **THEN** the test fails on the count of files visited
- **TEST** `crates/archangel_rules/tests/work.rs` › "transitive_walk_count_rejects_a_walk_per_file"

#### Scenario: A chain that ends at a server action

- **GIVEN** the same rule with `stopAt: { directive: 'use server' }`, and `app/a.tsx` that imports `app/actions.ts`, which starts with `'use server';` and imports `server-only`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "a_chain_stops_at_files_in_stop_at"

#### Scenario: A transitive rule on type imports

- **GIVEN** a rule `{ files: 'src/**', mayNotImport: 'db', transitively: true, imports: 'types' }`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at the rule, saying a chain of type imports never runs
- **TEST** `crates/archangel_config/tests/validate.rs` › "a_transitive_rule_on_type_imports_is_refused"

### Requirement: A noCycles rule reports each group of files that import each other

A rule with `noCycles: true` MUST give one violation for each group of files in which every file can reach every other file through imports of the kind that the rule checks. The violation MUST list every file in the group, and MUST show the shortest cycle through the group's first file in path order, with the import on each step. Between cycles of the same length, the cycle whose next file comes first in path order MUST win.

#### Scenario: Two files that import each other

- **GIVEN** a `noCycles` rule on `src/**`, and the files `src/a.ts` and `src/b.ts` that import each other at runtime
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation that shows the cycle `src/a.ts`, `src/b.ts`, `src/a.ts`
- **TEST** `crates/archangel_rules/tests/rules.rs` › "no_cycles_reports_two_files_that_import_each_other"

#### Scenario: A cycle through type imports only

- **GIVEN** a `noCycles` rule with no `imports` option
- **AND** the files `src/a.ts` and `src/b.ts` that import each other with type imports
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of the `noCycles` rule
- **TEST** `crates/archangel_rules/tests/rules.rs` › "no_cycles_ignores_type_import_cycles_by_default"

#### Scenario: A cycle through type imports with all imports

- **GIVEN** a `noCycles` rule with `imports: 'all'`, and the same two files
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation of the `noCycles` rule
- **TEST** `crates/archangel_rules/tests/rules.rs` › "no_cycles_with_all_imports_reports_type_import_cycles"

#### Scenario: Many cycles in one group

- **GIVEN** a `noCycles` rule on `src/**`, and five files in which each file imports every other file
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation for the group of five files
- **AND** the violation lists all five files
- **AND** the violation shows the cycle `src/a.ts`, `src/b.ts`, `src/a.ts`
- **TEST** `crates/archangel_rules/tests/rules.rs` › "no_cycles_gives_one_violation_per_group"

#### Scenario: A very long chain

- **GIVEN** a chain of 200,000 files in which each file imports the next one, and the last imports the first
- **WHEN** archangel makes a check on a thread with a stack of 2 MB
- **THEN** archangel gives one violation for the group, and does not stop with a stack overflow
- **TEST** `crates/archangel_rules/tests/rules.rs` › "no_cycles_handles_a_chain_of_200000_files"

#### Scenario: Cycle groups agree with a reference algorithm

- **GIVEN** random import graphs
- **WHEN** archangel finds the cycle groups in each graph
- **THEN** the groups are the same as the groups that a slow reference algorithm finds
- **TEST** `crates/archangel_rules/tests/graph_properties.rs` › "cycle_groups_agree_with_the_reference_algorithm"

### Requirement: One import can break several rules

Archangel MUST check each import against every rule. When an import breaks more than one rule, archangel MUST give one violation for each rule that it breaks, and the summary MUST count each one. A table row is a rule in this count. An exception MUST permit only the violation of the rule that it names. The baseline MUST record one entry for each rule. The order of the rules in the config MUST NOT change the result.

#### Scenario: An import that breaks two rules

- **GIVEN** a layers rule and an entry rule, and one import that breaks both
- **WHEN** archangel makes a check
- **THEN** archangel gives two violations for that import, one for each rule
- **TEST** `crates/archangel_rules/tests/rules.rs` › "one_import_can_break_two_rules"

#### Scenario: An import that breaks a table row and a rule

- **GIVEN** the enhance3-slice config with one more rule, `rule('domain-not-to-application', { files: 'domain', mayNotImport: 'application' })`
- **AND** a domain entity that imports a use case
- **WHEN** archangel makes a check
- **THEN** archangel gives two violations for that import, one of `domain.entity` and one of `domain-not-to-application`, and the summary counts two problems
- **TEST** `crates/archangel_rules/tests/rules.rs` › "a_table_row_and_a_rule_each_count_for_one_import"

#### Scenario: An exception for one of two rules

- **GIVEN** the same config and import, under the comment `// archangel-allow domain.entity: the move to a port is in progress`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, of `domain-not-to-application`, and counts one exception for `domain.entity`
- **TEST** `crates/archangel_rules/tests/rules.rs` › "an_exception_permits_only_the_row_that_it_names"

#### Scenario: A baseline for one import that breaks two rules

- **GIVEN** the same config and import, with no exception
- **WHEN** the developer runs `archangel baseline add`
- **THEN** the baseline has one entry for `domain.entity` and one for `domain-not-to-application`, each with the count 1
- **TEST** `crates/archangel/tests/baseline_command.rs` › "a_baseline_records_one_entry_for_each_rule_that_an_import_breaks"

#### Scenario: A table that does not name a package

- **GIVEN** the enhance3-slice config, whose table `domain` names no externals set, and a domain entity that imports `pg`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, of `core-not-to-driver`, and none of the table
- **TEST** `crates/archangel_rules/tests/rules.rs` › "a_table_that_names_no_externals_set_leaves_a_package_to_other_rules"

#### Scenario: The order of the rules

- **GIVEN** a config and the same config with its rules in the opposite order
- **WHEN** archangel checks one repository with each config
- **THEN** the two checks give the same diagnostics
- **TEST** `crates/archangel_rules/tests/rules.rs` › "rule_order_does_not_change_the_result"

#### Scenario: Rules agree with a reference evaluator

- **GIVEN** random configs and random import graphs
- **WHEN** archangel evaluates the rules
- **THEN** the violations are the same as the violations that a slow reference evaluator gives
- **TEST** `crates/archangel_rules/tests/rule_properties.rs` › "violations_agree_with_the_reference_evaluator"

### Requirement: The cost of an import does not grow with the number of rules

Archangel MUST evaluate each import with work that does not depend on the number of rules that cannot apply to its two elements.

#### Scenario: Five rules and five hundred rules

- **GIVEN** one import graph, and two configs: one with 5 rules, and one with the same 5 rules and 495 `mayNotImport` rules between other elements
- **WHEN** archangel makes a check with each config
- **THEN** the two checks have the same count of rule evaluations
- **TEST** `crates/archangel_rules/tests/work.rs` › "rule_evaluations_do_not_grow_with_rules_that_cannot_apply"

#### Scenario: The fault control for rule evaluations

- **GIVEN** a build of archangel with a fault that tries every rule for every import
- **WHEN** the rule evaluation work test runs against that build
- **THEN** the test fails on the count of rule evaluations
- **TEST** `crates/archangel_rules/tests/work.rs` › "rule_evaluation_count_rejects_a_search_of_every_rule"

### Requirement: Each shorthand kind means the sentences that it stands for

Each of the four shorthand kinds, layers, isolate, entry and mayImport, MUST give exactly the violations of the sentences it stands for.

#### Scenario: Shorthands agree with their sentences

- **GIVEN** random configs written with shorthand kinds, tables included, and the same configs written as sentences
- **WHEN** archangel checks random repositories with both
- **THEN** each pair of checks gives the same violations
- **TEST** `crates/archangel_rules/tests/rule_properties.rs` › "shorthands_agree_with_their_sentences"

### Requirement: The documentation shows each shorthand as its sentences

The documentation page of each shorthand kind, and `archangel docs` for it, MUST show the sentences it stands for.

#### Scenario: The isolate page

- **GIVEN** the documentation page for isolate
- **WHEN** the test reads it
- **THEN** it shows `rule({ files: 'feature', mayNotImport: 'feature', instance: 'different' })`
- **TEST** `crates/archangel_rules/tests/diagnostics.rs` › "each_shorthand_page_shows_its_sentences"

#### Scenario: The mayImport page

- **GIVEN** the documentation page for mayImport
- **WHEN** the test reads it
- **THEN** it shows a row as one `mayOnlyImport` sentence with its cells and its own name in the same instance
- **AND** it shows a row that mayNotImportItself lists as that sentence and one `mayNotImport` sentence of its own name
- **TEST** `crates/archangel_rules/tests/diagnostics.rs` › "each_shorthand_page_shows_its_sentences"

### Requirement: A mayOnlyImport rule with an area judges only imports into that area

A `mayOnlyImport` rule MAY have an area, written with `within` and with the values of a rule side. The rule MUST then give a violation only for an import whose other end is in the area, and neither on the other side nor among its own files. The rule MUST NOT judge an import whose other end is outside the area.

#### Scenario: An entity imports a use case

- **GIVEN** a rule `{ files: 'entity', mayOnlyImport: ['value-object'], within: 'apps/api/src/**' }`
- **AND** a file in "entity" that imports `apps/api/src/canvas/application/save.use-case.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation that names "value-object" as what the file may import and `apps/api/src/**` as the area
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_import_within_rejects_an_import_into_the_area_outside_its_list"

#### Scenario: An entity imports a package and a file of another app

- **GIVEN** the same rule
- **AND** a file in "entity" that imports `zod` and `apps/web/src/shared/lib/format.ts`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of that rule
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_import_within_does_not_judge_an_import_outside_the_area"

#### Scenario: An area that includes packages

- **GIVEN** a rule `{ files: 'entity', mayOnlyImport: [{ packages: ['zod'] }], within: ['apps/api/src/**', { packages: ['*'] }] }`
- **AND** a file in "entity" that imports `zod` and `lodash`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for the import of `lodash`
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_import_within_judges_the_packages_in_its_area"

#### Scenario: The files import each other

- **GIVEN** the same rule as in the first scenario
- **AND** a file in "entity" that imports another file in "entity"
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation of that rule
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_import_within_permits_its_own_files"

#### Scenario: An area with an import limit

- **GIVEN** a rule `{ files: 'entity', mayOnlyImport: ['value-object'], within: 'apps/api/src/**', specifier: 'parent' }`
- **AND** a file in "entity" that imports a use case through `../application/save.use-case`, and another through the alias `@api/application/load.use-case`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for the import that starts with `../`
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_import_within_combines_with_the_import_limits"

#### Scenario: An import that does not resolve

- **GIVEN** the rule `{ files: 'entity', mayOnlyImport: ['value-object'], within: 'apps/api/src/**' }`
- **AND** a file in "entity" that imports `./missing`
- **WHEN** archangel makes a check
- **THEN** archangel gives one `import-unresolved` diagnostic and no violation of the rule
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "an_unresolved_import_is_not_a_violation_of_a_rule_with_an_area"

#### Scenario: An area agrees with its double negative

- **GIVEN** random configs with `mayOnlyImport` rules that have an area and whose files have no capture, and the same configs with each rule written as `{ files, mayNotImport: { all: [area, { not: [side, files] }] } }`; the generator leaves captures out because the double negative permits every instance of `files`, where the rule permits only the same one, so the captured case is held by the scenario "One feature imports another" and by the reference evaluator
- **WHEN** archangel checks random repositories with both
- **THEN** each pair of checks gives the same violations
- **TEST** `crates/archangel_rules/tests/rule_properties.rs` › "an_area_agrees_with_its_double_negative"

### Requirement: A mayOnlyBeImportedBy rule with an area judges only importers in that area

A `mayOnlyBeImportedBy` rule MAY have an area, written with `within`. The rule MUST then give a violation only for an import from a file in the area into its files, when that file is neither on the other side nor in the rule's own selector. An importer outside the area MUST NOT be judged.

#### Scenario: A script outside the area imports the database

- **GIVEN** a rule `{ files: 'db', mayOnlyBeImportedBy: 'api', within: 'apps/**' }`
- **AND** the file `scripts/seed.ts` and a file in "web", which both import a file in "db"
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for the import in "web"
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_be_imported_by_within_does_not_judge_an_importer_outside_the_area"

### Requirement: An area is refused on mayNotImport and on a condition that checks no import

Archangel MUST refuse `within` on a rule whose condition is not `mayOnlyImport` or `mayOnlyBeImportedBy`, with `config-invalid` that names the condition. For `mayNotImport`, the message MUST give the fix `mayNotImport: { all: [side, area] }` with the rule's own side and area, because that form already means what the area would mean.

#### Scenario: An area on mayNotImport

- **GIVEN** a rule `{ files: 'entity', mayNotImport: 'use-case', within: 'apps/api/src/**' }`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at `within`, whose message says to write `mayNotImport: { all: ['use-case', 'apps/api/src/**'] }`
- **TEST** `crates/archangel_config/tests/externals_and_areas.rs` › "within_on_may_not_import_is_refused_with_the_narrowed_side_as_the_fix"

#### Scenario: An area on a naming rule

- **GIVEN** a rule `{ files: 'entity', mustBeNamed: 'kebab-case', within: 'apps/api/src/**' }`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at `within`, saying that `mustBeNamed` checks no import
- **TEST** `crates/archangel_config/tests/externals_and_areas.rs` › "within_on_a_condition_that_checks_no_import_is_refused"

### Requirement: An empty area is refused

An area with no value, such as an empty list or an `any` with no member, MUST be refused with `config-invalid` at `within`.

#### Scenario: An empty area

- **GIVEN** a rule `{ files: 'entity', mayOnlyImport: ['value-object'], within: [] }`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at `within`, saying that an area needs at least one value
- **TEST** `crates/archangel_config/tests/externals_and_areas.rs` › "an_empty_area_is_refused"

### Requirement: An area that names an undeclared selector is refused

A name in an area that names no declared element or selector MUST give `rule-unknown-element` at `within`, with the closest declared name.

#### Scenario: A misspelt selector in an area

- **GIVEN** the declared selector `api-src` and a rule `{ files: 'entity', mayOnlyImport: ['value-object'], within: 'api-scr' }`
- **WHEN** archangel loads the config
- **THEN** archangel gives `rule-unknown-element` at `within`, and names `api-src` as the closest name
- **TEST** `crates/archangel_config/tests/externals_and_areas.rs` › "within_naming_an_undeclared_selector_is_refused_with_the_closest_name"

### Requirement: An area that matches no file is an error

An inline pattern in an area that matches no file MUST give a `selector-empty` diagnostic that names the rule and the pattern, except a pattern that the area writes only inside a `not`.

#### Scenario: A misspelt area

- **GIVEN** a rule `{ name: 'entities', files: 'entity', mayOnlyImport: ['value-object'], within: 'apps/apii/src/**' }` and no file under `apps/apii`
- **WHEN** archangel makes a check
- **THEN** archangel gives a `selector-empty` diagnostic that names the rule "entities" and the pattern `apps/apii/src/**`
- **TEST** `crates/archangel_rules/tests/selectors.rs` › "an_area_pattern_that_matches_nothing_is_an_error"

### Requirement: An import outside an area costs no rule evaluation

An import whose other end is outside the area of a rule MUST NOT count a rule evaluation for that rule.

#### Scenario: Imports outside the area

- **GIVEN** a rule `{ files: 'entity', mayOnlyImport: ['value-object'], within: 'apps/api/src/**' }`, and two repositories: one where entity files have 10 imports of packages, and one where they have 1,000
- **WHEN** archangel makes a check of each repository
- **THEN** the two checks have the same count of rule evaluations
- **TEST** `crates/archangel_rules/tests/work.rs` › "imports_outside_an_area_cost_no_rule_evaluation"

#### Scenario: The fault control for areas

- **GIVEN** a build of archangel with a fault that tests the area of each rule for each import
- **WHEN** the area work test runs against that build
- **THEN** the test fails on the count of rule evaluations
- **TEST** `crates/archangel_rules/tests/work.rs` › "rule_evaluation_count_rejects_an_area_test_per_import"

### Requirement: A config without the new forms gives the results of 0.1

A config that uses none of the new keys, forms and options MUST load, check and test with the same diagnostics, summary, JSON output and exit status as with archangel 0.1.1. Two cases are exempt, and the test MUST name each fixture that they change: a `mayOnlyImport` rule whose files are a part with a capture, and a file whose tsconfig sets `verbatimModuleSyntax`. The JSON summary of such a config MUST have no key for rows with no file.

#### Scenario: The 0.1 test configs

- **GIVEN** the fixtures of `crates/archangel/tests/output.rs` and `crates/archangel/tests/test_command.rs`, with the text and JSON output of `archangel check` and `archangel test` on each, without `--stats`, recorded from main at 0.1.1 before any other commit of this change
- **WHEN** archangel 0.2.0 runs the same commands
- **THEN** the standard output and the exit status are the same as the recordings, byte for byte, except for the fixtures that the test names with one of the two exempt cases
- **AND** the JSON summary has no `rowsWithNoFile` key
- **TEST** `crates/archangel/tests/output.rs` › "configs_without_the_new_forms_give_the_recorded_output"

### Requirement: An area can name an externals set

An area MUST accept the name of an externals set. A `mayOnlyImport` rule whose area names an externals set MUST judge the imports of the packages in that set. The list of a `mayOnlyImport` rule MAY be empty only when the rule has an area: the rule then refuses every import into the area, except of its own files. Without an area, an empty list MUST stay `config-invalid`, and its message MUST name `within`.

#### Scenario: An area of drivers

- **GIVEN** the externals set `driver` as `['pg', 'drizzle-orm']`, and a rule `rule('drivers-in-infrastructure-only', { files: 'domain', mayOnlyImport: [], within: 'driver' })`
- **AND** a file in "domain" that imports `pg`, `zod` and a file in "domain"
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for the import of `pg`
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "may_only_import_within_an_externals_set_judges_its_packages"

#### Scenario: An empty list without an area

- **GIVEN** a rule `{ files: 'entity', mayOnlyImport: [] }`
- **AND** the same rule with `within: 'src/**'`
- **WHEN** archangel loads each config
- **THEN** the first gives `config-invalid` at `mayOnlyImport`, whose message names `within`, and the second loads
- **TEST** `crates/archangel_config/tests/externals_and_areas.rs` › "an_empty_may_only_import_needs_an_area"

### Requirement: A package in an area must exist somewhere

A package that an area names MUST give the `element-empty` diagnostic that a package on a rule side gives, when no import uses it and no package manifest lists it, because a misspelt package in an area stops the rule from judging it.

#### Scenario: A misspelt package in an area

- **GIVEN** a rule with `within: [{ packages: ['zodd'] }]`, and a repository where no import uses `zodd` and no manifest lists it
- **WHEN** archangel makes a check
- **THEN** archangel gives one `element-empty` diagnostic that names the rule and `zodd`
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "an_area_package_that_no_import_uses_and_no_manifest_lists_is_an_error"
