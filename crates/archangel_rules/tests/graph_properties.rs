//! The cycle search agrees with a slow reference: files are in one group exactly when each can
//! reach the other.
#![cfg(test)]

use archangel_rules::CycleGraph;
use proptest::prelude::*;

/// Groups by reachability: Floyd-Warshall on a boolean matrix, then mutual reachability.
fn reference(count: usize, edges: &[(usize, usize)]) -> Vec<Vec<usize>> {
    let mut reach = vec![vec![false; count]; count];
    for &(from, to) in edges {
        reach[from][to] = true;
    }
    for via in 0..count {
        let through = reach[via].clone();
        for row in &mut reach {
            if row[via] {
                for (cell, onward) in row.iter_mut().zip(&through) {
                    *cell |= *onward;
                }
            }
        }
    }
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for (file, row) in reach.iter().enumerate() {
        if groups.iter().any(|group| group.contains(&file)) {
            continue;
        }
        let group: Vec<usize> = (0..count)
            .filter(|&other| other == file || (row[other] && reach[other][file]))
            .collect();
        if group.len() > 1 || row[file] {
            groups.push(group);
        }
    }
    groups
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn cycle_groups_agree_with_the_reference_algorithm(
        count in 1usize..12,
        edges in prop::collection::vec((0usize..12, 0usize..12), 0..30),
    ) {
        let edges: Vec<(usize, usize)> = edges.into_iter().map(|(a, b)| (a % count, b % count)).collect();
        let mut graph = CycleGraph::with_files(count);
        for &(from, to) in &edges {
            graph.add_import(from, to);
        }
        let mut found = graph.groups();
        for group in &mut found {
            group.sort_unstable();
        }
        found.sort();
        let mut expected = reference(count, &edges);
        expected.sort();
        prop_assert_eq!(found, expected, "edges {:?}", edges);
    }
}
