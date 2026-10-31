//! Shared by the tests of empty elements and selectors: the messages of one code.
#![cfg(test)]

use archangel_rules::{Code, Report};

/// The messages of the diagnostics with one code, in report order.
pub(crate) fn messages(report: &Report, code: Code) -> Vec<String> {
    report
        .diagnostics
        .iter()
        .filter(|d| d.code == code)
        .map(|d| d.message.clone())
        .collect()
}
