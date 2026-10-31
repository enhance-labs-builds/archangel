//! The URI conversions on Windows, where a path has a drive letter and backslashes, which only a
//! Windows test run can show.

#[cfg(windows)]
use super::{uri, uri_path};

#[cfg(windows)]
#[test]
fn a_windows_path_and_its_file_uri_convert_both_ways() {
    let path = std::path::Path::new(r"C:\Users\dev\my repo\src\a.ts");
    let written = uri(path).map(|uri| uri.as_str().to_owned());
    assert_eq!(
        written.as_deref(),
        Some("file:///C:/Users/dev/my%20repo/src/a.ts"),
        "editors expect three slashes, a drive letter and forward slashes"
    );
    // The drive letter keeps the editor's case; `workspace_relative` canonicalizes it.
    let read = uri_path("file:///c%3A/Users/dev/my%20repo/src/a.ts");
    assert_eq!(
        read.map(|read| read.to_string_lossy().to_lowercase()),
        Some(r"c:\users\dev\my repo\src\a.ts".to_owned()),
        "VS Code sends the drive letter lower case with its colon escaped"
    );
}
