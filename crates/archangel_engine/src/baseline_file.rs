//! Reading and writing `archangel.baseline.json`, with stable sorting and formatting.

use std::io;
use std::path::Path;

use archangel_rules::{Baseline, Code, Diagnostic};

/// The baseline's file name, in the config folder.
pub const BASELINE_NAME: &str = "archangel.baseline.json";

/// The baseline in `config_dir`, `None` when there is none, or the `baseline-invalid` diagnostic
/// when it does not parse or cannot be read.
pub(crate) fn read_baseline(config_dir: &Path) -> Result<Option<Baseline>, Box<Diagnostic>> {
    let path = config_dir.join(BASELINE_NAME);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(Box::new(
                Diagnostic::new(
                    Code::BaselineInvalid,
                    format!("archangel cannot read the baseline: {error}"),
                )
                .at(BASELINE_NAME, 0, 0),
            ));
        }
    };
    Baseline::parse(&text)
        .map(Some)
        .map_err(|error| Box::new(baseline_invalid(&error)))
}

fn baseline_invalid(error: &archangel_rules::BaselineError) -> Diagnostic {
    {
        let line = u32::try_from(error.line).unwrap_or(u32::MAX);
        let column = u32::try_from(error.column).unwrap_or(u32::MAX);
        Diagnostic::new(
            Code::BaselineInvalid,
            format!(
                "the baseline is not valid: {}; fix the file, or delete it and run `archangel baseline add`",
                error.message
            ),
        )
        .at(BASELINE_NAME, line, column)
    }
}

/// Writes the baseline to `config_dir`, sorted, so two runs on the same code write the same bytes.
pub(crate) fn write_baseline(config_dir: &Path, baseline: &Baseline) -> io::Result<()> {
    std::fs::write(config_dir.join(BASELINE_NAME), baseline.to_json())
}
