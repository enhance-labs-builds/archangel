# command-line Specification

## Purpose
The command line is how developers, CI and agents use archangel outside the editor. This capability
covers each command, what it prints, and its exit status.

## Requirements

### Requirement: The exit status says what happened

Every command MUST exit with status 0 when it succeeds with no diagnostic. It MUST exit with status 1 when it finds a diagnostic or its question has a negative answer, such as a failed example or no chain for `why`. It MUST exit with status 2 when the config, the baseline or the command line stops it.

#### Scenario: A clean check

- **GIVEN** a repository with no violations
- **WHEN** the developer runs `archangel check`
- **THEN** archangel prints a summary with the number of files and imports it checked
- **AND** archangel exits with status 0
- **TEST** `crates/archangel/tests/cli.rs` › "clean_check_prints_a_summary_and_exits_zero"

#### Scenario: A command that does not exist

- **GIVEN** any repository
- **WHEN** the developer runs `archangel chekc`
- **THEN** archangel prints the list of commands and suggests `check`
- **AND** archangel exits with status 2
- **TEST** `crates/archangel/tests/cli.rs` › "unknown_command_suggests_the_nearest_and_exits_two"

### Requirement: The check command checks the repository

`archangel check` MUST make one check and print every diagnostic and the summary. With file paths as arguments, it MUST print only the diagnostics of those files. It MUST still check all of the repository, because a cycle and an empty element depend on all files.

#### Scenario: Check one file

- **GIVEN** violations in `src/a.ts` and in `src/b.ts`
- **WHEN** the developer runs `archangel check src/a.ts`
- **THEN** archangel prints the violations of `src/a.ts` only
- **TEST** `crates/archangel/tests/cli.rs` › "check_with_a_path_prints_only_that_files_diagnostics"

#### Scenario: A path that is not a source file

- **GIVEN** no source file at `src/missing.ts`
- **WHEN** the developer runs `archangel check src/missing.ts`, `archangel explain src/missing.ts` or `archangel why src/missing.ts react`, where each path must be a source file
- **THEN** archangel says that `src/missing.ts` is not a source file and why: it does not exist, it is ignored, or its extension is not a source extension
- **AND** archangel exits with status 2
- **TEST** `crates/archangel/tests/cli.rs` › "path_that_is_not_a_source_file_exits_two_for_each_command"

#### Scenario: Check one file that is in a cycle

- **GIVEN** `src/a.ts` and `src/b.ts`, which import each other, and a `noCycles` rule
- **WHEN** the developer runs `archangel check src/a.ts`
- **THEN** archangel prints the no-cycles violation
- **TEST** `crates/archangel/tests/cli.rs` › "check_with_a_path_still_finds_cycles"

### Requirement: The explain command describes one file's place in the architecture

`archangel explain <file>` MUST print the file's element and capture values, the rules that apply to it, each of its imports with its resolution and the rules that it breaks, and the files that import it. It MUST list the imports that archangel cannot follow.

#### Scenario: Explain a feature file

- **GIVEN** the file `src/features/login/a.ts` with two imports, one of which breaks an isolate rule
- **WHEN** the developer runs `archangel explain src/features/login/a.ts`
- **THEN** archangel prints the element "feature" with the capture value "login"
- **AND** archangel prints the rules that apply to the file
- **AND** archangel prints both imports, and marks the one that breaks the isolate rule
- **AND** archangel prints the files that import it
- **TEST** `crates/archangel/tests/cli.rs` › "explain_prints_the_element_imports_and_broken_rules"

#### Scenario: Explain a file in no element

- **GIVEN** the file `scripts/seed.ts`, which no pattern matches
- **WHEN** the developer runs `archangel explain scripts/seed.ts`
- **THEN** archangel says that the file is in no element, and lists each element pattern with the reason it does not match
- **TEST** `crates/archangel/tests/cli.rs` › "explain_says_why_a_file_is_in_no_element"

### Requirement: The why command shows how one file reaches another

`archangel why <from> <to>` MUST print the shortest chain of imports from the first file to the second, with the line of each import. The first MUST be a source file. The second MAY be any repository file or package in the graph. When there is no chain, it MUST say so and exit with status 1.

#### Scenario: A chain of three imports

- **GIVEN** `src/a.ts` imports `src/b.ts`, which imports `src/c.ts`, which imports `drizzle-orm`
- **WHEN** the developer runs `archangel why src/a.ts drizzle-orm`
- **THEN** archangel prints the chain `src/a.ts`, `src/b.ts`, `src/c.ts`, `drizzle-orm` with the line of each import
- **TEST** `crates/archangel/tests/cli.rs` › "why_prints_the_shortest_chain_with_lines"

#### Scenario: A chain to a CSS file

- **GIVEN** `src/a.tsx` imports `src/b.tsx`, which imports `./b.module.css`
- **WHEN** the developer runs `archangel why src/a.tsx src/b.module.css`
- **THEN** archangel prints the chain `src/a.tsx`, `src/b.tsx`, `src/b.module.css`
- **TEST** `crates/archangel/tests/cli.rs` › "why_reaches_a_file_that_is_not_a_source_file"

#### Scenario: No chain

- **GIVEN** two files with no chain of imports between them
- **WHEN** the developer runs `archangel why` with the two files
- **THEN** archangel says that the first file does not reach the second
- **AND** archangel exits with status 1
- **TEST** `crates/archangel/tests/cli.rs` › "why_without_a_chain_says_so_and_exits_one"

### Requirement: The watch command checks again after each change

`archangel watch` MUST make a check, and then make a check again after each change to a source file, the config, the baseline, a `.gitignore` file, a package manifest, or any file that a resolution read or found. After each check, it MUST print only the diagnostics that are new or gone, and a summary line. With `--format json`, it MUST print one JSON line for each check.

#### Scenario: A change adds a violation

- **GIVEN** `archangel watch` that runs with no violations
- **WHEN** the developer saves a file with an import that breaks a rule
- **THEN** archangel prints the new violation and a summary line
- **TEST** `crates/archangel/tests/watch.rs` › "watch_prints_a_new_violation_after_a_save"

#### Scenario: A change removes a violation

- **GIVEN** `archangel watch` that shows one violation
- **WHEN** the developer saves a fix for it
- **THEN** archangel prints that the violation is gone, and a summary line with zero diagnostics
- **TEST** `crates/archangel/tests/watch.rs` › "watch_prints_a_violation_as_gone_after_a_fix"

#### Scenario: Watch with JSON output

- **GIVEN** `archangel watch --format json` that runs with no violations
- **WHEN** the developer saves a file with an import that breaks a rule
- **THEN** archangel prints exactly one JSON line, which lists the new violation and the summary
- **TEST** `crates/archangel/tests/watch.rs` › "watch_json_prints_one_line_per_check"

#### Scenario: A change to the config in watch mode

- **GIVEN** `archangel watch` that runs with no violations
- **WHEN** the developer saves the config with a new `mayNotImport` rule that the code breaks
- **THEN** archangel prints the new violations of that rule
- **TEST** `crates/archangel/tests/watch.rs` › "watch_checks_again_after_a_config_change"

#### Scenario: Work for one save in watch mode

- **GIVEN** `archangel watch --format json` that runs on a repository with 200 source files
- **WHEN** the developer saves one file that has 4 imports
- **THEN** the JSON line for that check has 1 file parsed and 4 resolutions
- **TEST** `crates/archangel/tests/watch.rs` › "one_save_parses_one_file_in_watch_mode"

#### Scenario: The fault control for watch work

- **GIVEN** a build of archangel with a fault that makes a full check after each file event
- **WHEN** the watch work test runs against that build
- **THEN** the test fails on the count of files parsed
- **TEST** `crates/archangel/tests/watch.rs` › "watch_work_count_rejects_a_full_check_per_save"

#### Scenario: A save by temporary file and rename

- **GIVEN** `archangel watch --format json` that runs on a repository with 200 source files
- **WHEN** an editor saves `src/a.ts`, which has 4 imports, by a write to a temporary file and a rename over `src/a.ts`
- **THEN** the JSON line for that check has 1 file parsed and 4 resolutions
- **TEST** `crates/archangel/tests/watch.rs` › "save_by_rename_counts_as_a_text_change"

#### Scenario: A change to .gitignore in watch mode

- **GIVEN** `archangel watch` that runs, and a source file `src/gen/a.ts` that breaks a rule
- **WHEN** the developer adds `src/gen/` to `.gitignore`
- **THEN** archangel prints the violation in `src/gen/a.ts` as gone
- **TEST** `crates/archangel/tests/watch.rs` › "gitignore_change_updates_the_source_files"

#### Scenario: A change to the config's ignore list in watch mode

- **GIVEN** `archangel watch` that runs, and a source file `fixtures/a.ts` with a violation
- **WHEN** the developer saves the config with `fixtures/` added to its ignore list
- **THEN** archangel prints the violation in `fixtures/a.ts` as gone
- **TEST** `crates/archangel/tests/watch.rs` › "config_ignore_change_updates_the_source_files"

#### Scenario: A change to the baseline in watch mode

- **GIVEN** `archangel watch` that shows three violations
- **WHEN** the developer runs `archangel baseline add`
- **THEN** archangel prints the three violations as gone
- **TEST** `crates/archangel/tests/watch.rs` › "watch_applies_a_new_baseline"

#### Scenario: Saves during a check

- **GIVEN** `archangel watch` that runs
- **WHEN** two saves of one file arrive while a check runs
- **THEN** the last line that archangel prints answers the second save
- **TEST** `crates/archangel/tests/watch.rs` › "last_watch_line_answers_the_last_save"

#### Scenario: A save with no change

- **GIVEN** `archangel watch` that shows one violation
- **WHEN** the developer saves a file with the same text
- **THEN** archangel prints a summary line with no new or gone diagnostics
- **TEST** `crates/archangel/tests/watch.rs` › "watch_prints_nothing_new_for_an_unchanged_save"

### Requirement: Watch mode never shows old results without a word

When the operating system reports that it lost file events, watch mode MUST make a full check from the start. When watch mode cannot watch the files, it MUST check for changes at a fixed interval instead, and every summary line MUST say so and name the cause.

#### Scenario: The operating system lost events

- **GIVEN** a build of archangel whose file watcher reports lost events after the first check, which the `engine::watcher_reports_lost_events` fail point causes
- **WHEN** `archangel watch --format json` runs on a repository with 30 source files
- **THEN** archangel makes a full check from the start, and its JSON line has 30 files parsed
- **TEST** `crates/archangel/tests/watch.rs` › "lost_events_cause_a_full_check"

#### Scenario: The system has no watches left

- **GIVEN** a build of archangel whose watcher cannot add a watch, as on a Linux system with a low watch limit, which the `engine::watcher_has_no_watches` fail point causes
- **WHEN** the developer runs `archangel watch`
- **THEN** archangel checks for changes at a fixed interval
- **AND** each summary line says that it polls, names the limit, and names the command that raises it
- **TEST** `crates/archangel/tests/watch.rs` › "watch_without_watches_polls_and_says_why"

### Requirement: CI reads diagnostics in the formats it already shows

`check` and `test` MUST print GitHub Actions annotations with `--format github`, each with its place and rule, and a SARIF 2.1.0 log with `--format sarif`, with each code as a rule and each violation's identity as a fingerprint. A command MUST refuse a format it cannot print, naming the formats it takes, with status 2.

#### Scenario: Annotations on a pull request

- **GIVEN** a repository with one layers violation
- **WHEN** CI runs `archangel check --format github`
- **THEN** archangel prints one `::error` line with the file, line, column, end column and rule, and the message, help and reason on its lines
- **AND** the summary follows, and archangel exits with status 1
- **TEST** `crates/archangel/tests/formats.rs` › "github_format_prints_one_annotation_per_diagnostic_then_the_summary"

#### Scenario: A rule name that would end a property

- **GIVEN** a rule named `a, b: 100%`
- **WHEN** CI runs `archangel check --format github`
- **THEN** the title escapes the comma, the colon and the percent sign
- **TEST** `crates/archangel/tests/formats.rs` › "github_format_escapes_what_would_end_a_property_or_the_message"

#### Scenario: A config that stops the check

- **GIVEN** a config with an unknown field
- **WHEN** CI runs `archangel check --format github`
- **THEN** the annotation names the config file, and archangel exits with status 2
- **TEST** `crates/archangel/tests/formats.rs` › "github_format_annotates_the_config_that_stops_a_command"

#### Scenario: A SARIF log for code scanning

- **GIVEN** a repository with one layers violation
- **WHEN** CI runs `archangel check --format sarif`
- **THEN** the log has every code as a rule, and one result with the code, its rule index, the region of the specifier, the rule's name and the violation's identity
- **AND** the log says its columns count Unicode characters
- **TEST** `crates/archangel/tests/formats.rs` › "sarif_format_is_a_sarif_log_with_every_code_as_a_rule"
- **TEST** `crates/archangel/tests/formats.rs` › "a_sarif_result_has_the_code_place_rule_and_identity_of_its_violation"

#### Scenario: A format a command cannot print

- **GIVEN** any repository
- **WHEN** the developer runs `archangel explain src/app/main.ts --format sarif`
- **THEN** archangel says `explain` prints text or json, and exits with status 2
- **TEST** `crates/archangel/tests/formats.rs` › "a_command_refuses_a_format_it_cannot_print"

### Requirement: Explain and why answer in JSON too

With `--format json`, `explain` MUST print the file, its element and instance, the rules that apply, each import with its resolution, element and broken rules, the lines it cannot follow, and its importers; `why` MUST print each step of the chain, or `null` when there is none.

#### Scenario: Explain as data

- **GIVEN** a file with one import that breaks a layers rule
- **WHEN** an agent runs `archangel explain <file> --format json`
- **THEN** the document holds the same answer as the text
- **TEST** `crates/archangel/tests/formats.rs` › "explain_as_json_gives_the_element_rules_imports_and_importers"

#### Scenario: Why as data

- **GIVEN** a chain of two imports, and a package nothing imports
- **WHEN** an agent runs `archangel why` for each with `--format json`
- **THEN** the first gives each step, and the second gives `null` and exits with status 1
- **TEST** `crates/archangel/tests/formats.rs` › "why_as_json_gives_the_chain_or_null"

### Requirement: The docs command explains a diagnostic code

`archangel docs <code>` MUST print the code's page. For a code that does not exist it MUST suggest the nearest code, list every code and exit with status 2. Text output with diagnostics MUST end by naming `archangel docs` for each code it printed.

#### Scenario: A code's page

- **WHEN** the developer runs `archangel docs entry`
- **THEN** archangel prints the page of `entry`
- **TEST** `crates/archangel/tests/formats.rs` › "docs_prints_a_codes_page"

#### Scenario: A misspelt code

- **WHEN** the developer runs `archangel docs lyers`
- **THEN** archangel suggests `layers`, lists every code, and exits with status 2
- **TEST** `crates/archangel/tests/formats.rs` › "docs_of_an_unknown_code_suggests_the_nearest_and_lists_every_code"

#### Scenario: The hint after a check

- **GIVEN** a check that prints a layers violation, and one that prints nothing
- **THEN** the first ends with ``run `archangel docs layers` ``, and the second has no hint
- **TEST** `crates/archangel/tests/formats.rs` › "text_output_ends_with_where_to_read_about_the_codes_it_printed"

### Requirement: The init command writes a config that passes from the start

`archangel init` MUST write `archangel.config.json`, or `archangel.config.ts` with `--typescript`, with one element per folder of source files (in `src` when it has them, honouring `.gitignore`) and a `noCycles` rule with an accepted and a rejected example, so that `check` and `test` pass on it. It MUST NOT write over an existing config, and MUST stop with status 2 when there is no folder of source files.

#### Scenario: Elements from the folders

- **GIVEN** `src/app`, `src/features` and `src/shared` with source files, `src/assets` with none, and an ignored `src/generated`
- **WHEN** the developer runs `archangel init`
- **THEN** the config has the elements `app`, `features` and `shared`, and archangel says so
- **TEST** `crates/archangel/tests/setup.rs` › "init_names_an_element_for_each_folder_of_source_files"

#### Scenario: A fresh config passes

- **GIVEN** a config that `archangel init` wrote, as JSON and as TypeScript
- **WHEN** the developer runs `archangel check` and `archangel test`
- **THEN** both exit with status 0
- **TEST** `crates/archangel/tests/setup.rs` › "a_fresh_config_checks_and_proves_itself"

#### Scenario: A config already exists

- **GIVEN** a repository with `archangel.config.json`
- **WHEN** the developer runs `archangel init --typescript`
- **THEN** archangel exits with status 2, changes nothing and writes no second config
- **TEST** `crates/archangel/tests/setup.rs` › "init_never_writes_over_a_config"

#### Scenario: No source folders

- **GIVEN** a repository with no folder of source files
- **WHEN** the developer runs `archangel init`
- **THEN** archangel says so and exits with status 2
- **TEST** `crates/archangel/tests/setup.rs` › "init_with_no_source_folders_says_so"

### Requirement: The command line fits the shell

`--color` MUST take `auto`, `always` and `never`. `archangel completions <shell>` MUST print the completion script for bash, zsh, fish, elvish and PowerShell, and `archangel man` the manual page in roff, which the release attaches.

#### Scenario: Color on demand

- **WHEN** the developer runs `archangel check --color always` into a pipe, and `--color never`
- **THEN** the first has escape codes and the second has none
- **TEST** `crates/archangel/tests/setup.rs` › "color_always_colors_a_pipe_and_never_colors_nothing"

#### Scenario: Completions

- **WHEN** the developer runs `archangel completions` for each shell
- **THEN** each script is that shell's and knows the commands
- **TEST** `crates/archangel/tests/setup.rs` › "completions_print_a_script_for_each_shell"

#### Scenario: The manual

- **WHEN** the release runs `archangel man`
- **THEN** it gets a roff page that names every command
- **TEST** `crates/archangel/tests/setup.rs` › "man_prints_a_roff_page_naming_every_command"

### Requirement: The stats option reports the work of a check

The `--stats` option of each command MUST print the work counts and the duration of each stage of the check: files found, files read, bytes read, files parsed, imports found, resolutions, files classified, pattern passes, rule evaluations, files visited by cycle searches and by transitive walks, structure evaluations, folder evaluations, export evaluations, examples checked and files written. The JSON output MUST contain the same work counts.

#### Scenario: Stats for a check

- **GIVEN** a repository with 12 source files
- **WHEN** the developer runs `archangel check --stats`
- **THEN** archangel prints the work counts, with 12 files read and 12 files parsed, and a duration for each stage
- **TEST** `crates/archangel/tests/cli.rs` › "stats_prints_work_counts_and_stage_durations"

#### Scenario: Work counts in the JSON output

- **GIVEN** a repository with 12 source files
- **WHEN** the developer runs `archangel check --format json`
- **THEN** the summary has 12 files read and 12 files parsed
- **TEST** `crates/archangel/tests/output.rs` › "json_summary_has_the_work_counts"

### Requirement: Benchmarks report time on fixed repositories

The benchmark job MUST check fixed versions of public repositories and a generated repository of one million lines. It MUST report the duration of each stage. It MUST NOT pass or fail because of a duration.

#### Scenario: The scheduled benchmark

- **GIVEN** the scheduled benchmark job
- **WHEN** it runs
- **THEN** it records the duration of each stage for each repository, and its result does not depend on the durations
- **TEST** none

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

### Requirement: The reach of a rule follows the limits of the rule

The reach MUST NOT count an import that a rule's `names`, `specifier`, `instance` or `dependency` leaves out, and that import MUST be listed as judged by no rule when no other rule judges it. An import between the files of a `noCycles` rule MUST count as judged. A table MUST print in one block, also when a config rule replaces one of its rows.

#### Scenario: A limit that skips an import

- **GIVEN** a table with `specifier: 'parent'`, and an import through an alias
- **WHEN** the developer runs `archangel rules`
- **THEN** the row counts 0 imports judged, and the import is in the list of imports that no rule judged
- **TEST** `crates/archangel/tests/rules_command.rs` › "reach_does_not_count_an_import_that_a_limit_skips"

#### Scenario: An import that a cycle rule follows

- **GIVEN** a `noCycles` rule on `src/**`, and an import between two of its files
- **WHEN** the developer runs `archangel rules`
- **THEN** the import is not in the list of imports that no rule judged
- **TEST** `crates/archangel/tests/rules_command.rs` › "reach_counts_an_import_that_no_cycles_follows"

#### Scenario: A preset's row that a config rule replaces

- **GIVEN** a preset table with three rows, and a config rule that replaces the middle row
- **WHEN** the developer runs `archangel rules`
- **THEN** the table has one block
- **TEST** `crates/archangel/tests/rules_command.rs` › "a_table_stays_in_one_block_when_a_config_rule_replaces_a_row"

### Requirement: A table without files says what its own rule covers

The sentence of `<table>.every-file-has-a-kind` MUST NOT say that every file has a row on a table without `files`. It MUST say that a file with no kind, or in no row, is not checked here, and the list of what no rule judges MUST hold that file.

#### Scenario: A file with no kind in a table without files

- **GIVEN** a table with the kind pattern `src/{kind}/**` and no `files`, and the file `src/index.ts`
- **WHEN** the developer runs `archangel rules`
- **THEN** the rule says "a file with a kind has a row; a file with no kind is not checked here", and the list of files that no rule judges has `src/index.ts`
- **TEST** `crates/archangel/tests/rules_command.rs` › "a_table_without_files_says_what_it_covers_and_lists_the_rest"

### Requirement: A check error uses the word of the config and says what to do

A rule written with `because` MUST print `because:` in text output, and a rule written with `reason` MUST print `reason:`. A `may-import` error MUST have one help line that says what to do. A rule's `files` that names nothing MUST NOT list an empty set of names or suggest a package under the new spellings.

#### Scenario: A rule with because

- **GIVEN** a rule written with `because`
- **WHEN** it reports a violation
- **THEN** the text output has `because:`
- **TEST** `crates/archangel/tests/cli.rs` › "a_rule_written_with_because_prints_because"

#### Scenario: A rule with reason

- **GIVEN** a rule written with `reason`
- **WHEN** it reports a violation
- **THEN** the text output has `reason:`
- **TEST** `crates/archangel/tests/cli.rs` › "a_rule_written_with_reason_prints_reason"

#### Scenario: A row error

- **GIVEN** the small-app fixture
- **WHEN** the developer runs `archangel check`
- **THEN** the error says to remove the import, or to move the code to a file whose row permits the import
- **TEST** `crates/archangel/tests/cli.rs` › "a_row_error_says_what_to_do"

#### Scenario: A rule that names a kind as its files

- **GIVEN** a config with no parts, a table with the kind `routes`, and a rule with `files: 'routes'`
- **WHEN** archangel loads the config
- **THEN** the message says that a kind has a meaning only in its table, and suggests no package
- **TEST** `crates/archangel/tests/cli.rs` › "an_unknown_name_in_a_config_with_no_names_does_not_list_nothing"

### Requirement: The rules command says which rules check nothing and where each rule comes from

A rule or row with no file MUST say that it checks nothing, and name the name that has no file. A table's `every-file-has-a-kind` rule MUST count the files of the table. A preset rule MUST name its preset. A config rule that replaces a preset rule MUST name the rule that it replaces. A rule made by a builder MUST name the file and line of the builder call, in whichever file the call is, a helper module included. A rule written as a plain object MUST name the config file only.

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

The config file MUST NOT be in these lists, because it is no part of the architecture.

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

#### Scenario: Any line of a statement after the specifier

- **GIVEN** a file with `import data from './b.json'` and `with { type: 'json' }` on the next line, and `import(`, `'./c'` and `)` on three lines
- **WHEN** the developer runs `archangel explain` on each of these lines
- **THEN** the output explains the import of that statement
- **TEST** `crates/archangel/tests/cli.rs` › "explain_finds_an_import_on_each_line_of_its_statement"

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

### Requirement: Explain shows a baseline match too

A refused import that the baseline matches MUST show that beside the rule.

#### Scenario: An import under the baseline

- **GIVEN** the small-app fixture after `archangel baseline add`
- **WHEN** the developer runs `archangel explain src/features/cart/views/cart-page.tsx:2`
- **THEN** the output shows `feature-code.views` as refusing the import, permitted by the baseline
- **TEST** `crates/archangel/tests/cli.rs` › "explain_of_one_import_shows_a_baseline_match_beside_the_rule"

### Requirement: The rules command counts folders for a rule about folders

A rule about folders MUST show the count of its folders, and no count of files.

#### Scenario: A rule about folders

- **GIVEN** a rule `{ folders: 'src/*', mustBeNamed: 'kebab-case' }` and the folder `src/one`
- **WHEN** the developer runs `archangel rules`
- **THEN** the rule shows 1 folder and 0 problems
- **TEST** `crates/archangel/tests/rules_command.rs` › "rules_counts_folders_for_a_rule_about_folders"

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
