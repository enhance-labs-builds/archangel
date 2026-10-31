//! Scenarios from the import-graph spec: a full check reads, parses and resolves each thing once,
//! and the work count would catch a second parse.
#![cfg(test)]

use archangel_graph::{GraphOptions, ImportGraph, Work};
use archangel_testkit::FixtureRepo;

fn forty_files() -> FixtureRepo {
    let mut repo = FixtureRepo::new();
    for index in 0..40 {
        let imports: String = (1..=3)
            .map(|step| (index + step) % 40)
            .fold(String::new(), |text, target| {
                text + &format!("import './f{target}';\n")
            });
        repo = repo.file(&format!("src/f{index}.ts"), &imports);
    }
    repo
}

/// (found, read, bytes read, parsed, imports found, resolutions) for a full check.
type Counts = (u64, u64, u64, u64, u64, u64);

/// The counts the scenario states, or what differs from them.
fn full_check_counts(repo: &FixtureRepo) -> Result<(), String> {
    let bytes: u64 = (0..40)
        .map(|index| repo.read(&format!("src/f{index}.ts")).len() as u64)
        .sum();
    let mut work = Work::default();
    let _graph = ImportGraph::build(repo.path(), &GraphOptions::default(), &mut work);
    let expected: Counts = (40, 40, bytes, 40, 120, 120);
    let found: Counts = (
        work.files_found,
        work.files_read,
        work.bytes_read,
        work.files_parsed,
        work.imports_found,
        work.resolutions,
    );
    if found == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, found {found:?}"))
    }
}

#[test]
fn full_check_reads_and_parses_each_file_once() {
    // Fail points are shared by every test in the process, so a work count is only true while
    // no fault-control test has one switched on.
    #[cfg(feature = "failpoints")]
    let _quiet = fail::FailScenario::setup();
    assert_eq!(
        full_check_counts(&forty_files()),
        Ok(()),
        "40 found, read and parsed, every byte read once, one resolution per import"
    );
}

#[cfg(feature = "failpoints")]
#[test]
fn work_count_rejects_a_second_parse() {
    let scenario = fail::FailScenario::setup();
    fail::cfg("graph::parse_twice", "return").unwrap();
    let result = full_check_counts(&forty_files());
    scenario.teardown();
    let error = result.expect_err("the work count catches a second parse of each file");
    assert!(
        error.ends_with(", 80, 120, 120)"),
        "the count shows two parses of each of the 40 files: {error}"
    );
}

/// A count with a different value in every field, so that a swapped field shows.
fn distinct(start: u64) -> Work {
    Work {
        files_found: start,
        files_read: start + 1,
        bytes_read: start + 2,
        files_parsed: start + 3,
        imports_found: start + 4,
        resolutions: start + 5,
        files_classified: start + 6,
        pattern_passes: start + 7,
        rule_evaluations: start + 8,
        cycle_files_visited: start + 9,
        transitive_files_visited: start + 10,
        structure_evaluations: start + 11,
        folder_evaluations: start + 12,
        export_evaluations: start + 13,
        examples_checked: start + 14,
        files_written: start + 15,
        plugin_records_sent: start + 16,
        plugin_checks: start + 17,
    }
}

#[test]
fn counts_name_every_counter_in_the_order_of_the_spec() {
    assert_eq!(
        distinct(10).counts(),
        [
            ("filesFound", 10),
            ("filesRead", 11),
            ("bytesRead", 12),
            ("filesParsed", 13),
            ("importsFound", 14),
            ("resolutions", 15),
            ("filesClassified", 16),
            ("patternPasses", 17),
            ("ruleEvaluations", 18),
            ("cycleFilesVisited", 19),
            ("transitiveFilesVisited", 20),
            ("structureEvaluations", 21),
            ("folderEvaluations", 22),
            ("exportEvaluations", 23),
            ("examplesChecked", 24),
            ("filesWritten", 25),
            ("pluginRecordsSent", 26),
            ("pluginChecks", 27),
        ],
        "each counter under its name in output"
    );
}

#[test]
fn counts_json_is_an_object_keyed_by_the_names_in_output() {
    assert_eq!(
        distinct(10).counts_json(),
        serde_json::json!({
            "filesFound": 10,
            "filesRead": 11,
            "bytesRead": 12,
            "filesParsed": 13,
            "importsFound": 14,
            "resolutions": 15,
            "filesClassified": 16,
            "patternPasses": 17,
            "ruleEvaluations": 18,
            "cycleFilesVisited": 19,
            "transitiveFilesVisited": 20,
            "structureEvaluations": 21,
            "folderEvaluations": 22,
            "exportEvaluations": 23,
            "examplesChecked": 24,
            "filesWritten": 25,
            "pluginRecordsSent": 26,
            "pluginChecks": 27,
        }),
        "one key per counter"
    );
}

#[test]
fn adding_counts_sums_each_counter_with_its_own() {
    let mut total = distinct(10);
    total.add(&distinct(3));
    assert_eq!(
        total,
        Work {
            files_found: 13,
            files_read: 15,
            bytes_read: 17,
            files_parsed: 19,
            imports_found: 21,
            resolutions: 23,
            files_classified: 25,
            pattern_passes: 27,
            rule_evaluations: 29,
            cycle_files_visited: 31,
            transitive_files_visited: 33,
            structure_evaluations: 35,
            folder_evaluations: 37,
            export_evaluations: 39,
            examples_checked: 41,
            files_written: 43,
            plugin_records_sent: 45,
            plugin_checks: 47,
        },
        "the counts of a parallel stage add up field by field"
    );
}
