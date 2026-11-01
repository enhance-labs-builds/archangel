# command-line

## ADDED Requirements

### Requirement: The rules command prints every rule as a sentence with its reach

`archangel rules` MUST make a check and then print every rule and every table row, in config order. For each, it MUST print the sentence that it means in words, its reason, the file and line where it is written, and its reach. The reach MUST be the count of files in its files, of imports it judged, of problems, of exceptions and of baseline matches. The output MUST NOT leave out a rule, a row or a line.

#### Scenario: The rules of the small app

- **GIVEN** the small-app fixture, with one view that imports an api file
- **WHEN** the developer runs `archangel rules`
- **THEN** the output is the expected output of the fixture, which shows each row of `app` and `feature-code` with its sentence, the because text, `archangel.config.ts` and its line, and its counts
- **AND** the row `feature-code.views` shows 2 files, 3 imports judged and 1 problem
- **TEST** `crates/archangel/tests/rules_command.rs` › "rules_prints_each_rule_as_a_sentence_with_its_reach"

#### Scenario: Many rules

- **GIVEN** a config with 300 rules
- **WHEN** the developer runs `archangel rules`
- **THEN** the output has an entry for each of the 300 rules
- **TEST** `crates/archangel/tests/rules_command.rs` › "rules_prints_every_rule_with_no_elision"

### Requirement: The rules command says which rules check nothing and where each rule comes from

A rule or row with no file MUST say that it checks nothing, and name the name that has no file. A preset rule MUST name its preset. A config rule that replaces a preset rule MUST name the rule that it replaces. A rule that the loader cannot place in a line MUST name the config file only.

#### Scenario: A row that checks nothing

- **GIVEN** the enhance3-slice fixture, with no aggregate-root file
- **WHEN** the developer runs `archangel rules`
- **THEN** the row `domain.aggregate-root` says "checks nothing: no file is in aggregate-root"
- **TEST** `crates/archangel/tests/rules_command.rs` › "rules_shows_a_row_with_no_file_as_checking_nothing"

#### Scenario: A rule written as a plain object

- **GIVEN** a config whose rules are plain objects in a JSON config
- **WHEN** the developer runs `archangel rules`
- **THEN** each rule names `archangel.config.json` as its file, with no line
- **TEST** `crates/archangel/tests/rules_command.rs` › "rules_names_the_config_file_of_a_plain_rule"

#### Scenario: A preset rule and its replacement

- **GIVEN** a config that extends a preset `team` with the rule `views-thin`, and a config rule `team/views-thin` that replaces it
- **WHEN** the developer runs `archangel rules`
- **THEN** the rule `team/views-thin` says that it replaces the rule `views-thin` of the preset `team`
- **TEST** `crates/archangel/tests/rules_command.rs` › "rules_names_the_preset_rule_that_a_config_rule_replaces"

### Requirement: The rules command lists what no rule judges

After the rules, `archangel rules` MUST list each repository file that no rule's files include, and each import between repository files that no rule judged. It MUST list, for each outside package that no rule names, the package and its count of imports.

#### Scenario: A file and an import that no rule judges

- **GIVEN** a config whose only rule has the files `src/features/**`, and the files `src/main.ts`, which imports `src/shared/a.ts` and `react`
- **WHEN** the developer runs `archangel rules`
- **THEN** the output lists `src/main.ts` and `src/shared/a.ts` as files that no rule judges, and the import from `src/main.ts` to `src/shared/a.ts` as an import that no rule judges
- **AND** the output lists `react` with 1 import
- **TEST** `crates/archangel/tests/rules_command.rs` › "rules_lists_the_files_and_imports_that_no_rule_judges"

### Requirement: The rules command can look at one folder

With a path, `archangel rules` MUST print only the rules and rows whose files include a file under that path, and only the unjudged files and imports under it.

#### Scenario: The rules of one feature

- **GIVEN** the small-app fixture
- **WHEN** the developer runs `archangel rules src/shared`
- **THEN** the output shows the row `app.shared` and no row of `feature-code`
- **TEST** `crates/archangel/tests/rules_command.rs` › "rules_with_a_path_prints_the_rules_that_apply_there"

### Requirement: The rules command answers in JSON too

With `--format json`, `archangel rules` MUST print the same content as the text output, as one JSON document.

#### Scenario: Rules as data

- **GIVEN** the small-app fixture
- **WHEN** an agent runs `archangel rules --format json`
- **THEN** the document has an entry for each rule and row, with its sentence, reason, file, line and reach, and the lists of what no rule judges
- **TEST** `crates/archangel/tests/formats.rs` › "rules_as_json_gives_each_rule_with_its_reach"

### Requirement: Explain shows every rule that judged or skipped one import

`archangel explain <file>:<line>` MUST give the import on that line, its target, and the part, instance and kinds of both ends. It MUST list each rule that judged the import, with its verdict, and each rule that skipped it, with the reason. A reason MUST be one of these: the file is not in the rule's files, the target is outside the area, the import kind or an import limit does not match, or the instance differs. An exception or a baseline match MUST show beside the rule that it permits.

#### Scenario: A view that imports an api file

- **GIVEN** the small-app fixture
- **WHEN** the developer runs `archangel explain src/features/cart/views/cart-page.tsx:2`
- **THEN** the output shows that `app.feature` permits the import in the same instance, and that `feature-code.views` refuses it because `api` is not in its row
- **AND** the output shows that `app.shared`, `feature-code.models` and `feature-code.api` skipped it, each with its reason
- **TEST** `crates/archangel/tests/cli.rs` › "explain_of_one_import_lists_the_rules_that_judged_and_skipped_it"

#### Scenario: An import outside an area

- **GIVEN** a rule `rule('cli-uses-only-the-api-router', { files: 'cli', mayOnlyImport: 'apps/api/src/app.router.ts', within: 'api' })`, and a CLI file that imports `zod` on line 1
- **WHEN** the developer runs `archangel explain` on that line
- **THEN** the output says that the rule skipped the import because `zod` is outside the area `api`
- **TEST** `crates/archangel/tests/cli.rs` › "explain_of_one_import_names_the_area_that_skipped_it"

### Requirement: Explain of one import fails on a line with no import, and answers in JSON

`archangel explain <file>:<line>` MUST say so and exit with status 1 when the line has no import. With `--format json`, it MUST print the same content as the text output.

#### Scenario: A line with no import

- **GIVEN** a file whose line 3 has no import
- **WHEN** the developer runs `archangel explain` on that line
- **THEN** archangel says that line 3 has no import, and exits with status 1
- **TEST** `crates/archangel/tests/cli.rs` › "explain_of_a_line_with_no_import_says_so"

#### Scenario: One import as data

- **GIVEN** the small-app fixture
- **WHEN** an agent runs `archangel explain src/features/cart/views/cart-page.tsx:2 --format json`
- **THEN** the document holds the same answer as the text
- **TEST** `crates/archangel/tests/formats.rs` › "explain_of_one_import_as_json"
