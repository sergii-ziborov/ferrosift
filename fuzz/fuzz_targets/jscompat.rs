//! The JavaScript primitives every operation is built on.
//!
//! These are small functions and that is exactly why they are here: an
//! operation that fails on some input fails alone, and one of these that fails
//! on some input fails everywhere it is used. They are pinned against Node over
//! a fixture, which covers the inputs somebody thought of.

#![no_main]

use ferrosift_operations::jscompat_testing;
use libfuzzer_sys::fuzz_target;

/// Every radix `parseInt` is called with in this crate, plus the edges.
const RADICES: &[u32] = &[2, 8, 10, 16, 36];

fuzz_target!(|data: &[u8]| {
    let Some((radix, rest)) = ferrosift_fuzz::select(RADICES, data) else {
        return;
    };
    let Ok(text) = core::str::from_utf8(rest) else {
        return;
    };

    let _ = jscompat_testing::parse_int(text, radix);
    let value = jscompat_testing::parse_int_decimal(text);

    // The two coercions, over whatever `parseInt` produced -- which reaches
    // `NaN` and both infinities, the three values a narrowing cast gets wrong.
    let int32 = jscompat_testing::to_int32(value);
    let uint8 = jscompat_testing::to_uint8(value);
    // `ToUint8` is `ToInt32` read unsigned and cut to a byte, because 256
    // divides 2^32. Written as separate arithmetic, so it can drift.
    assert_eq!(
        uint8,
        (int32 as u32 & 0xff) as u8,
        "ToUint8 and ToInt32 disagree about {value}"
    );

    // Formatting, which has to terminate for every double including the ones
    // with the longest shortest-round-trip digit strings.
    let formatted = jscompat_testing::format_double(value);
    assert!(!formatted.is_empty(), "String({value}) produced nothing");

    for character in text.chars() {
        let _ = jscompat_testing::is_js_whitespace(character);
    }
});
