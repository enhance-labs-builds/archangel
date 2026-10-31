//! `archangel watch`: a check, then a check again after each change, printing only what is new
//! and what is gone, and a summary line. With `--format json`, one JSON line per check.

use std::collections::{BTreeSet, HashSet};
use std::io::Write;

use archangel_engine::{Applied, Batch, Change, Session, Watcher};
use archangel_rules::Diagnostic;

use crate::render::{self, json, text};
use crate::{Answer, Format, Options};

pub(crate) fn run_watch(options: &Options) -> Answer {
    let start = std::env::current_dir().unwrap_or_default();
    let root = archangel_config::discover(&start)
        .and_then(|config| config.parent().map(std::path::Path::to_path_buf))
        .unwrap_or_else(|| start.clone());
    let mut watcher = match Watcher::start(&root) {
        Ok(watcher) => watcher,
        Err(reason) => return Answer::stopped(format!("error: {reason}\n")),
    };
    let mut session = match super::open_session(options) {
        Ok(session) => session,
        Err(answer) => return answer,
    };
    let mut screen = Screen {
        options,
        shown: Vec::new(),
        out: std::io::stdout().lock(),
    };
    let first = Applied {
        changed: BTreeSet::new(),
        work: *session.work(),
    };
    let mut printed = screen.print(&session, &first, watcher.polling());
    while printed.is_ok()
        && let Some(batch) = watcher.wait()
    {
        let applied = match batch {
            Batch::Events(events) => session.apply(vec![Change::Paths(events)]),
            Batch::Lost => session.reload(),
        };
        printed = screen.answer(&mut session, &applied, watcher.polling());
    }
    match printed {
        Ok(()) => Answer::stopped("error: the file watcher stopped\n".to_owned()),
        Err(error) => Answer::stopped(format!("error: archangel cannot print: {error}\n")),
    }
}

/// What watch mode has shown so far, how it prints, and where.
struct Screen<'o, W: Write> {
    options: &'o Options,
    shown: Vec<Diagnostic>,
    out: W,
}

impl<W: Write> Screen<'_, W> {
    /// Prints a check, and then, when the plugin process works on it, the check again once the
    /// process answers, so archangel's own diagnostics never wait for the plugin rules'.
    fn answer(
        &mut self,
        session: &mut Session,
        applied: &Applied,
        polling: Option<&str>,
    ) -> std::io::Result<()> {
        self.print(session, applied, polling)?;
        if session.plugins_running().is_some() {
            let answered = session.wait_for_plugins();
            self.print(session, &answered, polling)?;
        }
        Ok(())
    }

    /// Prints what is new and what is gone since the last check, and the summary line.
    fn print(
        &mut self,
        session: &Session,
        applied: &Applied,
        polling: Option<&str>,
    ) -> std::io::Result<()> {
        let report = session.report();
        let now: HashSet<&Diagnostic> = report.diagnostics.iter().collect();
        let before: HashSet<&Diagnostic> = self.shown.iter().collect();
        let new: Vec<Diagnostic> = report
            .diagnostics
            .iter()
            .filter(|d| !before.contains(d))
            .cloned()
            .collect();
        let gone: Vec<Diagnostic> = self
            .shown
            .iter()
            .filter(|d| !now.contains(d))
            .cloned()
            .collect();
        let out = match self.options.format {
            Format::Json => json::watch_line(&json::WatchLine {
                new: &new,
                gone: &gone,
                report,
                work: &applied.work,
                polling,
            }),
            // `refuse_format` keeps github and sarif from watch, so only text is left.
            Format::Text | Format::Github | Format::Sarif => {
                let mut out = text::diagnostics(
                    session.config_dir(),
                    &new,
                    render::color(self.options.color),
                );
                for diagnostic in &gone {
                    out.push_str(&text::gone(diagnostic));
                }
                let mut summary = text::summary(&report.summary, report.diagnostics.len());
                if let Some(note) = polling {
                    let first_line = summary.find('\n').unwrap_or(summary.len());
                    summary.insert_str(first_line, &format!("; {note}"));
                }
                out.push_str(&summary);
                out
            }
        };
        self.shown.clone_from(&report.diagnostics);
        self.out.write_all(out.as_bytes())?;
        self.out.flush()
    }
}

#[cfg(test)]
mod tests;
