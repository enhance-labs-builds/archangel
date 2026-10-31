//! Any text as a pattern gives a pattern or a reason, and a pattern matches any path without stopping.
#![no_main]

libfuzzer_sys::fuzz_target!(|input: (&str, &str)| {
    let (source, path) = input;
    if let Ok(pattern) = archangel_config::Pattern::parse(source) {
        drop(pattern.matches(path));
        drop(pattern.matches(&archangel_config::normalize_path(path)));
    }
});
