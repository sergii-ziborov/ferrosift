//! Lexing and parsing a pattern, which is a language rather than a format.
//!
//! A hand-written recursive-descent parser over arbitrary text is the classic
//! place for an unbounded recursion and an index past the end. Nothing about
//! the result is asserted: a pattern that does not parse is an ordinary answer
//! and the error is the product.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(source) = core::str::from_utf8(data) else {
        return;
    };
    let _ = ferrosift_pattern::parse(source);
});
