# distribution Specification

## Purpose
TypeScript developers install their tools from npm and expect them to work without a compiler. This
capability covers how archangel is installed from npm and crates.io, and what the npm package gives a
config author.

## Requirements

### Requirement: The npm package installs a prebuilt binary

`npm install --save-dev @enhance-eng/archangel` MUST install a prebuilt binary for macOS on arm64 and x64, Linux on arm64 and x64 with glibc or musl, and Windows on x64. After the install, `npx archangel --version` MUST work without Rust on the machine.

#### Scenario: An install on each platform

- **GIVEN** a clean machine of each supported platform with Node and no Rust
- **WHEN** a developer installs the release candidate of the npm package and runs `npx archangel --version`
- **THEN** archangel prints the version of the package
- **TEST** `.github/workflows/release.yml` › "install-from-npm"

#### Scenario: A platform without a binary

- **GIVEN** a machine whose platform has no prebuilt binary
- **WHEN** a developer runs `npx archangel check`
- **THEN** archangel prints the platform name and says to install with `cargo install archangel`
- **TEST** `npm/archangel/test/platform.test.mjs` › "unsupported platform names itself and the cargo alternative"

### Requirement: The npm package gives types for the config

The npm package MUST export `defineConfig` and the types of the config. With these types, the TypeScript compiler MUST report an error for a rule that names an undeclared element or selector, for an isolate rule on an element without a capture, for a sentence with more than one condition, and for a rule name with a slash that does not follow the name of a preset it extends.

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

### Requirement: The types check tables

The TypeScript compiler MUST report an error for a table cell that names no row, declared name or path pattern, for a row of a table without a kind pattern that names no declared part or group, and for a table or row name with a refused character.

#### Scenario: A table cell that names nothing

- **GIVEN** a config that uses `defineConfig` and a table with the kind pattern `**/*.{kind}.ts` and the rows `entity` and `value-object`, whose row `entity` has the cell `value-objects`
- **WHEN** the TypeScript compiler checks the config
- **THEN** the compiler reports an error on "value-objects"
- **AND** the cells `value-object`, a declared part, an externals set and `apps/api/src/shared/result.ts` have no error
- **TEST** `npm/archangel/test/types.test-d.ts` › "a table cell that names no row, no declared name and no pattern is a type error"

#### Scenario: A row that names nothing in a table without a kind pattern

- **GIVEN** a config that uses `defineConfig`, declares the part "canvas-document", and has a table with no kind pattern and the row `canvas-documents`
- **WHEN** the TypeScript compiler checks the config
- **THEN** the compiler reports an error on "canvas-documents"
- **AND** the same table with the row `canvas-document` has no error
- **TEST** `npm/archangel/test/types.test-d.ts` › "a row of a table without kinds must be a declared part or group"

#### Scenario: A table name with a colon

- **GIVEN** a table named `api:domain`, and a table with the row `aggregate/root`
- **WHEN** the TypeScript compiler checks the config
- **THEN** the compiler reports an error on each name, which says that the name of a table or a row has no colon or slash
- **AND** a table named `api.domain` with the row `my views` has no error
- **TEST** `npm/archangel/test/types.test-d.ts` › "a table or row name with a colon or slash is a type error"

### Requirement: The types check the new spellings

The TypeScript compiler MUST report an error for an area on a condition that checks no import, and for a config that has both spellings of one key. The types MUST accept `parts`, `groups`, `externals`, `because`, and a rule name as the first argument of the rule builder.

#### Scenario: An area on a naming sentence

- **GIVEN** a config with `rule('names', { files: 'entity', mustBeNamed: 'kebab-case', within: 'apps/api/src/**' })`
- **WHEN** the TypeScript compiler checks the config
- **THEN** the compiler reports an error on the rule
- **AND** the same rule with `mayOnlyImport` or `mayOnlyBeImportedBy` in place of `mustBeNamed` has no error, with `mayNotImport` gives an error, and an area that names an undeclared group gives an error on that name
- **TEST** `npm/archangel/test/types.test-d.ts` › "within is typed as a rule side and only on mayOnlyImport and mayOnlyBeImportedBy"

#### Scenario: Two spellings of one key

- **GIVEN** a config with both `parts` and `elements`, and a rule with both `because` and `reason`
- **WHEN** the TypeScript compiler checks the config
- **THEN** the compiler reports an error on each second key
- **TEST** `npm/archangel/test/types.test-d.ts` › "a config with two spellings of one key is a type error"

#### Scenario: The checked fixtures

- **GIVEN** the configs of the small-app and enhance3-slice fixtures, written with `defineConfig` and literal names only
- **WHEN** the TypeScript compiler checks them with `strict` on
- **THEN** the compiler reports no error
- **TEST** `npm/archangel/test/types.test-d.ts` › "the configs of the checked fixtures type-check with literal names"

### Requirement: The crate installs from source

`cargo install archangel` MUST build and install the same command line as the npm package, with the Rust version that the repository pins.

#### Scenario: An install with Cargo

- **GIVEN** a machine with the pinned Rust version
- **WHEN** a developer runs `cargo install archangel` for the release candidate
- **THEN** `archangel --version` prints the version of the crate
- **TEST** `.github/workflows/release.yml` › "install-from-crates-io"

### Requirement: Each release binary is traceable to its source

Each release MUST publish the SHA-256 of each binary. The npm packages MUST be published with provenance, which links each package to the commit and the workflow run that built it. The release MUST publish to npm and crates.io with trusted publishing, and MUST store no registry token.

#### Scenario: A developer checks a binary

- **GIVEN** a published release
- **WHEN** a developer compares the SHA-256 of a downloaded binary with the release's list
- **THEN** the two values are the same
- **TEST** `.github/workflows/release.yml` › "checksums-match"

#### Scenario: A developer checks the provenance

- **GIVEN** an install of the published release candidate
- **WHEN** a developer runs `npm audit signatures`
- **THEN** npm reports a verified provenance for `@enhance-eng/archangel` and for its platform package
- **TEST** `.github/workflows/release.yml` › "provenance-verified"

### Requirement: A VS Code extension starts the language server

The extension in `editors/vscode` MUST start `archangel lsp` in a workspace with an archangel config, using the binary from the `archangel.path` setting, else the workspace's `node_modules/.bin`, else `PATH`. When none has it, it MUST say so, naming the three places and the install command, and start nothing.

#### Scenario: Diagnostics in the editor

- **GIVEN** a workspace with a layers violation, on the oldest and newest VS Code the extension supports
- **WHEN** VS Code opens the violating file
- **THEN** the file shows the layers diagnostic at the specifier, and Restart Server reads the setting again
- **TEST** `editors/vscode/test/integration/suite/diagnostics.test.ts` › "publishes the layers diagnostic for a file that imports an earlier layer"
- **TEST** `editors/vscode/test/integration/suite/diagnostics.test.ts` › "restarts the server with the current archangel.path on Archangel: Restart Server"

#### Scenario: Which binary

- **GIVEN** each combination of a setting, a workspace binary and a binary on `PATH`, on macOS, Linux and Windows names
- **THEN** the extension picks them in that order, and a setting that names no file is an error rather than a reason to look elsewhere
- **TEST** `editors/vscode/test/unit/binary.test.ts` › "uses the setting before the workspace and PATH"
- **TEST** `editors/vscode/test/unit/binary.test.ts` › "refuses a setting that names no file, rather than looking elsewhere"
- **TEST** `editors/vscode/test/unit/binary.test.ts` › "names the three places and how to install when nothing is found"
