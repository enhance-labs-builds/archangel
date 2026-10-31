//! Shared by the rules tests: check a fixture repository against a JSON config.
#![cfg(test)]

use archangel_graph::{GraphOptions, ImportGraph, Work};
use archangel_rules::{Architecture, Baseline, Report};
use archangel_testkit::FixtureRepo;

/// Checks the repository with the config, and an optional baseline, and returns the report with
/// the work it took.
pub(crate) fn check(
    config: &serde_json::Value,
    repo: &FixtureRepo,
    baseline: Option<&Baseline>,
) -> (Report, Work) {
    let config = archangel_config::parse_json(&config.to_string()).expect("a valid config");
    let options = GraphOptions {
        ignore: config.ignore.clone(),
    };
    let mut work = Work::default();
    let graph = ImportGraph::build(repo.path(), &options, &mut work);
    let architecture = Architecture::compile(config);
    let report = architecture.check(&graph, baseline, &mut work);
    (report, work)
}
