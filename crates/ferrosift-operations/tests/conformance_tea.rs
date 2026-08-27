//! What TEA and XTEA refuse, and the one scheme that has no answer to pin.
//!
//! The corpus holds what they produce: two ciphers, five modes, five padding
//! schemes, both directions, byte for byte. This holds what they do not — the
//! inputs where the reference throws and a corpus case cannot exist, plus the
//! one argument whose output the reference makes up freshly each time.

use ferrosift_model::{ArgumentValue, Arguments, Value};

mod support;

/// Sixteen bytes, which is the only key length either cipher takes.
const KEY: &str = "00112233445566778899aabbccddeeff";

/// Eight bytes, which is the only IV length either takes outside ECB.
const IV: &str = "0102030405060708";

fn toggle(name: &str, string: &str) -> (String, ArgumentValue) {
    (
        name.into(),
        ArgumentValue::Map(Arguments::from([
            ("option".into(), ArgumentValue::Text("Hex".into())),
            ("string".into(), ArgumentValue::Text(string.into())),
        ])),
    )
}

fn text(name: &str, value: &str) -> (String, ArgumentValue) {
    (name.into(), ArgumentValue::Text(value.into()))
}

struct Recipe<'a> {
    operation: &'a str,
    key: &'a str,
    iv: &'a str,
    mode: &'a str,
    padding: &'a str,
    cycles: Option<i128>,
}

impl Default for Recipe<'_> {
    fn default() -> Self {
        Self {
            operation: "crypto.tea.encrypt@1",
            key: KEY,
            iv: IV,
            mode: "CBC",
            padding: "PKCS5",
            cycles: None,
        }
    }
}

fn run(
    recipe: &Recipe<'_>,
    input: &str,
) -> Result<ferrosift_core::ExecutionResult, ferrosift_core::ExecutionError> {
    let mut arguments = Arguments::from([
        toggle("key", recipe.key),
        toggle("iv", recipe.iv),
        text("mode", recipe.mode),
        text("input", "Raw"),
        text("output", "Hex"),
        text("padding", recipe.padding),
    ]);
    if let Some(cycles) = recipe.cycles {
        arguments.insert("rounds".into(), ArgumentValue::Integer(cycles));
    }
    support::run_with_budget(
        recipe.operation,
        arguments,
        support::text(input),
        support::budget(),
    )
}

#[test]
fn the_key_is_exactly_sixteen_bytes() {
    assert!(run(&Recipe::default(), "message!").is_ok());
    for key in ["", "00", &KEY[..30], &format!("{KEY}00")] {
        let recipe = Recipe {
            key,
            ..Recipe::default()
        };
        assert!(
            run(&recipe, "message!").is_err(),
            "a key of {} hex digits must be refused",
            key.len()
        );
    }
}

/// The IV is eight bytes, or absent, or — in ECB — anything at all.
#[test]
fn the_iv_is_checked_only_where_it_is_used() {
    // Absent means eight null bytes rather than an error.
    let absent = Recipe {
        iv: "",
        ..Recipe::default()
    };
    assert!(run(&absent, "message!").is_ok());

    let wrong = Recipe {
        iv: "0102",
        ..Recipe::default()
    };
    assert!(run(&wrong, "message!").is_err(), "CBC needs a whole block");

    // ECB never reads it, and the reference does not check what it never reads.
    let ecb = Recipe {
        iv: "0102",
        mode: "ECB",
        ..Recipe::default()
    };
    assert!(
        run(&ecb, "message!").is_ok(),
        "ECB ignores the IV rather than refusing it"
    );
}

/// `NO` padding refuses a message that is not a whole number of blocks.
///
/// Except an empty one: the reference returns early for an empty message,
/// *before* it pads, so the scheme never gets the chance to complain.
#[test]
fn no_padding_refuses_a_partial_block() {
    let recipe = Recipe {
        padding: "NO",
        ..Recipe::default()
    };
    assert!(run(&recipe, "").is_ok(), "an empty message pads nothing");
    assert!(run(&recipe, "eightpad").is_ok());
    assert!(run(&recipe, "seven..").is_err());
}

/// `RANDOM` padding is refused exactly where the reference is unpredictable.
///
/// It fills the padding with `Math.random()`, so there is no output to be
/// byte-exact against. A message that is already a whole number of blocks adds
/// no padding at all and takes the same early return every other scheme does,
/// so it works here as it does there.
#[test]
fn random_padding_works_only_where_it_adds_nothing() {
    let recipe = Recipe {
        padding: "RANDOM",
        ..Recipe::default()
    };
    assert!(run(&recipe, "").is_ok());
    assert!(run(&recipe, "eightpad").is_ok());
    assert!(
        run(&recipe, "seven..").is_err(),
        "a padded RANDOM message has no reproducible output"
    );
}

/// `BIT` padding does not round-trip an aligned message, in either project.
///
/// The reference's `applyPadding` returns early for every scheme but PKCS5 when
/// the message already fills its blocks, so no `0x80` marker is written — and
/// the removal then scans back for one that is not there and throws. Reproduced
/// rather than fixed: a recipe that works there must work here, and one that
/// throws there must throw here.
#[test]
fn bit_padding_cannot_be_removed_from_an_aligned_message() {
    let encrypt = Recipe {
        padding: "BIT",
        mode: "ECB",
        ..Recipe::default()
    };
    let ciphertext = support::output_text(run(&encrypt, "eightpad").expect("encrypts"));

    let mut arguments = Arguments::from([
        toggle("key", KEY),
        toggle("iv", IV),
        text("mode", "ECB"),
        text("input", "Hex"),
        text("output", "Raw"),
        text("padding", "BIT"),
    ]);
    arguments.insert("input".into(), ArgumentValue::Text("Hex".into()));
    let back = support::run_with_budget(
        "crypto.tea.decrypt@1",
        arguments,
        support::text(&ciphertext),
        support::budget(),
    );
    assert!(
        back.is_err(),
        "the marker was never written, so there is nothing to strip"
    );
}

/// XTEA's cycle count is bounded by its own interface.
#[test]
fn xtea_accepts_only_the_cycle_counts_its_interface_offers() {
    for cycles in [1, 8, 32, 64, 255] {
        let recipe = Recipe {
            operation: "crypto.xtea.encrypt@1",
            cycles: Some(cycles),
            ..Recipe::default()
        };
        assert!(
            run(&recipe, "message!").is_ok(),
            "{cycles} cycles must work"
        );
    }
    for cycles in [0, -1, 256, 1_000] {
        let recipe = Recipe {
            operation: "crypto.xtea.encrypt@1",
            cycles: Some(cycles),
            ..Recipe::default()
        };
        assert!(
            run(&recipe, "message!").is_err(),
            "{cycles} cycles must be refused"
        );
    }
}

/// TEA and XTEA are different ciphers, at every cycle count.
///
/// Worth asserting because the two block functions are ten lines apart and a
/// port that pasted one over the other would still round-trip perfectly.
#[test]
fn the_two_ciphers_do_not_agree() {
    let tea = support::output_text(run(&Recipe::default(), "message!").expect("TEA"));
    let xtea = support::output_text(
        run(
            &Recipe {
                operation: "crypto.xtea.encrypt@1",
                cycles: Some(32),
                ..Recipe::default()
            },
            "message!",
        )
        .expect("XTEA"),
    );
    assert_ne!(tea, xtea);
}

/// The mode and padding names are the reference's, and nothing else is one.
#[test]
fn an_unknown_mode_or_padding_is_refused() {
    for mode in ["", "cbc", "GCM", "CBC/NoPadding"] {
        let recipe = Recipe {
            mode,
            ..Recipe::default()
        };
        assert!(run(&recipe, "message!").is_err(), "{mode} is not a mode");
    }
    for padding in ["", "pkcs5", "PKCS7", "NONE"] {
        let recipe = Recipe {
            padding,
            ..Recipe::default()
        };
        assert!(
            run(&recipe, "message!").is_err(),
            "{padding} is not a padding scheme"
        );
    }
}

/// An empty message is empty output, whatever else was asked for.
#[test]
fn an_empty_message_is_empty_output() {
    for mode in ["CBC", "CFB", "OFB", "CTR", "ECB"] {
        let recipe = Recipe {
            mode,
            ..Recipe::default()
        };
        let result = run(&recipe, "").expect("empty encrypts");
        assert_eq!(support::output_text(result), "", "{mode} on nothing");
    }
}

/// Bytes reaching the operation are the message, not text to be re-read.
#[test]
fn byte_input_is_the_message_itself() {
    let arguments = Arguments::from([
        toggle("key", KEY),
        toggle("iv", IV),
        text("mode", "ECB"),
        text("input", "Raw"),
        text("output", "Hex"),
        text("padding", "PKCS5"),
    ]);
    let from_bytes = support::run_with_budget(
        "crypto.tea.encrypt@1",
        arguments.clone(),
        Value::Bytes(b"eightpad".to_vec()),
        support::budget(),
    )
    .expect("bytes encrypt");
    let from_text = support::run_with_budget(
        "crypto.tea.encrypt@1",
        arguments,
        support::text("eightpad"),
        support::budget(),
    )
    .expect("text encrypts");
    assert_eq!(
        support::output_text(from_bytes),
        support::output_text(from_text)
    );
}
