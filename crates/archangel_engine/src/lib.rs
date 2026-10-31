//! Archangel's engine: a [`Session`] loads the config, builds the import graph, checks it against
//! the rules, and runs the rules' examples. Every command and the language server start through it.
//!
//! This crate is part of [archangel](https://crates.io/crates/archangel) and is published so the
//! `archangel` command can be installed with Cargo. Its API follows the command's needs and may
//! change in any release; depend on the command, its JSON output or its language server instead.

mod apply;
mod baseline_file;
mod events;
mod fixes;
mod plugins;
mod session;
mod test_command;
pub mod view;
mod watcher;

pub use apply::{Applied, Change};
pub use baseline_file::BASELINE_NAME;
pub use events::PathEvent;
pub use plugins::PluginAnswer;
pub use session::{Session, Stop, Timings};
pub use test_command::{ExampleReport, ExampleRun};
pub use watcher::{Batch, POLL_INTERVAL, Watcher};

/// Loads and validates the config at `path`: JSON directly, and TypeScript or JavaScript through
/// Node, whose process is stopped before this returns.
///
/// # Errors
///
/// Returns every problem with the config, or the reason it did not load.
pub fn load_config(
    path: &std::path::Path,
) -> Result<archangel_config::Config, Vec<archangel_config::ConfigProblem>> {
    plugins::load(path)
        .map(|loaded| loaded.config)
        .map_err(|refused| refused.problems)
}
