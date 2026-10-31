//! The parser limit's boundary, which only a unit test can reach: a file at the limit is 4 GiB,
//! too large to write in a test, so `tests/files.rs` tests a file above it.

use super::{PARSER_LIMIT_BYTES, too_large};

#[test]
fn a_file_at_the_parser_limit_is_parsed_and_one_byte_more_is_too_large() {
    let most = PARSER_LIMIT_BYTES;
    assert!(
        !too_large(most),
        "a file of {most} bytes, the most oxc's parser takes, must be parsed; when oxc changes its \
         limit, change the constant in imports.rs"
    );
    assert!(
        too_large(most + 1),
        "a file of {} bytes must be refused before the parse, which would fail on it",
        most + 1
    );
    assert!(
        !too_large(0),
        "an empty file of 0 bytes must be parsed, as every file under the limit is"
    );
}
