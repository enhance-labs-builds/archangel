# command-line

## ADDED Requirements

### Requirement: The rules command prints every rule as a sentence with its reach

`archangel rules` MUST make a check and then print every rule and every table row, in config order. For each, it MUST print the sentence that it means in words, its reason, the file and line where it is written, and its reach. The output MUST NOT leave out a rule, a row or a line.

#### Scenario: The rules of the small app

- **GIVEN** the small-app fixture, with one view that imports an api file
- **WHEN** the developer runs `archangel rules`
- **THEN** the output is the expected output of the fixture, which shows each row of `app` and `feature-code` with its sentence, the because text, `archangel.config.ts` and its line, and its counts
- **AND** the row `feature-code.views` shows 2 files, 3 imports judged and 1 problem
- **TEST** `crates/archangel/tests/rules_command.rs` › "rules_prints_each_rule_as_a_sentence_with_its_reach"

### Requirement: The reach of a rule counts what it covered

The reach MUST be the count of files in the rule's files, of imports it judged, of problems, of exceptions and of baseline matches.

#### Scenario: An exception and a baseline match

- **GIVEN** the small-app fixture with a second view that imports an api file under `// archangel-allow feature-code.views: migration`, and a baseline entry for the first
- **WHEN** the developer runs `archangel rules`
- **THEN** the row `feature-code.views` shows 0 problems, 1 exception and 1 baseline match
- **TEST** `crates/archangel/tests/rules_command.rs` › "rules_counts_exceptions_and_baseline_matches_in_the_reach"

### Requirement: The rules command describes and does not fail

`archangel rules` MUST exit with status 0 when it prints, whatever problems the check finds, because its job is to describe. It MUST exit with status 2 for an invalid config.

#### Scenario: Rules of a repository with a problem

- **GIVEN** the small-app fixture, whose check finds one problem
- **WHEN** the developer runs `archangel rules`
- **THEN** archangel prints the rules and exits with status 0
- **TEST** `crates/archangel/tests/rules_command.rs` › "rules_exits_zero_whatever_the_check_finds"

#### Scenario: Many rules

- **GIVEN** a config with 300 rules
- **WHEN** the developer runs `archangel rules`
- **THEN** the output has an entry for each of the 300 rules
- **TEST** `crates/archangel/tests/rules_command.rs` › "rules_prints_every_rule_with_no_elision"

### Requirement: The rules command says which rules check nothing and where each rule comes from

A rule or row with no file MUST say that it checks nothing, and name the name that has no file. A preset rule MUST name its preset. A config rule that replaces a preset rule MUST name the rule that it replaces. A rule made by a builder MUST name the file and line of the builder call, in whichever file the call is, a helper module included. A rule written as a plain object MUST name the config file only.

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

#### Scenario: A rule built in a helper module

- **GIVEN** a config `archangel.config.ts` whose rules come from `rules/domain.ts`, which calls `rule('entities-stay-pure', ...)` on its line 4
- **WHEN** the developer runs `archangel rules`
- **THEN** the rule `entities-stay-pure` names `rules/domain.ts` and the line 4
- **TEST** `crates/archangel/tests/rules_command.rs` › "rules_names_the_helper_module_that_built_a_rule"

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

With a path, `archangel rules` MUST print only the rules and rows whose files include a file under that path, and only the unjudged files and imports under it. A path with no source file under it MUST be refused with status 2, as for any path argument.

#### Scenario: The rules of one feature

- **GIVEN** the small-app fixture
- **WHEN** the developer runs `archangel rules src/shared`
- **THEN** the output shows the row `app.shared` and no row of `feature-code`
- **TEST** `crates/archangel/tests/rules_command.rs` › "rules_with_a_path_prints_the_rules_that_apply_there"

#### Scenario: A folder with no file

- **GIVEN** the small-app fixture
- **WHEN** the developer runs `archangel rules src/nothing`
- **THEN** archangel says that no source file is under `src/nothing`, and exits with status 2
- **TEST** `crates/archangel/tests/rules_command.rs` › "rules_with_a_path_that_holds_no_file_is_refused"

### Requirement: The rules command answers in JSON too

With `--format json`, `archangel rules` MUST print the same content as the text output, as one JSON document.

#### Scenario: Rules as data

- **GIVEN** the small-app fixture
- **WHEN** an agent runs `archangel rules --format json`
- **THEN** the document has an entry for each rule and row, with its sentence, reason, file, line and reach, and the lists of what no rule judges
- **TEST** `crates/archangel/tests/formats.rs` › "rules_as_json_gives_each_rule_with_its_reach"

### Requirement: Explain shows every rule that judged or skipped one import

`archangel explain <file>:<line>` MUST give the import on that line, its target, and the part, instance and kinds of both ends. It MUST list each rule that judged the import, with its verdict, and each rule that skipped it, with one reason: not in the rule's files, outside the area, the import kind or a limit does not match, another instance, an unresolved import, the rule is off, or the condition judges no import. An unresolved import MUST show its `import-unresolved` reason and no target.

#### Scenario: A view that imports an api file

- **GIVEN** the small-app fixture
- **WHEN** the developer runs `archangel explain src/features/cart/views/cart-page.tsx:2`
- **THEN** the output shows that `app.feature` permits the import in the same instance, and that `feature-code.views` refuses it because `api` is not in its row
- **AND** the output shows that `app.shared` skipped it because the file is not in `shared`, and that `feature-code.models` and `feature-code.api` skipped it because the file has the kind `views`, not theirs
- **TEST** `crates/archangel/tests/cli.rs` › "explain_of_one_import_lists_the_rules_that_judged_and_skipped_it"

#### Scenario: An import that does not resolve

- **GIVEN** the small-app fixture with a view whose line 3 is `import './missing'`
- **WHEN** the developer runs `archangel explain` on that line
- **THEN** the output shows the import of `./missing`, says that it does not resolve with the reason of its `import-unresolved` diagnostic, and names no target
- **AND** the output lists each import rule whose files include the view, `app.feature` and `feature-code.views`, as skipping it because the import does not resolve
- **TEST** `crates/archangel/tests/cli.rs` › "explain_of_an_unresolved_import_says_so_and_names_no_target"

#### Scenario: A rule that is off and a rule that checks no import

- **GIVEN** the small-app fixture with `off: ['feature-code.views']` and one more rule, `rule('kebab-case-files', { files: 'src/**', mustBeNamed: 'kebab-case' })`
- **WHEN** the developer runs `archangel explain src/features/cart/views/cart-page.tsx:2`
- **THEN** the output lists `feature-code.views` as skipped because the rule is off
- **AND** the output lists `kebab-case-files` as skipped because `mustBeNamed` checks no import
- **TEST** `crates/archangel/tests/cli.rs` › "explain_of_one_import_shows_a_rule_that_is_off_and_one_that_checks_no_import"

### Requirement: Explain answers for any line of a statement and for each import on a line

The line MAY be any line of the import statement. A line with two or more imports MUST give one block for each.

#### Scenario: An import statement over two lines

- **GIVEN** a file whose lines 1 and 2 are `import {` and `  fetchCart } from '../api/fetch-cart';`
- **WHEN** the developer runs `archangel explain` on line 2
- **THEN** the output explains the import of `../api/fetch-cart`
- **TEST** `crates/archangel/tests/cli.rs` › "explain_of_a_line_inside_an_import_statement_explains_it"

#### Scenario: Two imports on one line

- **GIVEN** a file whose line 1 is `import a from './a'; import b from './b';`
- **WHEN** the developer runs `archangel explain` on line 1
- **THEN** the output has one block for the import of `./a` and one for the import of `./b`
- **TEST** `crates/archangel/tests/cli.rs` › "explain_of_a_line_with_two_imports_explains_each"

### Requirement: Explain shows an exception or a baseline match beside its rule

An exception or a baseline match MUST show beside the rule that it permits.

#### Scenario: An import under an exception

- **GIVEN** the small-app fixture, with the view's import of the api file under `// archangel-allow feature-code.views: migration`
- **WHEN** the developer runs `archangel explain` on that line
- **THEN** the output shows `feature-code.views` as refusing the import, with the exception beside it
- **TEST** `crates/archangel/tests/cli.rs` › "explain_of_one_import_shows_the_exception_beside_the_rule"

#### Scenario: An import outside an area

- **GIVEN** a rule `rule('cli-uses-only-the-api-router', { files: 'cli', mayOnlyImport: 'apps/api/src/app.router.ts', within: 'api' })`, and a CLI file that imports `zod` on line 1
- **WHEN** the developer runs `archangel explain` on that line
- **THEN** the output says that the rule skipped the import because `zod` is outside the area `api`
- **TEST** `crates/archangel/tests/cli.rs` › "explain_of_one_import_names_the_area_that_skipped_it"

### Requirement: The line of explain follows the last colon

In `archangel explain <file>:<line>`, the line MUST be what follows the last colon of the argument, and the file everything before it, so that a Windows path such as `C:\\repo\\src\\a.ts:2` is read as a file and a line.

#### Scenario: A file whose path holds a colon

- **GIVEN** a source file `src/a:b.ts` with an import on line 2
- **WHEN** the developer runs `archangel explain src/a:b.ts:2`
- **THEN** the output explains the import on line 2 of `src/a:b.ts`
- **TEST** `crates/archangel/tests/cli.rs` › "explain_reads_the_line_after_the_last_colon"

### Requirement: Explain of a line with no import fails

`archangel explain <file>:<line>` MUST say so and exit with status 1 when the line has no import. A file that does not parse MUST give its `parse-failed` diagnostic, say that archangel cannot explain a file that does not parse, and exit with status 1. A file that does not exist, or that is not a source file under the config folder, MUST be refused with status 2, as for `archangel explain <file>`.

#### Scenario: A line with no import

- **GIVEN** a file whose line 3 has no import
- **WHEN** the developer runs `archangel explain` on that line
- **THEN** archangel says that line 3 has no import, and exits with status 1
- **TEST** `crates/archangel/tests/cli.rs` › "explain_of_a_line_with_no_import_says_so"

#### Scenario: A file that does not parse

- **GIVEN** a file with an import statement on line 1 and a syntax error on line 5
- **WHEN** the developer runs `archangel explain` on line 1
- **THEN** archangel gives `parse-failed` for the file, says that it cannot explain a file that does not parse, and exits with status 1
- **TEST** `crates/archangel/tests/cli.rs` › "explain_of_a_line_in_a_file_that_does_not_parse_gives_parse_failed"

#### Scenario: A file that does not exist

- **GIVEN** the small-app fixture
- **WHEN** the developer runs `archangel explain src/missing.ts:1`
- **THEN** archangel says that `src/missing.ts` does not exist, and exits with status 2
- **TEST** `crates/archangel/tests/cli.rs` › "explain_of_a_missing_file_is_refused"

### Requirement: Explain of one import answers in JSON

With `--format json`, `archangel explain <file>:<line>` MUST print the same content as the text output.

#### Scenario: One import as data

- **GIVEN** the small-app fixture
- **WHEN** an agent runs `archangel explain src/features/cart/views/cart-page.tsx:2 --format json`
- **THEN** the document has the import, its target, the part, instance and kinds of both ends, one entry for each rule that judged it with its verdict, one entry for each rule that skipped it with its reason, and the exception or baseline match beside its rule
- **TEST** `crates/archangel/tests/formats.rs` › "explain_of_one_import_as_json"
