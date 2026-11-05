# architecture-rules

## MODIFIED Requirements

### Requirement: A rule applies to the import kinds that it names

Each import rule MUST have the option `imports`, with the value `all`, `runtime`, `static` or `types`, and MUST check only the imports of that kind. `static` MUST mean the runtime imports other than `import()`, so that a transitive rule with it stops each chain at an `import()`. The default MUST be `all`, except for a `noCycles` rule and a transitive rule, whose default MUST be `runtime`.

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

#### Scenario: A rule that checks static imports only

- **GIVEN** a rule `{ files: 'web', mayNotImport: 'api', imports: 'static' }`
- **AND** a file in "web" with `import('../api/a')`, and another with `require('../api/a')`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for the `require`
- **TEST** `crates/archangel_rules/tests/rules.rs` › "static_rule_skips_dynamic_imports"

#### Scenario: A transitive rule that stops at `import()`

- **GIVEN** a rule `{ files: 'entry', mayNotImport: { packages: ['carousel'] }, transitively: true, imports: 'static' }`
- **AND** `src/entry/main.ts` that imports `src/lib/viewer.ts`, which has `import('./gallery')`, and `src/lib/gallery.ts` that imports `carousel`
- **WHEN** archangel makes a check
- **THEN** archangel gives no violation
- **AND** when `src/entry/main.ts` also imports `src/lib/gallery.ts` statically, archangel gives one violation
- **TEST** `crates/archangel_rules/tests/conditions.rs` › "a_static_transitive_rule_stops_at_a_dynamic_import"

### Requirement: A layers rule permits imports only toward later elements in its list

A layers rule MUST have an ordered list of parts, package parts or groups. An import from a file in a listed layer into a file or package in an earlier listed layer MUST be a violation. The rule MUST NOT check an import when either end is in no listed layer. Imports within one layer and toward later layers MUST be permitted. A file in two layers MUST be judged as a file of each.

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

#### Scenario: Layers of groups

- **GIVEN** a part `feature` for `src/features/{feature}/**`, groups "pagesLayer", "featuresLayer" and "sharedLayer", and a layers rule with that list of groups
- **AND** a file in "sharedLayer" that imports a file in "pagesLayer", and a file in "pagesLayer" that imports a file in "featuresLayer"
- **WHEN** archangel makes a check
- **THEN** archangel gives one `layers` violation, for the import from "sharedLayer", whose message names "sharedLayer" and the earlier layers
- **TEST** `crates/archangel_rules/tests/rules.rs` › "layers_of_groups_reject_an_import_toward_an_earlier_group"
