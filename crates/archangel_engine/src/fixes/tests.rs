//! The specifiers that the entry fix tries toward an entry file in a sibling folder.

use super::toward_entries;

#[test]
fn a_specifier_toward_an_entry_keeps_its_leading_names() {
    assert_eq!(
        toward_entries(
            "lib/a",
            "src/lib/a.ts",
            &["src/lib/public/index.ts".to_owned()]
        ),
        ["lib/public"],
        "a bare alias keeps its first name, and the entry's index is dropped"
    );
}

#[test]
fn no_specifier_starts_with_only_dots_and_slashes() {
    assert_eq!(
        toward_entries(
            "./components/Button",
            "src/shared/ui/components/Button.tsx",
            &["src/shared/ui/public/index.ts".to_owned()]
        ),
        Vec::<String>::new(),
        "`.` names a folder that holds the importing file, so no candidate starts there"
    );
}
