#![no_main]

use libfuzzer_sys::fuzz_target;

/// Fuzz target: the tokenizer must never panic on arbitrary input.
///
/// Any panic or crash in the tokenizer is a bug.
fuzz_target!(|data: &[u8]| {
    // Try to interpret the bytes as a string — if invalid UTF-8, that's fine
    if let Ok(input) = std::str::from_utf8(data) {
        // The tokenizer should never panic, even on garbage input
        let _ = a2x_sigma::lex(input);
    }
});
