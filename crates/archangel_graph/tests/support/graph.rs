//! Shared by the graph tests that list which files a graph holds.
#![cfg(test)]

use archangel_graph::ImportGraph;

/// The paths of the source files the graph holds, in its order.
pub(crate) fn sources(graph: &ImportGraph) -> Vec<&str> {
    graph
        .files()
        .iter()
        .map(|file| file.path.as_str())
        .collect()
}
