//! Framings that carry their own length or structure inside the bytes.
//!
//! A decoder whose next read is decided by a field it just read is the shape
//! most likely to index past the end: the length says one thing and the buffer
//! says another, and every one of these has at least one such field.

#![no_main]

use libfuzzer_sys::fuzz_target;

/// Framings read from bytes.
const BYTES: &[&str] = &[
    "encoding.cobs.decode@1",
    "encoding.cobs.encode@1",
    "parsing.tlv@1",
    "compression.lznt1.decompress@1",
];

/// Framings read from text.
const TEXT: &[&str] = &[
    "encoding.punycode.decode@1",
    "encoding.punycode.encode@1",
    "encoding.bech32.decode@1",
    "encoding.quoted_printable.decode@1",
    "encoding.quoted_printable.encode@1",
    "encoding.hexdump.decode@1",
    "encoding.base62.decode@1",
];

fuzz_target!(|data: &[u8]| {
    if let Some((operation, rest)) = ferrosift_fuzz::select(BYTES, data) {
        ferrosift_fuzz::run_bytes(operation, rest);
    }
    if let Some((operation, rest)) = ferrosift_fuzz::select(TEXT, data)
        && let Some(text) = ferrosift_fuzz::as_text(rest)
    {
        ferrosift_fuzz::run(operation, ferrosift_model::Arguments::new(), text);
    }
});
