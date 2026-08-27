//! `DecimalValue`, and the promise its measurement makes.
//!
//! This one checks a property rather than only looking for a panic.
//! `rendered_len` exists so a budget can ask how large a value is without
//! building it, and the executor believes the answer: a decimal that reported
//! less than it renders would walk straight past the ceiling. The two are
//! written as separate arithmetic over the same parts, so they can drift, and
//! only a comparison notices.
//!
//! Guarded by size, because the point is to check the *arithmetic*, not to
//! spend the run building a ten-megabyte string a random exponent asked for.

#![no_main]

use ferrosift_model::DecimalValue;
use libfuzzer_sys::fuzz_target;

/// Above this the rendering is skipped and only the prediction is taken.
const RENDERABLE: u64 = 1 << 16;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    let value = DecimalValue::parse(text);
    let predicted = value.rendered_len();
    if predicted <= RENDERABLE {
        let rendered = value.to_fixed();
        assert_eq!(
            predicted,
            rendered.len() as u64,
            "rendered_len promised {predicted} for {text:?} and to_fixed produced {}",
            rendered.len()
        );
    }

    // Re-reading a rendering must land on the same value. `to_fixed` writes
    // every digit, so nothing is rounded away and the trip is exact.
    if predicted <= RENDERABLE && !value.is_not_a_number() {
        let again = DecimalValue::parse(&value.to_fixed());
        assert_eq!(
            again.to_fixed(),
            value.to_fixed(),
            "{text:?} did not survive a round trip"
        );
    }
});
