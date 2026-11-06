//! The small parts of resolution that no import shows alone: a bare specifier's package, and the
//! match of one name of a template's pattern.

use super::{bare_package, wildcard};

/// The package that a type import falls back from: a bare specifier names one, and nothing else
/// does.
#[test]
fn a_bare_specifier_names_its_package() {
    let cases = [
        ("geo", Some("geo")),
        ("geo/point", Some("geo")),
        ("@acme/maps/tiles", Some("@acme/maps")),
        ("@acme", None),
        ("./geo", None),
        ("#internal", None),
        ("node:fs", None),
        ("@types/geo", None),
    ];
    for (specifier, package) in cases {
        assert_eq!(
            bare_package(specifier),
            package,
            "{specifier}: a path, a `#` import, a URL-like specifier, an @types package or a scope \
             alone names no package"
        );
    }
}

/// A template's `*` matches any text within one file or folder name, piece after piece.
#[test]
fn a_star_stands_for_any_text_in_one_name() {
    let cases = [
        ("*.json", "en.json", true),
        ("*.json", "en.ts", false),
        ("a*b*c", "aXbYc", true),
        ("*ab*b", "ab", false),
        ("a*b", "ac", false),
        ("page-*", "page-home", true),
        ("page", "page", true),
        ("page", "pages", false),
    ];
    for (pattern, name, matches) in cases {
        assert_eq!(
            wildcard(pattern, name),
            matches,
            "{pattern} against {name}: each piece is found after the one before it"
        );
    }
}
