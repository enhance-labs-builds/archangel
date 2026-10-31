//! Any text as a JSON config gives a config or problems, and never stops archangel otherwise.
#![no_main]

libfuzzer_sys::fuzz_target!(|text: &str| {
    drop(archangel_config::parse_json(text));
});
