//! The decompressors, which turn a small input into a large one on purpose.
//!
//! Worth its own target for two reasons. The budget is what stops a bomb, so
//! every input here exercises that path rather than only the happy one — the
//! ceiling in `ferrosift_fuzz::budget` is deliberately low so a random input
//! reaches it often. And a malformed stream is the case a decompressor is least
//! likely to have been written for.

#![no_main]

use libfuzzer_sys::fuzz_target;

const OPERATIONS: &[&str] = &[
    "compression.gunzip@1",
    "compression.zlib.inflate@1",
    "compression.raw.inflate@1",
    "compression.bzip2.decompress@1",
    "compression.gzip@1",
    "compression.zlib.deflate@1",
    "compression.raw.deflate@1",
];

fuzz_target!(|data: &[u8]| {
    let Some((operation, rest)) = ferrosift_fuzz::select(OPERATIONS, data) else {
        return;
    };
    ferrosift_fuzz::run_bytes(operation, rest);
});
