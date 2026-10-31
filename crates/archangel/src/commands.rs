//! One function per command. Each opens a session, asks it one question, and hands the answer to a
//! renderer.

mod explain;
mod init;
mod watch;
mod why;

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use archangel_engine::{BASELINE_NAME, ExampleReport, Session, Stop};
use archangel_rules::{Diagnostic, ExampleKind};

use crate::render::{self, github, json, sarif, text};
use crate::{Answer, Format, Options, Outcome, paths};

pub(crate) use explain::run_explain;
pub(crate) use init::run_init;
pub(crate) use watch::run_watch;
pub(crate) use why::run_why;

/// Opens the session from the current folder, or the answer that stops the command.
pub(crate) fn open_session(options: &Options) -> Result<Session, Answer> {
    let start = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    Session::open(&start, options.config.as_deref())
        .map_err(|stop| stop_answer(options, &stop, &config_dir(options, &start)))
}

/// The folder the session takes its config from, which a stop's paths are relative to, or `start`
/// when there is no config.
fn config_dir(options: &Options, start: &Path) -> PathBuf {
    let config = match options.config.as_deref() {
        Some(path) => Some(start.join(path)),
        None => archangel_config::discover(start),
    };
    config
        .map(|path| dunce::canonicalize(&path).unwrap_or(path))
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| start.to_path_buf())
}

fn stop_answer(options: &Options, stop: &Stop, dir: &Path) -> Answer {
    Answer::stopped(match options.format {
        Format::Json => json::stopped_document(&stop.diagnostics),
        Format::Github => github::annotations(dir, &stop.diagnostics),
        Format::Sarif => sarif::log(dir, &stop.diagnostics.iter().collect::<Vec<_>>()),
        Format::Text => text::diagnostics(dir, &stop.diagnostics, false),
    })
}

/// The answer for a format the command cannot print, or `None` when it can.
pub(crate) fn refuse_format(options: &Options, command: &str, takes: &[Format]) -> Option<Answer> {
    if takes.contains(&options.format) {
        return None;
    }
    let names: Vec<&str> = takes.iter().map(|format| format.name()).collect();
    let list = match names.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} or {last}", rest.join(", ")),
        _ => names.join(""),
    };
    Some(Answer::stopped(format!(
        "error: `archangel {command}` prints --format {list}, not {}\n",
        options.format.name()
    )))
}

/// The answer for a path that is not a source file.
pub(crate) fn not_a_source_file(reason: &str) -> Answer {
    Answer::stopped(format!("error: {reason}\n"))
}

/// `archangel check`.
pub(crate) fn run_check(options: &Options, files: &[PathBuf]) -> Answer {
    let mut session = match open_session(options) {
        Ok(session) => session,
        Err(answer) => return answer,
    };
    let printed = plugin_output(&mut session);
    let mut chosen = Vec::new();
    for file in files {
        match paths::source(&session, file) {
            Ok(path) => chosen.push(path),
            Err(reason) => return not_a_source_file(&reason),
        }
    }
    let report = session.report();
    let shown: Vec<&Diagnostic> = report
        .diagnostics
        .iter()
        .filter(|d| chosen.is_empty() || concerns(d, &chosen))
        .collect();
    let outcome = if shown.is_empty() {
        Outcome::Clean
    } else {
        Outcome::Found
    };
    let timings = options.stats.then(|| session.timings());
    let dir = session.config_dir();
    let stdout = match options.format {
        Format::Json => json::check_document(&shown, &report.summary, session.work(), timings),
        Format::Sarif => sarif::log(dir, &shown),
        Format::Github => {
            let owned: Vec<Diagnostic> = shown.into_iter().cloned().collect();
            let mut out = github::annotations(dir, &owned);
            out.push_str(&text::summary(&report.summary, owned.len()));
            out
        }
        Format::Text => {
            let owned: Vec<Diagnostic> = shown.into_iter().cloned().collect();
            let mut out = text::diagnostics(dir, &owned, render::color(options.color));
            out.push_str(&text::summary(&report.summary, owned.len()));
            if options.stats {
                out.push_str(&text::stats(session.work(), timings));
            }
            out.push_str(&text::docs_hint(&owned));
            out
        }
    };
    Answer {
        stderr: printed,
        ..Answer::out(stdout, outcome)
    }
}

/// What the config and its plugins printed, each line with the name of the code that printed it,
/// which goes to standard error. The command is done with the plugin process, so it ends first.
fn plugin_output(session: &mut Session) -> String {
    session.close_plugins();
    session
        .take_plugin_output()
        .into_iter()
        .map(|line| line + "\n")
        .collect()
}

/// Whether a diagnostic is about one of the chosen files: its own file, or a cycle through it.
fn concerns(diagnostic: &Diagnostic, chosen: &[String]) -> bool {
    diagnostic.file.as_ref().is_some_and(|f| chosen.contains(f))
        || diagnostic.group.iter().any(|f| chosen.contains(f))
}

/// `archangel test`.
pub(crate) fn run_test(options: &Options) -> Answer {
    let mut session = match open_session(options) {
        Ok(session) => session,
        Err(answer) => return answer,
    };
    let report = session.run_examples();
    let printed = plugin_output(&mut session);
    let outcome = if report.passed() {
        Outcome::Clean
    } else {
        Outcome::Found
    };
    let stdout = match options.format {
        Format::Json => json::test_document(&report),
        Format::Github | Format::Sarif => {
            let mut failed: Vec<Diagnostic> = report
                .runs
                .iter()
                .filter_map(|run| run.outcome.failure.clone())
                .collect();
            failed.extend(report.unproven.iter().cloned());
            if options.format == Format::Sarif {
                sarif::log(session.config_dir(), &failed.iter().collect::<Vec<_>>())
            } else {
                let mut out = github::annotations(session.config_dir(), &failed);
                out.push_str(&examples_count(&report));
                out
            }
        }
        Format::Text => {
            let mut out = examples_text(&session, &report, options);
            if options.stats {
                out.push_str(&text::stats(&report.work, None));
            }
            out
        }
    };
    Answer {
        stderr: printed,
        ..Answer::out(stdout, outcome)
    }
}

fn examples_text(session: &Session, report: &ExampleReport, options: &Options) -> String {
    let mut out = String::new();
    for run in &report.runs {
        let verdict = if run.outcome.passed { "pass" } else { "fail" };
        let kind = match run.kind {
            ExampleKind::Accepted => "accepted",
            ExampleKind::Rejected => "rejected",
        };
        let _ = writeln!(
            out,
            "{verdict}  {}  {kind} #{}  {}",
            run.rule,
            run.number,
            run.files.join(", ")
        );
        if let Some(failure) = &run.outcome.failure {
            let _ = writeln!(out, "      {}", failure.message);
            let shown: Vec<Diagnostic> = run
                .outcome
                .diagnostics
                .iter()
                .filter(|d| d.file.as_ref().is_some_and(|f| run.files.contains(f)))
                .cloned()
                .collect();
            let dir = session.config_dir();
            out.push_str(&text::diagnostics_over(dir, &run.texts, &shown, false));
        }
    }
    if !report.unproven.is_empty() {
        out.push_str(&text::diagnostics(
            session.config_dir(),
            &report.unproven,
            render::color(options.color),
        ));
    }
    out.push_str(&examples_count(report));
    out
}

/// The closing line of `test`: how many examples ran and failed, and how many rules are unproven.
fn examples_count(report: &ExampleReport) -> String {
    let failed = report.runs.iter().filter(|run| !run.outcome.passed).count();
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{} examples, {}; {} rules without a rejected example",
        report.runs.len(),
        match failed {
            0 => "none failed".to_owned(),
            n => format!("{n} failed"),
        },
        report.unproven.len()
    );
    out
}

/// `archangel baseline add` and `archangel baseline prune`.
pub(crate) fn run_baseline(options: &Options, add: bool) -> Answer {
    let session = match open_session(options) {
        Ok(session) => session,
        Err(answer) => return answer,
    };
    let written = if add {
        session.baseline_add()
    } else {
        session.baseline_prune()
    };
    match written {
        Ok(baseline) => {
            let entries = match baseline.entries.len() {
                1 => "1 entry".to_owned(),
                n => format!("{n} entries"),
            };
            Answer::out(
                format!("wrote {entries} to {BASELINE_NAME}\n"),
                Outcome::Clean,
            )
        }
        Err(error) => Answer::stopped(format!(
            "error: archangel cannot write {BASELINE_NAME}: {error}\n"
        )),
    }
}

/// `archangel lsp`: the language server, on standard input and output.
pub(crate) fn run_lsp() -> Answer {
    let (connection, threads) = lsp_server::Connection::stdio();
    let served = archangel_lsp::serve(&connection);
    drop(connection);
    let joined = threads.join().map_err(|e| e.to_string());
    match served.and(joined) {
        Ok(()) => Answer::out(String::new(), Outcome::Clean),
        Err(reason) => Answer::stopped(format!("error: the language server stopped: {reason}\n")),
    }
}

/// `archangel docs <code>`: the code's page, or the nearest code and every code.
pub(crate) fn run_docs(name: &str) -> Answer {
    if let Some(code) = archangel_rules::Code::from_name(name) {
        return Answer::out(code.docs().to_owned(), Outcome::Clean);
    }
    let names = archangel_rules::Code::ALL.map(archangel_rules::Code::as_str);
    let nearest = names
        .iter()
        .map(|candidate| (strsim::damerau_levenshtein(name, candidate), candidate))
        .min()
        .filter(|(distance, _)| *distance <= 3)
        .map(|(_, candidate)| format!("; did you mean \"{candidate}\"?"))
        .unwrap_or_default();
    Answer::stopped(format!(
        "error: archangel has no code \"{name}\"{nearest}\nThe codes are: {}\n",
        names.join(", ")
    ))
}

/// `archangel completions <shell>`.
pub(crate) fn run_completions(shell: clap_complete::Shell) -> Answer {
    let mut command = <crate::Cli as clap::CommandFactory>::command();
    let mut script = Vec::new();
    clap_complete::generate(shell, &mut command, "archangel", &mut script);
    Answer::out(
        String::from_utf8_lossy(&script).into_owned(),
        Outcome::Clean,
    )
}

/// `archangel man`: the manual page, which the release packs beside the binary.
pub(crate) fn run_man() -> Answer {
    let command = <crate::Cli as clap::CommandFactory>::command();
    let mut page = Vec::new();
    match clap_mangen::Man::new(command).render(&mut page) {
        Ok(()) => Answer::out(String::from_utf8_lossy(&page).into_owned(), Outcome::Clean),
        Err(error) => Answer::stopped(format!(
            "error: archangel cannot write its manual: {error}\n"
        )),
    }
}

/// The answer for a command line that clap refused: help and the version print normally; an
/// unknown command lists every command, with clap's suggestion of the nearest.
pub(crate) fn usage_error(error: &clap::Error) -> Answer {
    use clap::error::ErrorKind;
    let rendered = error.render().to_string();
    match error.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => Answer::out(rendered, Outcome::Clean),
        ErrorKind::InvalidSubcommand => {
            let mut command = <crate::Cli as clap::CommandFactory>::command();
            let commands: Vec<String> = command
                .get_subcommands_mut()
                .map(|sub| {
                    format!(
                        "  {:<10}{}",
                        sub.get_name(),
                        sub.get_about().map(ToString::to_string).unwrap_or_default()
                    )
                })
                .collect();
            Answer::stopped(format!(
                "{rendered}\nThe commands are:\n{}\n",
                commands.join("\n")
            ))
        }
        _ => Answer::stopped(rendered),
    }
}
