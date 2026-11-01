//! The archangel config: the elements of a repository and the rules between them.
//!
//! A config is data after it loads. [`load_json`] reads JSON directly; for a TypeScript or
//! JavaScript config the engine runs [`LOADER`] in Node, and [`from_loader`] validates its answer.
//! Either way the result is a [`Config`] in which every element name resolves, every pattern
//! compiles and every plugin rule's options fit their schema. Every problem comes back at once, so
//! the developer can fix a config in one pass. This crate starts no process.
//!
//! This crate is part of [archangel](https://crates.io/crates/archangel) and is published so the
//! `archangel` command can be installed with Cargo. Its API follows the command's needs and may
//! change in any release; depend on the command, its JSON output or its language server instead.

mod examples;
mod load;
mod locate;
mod matcher;
mod merge;
mod model;
mod names;
mod parse;
mod pattern;
mod plugins;
mod problem;
mod rules;
mod selectors;
mod tables;

pub use load::{
    CONFIG_NAMES, LOADER, discover, from_loader, is_json, load_json, load_stopped, missing,
    node_missing, node_version_problem,
};
pub use matcher::{Classification, ElementMatcher, PathMatch, package_matches};
pub use model::{
    AllowUnresolved, Condition, Config, DependencyKind, Element, ElementId, ElementKind,
    ExampleFiles, Examples, ExportKind, FileSet, Folders, ImportKinds, ImportOptions,
    InstanceMatch, KindPattern, KindPlace, NameRule, OwnFiles, OwnInstance, PatternId, Plugin,
    PluginCondition, PluginUse, Preset, Reads, Rule, RuleKind, RuleSource, Selector, SelectorId,
    Sentence, Side, SpecifierForm, Table, TableId, Term,
};
pub use names::{NamePart, NamePattern, Owner, Style, placeholders};
pub use parse::{from_value, nearest, parse_json};
pub use pattern::{Pattern, PatternError, PatternMatch, normalize_path};
pub use plugins::VIEW_SCHEMA_VERSION;
pub use problem::{ConfigProblem, ProblemCode};
pub use tables::EVERY_FILE_HAS_A_KIND;
