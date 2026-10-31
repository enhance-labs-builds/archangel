# Spec Delta

## Purpose

The command line is how developers, CI and agents use archangel outside the editor. This capability
covers each command, what it prints, and its exit status.

## ADDED Requirements

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
