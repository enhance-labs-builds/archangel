# Spec Delta

## ADDED Requirements

### Requirement: An unknown command lists the commands in columns

After an unknown command, archangel MUST list each command that is not hidden, with its description in one column.

#### Scenario: A misspelt command

- **GIVEN** the command line `archangel chekc`
- **WHEN** archangel runs
- **THEN** it exits with 2, each description starts in one column, the longest name has a space after it, and the hidden command `man` is not listed
- **TEST** `crates/archangel/tests/cli.rs` › "an_unknown_command_lists_the_commands_in_columns"

### Requirement: Explain says how each rule is about the file

`archangel explain <file>` MUST say, for each sentence about the file, whether it judges the file's own imports, imports of the file, or both. The JSON output MUST give the same as the field `judges`.

#### Scenario: A shared file in a table

- **GIVEN** a table `app` with the rows `feature: ['shared']` and `shared: []`, and a file in shared
- **WHEN** the developer runs `archangel explain` on the file
- **THEN** the row `app.feature` says "judges imports of it", and the row `app.shared` says "judges its imports"
- **TEST** `crates/archangel/tests/cli.rs` › "explain_says_which_rules_judge_the_files_imports_and_which_judge_imports_of_it"
