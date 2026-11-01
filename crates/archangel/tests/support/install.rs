//! Shared by the tests of TypeScript configs: the package that a config imports, and the packages
//! that a fixture's manifest lists.
#![cfg(test)]

use archangel_testkit::FixtureRepo;

/// The package's own entry and the module it re-exports, standing in for the `archangel` that a
/// TypeScript config imports.
pub(crate) fn install_archangel(repo: &FixtureRepo) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    repo.write(
        "node_modules/@enhance-eng/archangel/package.json",
        r#"{ "name": "@enhance-eng/archangel", "type": "module", "main": "index.js" }"#,
    );
    let copied = [
        (
            "npm/archangel/index.js",
            "node_modules/@enhance-eng/archangel/index.js",
        ),
        (
            "npm/archangel/plugin.js",
            "node_modules/@enhance-eng/archangel/plugin.js",
        ),
        (
            "npm/archangel/presets/fsd.js",
            "node_modules/@enhance-eng/archangel/presets/fsd.js",
        ),
        (
            "npm/archangel/presets/fsd-conditions.js",
            "node_modules/@enhance-eng/archangel/presets/fsd-conditions.js",
        ),
        (
            "npm/archangel/presets/nextjs.js",
            "node_modules/@enhance-eng/archangel/presets/nextjs.js",
        ),
        (
            "node_modules/pluralize/package.json",
            "node_modules/pluralize/package.json",
        ),
        (
            "node_modules/pluralize/pluralize.js",
            "node_modules/pluralize/pluralize.js",
        ),
    ];
    for (from, to) in copied {
        let text =
            std::fs::read_to_string(root.join(from)).unwrap_or_else(|e| panic!("{from}: {e}"));
        repo.write(to, &text);
    }
}
