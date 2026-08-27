//! A toggleString field read every way the reference reads one.
//!
//! The field is a string and an option name, and the option decides which of
//! six decoders runs. All six are deliberately permissive — hex splits on
//! anything that is not a digit, base64 strips what is not in its alphabet,
//! binary chunks across the gaps — so all six are parsers over arbitrary text
//! with no error branch to fall back on.
//!
//! Both readings are covered, because they are two functions: `XOR` takes the
//! array reading and `HMAC` the string one, and the two part on Latin1 and on
//! any option name neither recognises.

#![no_main]

use libfuzzer_sys::fuzz_target;

/// Every option the reference's `switch` names, and one it does not.
const OPTIONS: &[&str] = &[
    "Hex", "Decimal", "Binary", "Base64", "UTF8", "Latin1", "Nonsense",
];

/// One operation per distinct consumer, not per operation that exists.
const CONSUMERS: &[(&str, &str)] = &[
    // The array reading, whose key is not coerced at all.
    ("logic.xor@1", "key"),
    ("logic.and@1", "key"),
    ("logic.add@1", "key"),
    ("logic.sub@1", "key"),
    // The array reading, stored into a typed array.
    ("crypto.xxtea.encrypt@1", "key"),
    // The string reading, masked by its consumer.
    ("hash.hmac@1", "key"),
    ("crypto.aes.encrypt@1", "key"),
];

fuzz_target!(|data: &[u8]| {
    let Some((option, rest)) = ferrosift_fuzz::select(OPTIONS, data) else {
        return;
    };
    let Some((consumer, rest)) = ferrosift_fuzz::select(CONSUMERS, rest) else {
        return;
    };
    // The field is text in the model, so an input that is not UTF-8 describes
    // no reachable argument.
    let Ok(field) = core::str::from_utf8(rest) else {
        return;
    };
    let (name, key) = (consumer.1, field);
    let arguments = ferrosift_model::Arguments::from([ferrosift_fuzz::toggle(name, option, key)]);
    ferrosift_fuzz::run(
        consumer.0,
        arguments,
        ferrosift_model::Value::Bytes(vec![0; 32]),
    );
});
