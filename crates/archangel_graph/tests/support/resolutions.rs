//! Building a fixture's import graph and reading one import's resolution, for the graph tests.
#![cfg(test)]

use archangel_graph::{GraphOptions, ImportGraph, Resolution, Work};
use archangel_testkit::FixtureRepo;

pub(crate) fn build(repo: &FixtureRepo) -> ImportGraph {
    ImportGraph::build(repo.path(), &GraphOptions::default(), &mut Work::default())
}

pub(crate) fn resolution(graph: &ImportGraph, file: &str, specifier: &str) -> Resolution {
    let facts = graph
        .file(file)
        .and_then(|f| f.facts.as_ref())
        .expect("the file has facts");
    let index = facts
        .imports
        .iter()
        .position(|i| i.specifier == specifier)
        .expect("the import");
    graph.resolutions(file).expect("the file has resolutions")[index].clone()
}
