//! Archangel's rules: putting files into elements and checking every import against every rule.
//!
//! This crate reads no files and prints nothing. It takes an import graph and a validated config,
//! and gives diagnostics as data, which the command line and the language server turn into their
//! own output.
//!
//! This crate is part of [archangel](https://crates.io/crates/archangel) and is published so the
//! `archangel` command can be installed with Cargo. Its API follows the command's needs and may
//! change in any release; depend on the command, its JSON output or its language server instead.

mod architecture;
mod baseline;
mod check;
mod classes;
mod cycles;
mod diagnostic;
mod examples;
mod exceptions;
mod exports;
mod facts;
mod judgement;
mod messages;
mod naming;
mod plugin;
mod reach;
mod structure;
mod transitive;
mod words;

pub use architecture::{Architecture, Target};
pub use baseline::{Baseline, BaselineEntry, BaselineError};
pub use check::{FileCheck, Report, Results, Summary};
pub use classes::{Classes, Lookup};
pub use cycles::CycleGraph;
pub use diagnostic::{Code, Diagnostic, Fix, ViolationId};
pub use examples::{ExampleKind, ExampleOutcome};
pub use exceptions::{Exception, read as read_exceptions};
pub use exports::{ExportedNames, Exports, exported_names, exports_name, star_importers};
pub use judgement::{ImportEnd, ImportJudgement, RuleJudgement, Verdict};
pub use naming::{FileName, convert, is_named};
pub use plugin::{PluginFindings, PluginPlace};
pub use reach::{Reach, RuleReach, UnjudgedImport};
pub use structure::{Kept, Structure, StructureChange};
pub use transitive::ReachGraph;
