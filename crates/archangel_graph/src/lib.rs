//! The import graph of a repository: which source files it has, what each one imports, and
//! where each import resolves.
//!
//! The graph is facts about the code alone. It does not know the config's elements or rules, so a
//! change to the config never means a second read of any file.
//!
//! This crate is part of [archangel](https://crates.io/crates/archangel) and is published so the
//! `archangel` command can be installed with Cargo. Its API follows the command's needs and may
//! change in any release; depend on the command, its JSON output or its language server instead.

mod files;
mod graph;
mod imports;
mod overlay;
mod resolve;
mod text;
mod work;

pub use files::{SOURCE_EXTENSIONS, is_source};
pub use graph::{
    GraphOptions, GraphProblem, GraphTimings, ImportGraph, Manifest, OverlayFiles,
    PARSE_STACK_BYTES, SourceFile,
};
pub use imports::{
    CommentText, Export, ExportNames, FileFacts, Import, ImportForm, ImportKind, OwnStatement,
    PARSER_LIMIT_BYTES, ParseError, ParseFailure, extract, extract_file,
};
pub use resolve::{Resolution, ResolutionInputs, UnresolvedReason, describe_resolve_error};
pub use work::Work;
