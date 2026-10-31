//! SARIF 2.1.0, the log format that GitHub code scanning and most CI dashboards read.

use std::path::Path;

use archangel_rules::{Code, Diagnostic};
use serde_json::{Value, json};

/// One SARIF log of one run, with every code as a rule so a result can point at its page.
pub(crate) fn log(dir: &Path, diagnostics: &[&Diagnostic]) -> String {
    let rules: Vec<Value> = Code::ALL
        .iter()
        .map(|code| {
            json!({
                "id": code.as_str(),
                "shortDescription": { "text": code.summary() },
                "help": { "text": code.docs(), "markdown": code.docs() },
                "defaultConfiguration": { "level": "error" },
            })
        })
        .collect();
    let results: Vec<Value> = diagnostics.iter().map(|d| result(dir, d)).collect();
    let root = url_of(dir);
    let log = json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": { "driver": {
                "name": "archangel",
                "version": env!("CARGO_PKG_VERSION"),
                "rules": rules,
            }},
            "originalUriBaseIds": { "SRCROOT": { "uri": root } },
            // archangel counts columns in characters; SARIF's default is UTF-16 code units.
            "columnKind": "unicodeCodePoints",
            "results": results,
        }],
    });
    let mut text = serde_json::to_string_pretty(&log).unwrap_or_default();
    text.push('\n');
    text
}

fn result(dir: &Path, diagnostic: &Diagnostic) -> Value {
    let mut message = diagnostic.message.clone();
    if let Some(help) = &diagnostic.help {
        message.push_str("\nhelp: ");
        message.push_str(help);
    }
    let mut result = json!({
        "ruleId": diagnostic.code.as_str(),
        "ruleIndex": diagnostic.code as usize,
        "level": "error",
        "message": { "text": message },
    });
    if let Some(file) = diagnostic.file.as_deref() {
        let mut location = json!({ "artifactLocation": { "uri": file, "uriBaseId": "SRCROOT" } });
        if diagnostic.line > 0 {
            let mut region =
                json!({ "startLine": diagnostic.line, "startColumn": diagnostic.column });
            if let Some(end) = super::end_column(dir, diagnostic) {
                region["endColumn"] = json!(end);
            }
            location["region"] = region;
        }
        result["locations"] = json!([{ "physicalLocation": location }]);
    }
    if let Some(id) = &diagnostic.id {
        result["partialFingerprints"] =
            json!({ "archangelViolation/v1": serde_json::to_string(id).unwrap_or_default() });
    }
    let mut properties = serde_json::Map::new();
    for (key, value) in [
        ("rule", &diagnostic.rule),
        ("reason", &diagnostic.reason),
        ("link", &diagnostic.link),
    ] {
        if let Some(value) = value {
            properties.insert(key.to_owned(), json!(value));
        }
    }
    if !properties.is_empty() {
        result["properties"] = Value::Object(properties);
    }
    result
}

/// The config folder, which is absolute, as a `file:` URL that ends in a slash, as SARIF's base URIs must.
fn url_of(dir: &Path) -> String {
    let mut path = dir.to_string_lossy().replace('\\', "/");
    if !path.starts_with('/') {
        path.insert(0, '/');
    }
    if !path.ends_with('/') {
        path.push('/');
    }
    let encoded: String = path
        .chars()
        .map(|c| match c {
            ' ' => "%20".to_owned(),
            '#' => "%23".to_owned(),
            '%' => "%25".to_owned(),
            '?' => "%3F".to_owned(),
            c => c.to_string(),
        })
        .collect();
    format!("file://{encoded}")
}
