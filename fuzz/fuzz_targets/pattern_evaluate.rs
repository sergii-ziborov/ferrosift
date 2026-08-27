//! Evaluating a pattern against bytes, which is where the offsets are.
//!
//! Parsing decides whether a pattern is well formed; evaluation decides what it
//! reads, and every read is an offset and a width computed from an expression
//! the source chose. A pattern is allowed to ask for anything, so refusing is
//! an ordinary answer — what is not allowed is reading a byte that is not
//! there, or looping without making progress.
//!
//! Split from the parser so a crash names which half it was in. A source and a
//! buffer both come out of the same input: the first byte says how much of it
//! is source and the rest is data, which lets the fuzzer move the boundary.

#![no_main]

use ferrosift_pattern::EvalOptions;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((&split, rest)) = data.split_first() else {
        return;
    };
    let cut = usize::from(split).min(rest.len());
    let (source, bytes) = rest.split_at(cut);
    let Ok(source) = core::str::from_utf8(source) else {
        return;
    };
    let Ok(pattern) = ferrosift_pattern::parse(source) else {
        return;
    };
    let _ = ferrosift_pattern::evaluate(&pattern, bytes, &EvalOptions::default());
});
