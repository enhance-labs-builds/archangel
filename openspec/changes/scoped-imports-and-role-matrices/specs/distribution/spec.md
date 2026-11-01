# distribution

## MODIFIED Requirements

### Requirement: The npm package gives types for the config

The npm package MUST export `defineConfig` and the types of the config. With these types, the TypeScript compiler MUST report an error for a rule that names an undeclared element or selector, for an isolate rule on an element without a capture, for a sentence with more than one condition, for a rule name with a slash that does not follow the name of a preset it extends, for a matrix cell that names nothing, for a row of a matrix without a role pattern that names no declared element or selector, for a matrix or row name with a refused character, and for an area on a condition that checks no import.

#### Scenario: A rule names an undeclared element

- **GIVEN** a config that uses `defineConfig` and a layers rule on the undeclared element "reactt"
- **WHEN** the TypeScript compiler checks the config
- **THEN** the compiler reports an error on "reactt"
- **TEST** `npm/archangel/test/types.test-d.ts` › "rule naming an undeclared element is a type error"

#### Scenario: An isolate rule on an element without a capture

- **GIVEN** a config with the element "shared" as `src/shared/**` and an isolate rule on "shared"
- **WHEN** the TypeScript compiler checks the config
- **THEN** the compiler reports an error on "shared"
- **TEST** `npm/archangel/test/types.test-d.ts` › "isolate on an element without a capture is a type error"

#### Scenario: A sentence names an undeclared selector

- **GIVEN** a config that uses `defineConfig`, declares the selector "view", and has `rule({ files: 'vieww', mayNotImport: 'test' })`
- **WHEN** the TypeScript compiler checks the config
- **THEN** the compiler reports an error on "vieww"
- **TEST** `npm/archangel/test/types.test-d.ts` › "a sentence naming an undeclared selector is a type error"

#### Scenario: A sentence with two conditions

- **GIVEN** a config with `rule({ files: 'view', mayNotImport: 'test', mustExport: '{FileName}' })`
- **WHEN** the TypeScript compiler checks the config
- **THEN** the compiler reports an error on the rule
- **TEST** `npm/archangel/test/types.test-d.ts` › "a sentence with two conditions is a type error"

#### Scenario: A rule name with a slash

- **GIVEN** a config that uses `defineConfig` and names a rule `team/no-legacy`, in a rule object or through `rule`, `layers`, `isolate` or `entry`, and extends no preset named "team"
- **WHEN** the TypeScript compiler checks the config
- **THEN** the compiler reports an error on the name, which says a rule's own name has no slash except after the name of a preset the config extends
- **AND** a rule named `fsd-admin/public-api` in a config that extends `fsd({ root: 'src', name: 'fsd-admin' })` has no error, and the rules inside the preset are not checked
- **TEST** `npm/archangel/test/types.test-d.ts` › "a config's own rule name cannot contain a slash, which separates a preset's name from its rule's"

#### Scenario: A matrix cell that names nothing

- **GIVEN** a config that uses `defineConfig` and a matrix with the rows `entity` and `value-object`, whose row `entity` has the cell `value-objekt`
- **WHEN** the TypeScript compiler checks the config
- **THEN** the compiler reports an error on "value-objekt"
- **TEST** `npm/archangel/test/types.test-d.ts` › "a matrix cell that names no role and no declared name is a type error"

#### Scenario: A row that names nothing in a matrix without a role pattern

- **GIVEN** a config that uses `defineConfig`, declares the element "canvas-document", and has a matrix with no role pattern and the row `canvas-documnet`
- **WHEN** the TypeScript compiler checks the config
- **THEN** the compiler reports an error on "canvas-documnet"
- **AND** the same matrix with the row `canvas-document` has no error
- **TEST** `npm/archangel/test/types.test-d.ts` › "a row of a matrix without roles must be a declared element or selector"

#### Scenario: A matrix name with a dot

- **GIVEN** a matrix named `api.domain`, and a matrix with the row `aggregate.root`
- **WHEN** the TypeScript compiler checks the config
- **THEN** the compiler reports an error on each name, which says that the name of a matrix or a row has no dot, colon, slash or space
- **TEST** `npm/archangel/test/types.test-d.ts` › "a matrix or row name with a dot, colon, slash or space is a type error"

#### Scenario: An area on a naming sentence

- **GIVEN** a config with `rule({ files: 'entity', mustBeNamed: 'kebab-case', within: 'apps/api/src/**' })`
- **WHEN** the TypeScript compiler checks the config
- **THEN** the compiler reports an error on the rule
- **AND** the same rule with `mayNotImport`, `mayOnlyImport` or `mayOnlyBeImportedBy` in place of `mustBeNamed` has no error, and an area that names an undeclared selector gives an error on that name
- **TEST** `npm/archangel/test/types.test-d.ts` › "within is typed as a rule side and only on import conditions"

#### Scenario: A config written with the new forms

- **GIVEN** the matrix of the API domain layer with `everyFileHasARow: true`, a matrix of packages, and one rule with an area, written with `defineConfig` and literal names only
- **WHEN** the TypeScript compiler checks the config with `strict` on
- **THEN** the compiler reports no error
- **TEST** `npm/archangel/test/types.test-d.ts` › "a config with a matrix and an area type-checks with literal names"
