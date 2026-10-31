# Spec Delta

## Purpose

The config is where a team writes down its architecture. This capability covers how archangel finds
the config, loads it, and refuses a config that would check less than it says.

## ADDED Requirements

### Requirement: Archangel finds the nearest config

Archangel MUST use the config in the current folder or in the nearest parent folder that has one. The `--config` option MUST replace this search with the given file. The config folder MUST be the folder of the config file, so archangel reads the source files below it and reports paths relative to it. When no config exists, archangel MUST stop with a `config-missing` diagnostic and exit status 2.

#### Scenario: The config is in a parent folder

- **GIVEN** a repository with `archangel.config.json` at its top folder
- **WHEN** the developer runs `archangel check` in the folder `packages/web`
- **THEN** archangel uses the config at the top folder
- **AND** archangel reports paths relative to the top folder
- **TEST** `crates/archangel/tests/config_discovery.rs` › "uses_the_config_in_the_nearest_parent_folder"

#### Scenario: The config option names a file

- **GIVEN** a repository with two configs at its top folder, `archangel.config.json` and `archangel.strict.json`
- **WHEN** the developer runs `archangel check --config archangel.strict.json`
- **THEN** archangel uses `archangel.strict.json` and ignores the other config
- **AND** archangel reports paths relative to the top folder
- **TEST** `crates/archangel/tests/config_discovery.rs` › "config_option_replaces_the_search"

#### Scenario: There is no config

- **GIVEN** a folder with no config in it or in a parent folder
- **WHEN** the developer runs `archangel check`
- **THEN** archangel prints a `config-missing` diagnostic that names the file names it looked for
- **AND** archangel exits with status 2
- **TEST** `crates/archangel/tests/config_discovery.rs` › "no_config_stops_with_config_missing"

### Requirement: A TypeScript or JavaScript config is evaluated once with Node

Archangel MUST evaluate a TypeScript or JavaScript config with Node 22.18 or later, once for each load, and MUST use its default export. When the `rules` field is a function, archangel MUST call it with the rule builders and use the list that it returns. After the load, archangel MUST NOT run config code again.

#### Scenario: A TypeScript config with a rules function

- **GIVEN** an `archangel.config.ts` whose `rules` field is a function that returns a layers rule
- **WHEN** archangel loads the config
- **THEN** archangel has the elements of the config and one layers rule
- **TEST** `crates/archangel_engine/tests/load.rs` › "typescript_config_with_a_rules_function_loads"

#### Scenario: Config code runs once

- **GIVEN** an `archangel.config.ts` that adds one line to a counter file each time Node evaluates it
- **AND** a repository with 20 source files
- **WHEN** the developer runs `archangel check`
- **THEN** the counter file has one line
- **TEST** `crates/archangel/tests/config_discovery.rs` › "typescript_config_is_evaluated_once_per_check"

#### Scenario: Node is not available

- **GIVEN** an `archangel.config.ts` and no `node` program on the search path
- **WHEN** the developer runs `archangel check`
- **THEN** archangel prints a `config-needs-node` diagnostic that says a JSON config needs no Node
- **AND** archangel exits with status 2
- **TEST** `crates/archangel/tests/config_discovery.rs` › "typescript_config_without_node_stops_with_config_needs_node"

#### Scenario: Node is too old

- **GIVEN** an `archangel.config.ts` and a `node` program of version 20.11.0
- **WHEN** the developer runs `archangel check`
- **THEN** archangel prints a `config-needs-node` diagnostic that names 20.11.0 and 22.18
- **AND** archangel exits with status 2
- **TEST** `crates/archangel/tests/config_discovery.rs` › "typescript_config_with_an_old_node_names_both_versions"

#### Scenario: The config code throws an error

- **GIVEN** an `archangel.config.ts` that throws an error with the message "bad element list"
- **WHEN** the developer runs `archangel check`
- **THEN** archangel prints a `config-load-failed` diagnostic that contains "bad element list"
- **AND** archangel exits with status 2
- **TEST** `crates/archangel/tests/config_discovery.rs` › "config_that_throws_stops_with_its_own_message"

### Requirement: A JSON config has the same content as a loaded TypeScript config

Archangel MUST accept a JSON config whose content is the same data that a TypeScript config gives after its load. A JSON config MUST NOT need Node.

#### Scenario: Equal configs in two formats

- **GIVEN** a TypeScript config and a JSON config that declare the same elements and rules
- **WHEN** archangel loads each config
- **THEN** the two loaded configs are equal
- **TEST** `crates/archangel_engine/tests/load.rs` › "json_and_typescript_configs_with_equal_content_load_equal"

#### Scenario: A JSON config without Node

- **GIVEN** an `archangel.config.json`, a repository with no violations, and no `node` program on the search path
- **WHEN** the developer runs `archangel check`
- **THEN** archangel makes the check and exits with status 0
- **TEST** `crates/archangel/tests/config_discovery.rs` › "json_config_needs_no_node"

### Requirement: Archangel refuses a config with fields it does not know

Archangel MUST refuse a config that has a field it does not know, or a field with a value of the wrong type. The `config-invalid` diagnostic MUST name the field and its location in the config. Archangel MUST report every such problem in the config, not only the first.

#### Scenario: A field name has a spelling mistake

- **GIVEN** a config with a rule that has the field `reasn` instead of `reason`
- **WHEN** the developer runs `archangel check`
- **THEN** archangel prints a `config-invalid` diagnostic that names `rules[0].reasn`
- **AND** archangel exits with status 2
- **TEST** `crates/archangel_config/tests/validate.rs` › "unknown_field_is_config_invalid_with_its_location"
- **TEST** `crates/archangel/tests/config_discovery.rs` › "invalid_config_exits_two"

#### Scenario: A JSON config that does not parse

- **GIVEN** an `archangel.config.json` with a missing comma on line 6
- **WHEN** the developer runs `archangel check`
- **THEN** archangel prints a `config-invalid` diagnostic on line 6 with its column
- **AND** archangel exits with status 2
- **TEST** `crates/archangel_engine/tests/load.rs` › "json_config_that_does_not_parse_is_config_invalid_with_its_line"
- **TEST** `crates/archangel/tests/config_discovery.rs` › "json_config_that_does_not_parse_exits_two"

#### Scenario: A TypeScript config with no default export

- **GIVEN** an `archangel.config.ts` that exports only a named value
- **WHEN** the developer runs `archangel check`
- **THEN** archangel prints a `config-load-failed` diagnostic that says the config has no default export
- **AND** archangel exits with status 2
- **TEST** `crates/archangel/tests/config_discovery.rs` › "typescript_config_without_a_default_export_exits_two"

#### Scenario: A TypeScript config that Node cannot strip

- **GIVEN** an `archangel.config.ts` that declares an `enum`
- **WHEN** the developer runs `archangel check`
- **THEN** archangel prints a `config-load-failed` diagnostic that names the enum and says to use a constant object
- **TEST** `crates/archangel/tests/config_discovery.rs` › "config_with_an_enum_names_the_construct_and_the_alternative"

#### Scenario: A config that prints to standard output

- **GIVEN** an `archangel.config.ts` that calls `console.log` while it loads
- **WHEN** the developer runs `archangel check`
- **THEN** archangel loads the config, and the check gives the same result as without the call
- **TEST** `crates/archangel/tests/config_discovery.rs` › "config_output_does_not_break_the_load"

#### Scenario: A field with the wrong type

- **GIVEN** a config with a rule whose `imports` field is the number 5
- **WHEN** archangel loads the config
- **THEN** archangel gives a `config-invalid` diagnostic that names `rules[0].imports` and its permitted values
- **TEST** `crates/archangel_config/tests/validate.rs` › "wrong_type_is_config_invalid_with_its_location"

#### Scenario: Two problems in one config

- **GIVEN** a config with an unknown field in its first rule and an undeclared element in its second rule
- **WHEN** archangel loads the config
- **THEN** archangel gives a diagnostic for each of the two problems
- **TEST** `crates/archangel_config/tests/validate.rs` › "every_problem_in_a_config_is_reported"

### Requirement: A config problem says where it is and what was meant

A problem with a field of a JSON config MUST carry the line and column of that field, or of the object that lacks it, counting from 1. A misspelt field, rule kind or element name MUST name the nearest known one when one is near. A JSON config MAY name its schema in `$schema`, which must be a string, and the npm package MUST ship that schema with exactly the fields archangel knows.

#### Scenario: The line of an unknown field

- **GIVEN** a JSON config with `"reasn"` on line 5
- **WHEN** archangel loads it
- **THEN** the problem is at line 5, column 7
- **TEST** `crates/archangel_config/tests/validate.rs` › "a_problem_in_a_json_config_has_the_line_and_column_of_its_field"

#### Scenario: A missing field

- **GIVEN** a layers rule with no `order` on line 4
- **THEN** the problem names `rules[0].order` at the rule's line and column
- **TEST** `crates/archangel_config/tests/validate.rs` › "a_missing_field_points_at_the_object_that_lacks_it"

#### Scenario: A misspelling

- **GIVEN** `reasn`, `isolat`, `entiy` and `elemnts` in four configs, and `colour` in a fifth
- **THEN** the first four suggest `reason`, `isolate`, `entity` and `elements`, and the fifth suggests nothing
- **TEST** `crates/archangel_config/tests/validate.rs` › "a_misspelt_field_kind_or_element_gets_the_nearest_name"

#### Scenario: The schema of a JSON config

- **GIVEN** a config with `"$schema"`, and one where it is a number
- **THEN** the first loads and the second is config-invalid at `$schema`
- **AND** the published schema has the fields the parser knows, no more and no fewer, accepts this repository's configs, and refuses what archangel refuses
- **TEST** `crates/archangel_config/tests/validate.rs` › "a_json_config_may_name_its_schema"
- **TEST** `crates/archangel_config/tests/schema.rs` › "the_published_schema_has_exactly_the_fields_the_parser_knows"
- **TEST** `npm/archangel/test/schema.test.mjs` › "what archangel refuses, the schema refuses"

#### Scenario: A parse error at the start of a line

- **GIVEN** JSON text that ends after `{`
- **THEN** the problem is at line 2, column 1
- **TEST** `crates/archangel_config/tests/validate.rs` › "a_parse_error_at_the_start_of_a_line_has_column_one"

### Requirement: A pattern matches paths by folders, names and captures

In a pattern, `**` MUST match zero or more folders. `*` MUST match zero or more characters in one folder or file name. A capture MUST match one or more characters in one folder or file name. Braces that contain a comma MUST list alternatives, as in `{a,b}`, and braces without a comma MUST name a capture. Archangel MUST compare patterns with paths relative to the config folder, with `/` between folders on every operating system.

#### Scenario: A capture matches one folder

- **GIVEN** an element "feature" with the pattern `src/features/{feature}/**`
- **WHEN** archangel classifies `src/features/login/ui/form.tsx`
- **THEN** the file is in the instance of "feature" with the capture value "login"
- **AND** the instance root is `src/features/login`
- **TEST** `crates/archangel_config/tests/pattern.rs` › "capture_matches_one_folder_and_sets_the_instance_root"

#### Scenario: A capture does not match an empty part

- **GIVEN** the pattern `src/{name}.ts`
- **WHEN** archangel compares it with the path `src/.ts`
- **THEN** the pattern does not match
- **TEST** `crates/archangel_config/tests/pattern.rs` › "capture_never_matches_an_empty_part"

#### Scenario: Braces with alternatives

- **GIVEN** an element "ui" with the pattern `src/{components,widgets}/**`
- **WHEN** archangel classifies `src/components/a.ts` and `src/widgets/b.ts`
- **THEN** both files are in "ui", and the element has no capture
- **TEST** `crates/archangel_config/tests/pattern.rs` › "braces_with_a_comma_list_alternatives"

#### Scenario: Paths from Windows use forward slashes

- **GIVEN** the pattern `src/shared/**` and a check on Windows
- **WHEN** archangel classifies the file `src\shared\ui\button.ts`
- **THEN** the file is in the element of that pattern
- **TEST** `crates/archangel_config/tests/pattern.rs` › "windows_paths_match_with_forward_slashes"

#### Scenario: The laws of patterns hold for random paths

- **GIVEN** random patterns and random paths
- **WHEN** archangel compares each pattern with each path
- **THEN** the result is the same as the result of a slow reference matcher
- **TEST** `crates/archangel_config/tests/pattern_properties.rs` › "patterns_agree_with_the_reference_matcher"

### Requirement: Archangel refuses a pattern that cannot mean what it says

Archangel MUST refuse a pattern that has an empty folder name, a `**` that is not a full folder name, a capture without a name, or the same capture name two times. The diagnostic MUST be `pattern-invalid` and MUST name the element and the pattern.

#### Scenario: A double star inside a folder name

- **GIVEN** an element "ui" with the pattern `src/ui**/x`
- **WHEN** the developer runs `archangel check`
- **THEN** archangel prints a `pattern-invalid` diagnostic that names "ui" and `src/ui**/x`
- **AND** archangel exits with status 2
- **TEST** `crates/archangel_config/tests/validate.rs` › "double_star_inside_a_name_is_pattern_invalid"
- **TEST** `crates/archangel/tests/config_discovery.rs` › "invalid_pattern_exits_two"

#### Scenario: An empty folder name

- **GIVEN** an element "x" with the pattern `src//x`
- **WHEN** archangel loads the config
- **THEN** archangel gives a `pattern-invalid` diagnostic that names "x" and `src//x`
- **TEST** `crates/archangel_config/tests/validate.rs` › "empty_folder_name_is_pattern_invalid"

#### Scenario: A capture without a name

- **GIVEN** an element "x" with the pattern `src/{}/x`
- **WHEN** archangel loads the config
- **THEN** archangel gives a `pattern-invalid` diagnostic that names "x" and `src/{}/x`
- **TEST** `crates/archangel_config/tests/validate.rs` › "capture_without_a_name_is_pattern_invalid"

#### Scenario: One capture name two times

- **GIVEN** an element "x" with the pattern `src/{a}/{a}`
- **WHEN** archangel loads the config
- **THEN** archangel gives a `pattern-invalid` diagnostic that names "x", `src/{a}/{a}` and the capture "a"
- **TEST** `crates/archangel_config/tests/validate.rs` › "repeated_capture_name_is_pattern_invalid"

#### Scenario: Random text never makes the pattern parser fail without a diagnostic

- **GIVEN** random text as a pattern
- **WHEN** archangel parses the pattern
- **THEN** archangel gives a pattern or a `pattern-invalid` diagnostic, and does not stop in any other way
- **TEST** `fuzz/fuzz_targets/pattern.rs` › "pattern"

### Requirement: Each file is in one element at most

Archangel MUST put each repository file in the first file element, in declaration order, that has a pattern which matches the file. A file that no pattern matches MUST be in no element.

#### Scenario: Two elements match the same file

- **GIVEN** the element "model" with `src/**/*.model.ts`, declared before the element "feature" with `src/features/{feature}/**`
- **WHEN** archangel classifies `src/features/login/session.model.ts`
- **THEN** the file is in the element "model" only
- **TEST** `crates/archangel_config/tests/pattern.rs` › "first_declared_element_wins"

### Requirement: A package element names packages

A package element MUST list package names, and a name MAY end with `/*` to match every package in a scope. An import whose resolution is a package MUST be in the first package element, in declaration order, that lists that package. A Node built-in module MUST have the package name with the `node:` prefix.

#### Scenario: A scope matches all its packages

- **GIVEN** a package element "contracts" that lists `@acme/*`
- **WHEN** archangel classifies the package `@acme/contract`
- **THEN** the package is in the element "contracts"
- **TEST** `crates/archangel_config/tests/pattern.rs` › "package_scope_matches_every_package_in_it"

#### Scenario: Two package elements list one package

- **GIVEN** the package element "ui" that lists `react`, declared before the package element "vendor" that lists `@*/*` and `react`
- **WHEN** archangel classifies the package `react`
- **THEN** the package is in the element "ui" only
- **TEST** `crates/archangel_config/tests/pattern.rs` › "first_declared_package_element_wins"

#### Scenario: A built-in module without its prefix

- **GIVEN** a package element "node" that lists `node:*`
- **WHEN** a source file imports `fs`
- **THEN** the import's resolution is the package `node:fs` in the element "node"
- **TEST** `crates/archangel_graph/tests/resolve.rs` › "builtin_without_prefix_resolves_to_its_node_name"
- **TEST** `crates/archangel_rules/tests/elements.rs` › "builtin_package_is_in_the_node_prefix_element"

### Requirement: An element that can match nothing is an error

Archangel MUST give an `element-empty` diagnostic for a file element that the config declares and whose patterns match no repository file, and for a package element when no import uses its packages and no manifest lists them. The check MUST fail. A preset's elements, and an inline package side of a `mayNotImport` rule, are exempt.

#### Scenario: A pattern has a spelling mistake

- **GIVEN** an element "shared" with the pattern `src/shraed/**` and no folder of that name
- **WHEN** the developer runs `archangel check`
- **THEN** archangel prints an `element-empty` diagnostic that names "shared" and its pattern
- **AND** archangel exits with status 1
- **TEST** `crates/archangel_rules/tests/elements.rs` › "file_element_that_matches_nothing_is_element_empty"
- **TEST** `crates/archangel/tests/cli.rs` › "element_empty_exits_one"

#### Scenario: A package that is installed but not imported yet

- **GIVEN** a package element "react" that lists `react`, and a package manifest that lists `react` as a dependency
- **AND** no source file imports `react`
- **WHEN** archangel makes a check
- **THEN** archangel gives no `element-empty` diagnostic for "react"
- **TEST** `crates/archangel_rules/tests/elements.rs` › "package_element_listed_in_a_manifest_is_not_empty"

#### Scenario: A package name has a spelling mistake

- **GIVEN** a package element "react" that lists `recat`, which no import uses and no package manifest lists
- **WHEN** archangel makes a check
- **THEN** archangel gives an `element-empty` diagnostic that names "react" and `recat`
- **TEST** `crates/archangel_rules/tests/elements.rs` › "package_element_with_an_unknown_name_is_element_empty"

#### Scenario: A rule that forbids a package nobody has installed

- **GIVEN** a rule `{ files: 'src/**', mayNotImport: { packages: ['pg'] } }`, and no import or manifest that names `pg`
- **WHEN** archangel makes a check
- **THEN** archangel gives no `element-empty` diagnostic and no violation
- **TEST** `crates/archangel_rules/tests/elements.rs` › "an_inline_forbidden_package_may_be_uninstalled"

### Requirement: Every visited file and every inline package side counts toward an element

A file element MUST count every file the walk visits, not only source files. An inline package side of a rule MUST be a package element without a name, to which the `element-empty` test applies, except on a `mayNotImport` rule.

#### Scenario: A rule that permits a misspelt package

- **GIVEN** a rule `{ name: 'only-react', files: 'src/**', mayOnlyImport: { packages: ['recat'] } }`, and no import or manifest that names `recat`
- **WHEN** archangel makes a check
- **THEN** archangel gives an `element-empty` diagnostic that names the rule "only-react" and `recat`
- **TEST** `crates/archangel_rules/tests/elements.rs` › "an_inline_permitted_package_that_nothing_names_is_element_empty"

#### Scenario: An element of files that are not source

- **GIVEN** an element "assets" with the pattern `assets/**`, and the file `assets/logo.png`, which no import reaches
- **WHEN** archangel makes a check
- **THEN** archangel gives no `element-empty` diagnostic for "assets"
- **TEST** `crates/archangel_rules/tests/elements.rs` › "a_file_element_of_files_that_are_not_source_is_not_empty"

### Requirement: A rule refers only to elements and selectors that the config declares

Archangel MUST refuse a rule that names an element or selector which the config does not declare, written as a shorthand's element or as a string on a rule side that is not a pattern, with a `rule-unknown-element` diagnostic that lists the closest declared names. Archangel MUST refuse an isolate rule, an `instance` option or a `mustContain` rule on an element whose patterns have no capture, with a `rule-needs-capture` diagnostic.

#### Scenario: A rule names an undeclared element

- **GIVEN** a layers rule on the elements "entity" and "reactt", and no element "reactt"
- **WHEN** the developer runs `archangel check`
- **THEN** archangel prints a `rule-unknown-element` diagnostic that names "reactt" and the declared element names
- **AND** archangel exits with status 2
- **TEST** `crates/archangel_config/tests/validate.rs` › "rule_naming_an_undeclared_element_is_rule_unknown_element"
- **TEST** `crates/archangel/tests/config_discovery.rs` › "rule_with_an_unknown_element_exits_two"

#### Scenario: An isolate rule on an element without a capture

- **GIVEN** an element "shared" with the pattern `src/shared/**` and an isolate rule on "shared"
- **WHEN** archangel loads the config
- **THEN** archangel gives a `rule-needs-capture` diagnostic that names "shared"
- **TEST** `crates/archangel_config/tests/validate.rs` › "isolate_without_a_capture_is_rule_needs_capture"

### Requirement: Each rule has a unique name

Each rule MUST have a name. When the config gives no name, archangel MUST make one from the shorthand kind or the condition and the rule's sides, such as `entry(entity)` or `mayNotImport(entity, react)`. When two rules have the same name, archangel MUST refuse the config with a `rule-name-duplicate` diagnostic that says to give each rule its own name, except that a config rule replaces a preset rule of the same prefixed name.

#### Scenario: A rule without a name

- **GIVEN** an entry rule on the element "entity" with no name
- **WHEN** archangel loads the config
- **THEN** the rule has the name `entry(entity)`
- **TEST** `crates/archangel_config/tests/validate.rs` › "rule_without_a_name_gets_one_from_its_kind_and_elements"

#### Scenario: Two rules get the same name

- **GIVEN** an element "react", and two rules `{ files: 'entity', mayNotImport: 'react' }` with no names, one with `imports: 'runtime'`
- **WHEN** archangel loads the config
- **THEN** archangel gives a `rule-name-duplicate` diagnostic that names `mayNotImport(entity, react)`
- **AND** the diagnostic says to give each rule a name
- **TEST** `crates/archangel_config/tests/validate.rs` › "two_rules_with_one_name_is_rule_name_duplicate"

#### Scenario: Random configs never make the loader fail without a diagnostic

- **GIVEN** random JSON as a config
- **WHEN** archangel loads it
- **THEN** archangel gives a config or a diagnostic, and does not stop in any other way
- **TEST** `fuzz/fuzz_targets/config.rs` › "config"
