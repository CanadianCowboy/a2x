#![no_main]

use libfuzzer_sys::fuzz_target;

/// Fuzz target: the parser must never panic on arbitrary token streams.
///
/// Generates valid tokens from raw bytes and feeds them to the parser.
/// Any panic in the parser is a bug.
fuzz_target!(|data: &[u8]| {
    // First, try to produce tokens from the raw bytes
    if let Ok(input) = std::str::from_utf8(data) {
        // Tokenize — this should never panic
        if let Ok(tokens) = a2x_sigma::lex(input) {
            // Parse — this should never panic
            let _ = a2x_sigma::parse(&tokens);
        }
    }
});
