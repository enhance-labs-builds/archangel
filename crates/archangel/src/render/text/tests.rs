//! A diagnostic whose span does not mark a part of its file's text, as when the file changed
//! after the check, is shown without its snippet rather than stopping the command.

use std::collections::BTreeMap;
use std::path::Path;

use archangel_rules::{Code, Diagnostic};

fn shown_over(text: &str, span: (u32, u32)) -> String {
    let mut diagnostic =
        Diagnostic::new(Code::MayNotImport, "a.ts may not import \"vitest\"").at("a.ts", 1, 19);
    diagnostic.span = Some(span);
    let own = BTreeMap::from([("a.ts".to_owned(), text.to_owned())]);
    super::diagnostics_over(Path::new("."), &own, &[diagnostic], false)
}

const WITHOUT_SNIPPET: &str =
    "error[may-not-import]: a.ts may not import \"vitest\"\n--> a.ts:1:19\n\n";

#[test]
fn a_span_beyond_the_end_of_its_text_is_shown_without_the_snippet() {
    assert_eq!(
        shown_over("export {}\n", (18, 26)),
        WITHOUT_SNIPPET,
        "the place, and no source line"
    );
}

#[test]
fn a_span_inside_a_character_is_shown_without_the_snippet() {
    assert_eq!(
        shown_over("import 'é';\n", (9, 10)),
        WITHOUT_SNIPPET,
        "byte 9 is inside the two bytes of é"
    );
}

#[test]
fn a_span_that_fits_its_text_is_shown_with_the_snippet() {
    assert_eq!(
        shown_over("import { x } from 'vitest';\n", (18, 26)),
        "error[may-not-import]: a.ts may not import \"vitest\"\n \
         --> a.ts:1:19\n  \
         |\n\
         1 | import { x } from 'vitest';\n  \
         |                   ^^^^^^^^\n\n",
        "the source line and a mark under the specifier"
    );
}
