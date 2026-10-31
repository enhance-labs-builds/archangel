//! Any text after `archangel-allow` in any comment gives an exception or nothing, and never stops.
#![no_main]

libfuzzer_sys::fuzz_target!(|text: &str| {
    let source = format!("// archangel-allow {text}\nimport './a';\n/* archangel-allow {text} */\n");
    if let Ok(facts) = archangel_graph::extract_file("a.ts", &source) {
        drop(archangel_rules::read_exceptions(&facts.allow_comments));
    }
});
