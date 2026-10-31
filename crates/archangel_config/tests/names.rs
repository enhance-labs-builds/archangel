//! Scenarios from the structure-rules spec: the name styles a naming rule takes, and the name
//! patterns, with their placeholders, that naming and export rules match names against.
#![cfg(test)]

use archangel_config::{NamePart, NamePattern, Owner, Style, placeholders};

#[test]
fn each_style_is_written_as_the_config_names_it() {
    let written: Vec<&str> = Style::ALL.iter().map(|style| style.as_str()).collect();
    assert_eq!(
        written,
        [
            "kebab-case",
            "camelCase",
            "PascalCase",
            "snake_case",
            "SCREAMING_SNAKE_CASE"
        ],
        "each style is its own name written in that style"
    );
    let read: Vec<Option<Style>> = written.iter().map(|name| Style::from_name(name)).collect();
    assert_eq!(
        read,
        Style::ALL.map(Some),
        "each name reads back as its style"
    );
    assert_eq!(
        Style::from_name("Kebab-Case"),
        None,
        "a name is matched exactly"
    );
    assert_eq!(
        Style::ScreamingSnake.to_string(),
        "SCREAMING_SNAKE_CASE",
        "a style displays as the config writes it"
    );
}

#[test]
fn a_name_pattern_takes_the_five_spellings_of_file_name_and_of_folder_name() {
    assert_eq!(
        placeholders(),
        [
            "{file-name}",
            "{fileName}",
            "{FileName}",
            "{file_name}",
            "{FILE_NAME}",
            "{folder-name}",
            "{folderName}",
            "{FolderName}",
            "{folder_name}",
            "{FOLDER_NAME}",
        ],
        "the name of the file and of its folder, each in every style"
    );
}

#[test]
fn a_name_pattern_reads_into_text_wildcards_and_placeholders() {
    let pattern = NamePattern::parse("use{FileName}*.test").expect("a valid pattern");
    assert_eq!(
        pattern.source(),
        "use{FileName}*.test",
        "the source as written"
    );
    assert_eq!(
        pattern.parts(),
        [
            NamePart::Text("use".into()),
            NamePart::Name(Owner::File, Style::Pascal),
            NamePart::Any,
            NamePart::Text(".test".into()),
        ],
        "text before and after the placeholder, and no empty text between two parts"
    );
    let folder = NamePattern::parse("{folder_name}").expect("a valid pattern");
    assert_eq!(
        folder.parts(),
        [NamePart::Name(Owner::Folder, Style::Snake)],
        "a placeholder alone is one part"
    );
    assert_eq!(
        (
            pattern.uses(Owner::File),
            pattern.uses(Owner::Folder),
            folder.uses(Owner::File),
            folder.uses(Owner::Folder),
        ),
        (true, false, false, true),
        "each pattern uses the names its placeholders stand for"
    );
}

#[test]
fn an_unknown_placeholder_is_refused_with_the_list_of_placeholders() {
    let message = NamePattern::parse("{Filename}.test").expect_err("not a placeholder");
    assert!(
        message.contains("`{Filename}`") && message.contains(&placeholders().join(", ")),
        "the message names the placeholder and lists every one: {message}"
    );
}
