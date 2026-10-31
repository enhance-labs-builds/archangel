//! The durations of `check --format json --stats`, from fixed timings, since no test may time a
//! real check.

use std::time::Duration;

use archangel_engine::Timings;
use archangel_graph::{GraphTimings, Work};
use archangel_rules::Summary;
use serde_json::{Value, json};

/// Each duration of `--stats` is a number of milliseconds.
#[test]
fn durations_are_in_milliseconds() {
    let timings = Timings {
        config: Duration::from_millis(500),
        graph: Duration::from_secs(2),
        rules: Duration::from_millis(250),
        steps: GraphTimings {
            walk: Duration::from_millis(125),
            read_and_parse: Duration::from_millis(1500),
            resolve: Duration::from_millis(375),
        },
    };
    let document = super::check_document(&[], &Summary::default(), &Work::default(), Some(timings));
    let parsed: Value = serde_json::from_str(&document)
        .unwrap_or_else(|error| panic!("one JSON document: {error}\n{document}"));
    assert_eq!(
        parsed["summary"]["durations"],
        json!({
            "configMs": 500.0,
            "graphMs": 2000.0,
            "listFilesMs": 125.0,
            "readAndParseMs": 1500.0,
            "resolveMs": 375.0,
            "rulesMs": 250.0,
        }),
        "each stage and step, in milliseconds"
    );
}

/// Without `--stats` the document has no durations.
#[test]
fn durations_are_left_out_without_stats() {
    let document = super::check_document(&[], &Summary::default(), &Work::default(), None);
    let parsed: Value = serde_json::from_str(&document)
        .unwrap_or_else(|error| panic!("one JSON document: {error}\n{document}"));
    assert_eq!(
        parsed["summary"].get("durations"),
        None,
        "two checks of the same code print the same bytes"
    );
}
