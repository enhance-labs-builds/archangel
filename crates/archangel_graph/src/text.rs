//! Turning a source file's bytes into text, as the TypeScript compiler reads them.

/// Decodes UTF-8, or UTF-16 when the file starts with a UTF-16 byte order mark.
///
/// # Errors
///
/// Returns the reason in plain words when the bytes are not text in one of those encodings.
pub(crate) fn decode(bytes: &[u8]) -> Result<String, String> {
    match bytes {
        [0xFF, 0xFE, rest @ ..] => utf16(rest, u16::from_le_bytes),
        [0xFE, 0xFF, rest @ ..] => utf16(rest, u16::from_be_bytes),
        [0xEF, 0xBB, 0xBF, rest @ ..] => utf8(rest),
        _ => utf8(bytes),
    }
}

fn utf8(bytes: &[u8]) -> Result<String, String> {
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|error| {
            format!(
                "the file is not UTF-8 text: the byte at position {} cannot start a character, and \
             the file has no UTF-16 byte order mark",
                error.valid_up_to()
            )
        })
}

fn utf16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> Result<String, String> {
    if !bytes.len().is_multiple_of(2) {
        return Err("the file has a UTF-16 byte order mark but an odd number of bytes".to_owned());
    }
    let units = bytes.chunks_exact(2).map(|pair| unit([pair[0], pair[1]]));
    char::decode_utf16(units)
        .collect::<Result<String, _>>()
        .map_err(|_| "the file has a UTF-16 byte order mark but is not valid UTF-16".to_owned())
}
