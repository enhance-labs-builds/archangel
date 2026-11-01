# configuration

## MODIFIED Requirements

### Requirement: Archangel refuses a pattern that cannot mean what it says

Archangel MUST refuse a pattern that has an empty folder name, a `**` that is not a full folder name, a capture without a name, or the same capture name two times. A matrix's role pattern MAY contain the capture `{role}` two or more times, because each place takes the same role name. Another capture name two times in a role pattern MUST be refused. The diagnostic MUST be `pattern-invalid`. It MUST name the pattern, and the element, or the matrix for a role pattern.

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

#### Scenario: The role capture two times in a role pattern

- **GIVEN** a matrix `api-domain` with the role pattern `**/{role}s/*.{role}.ts`
- **AND** a matrix `api-app` with the role pattern `apps/{app}/{app}/*.{role}.ts`
- **WHEN** archangel loads the config
- **THEN** archangel gives no diagnostic for the pattern of `api-domain`
- **AND** archangel gives a `pattern-invalid` diagnostic that names `api-app`, its pattern and the capture "app"
- **TEST** `crates/archangel_config/tests/matrices.rs` › "only_the_role_capture_may_repeat_in_a_role_pattern"

#### Scenario: A role pattern that cannot mean what it says

- **GIVEN** a matrix `api-domain` with the role pattern `**//*.{role}.ts`
- **WHEN** archangel loads the config
- **THEN** archangel gives a `pattern-invalid` diagnostic that names the matrix `api-domain` and the pattern `**//*.{role}.ts`
- **TEST** `crates/archangel_config/tests/matrices.rs` › "a_role_pattern_that_cannot_mean_what_it_says_is_pattern_invalid"

#### Scenario: Random text never makes the pattern parser fail without a diagnostic

- **GIVEN** random text as a pattern
- **WHEN** archangel parses the pattern
- **THEN** archangel gives a pattern or a `pattern-invalid` diagnostic, and does not stop in any other way
- **TEST** `fuzz/fuzz_targets/pattern.rs` › "pattern"
