//! The `archangel` command: architecture rules for TypeScript, checked in milliseconds.

mod commands;
mod paths;
mod render;

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

/// Architecture rules for TypeScript: layers, isolated features, entry files and no cycles.
#[derive(Debug, Parser)]
// clap names the program after the path it was started by, which is `archangel.exe` on Windows;
// help and errors say `archangel`, which is what a developer types on every platform.
#[command(name = "archangel", bin_name = "archangel", version, about)]
pub(crate) struct Cli {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    options: Options,
}

/// The options every command takes.
#[derive(Debug, Clone, clap::Args)]
pub(crate) struct Options {
    /// The config file to use, instead of the nearest one at or above the current folder.
    #[arg(long, global = true)]
    pub(crate) config: Option<PathBuf>,
    /// The output format.
    #[arg(long, global = true, value_enum, default_value_t = Format::Text)]
    pub(crate) format: Format,
    /// Print the work counts and the duration of each stage.
    #[arg(long, global = true)]
    pub(crate) stats: bool,
    /// When to color text output.
    #[arg(long, global = true, value_enum, default_value_t = Color::Auto)]
    pub(crate) color: Color,
}

/// When text output has color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum Color {
    /// On a terminal, unless `NO_COLOR` is set.
    Auto,
    /// Always, as for a CI log that shows color.
    Always,
    /// Never.
    Never,
}

/// How a command prints its result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum Format {
    /// For people, with the line of source code under each diagnostic.
    Text,
    /// One JSON document, for programs.
    Json,
    /// GitHub Actions annotations, which show on the lines of a pull request.
    Github,
    /// A SARIF 2.1.0 log, for GitHub code scanning and other CI dashboards.
    Sarif,
}

impl Format {
    /// The name the command line spells it with.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
            Self::Github => "github",
            Self::Sarif => "sarif",
        }
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Check the repository, and print every diagnostic, or only those of the given files.
    Check {
        /// Source files whose diagnostics to print; the whole repository is still checked.
        files: Vec<PathBuf>,
    },
    /// Check every example of every rule.
    Test,
    /// Show a file's part, the rules that apply to it, its imports and its importers; or, with
    /// `<file>:<line>`, every rule that judged or skipped the import on that line.
    Explain {
        /// A source file, or a source file and a line, as `src/a.ts:2`.
        file: PathBuf,
    },
    /// Show every rule as a sentence with what it covers, and what no rule judges.
    Rules {
        /// Show only the rules and the unjudged files and imports under this folder or file.
        path: Option<PathBuf>,
    },
    /// Show the shortest chain of imports from one file to another file or a package.
    Why {
        /// A source file.
        from: PathBuf,
        /// A repository file, or a package name.
        to: String,
    },
    /// Check, then check again after each change, printing what is new and what is gone.
    Watch,
    /// Serve the language server protocol on standard input and output, for editors.
    Lsp,
    /// Write a starter config, with one element for each folder of source files.
    Init {
        /// Write `archangel.config.ts`, with `defineConfig`, instead of `archangel.config.json`.
        #[arg(long)]
        typescript: bool,
    },
    /// Print what a diagnostic code means, with an example and what to do.
    Docs {
        /// A code, such as `layers` or `import-unresolved`.
        code: String,
    },
    /// Print a shell's completion script.
    Completions {
        /// The shell.
        shell: clap_complete::Shell,
    },
    /// Print the manual page, in roff.
    #[command(hide = true)]
    Man,
    /// Write or shrink `archangel.baseline.json`.
    Baseline {
        #[command(subcommand)]
        action: BaselineAction,
    },
}

#[derive(Debug, Subcommand)]
enum BaselineAction {
    /// Record every current violation.
    Add,
    /// Lower or remove the entries that the code no longer needs; never adds one.
    Prune,
}

/// How a command ended, which decides the exit status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// Nothing to report: status 0.
    Clean,
    /// A diagnostic, a failed example, or a negative answer: status 1.
    Found,
    /// The config, the baseline or the command line stopped the command: status 2.
    Stopped,
}

/// What a command prints, and how it ended.
pub(crate) struct Answer {
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) outcome: Outcome,
}

impl Answer {
    pub(crate) fn out(stdout: String, outcome: Outcome) -> Self {
        Self {
            stdout,
            stderr: String::new(),
            outcome,
        }
    }

    pub(crate) fn stopped(stderr: String) -> Self {
        Self {
            stdout: String::new(),
            stderr,
            outcome: Outcome::Stopped,
        }
    }
}

fn main() -> ExitCode {
    #[cfg(feature = "failpoints")]
    let _faults = fail::FailScenario::setup();
    let answer = match Cli::try_parse() {
        Ok(cli) => run(cli),
        Err(error) => commands::usage_error(&error),
    };
    let printed = std::io::stdout()
        .lock()
        .write_all(answer.stdout.as_bytes())
        .and_then(|()| std::io::stderr().lock().write_all(answer.stderr.as_bytes()));
    match (printed, answer.outcome) {
        (Err(_), _) | (_, Outcome::Stopped) => ExitCode::from(2),
        (Ok(()), Outcome::Found) => ExitCode::from(1),
        (Ok(()), Outcome::Clean) => ExitCode::SUCCESS,
    }
}

/// A command's name, and the formats it can print.
fn formats(command: &Command) -> (&'static str, &'static [Format]) {
    use Format::{Github, Json, Sarif, Text};
    match command {
        Command::Check { .. } => ("check", &[Text, Json, Github, Sarif]),
        Command::Test => ("test", &[Text, Json, Github, Sarif]),
        Command::Explain { .. } => ("explain", &[Text, Json]),
        Command::Rules { .. } => ("rules", &[Text, Json]),
        Command::Why { .. } => ("why", &[Text, Json]),
        Command::Watch => ("watch", &[Text, Json]),
        Command::Lsp => ("lsp", &[Text]),
        Command::Baseline { .. } => ("baseline", &[Text]),
        Command::Init { .. } => ("init", &[Text]),
        Command::Docs { .. } => ("docs", &[Text]),
        Command::Completions { .. } => ("completions", &[Text]),
        Command::Man => ("man", &[Text]),
    }
}

fn run(cli: Cli) -> Answer {
    let options = cli.options;
    let (name, takes) = formats(&cli.command);
    if let Some(refused) = commands::refuse_format(&options, name, takes) {
        return refused;
    }
    match cli.command {
        Command::Check { files } => commands::run_check(&options, &files),
        Command::Test => commands::run_test(&options),
        Command::Explain { file } => commands::run_explain(&options, &file),
        Command::Rules { path } => commands::run_rules(&options, path.as_deref()),
        Command::Why { from, to } => commands::run_why(&options, &from, &to),
        Command::Watch => commands::run_watch(&options),
        Command::Lsp => commands::run_lsp(),
        Command::Init { typescript } => commands::run_init(&options, typescript),
        Command::Docs { code } => commands::run_docs(&code),
        Command::Completions { shell } => commands::run_completions(shell),
        Command::Man => commands::run_man(),
        Command::Baseline { action } => {
            commands::run_baseline(&options, matches!(action, BaselineAction::Add))
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_command_line_is_well_formed() {
        <super::Cli as clap::CommandFactory>::command().debug_assert();
    }
}
