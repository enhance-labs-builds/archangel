//! Shared by the tests that read violations: the diagnostics of rules.
#![cfg(test)]

use archangel_rules::{Diagnostic, Report};

/// The violations of rules, in report order.
pub(crate) fn rule_violations(report: &Report) -> Vec<&Diagnostic> {
    report
        .diagnostics
        .iter()
        .filter(|d| d.code.is_rule())
        .collect()
}
