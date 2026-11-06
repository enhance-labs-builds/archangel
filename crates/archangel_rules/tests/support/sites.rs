//! The violations of a report as (importing file, specifier), for the tests of import conditions.
#![cfg(test)]

use archangel_rules::Report;

use crate::violations::rule_violations;

/// The violations, as (importing file, specifier), in report order.
pub(crate) fn sites(report: &Report) -> Vec<(String, String)> {
    rule_violations(report)
        .into_iter()
        .map(|d| {
            (
                d.file.clone().unwrap_or_default(),
                d.specifier.clone().unwrap_or_default(),
            )
        })
        .collect()
}

pub(crate) fn site(file: &str, specifier: &str) -> (String, String) {
    (file.to_owned(), specifier.to_owned())
}
