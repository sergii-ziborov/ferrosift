//! What COBS, Base62, and the object identifiers refuse.
//!
//! The corpus pins outputs, so it cannot pin an operation that declines to
//! produce one. Three of these refusals match the reference and two do not,
//! and the difference is the point of holding them together in one file: a
//! reader comparing `FerroSift` against `CyberChef` needs to see both lists, not
//! just the flattering one.
//!
//! The two divergences are the object identifiers' malformed-input paths,
//! where the reference returns a number derived from the letters of the word
//! `NaN`. `crates/ferrosift-operations/src/oid/mod.rs` argues why refusing is
//! the better answer, and
//! `docs/compatibility/cyberchef-v11.3.0.md` records it as a divergence rather
//! than leaving it to be discovered.

use ferrosift_core::{ExecutionBudget, Executor, NeverCancelled};
use ferrosift_model::{
    ArgumentValue, Arguments, CapabilitySet, OperationId, Recipe, RecipeMetadata, RecipeStep,
    StepId, TextEncoding, TextValue, Value,
};

/// Runs one operation over one input, returning the output bytes.
fn run(operation: &str, arguments: Arguments, input: Value) -> Result<Value, ()> {
    let registry = ferrosift_operations::default_registry().expect("registry");
    let recipe = Recipe::new(
        vec![RecipeStep {
            id: StepId::new("s").expect("step id"),
            operation: OperationId::new(operation).expect("operation id"),
            arguments,
            disabled: false,
            breakpoint: false,
        }],
        RecipeMetadata::default(),
    )
    .expect("recipe");

    Executor::new(&registry)
        .execute(
            &recipe,
            input,
            ExecutionBudget::generous(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .map_err(|_| ())
        .map(|outcome| outcome.value)
}

fn text(value: &str) -> Value {
    Value::Text(TextValue {
        text: value.to_owned(),
        encoding: TextEncoding::Utf8,
    })
}

fn alphabet(value: &str) -> Arguments {
    [("alphabet".to_owned(), ArgumentValue::Text(value.to_owned()))]
        .into_iter()
        .collect()
}

#[test]
fn cobs_decode_refuses_a_zero_byte() {
    // The reference's only rejection, and the right one: a zero byte in a COBS
    // frame is a frame boundary, so a payload containing one is two frames
    // being read as one.
    for frame in [vec![0, 1], vec![3, 1, 0, 2], vec![1, 1, 0]] {
        assert!(
            run(
                "encoding.cobs.decode@1",
                Arguments::new(),
                Value::Bytes(frame.clone())
            )
            .is_err(),
            "a payload containing 0x00 must be refused: {frame:?}"
        );
    }
}

#[test]
fn cobs_decode_accepts_a_truncated_frame() {
    // Not a refusal, and deliberately so. A block that promises four bytes and
    // supplies two returns the two, because the reference does. Tightening
    // this would refuse frames the reference accepts, which is the same kind
    // of silent disagreement as accepting ones it refuses.
    let decoded = run(
        "encoding.cobs.decode@1",
        Arguments::new(),
        Value::Bytes(vec![5, 1, 2]),
    )
    .expect("a truncated frame is accepted");
    assert_eq!(decoded, Value::Bytes(vec![1, 2]));
}

#[test]
fn base62_refuses_a_malformed_alphabet() {
    // The reference's bignum validates the alphabet when it is installed: at
    // least two characters, no duplicates, and none of `+`, `-`, `.` or
    // whitespace, because those collide with sign and decimal-point syntax.
    for expression in [
        "",
        "0",
        "0-9A-Za-zz",
        "0-9A-Za-z+",
        "0-9A-Za-z-",
        "0-9A-Za-z.",
    ] {
        assert!(
            run(
                "encoding.base62.encode@1",
                alphabet(expression),
                Value::Bytes(b"hi".to_vec())
            )
            .is_err(),
            "malformed alphabet must be refused: {expression:?}"
        );
    }
}

#[test]
fn base62_refuses_an_alphabet_shorter_than_the_base() {
    // A separate rejection from the one above, at a separate point: this
    // alphabet is well-formed and simply too short to name sixty-two digits.
    assert!(
        run(
            "encoding.base62.encode@1",
            alphabet("0-9A-Za-y"),
            Value::Bytes(b"hi".to_vec())
        )
        .is_err(),
        "61 characters cannot name 62 digits"
    );
    assert!(
        run("encoding.base62.decode@1", alphabet("01"), text("10")).is_err(),
        "a two-character alphabet cannot name 62 digits"
    );
}

#[test]
fn base62_checks_the_alphabet_only_when_there_is_input() {
    // Ordering, not tolerance. Both operations return early on empty input,
    // before the alphabet is looked at, so a malformed alphabet goes
    // unreported there. Validating first would be tidier and would disagree.
    assert_eq!(
        run(
            "encoding.base62.encode@1",
            alphabet("!"),
            Value::Bytes(Vec::new())
        ),
        Ok(text(""))
    );
    assert_eq!(
        run("encoding.base62.decode@1", alphabet("!"), text("")),
        Ok(Value::Bytes(Vec::new()))
    );
}

#[test]
fn base62_refuses_a_character_that_is_in_the_alphabet_but_not_a_digit() {
    // Only reachable with an alphabet longer than the base, where the filter
    // and the digit table disagree: `!` survives the filter because it is in
    // the alphabet, then fails as a digit. Skipping it would be the friendlier
    // reading and the wrong one.
    assert!(
        run(
            "encoding.base62.decode@1",
            alphabet("0-9A-Za-z!"),
            text("6x7!")
        )
        .is_err(),
        "a filter-surviving non-digit must be refused"
    );
    // The same alphabet with no such character still works.
    assert_eq!(
        run(
            "encoding.base62.decode@1",
            alphabet("0-9A-Za-z!"),
            text("6x7")
        ),
        Ok(Value::Bytes(b"hi".to_vec()))
    );
}

#[test]
fn base62_decode_drops_characters_outside_the_alphabet() {
    // Not a refusal: characters the alphabet does not contain are filtered
    // out, and an input left empty by that filter reads as zero.
    assert_eq!(
        run(
            "encoding.base62.decode@1",
            alphabet("0-9A-Za-z"),
            text("-!-")
        ),
        Ok(Value::Bytes(vec![0]))
    );
}

#[test]
fn object_identifier_refuses_a_malformed_string() {
    // These match the reference, which throws `malformed oid string`.
    for oid in ["1.2.3.abc", "1.2.-3", "1.2.3 ", " 1.2", "1.2\n", ""] {
        assert!(
            run("asn1.oid.encode@1", Arguments::new(), text(oid)).is_err(),
            "malformed identifier must be refused: {oid:?}"
        );
    }
}

/// A divergence, stated as one.
///
/// The reference answers these with a number derived from the letters of the
/// word `NaN`: `"1"` becomes the literal string `"NaN"`, and `"1..2"` becomes
/// `"NaN02"`. `FerroSift` refuses instead. Reproducing it would mean reproducing
/// a specific bignum's digit table, and the result would be misleading output
/// rather than compatibility.
#[test]
fn object_identifier_diverges_by_refusing_a_missing_first_pair() {
    for oid in ["1", "1.", ".", "..", ".1.2", "1..2"] {
        assert!(
            run("asn1.oid.encode@1", Arguments::new(), text(oid)).is_err(),
            "an identifier without two leading arcs must be refused: {oid:?}"
        );
    }
    // A *later* empty arc is a different case and is not refused: the
    // reference reads it as zero, and so does this.
    assert_eq!(
        run("asn1.oid.encode@1", Arguments::new(), text("1.2.")),
        Ok(text("2a00"))
    );
}

/// The same divergence in the other direction.
///
/// The reference answers `"2azz"` with `"1.2.95"`, where 95 comes from reading
/// the characters `N`, `a`, `N` as bignum digits.
#[test]
fn object_identifier_diverges_by_refusing_non_hexadecimal() {
    for hex in ["", "zz", "2azz", "2az0", "2a80zz00", "2a-1"] {
        assert!(
            run("asn1.oid.decode@1", Arguments::new(), text(hex)).is_err(),
            "non-hexadecimal input must be refused: {hex:?}"
        );
    }
    // A chunk that merely *starts* with a hex digit is not malformed: the
    // reference reads `0z` as zero, and the corpus pins that.
    assert_eq!(
        run("asn1.oid.decode@1", Arguments::new(), text("2a0z")),
        Ok(text("1.2.0"))
    );
}

/// The encoder's own bug, reproduced rather than fixed.
///
/// A first pair above 255 is written as plain hexadecimal with no base-128
/// continuation and no padding, so `2.999` produces three hex digits that no
/// ASN.1 decoder — including this crate's own — will read back. It is
/// reproduced because the operation exists to say what this reference emits,
/// and a port that silently produced correct DER here would disagree with
/// every certificate the reference wrote.
#[test]
fn object_identifier_reproduces_the_reference_first_pair_bug() {
    assert_eq!(
        run("asn1.oid.encode@1", Arguments::new(), text("2.999")),
        Ok(text("437"))
    );
    // And the inverse cannot undo it, which is the visible consequence: the
    // decoder reads `43` as a first byte and `7` as a whole second arc, so
    // `2.999` comes back as something else entirely.
    assert_eq!(
        run("asn1.oid.decode@1", Arguments::new(), text("437")),
        Ok(text("1.27.7"))
    );
}

/// The first pair is double arithmetic; every later arc is exact.
///
/// Two identifiers that differ only past the 53rd bit of the first arc encode
/// identically, because the reference reads that arc with `parseInt` and
/// multiplies with JavaScript's `*`. The corpus pins the bytes; this states
/// the property, which is the part a reader needs and the bytes do not say.
#[test]
fn object_identifier_rounds_the_first_pair_and_not_the_rest() {
    let encode = |oid: &str| run("asn1.oid.encode@1", Arguments::new(), text(oid));
    assert_eq!(
        encode("9007199254740993.1"),
        encode("9007199254740992.1"),
        "a first arc past 2^53 rounds before it is written"
    );
    // The same magnitude in a later arc keeps every digit.
    assert_ne!(
        encode("1.2.9007199254740993"),
        encode("1.2.9007199254740992"),
        "later arcs go through an exact big integer"
    );
}
