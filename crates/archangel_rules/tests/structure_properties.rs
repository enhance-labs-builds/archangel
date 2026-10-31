//! Properties from the structure-rules spec: archangel's five name styles agree with a reference
//! implementation of case conversion, the `heck` crate.
#![cfg(test)]

use archangel_config::Style;
use archangel_rules::{convert, is_named};
use heck::{ToKebabCase, ToLowerCamelCase, ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use proptest::prelude::*;

/// The reference conversion of a name to a style.
fn reference(name: &str, style: Style) -> String {
    match style {
        Style::Kebab => name.to_kebab_case(),
        Style::Camel => name.to_lower_camel_case(),
        Style::Pascal => name.to_upper_camel_case(),
        Style::Snake => name.to_snake_case(),
        Style::ScreamingSnake => name.to_shouty_snake_case(),
    }
}

/// Names made of words in every case, with acronyms, digits and separators between them.
fn names() -> impl Strategy<Value = String> {
    let word = prop_oneof![
        "[a-z]{1,6}",
        "[A-Z][a-z]{0,5}",
        "[A-Z]{2,5}",
        "[0-9]{1,3}",
        "v[0-9]",
        Just("HTTPServer".to_owned()),
        Just("v2Api".to_owned()),
    ];
    let separator = prop_oneof![Just(""), Just("-"), Just("_"), Just("--")];
    prop::collection::vec((word, separator), 1..5).prop_map(|parts| {
        parts
            .into_iter()
            .flat_map(|(word, separator)| [word, separator.to_owned()])
            .collect()
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 2000, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn name_styles_agree_with_the_reference_conversion(name in names()) {
        for style in Style::ALL {
            let expected = reference(&name, style);
            prop_assert_eq!(convert(&name, style), expected.clone(), "{} in {:?}", name, style);
            prop_assert_eq!(
                is_named(&name, style),
                expected == name,
                "{} is in {:?} exactly when converting it changes nothing", name, style
            );
        }
    }
}
