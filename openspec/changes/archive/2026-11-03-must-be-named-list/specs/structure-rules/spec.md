# structure-rules

## ADDED Requirements

### Requirement: A mustBeNamed rule can list the names it permits

A `mustBeNamed` rule MUST accept a list of styles and name patterns. A file or folder MUST pass when at least one entry matches. A list with one entry MUST give the same result and message as the single value.

#### Scenario: A file that one entry permits

- **GIVEN** a rule `{ files: 'src/ui/*', mustBeNamed: ['*.view', '*.container'] }`, and the files `src/ui/cart.view.tsx`, `src/ui/cart.container.tsx` and `src/ui/cart.tsx`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for `cart.tsx`, that names the file, its full name and each entry
- **TEST** `crates/archangel_rules/tests/name_lists.rs` › "a_file_passes_when_one_entry_matches_and_the_message_lists_each_entry"

#### Scenario: A pattern with a star matches a full name

- **GIVEN** a rule with the entries `*.view` and `*.styles`, and the files `cart.view.tsx`, `view.tsx` and `cart.view.extra.tsx`
- **WHEN** archangel makes a check
- **THEN** archangel accepts `cart.view.tsx`, and gives a violation for each of the other two
- **TEST** `crates/archangel_rules/tests/name_lists.rs` › "a_star_entry_matches_the_whole_full_name"

#### Scenario: A list of one is the single value

- **GIVEN** the rule `{ files: 'src/*', mustBeNamed: ['PascalCase'] }` and the same rule with the value `'PascalCase'`
- **WHEN** archangel makes a check of the file `src/my-card.ts`
- **THEN** the two violations have the same message, which suggests `MyCard.ts`
- **TEST** `crates/archangel_rules/tests/name_lists.rs` › "a_list_of_one_entry_is_the_single_value"

#### Scenario: A folder that no entry permits

- **GIVEN** a rule `{ folders: 'src/*', mustBeNamed: ['*-kit', 'PascalCase'] }`, and the folders `src/ui-kit`, `src/Tools` and `src/my_tools`
- **WHEN** archangel makes a check
- **THEN** archangel gives one violation, for `src/my_tools`, that names each entry
- **TEST** `crates/archangel_rules/tests/name_lists.rs` › "a_folder_passes_when_one_entry_matches_and_the_message_lists_each_entry"

#### Scenario: Examples of a rule with a list

- **GIVEN** a rule with a list, a rejected file set, a rejected one-line example and an accepted one-line example
- **WHEN** archangel runs the examples of the rule
- **THEN** each example passes
- **TEST** `crates/archangel_rules/tests/name_lists.rs` › "examples_prove_a_rule_with_a_list"

### Requirement: A list of names follows the rules of a single value

The config MUST refuse an empty list and an entry that is not a string, and say what to write. Each entry MUST follow the rules of a single value, and each problem MUST name the place of its entry.

#### Scenario: An empty list

- **GIVEN** a rule `{ files: 'src/**', mustBeNamed: [] }`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at `mustBeNamed`, saying that the list must have an entry
- **TEST** `crates/archangel_config/tests/name_lists.rs` › "an_empty_list_is_refused_and_says_what_to_write"

#### Scenario: An entry that is not a string

- **GIVEN** a rule `{ files: 'src/**', mustBeNamed: ['*.view', 3] }`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at `mustBeNamed[1]`, saying that an entry must be a style or a name pattern
- **TEST** `crates/archangel_config/tests/name_lists.rs` › "an_entry_that_is_not_a_string_is_refused_at_its_place"

#### Scenario: A folder rule with a file name entry

- **GIVEN** a rule `{ folders: 'src/*', mustBeNamed: ['*-kit', '{FileName}'] }`
- **WHEN** archangel loads the config
- **THEN** archangel gives `config-invalid` at `mustBeNamed[1]`, saying that a folder has no file name
- **TEST** `crates/archangel_config/tests/name_lists.rs` › "a_folder_rule_refuses_a_file_name_in_any_entry"

#### Scenario: The schema refuses an empty list

- **GIVEN** a config whose rule has `mustBeNamed: []`, and a config whose rule has `mustBeNamed: ['*.view', 3]`
- **WHEN** the published JSON schema checks each config
- **THEN** the schema refuses each one
- **TEST** `npm/archangel/test/schema.test.mjs` › "a mustBeNamed list has at least one entry, and each entry is text"
