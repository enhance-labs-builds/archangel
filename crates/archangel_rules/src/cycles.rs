//! Groups of files that import each other, found with petgraph's Kosaraju search, which walks
//! with an explicit stack so a long chain cannot overflow the thread's stack.

use std::collections::{HashSet, VecDeque};

use petgraph::Directed;
use petgraph::graph::{Graph, NodeIndex};

/// A graph of files, by index, and their imports.
#[derive(Debug, Clone)]
pub struct CycleGraph {
    graph: Graph<(), (), Directed, u32>,
}

impl CycleGraph {
    /// A graph with `count` files and no imports.
    #[must_use]
    pub fn with_files(count: usize) -> Self {
        let mut graph = Graph::with_capacity(count, count);
        for _ in 0..count {
            graph.add_node(());
        }
        Self { graph }
    }

    /// Records that file `from` imports file `to`.
    pub fn add_import(&mut self, from: usize, to: usize) {
        self.graph
            .update_edge(NodeIndex::<u32>::new(from), NodeIndex::<u32>::new(to), ());
    }

    /// Each group of files that can all reach each other, and each file that imports itself.
    #[must_use]
    pub fn groups(&self) -> Vec<Vec<usize>> {
        petgraph::algo::kosaraju_scc(&self.graph)
            .into_iter()
            .filter(|group| group.len() > 1 || self.graph.contains_edge(group[0], group[0]))
            .map(|group| group.into_iter().map(NodeIndex::index).collect())
            .collect()
    }

    /// The number of files in the graph.
    #[must_use]
    pub fn file_count(&self) -> usize {
        self.graph.node_count()
    }

    /// The shortest cycle from `start` back to itself, inside `group`. Neighbours are visited in
    /// index order, which is path order, so ties go to the cycle whose next file sorts first.
    #[must_use]
    pub fn shortest_cycle(&self, start: usize, group: &[usize]) -> Vec<usize> {
        let members: HashSet<usize> = group.iter().copied().collect();
        let mut parent = vec![None; self.graph.node_count()];
        let mut queue = VecDeque::from([start]);
        while let Some(current) = queue.pop_front() {
            let mut next: Vec<usize> = self
                .graph
                .neighbors(NodeIndex::<u32>::new(current))
                .map(NodeIndex::index)
                .collect();
            next.sort_unstable();
            for neighbour in next {
                if neighbour == start {
                    let mut path = vec![current];
                    while let Some(previous) = parent[*path.last().unwrap_or(&start)] {
                        path.push(previous);
                    }
                    path.reverse();
                    path.push(start);
                    return path;
                }
                if members.contains(&neighbour) && parent[neighbour].is_none() {
                    parent[neighbour] = Some(current);
                    queue.push_back(neighbour);
                }
            }
        }
        vec![start, start]
    }
}
