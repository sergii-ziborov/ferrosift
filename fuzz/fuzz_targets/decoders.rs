//! Every decoder that reads arbitrary text and claims to know what it means.
//!
//! These are the operations most exposed to input nobody chose: a decoder is
//! offered whatever the previous step produced, and several of them are
//! deliberately permissive about what they will accept. Permissive parsing over
//! arbitrary bytes is where a slicing panic lives.

#![no_main]

use libfuzzer_sys::fuzz_target;

/// Decoders taking their input as text.
const TEXT: &[&str] = &[
    "encoding.hex.decode@1",
    "encoding.base32.decode@1",
    "encoding.base45.decode@1",
    "encoding.base58.decode@1",
    "encoding.base64.decode@1",
    "encoding.base85.decode@1",
    "encoding.binary.decode@1",
    "encoding.decimal.decode@1",
    "encoding.octal.decode@1",
    "encoding.url.decode@1",
    "encoding.charcode.decode@1",
    "encoding.html.decode@1",
    "encoding.modhex.decode@1",
    "encoding.morse.decode@1",
];

/// Decoders taking their input as bytes.
const BYTES: &[&str] = &[
    "encoding.hex.encode@1",
    "encoding.base64.encode@1",
    "encoding.base85.encode@1",
    "encoding.hexdump.decode@1",
];

fuzz_target!(|data: &[u8]| {
    let Some((operation, rest)) = ferrosift_fuzz::select(TEXT, data) else {
        return;
    };
    if let Some(text) = ferrosift_fuzz::as_text(rest) {
        ferrosift_fuzz::run(operation, ferrosift_model::Arguments::new(), text);
    }

    let Some((operation, rest)) = ferrosift_fuzz::select(BYTES, data) else {
        return;
    };
    ferrosift_fuzz::run_bytes(operation, rest);
});
