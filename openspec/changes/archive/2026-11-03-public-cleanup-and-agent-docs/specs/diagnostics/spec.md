# Spec Delta

## ADDED Requirements

### Requirement: A message says what its rule refuses, in words

The message of an import violation MUST name the limits of its rule: the instance, the names and the form of the specifier. It MUST say a package pattern in words. An `entry` sentence in a rule of several sentences MUST give the message of an entry rule.

#### Scenario: An entry sentence in a rule of several sentences

- **GIVEN** a rule `public-api` with one sentence `{ kind: 'entry', element: 'entity', files: ['index.ts'] }`, and a file outside the instance that imports `src/entities/ledger/model/store.ts`
- **WHEN** archangel checks the repository
- **THEN** the violation has the code `entry`, and its help names `src/entities/ledger/index.ts`
- **TEST** `crates/archangel_rules/tests/diagnostics.rs` › "an_entry_sentence_in_a_rule_of_several_sentences_names_its_entry_files"

#### Scenario: A rule with a limit on the specifier

- **GIVEN** a rule `{ files: 'app', mayNotImport: 'shared', specifier: 'relative' }`, and a relative import from app into shared
- **WHEN** archangel checks the repository
- **THEN** the message ends with "may not import "shared" through a relative specifier"
- **TEST** `crates/archangel_rules/tests/diagnostics.rs` › "a_may_not_import_message_names_the_limits_of_its_rule"

#### Scenario: Package patterns in a message

- **GIVEN** a rule that refuses `{ packages: ['*'] }` with `dependency: 'undeclared'`, and a rule that refuses `{ packages: ['@aws-sdk/*'] }`
- **WHEN** a file imports `zod`, which no manifest lists, and a file imports `@aws-sdk/s3`
- **THEN** the messages say "any package that package.json does not list" and "a package in `@aws-sdk`"
- **TEST** `crates/archangel_rules/tests/diagnostics.rs` › "a_package_pattern_reads_as_words"

### Requirement: A part that only overlaps another part is not empty

A part whose pattern matches a file MUST NOT be `element-empty`, also when the file is in two parts. The `part-overlap` diagnostic MUST report the file.

#### Scenario: Two parts and one file

- **GIVEN** the parts `model: 'src/**/*.model.ts'` and `feature: 'src/features/{feature}/**'`, and only the file `src/features/login/session.model.ts`
- **WHEN** archangel checks the repository
- **THEN** it gives one `part-overlap` and no `element-empty`
- **TEST** `crates/archangel_rules/tests/elements.rs` › "a_part_that_matches_only_overlapping_files_is_not_empty"

### Requirement: The page of each code uses the words of the config

The page of each diagnostic code MUST say "part", "group" or "files" in its prose, not "element" or "selector". The name of a code and the text in code spans do not count.

#### Scenario: The pages of the codes

- **GIVEN** the page of each code
- **WHEN** the test reads its prose
- **THEN** no page says "element" or "selector"
- **TEST** `crates/archangel_rules/tests/diagnostics.rs` › "every_page_uses_the_words_of_the_config"
