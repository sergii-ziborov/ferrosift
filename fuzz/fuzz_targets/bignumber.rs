//! Arbitrary-precision arithmetic and base conversion.
//!
//! Two things are being looked for. The base reader and writer are a pair, so
//! a value written in some base and read back must be the value again — a
//! property that holds for every base or the pair is wrong for one of them.
//! And the arithmetic builds powers of ten from exponent differences, which is
//! where an input pair costs far more than it looks like it should.

#![no_main]

use ferrosift_model::DecimalValue;
use ferrosift_operations::jscompat_testing::bignumber;
use libfuzzer_sys::fuzz_target;

/// Every base the reference will read or write.
const BASES: &[u32] = &[
    2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 16, 20, 26, 32, 33, 34, 35, 36,
];

fuzz_target!(|data: &[u8]| {
    let Some((base, rest)) = ferrosift_fuzz::select(BASES, data) else {
        return;
    };
    let Ok(text) = core::str::from_utf8(rest) else {
        return;
    };

    // Reading, which is allowed to refuse.
    let Some(value) = bignumber::parse_in_base(text, base) else {
        return;
    };

    // Writing, and reading back. A value the reader accepted is one the writer
    // must be able to spell, and the spelling must read as the same value.
    if let Some(written) = bignumber::to_base(&value, base) {
        // Skip what would cost more to compare than it proves: a value with a
        // huge exponent spells out to megabytes, and the property is about the
        // conversion rather than about the size.
        if written.len() <= 4096
            && let Some(again) = bignumber::parse_in_base(&written, base)
        {
            assert_eq!(
                bignumber::to_base(&again, base),
                Some(written),
                "{text:?} in base {base} did not survive a round trip"
            );
        }
    }

    // Arithmetic against a small constant, which is where the exponent gap
    // between two operands decides the cost.
    let one = DecimalValue::from(1_i128);
    let _ = bignumber::plus(&value, &one);
    let _ = bignumber::minus(&value, &one);
    let _ = bignumber::times(&value, &one);
    let _ = bignumber::divide(&value, &one);
    let _ = bignumber::modulo(&value, &one);
    let _ = bignumber::square_root(&value);
    let _ = bignumber::negate(&value);
    let _ = bignumber::absolute(&value);
});
