//! The conversions a value goes through when a later step reads it.
//!
//! These are the reference's dish translations, and the reason they need tests
//! of their own is that a recipe corpus only reaches them by accident. A
//! one-step fixture never converts anything, and a two-step one converts only
//! the pair those two steps happen to form -- so a conversion nobody chained
//! is a conversion nobody checked.

use ferrosift_model::{
    DecimalValue, NumberValue, StructuredValue, TextEncoding, TextValue, Value, ValueKind,
};

/// Text as bytes: one byte per code unit, until one will not fit.
#[test]
fn text_becomes_bytes_the_way_the_reference_encodes_it() {
    // The reference writes one byte per UTF-16 code unit and only falls back
    // to UTF-8 when a unit exceeds 255. So a Latin-1 character is one byte
    // there, not the two that UTF-8 would give it -- which is what this crate
    // produced until the conversion moved into the model.
    for (text, expected) in [
        ("abc", vec![0x61, 0x62, 0x63]),
        ("é", vec![0xe9]),
        ("ÿ", vec![0xff]),
        // One unit above 255 sends the *whole* string to UTF-8, not just that
        // character: the reference gives up on the byte-per-unit encoding
        // rather than mixing the two.
        ("aé€", "aé€".as_bytes().to_vec()),
        ("", Vec::new()),
    ] {
        let value = Value::Text(TextValue {
            text: String::from(text),
            encoding: TextEncoding::Utf8,
        });
        assert_eq!(
            value.into_dish_bytes(),
            Some(expected),
            "encoding {text:?} disagreed with the reference"
        );
    }
}

/// Bytes back to text, preferring UTF-8 and keeping byte values otherwise.
#[test]
fn bytes_become_text_without_refusing_invalid_utf8() {
    let value = Value::from_dish_bytes(ValueKind::Text, vec![0xff, 0xfe])
        .expect("bytes always read as text");
    let Value::Text(text) = value else {
        panic!("asked for text");
    };
    assert_eq!(text.text, "ÿþ", "invalid UTF-8 keeps its byte values");
}

/// Markup loses its tags on the way out, and that is the point of the kind.
#[test]
fn markup_arrives_stripped_at_the_next_step() {
    let markup = Value::Markup(String::from("<b>a &amp; b</b>"));
    let text = markup
        .reinterpret(ValueKind::Text)
        .expect("markup reads as text");
    assert_eq!(
        text,
        Value::Text(TextValue {
            text: String::from("a & b"),
            encoding: TextEncoding::Utf8,
        })
    );
}

/// Conversions compose through bytes, so pairs nobody wrote out still work.
#[test]
fn a_pair_nobody_wrote_a_rule_for_still_converts() {
    // Neither of these was an entry in the old table. They work because the
    // conversion goes through canonical bytes rather than through a list of
    // ordered pairs -- which is the whole reason for the rewrite.
    let number = Value::Number(NumberValue::new(1.5));
    assert_eq!(
        number.reinterpret(ValueKind::Decimal),
        Some(Value::Decimal(DecimalValue::parse("1.5"))),
        "a number should read as a decimal"
    );

    let decimal = Value::Decimal(DecimalValue::parse("2.25"));
    let Some(Value::Number(back)) = decimal.reinterpret(ValueKind::Number) else {
        panic!("a decimal should read as a number");
    };
    assert!(
        (back.get() - 2.25).abs() < f64::EPSILON,
        "the value survived the round trip"
    );
}

/// A decimal too large to render is still too large after a conversion.
#[test]
fn a_structure_reads_out_but_does_not_read_back() {
    // Asymmetric on purpose: rendering a structure is `JSON.stringify`, and
    // reading one back is `JSON.parse`, which is a parser this crate does not
    // have. The asymmetry is reported rather than discovered at run time.
    assert!(ValueKind::Structured.converts_to(ValueKind::Text));
    assert!(!ValueKind::Text.converts_to(ValueKind::Structured));

    let structure = Value::Structured(StructuredValue::Integer(7));
    assert_eq!(
        structure.reinterpret(ValueKind::Text),
        Some(Value::Text(TextValue {
            text: String::from("7"),
            encoding: TextEncoding::Utf8,
        }))
    );
}

/// What the reference has no dish for, this model refuses to convert.
#[test]
fn kinds_the_reference_does_not_have_do_not_convert() {
    // Giving these a byte form would invent a conversion the reference does
    // not define, and would accept a recipe it refuses.
    for kind in [ValueKind::Empty, ValueKind::Boolean, ValueKind::Files] {
        assert!(
            !kind.converts_to(ValueKind::Text),
            "{kind} should not convert to text"
        );
    }
    assert!(Value::Boolean(true).into_dish_bytes().is_none());
    assert!(Value::Empty.into_dish_bytes().is_none());
}

/// Every kind that claims to convert actually does.
#[test]
fn the_table_and_the_conversion_agree() {
    // `converts_to` is what preflight consults and `reinterpret` is what
    // execution performs. A recipe accepted by one and refused by the other
    // would fail after side effects rather than before them.
    let samples = [
        Value::Bytes(vec![0x31]),
        Value::Text(TextValue {
            text: String::from("1"),
            encoding: TextEncoding::Utf8,
        }),
        Value::Integer(1),
        Value::Number(NumberValue::new(1.0)),
        Value::Decimal(DecimalValue::parse("1")),
        Value::Markup(String::from("1")),
        Value::Structured(StructuredValue::Integer(1)),
        Value::Boolean(true),
        Value::Empty,
    ];
    for sample in samples {
        for target in ValueKind::ALL {
            let promised = sample.kind().converts_to(target);
            let performed = sample.clone().reinterpret(target).is_some();
            assert_eq!(
                promised,
                performed,
                "{} to {target}: preflight promised {promised} and execution did {performed}",
                sample.kind()
            );
        }
    }
}
