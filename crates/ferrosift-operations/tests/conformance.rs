//! Portable operation conformance vectors.

use ferrosift_core::{Executor, NeverCancelled};
use ferrosift_model::{
    ArgumentValue, Arguments, CapabilitySet, OperationId, Recipe, RecipeMetadata, RecipeStep,
    StepId, Value,
};

mod support;

#[test]
fn identity_preserves_every_value_representation() {
    let input = Value::Structured(ferrosift_model::StructuredValue::List(vec![
        ferrosift_model::StructuredValue::Integer(7),
        ferrosift_model::StructuredValue::Text("ferro".into()),
    ]));
    let result = support::run("core.identity@1", Arguments::new(), input.clone());
    assert_eq!(result.value, input);
}

#[test]
fn hex_encoding_supports_delimiters_and_line_widths() {
    let input = Value::Bytes(vec![0x00, 0x0f, 0x10, 0xff]);
    assert_eq!(
        support::output_text(support::run(
            "encoding.hex.encode@1",
            Arguments::new(),
            input.clone()
        )),
        "00 0f 10 ff"
    );

    let arguments = Arguments::from([
        (
            "delimiter".into(),
            ArgumentValue::Text("0x with comma".into()),
        ),
        ("bytes_per_line".into(), ArgumentValue::Integer(0)),
    ]);
    assert_eq!(
        support::output_text(support::run(
            "encoding.hex.encode@1",
            arguments,
            input.clone()
        )),
        "0x00,0x0f,0x10,0xff"
    );

    let arguments = Arguments::from([
        ("delimiter".into(), ArgumentValue::Text("Colon".into())),
        ("bytes_per_line".into(), ArgumentValue::Integer(2)),
    ]);
    assert_eq!(
        support::output_text(support::run("encoding.hex.encode@1", arguments, input)),
        "00:0f:\n10:ff"
    );
}

#[test]
fn hex_decoding_supports_auto_and_explicit_formats() {
    assert_eq!(
        support::output_bytes(support::run(
            "encoding.hex.decode@1",
            Arguments::new(),
            support::text("0x00, 0f:10\nff")
        )),
        [0x00, 0x0f, 0x10, 0xff]
    );

    let arguments = support::argument("delimiter", ArgumentValue::Text("None".into()));
    assert_eq!(
        support::output_bytes(support::run(
            "encoding.hex.decode@1",
            arguments,
            support::text("000f10ff")
        )),
        [0x00, 0x0f, 0x10, 0xff]
    );
}

#[test]
fn every_hex_delimiter_has_a_stable_wire_format() {
    for (name, expected) in [
        ("Space", "00 ff"),
        ("Percent", "%00%ff"),
        ("Comma", "00,ff"),
        ("Semi-colon", "00;ff"),
        ("Colon", "00:ff"),
        ("Line feed", "00\nff"),
        ("CRLF", "00\r\nff"),
        ("0x", "0x000xff"),
        ("0x with comma", "0x00,0xff"),
        ("\\x", "\\x00\\xff"),
        ("None", "00ff"),
    ] {
        let arguments = support::argument("delimiter", ArgumentValue::Text(name.into()));
        let encoded = support::run(
            "encoding.hex.encode@1",
            arguments.clone(),
            Value::Bytes(vec![0x00, 0xff]),
        );
        assert_eq!(support::output_text(encoded), expected, "{name}");

        let decoded = support::run("encoding.hex.decode@1", arguments, support::text(expected));
        assert_eq!(support::output_bytes(decoded), [0x00, 0xff], "{name}");
    }
}

#[test]
fn base64_matches_rfc_4648_vectors() {
    for (plain, encoded) in [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ] {
        let encoded_result = support::run(
            "encoding.base64.encode@1",
            Arguments::new(),
            Value::Bytes(plain.as_bytes().to_vec()),
        );
        assert_eq!(support::output_text(encoded_result), encoded);

        let decoded_result = support::run(
            "encoding.base64.decode@1",
            Arguments::new(),
            support::text(encoded),
        );
        assert_eq!(support::output_bytes(decoded_result), plain.as_bytes());
    }
}

#[test]
fn base64_supports_unpadded_custom_alphabets() {
    let arguments = support::argument("alphabet", ArgumentValue::Text("A-Za-z0-9-_".into()));
    let result = support::run(
        "encoding.base64.encode@1",
        arguments.clone(),
        Value::Bytes(vec![0xfb, 0xff]),
    );
    assert_eq!(support::output_text(result), "-_8");

    let result = support::run("encoding.base64.decode@1", arguments, support::text("-_8"));
    assert_eq!(support::output_bytes(result), [0xfb, 0xff]);
}

#[test]
fn base64_round_trips_every_builtin_cyberchef_alphabet() {
    let input: Vec<_> = (0_u8..=u8::MAX).collect();
    for alphabet in [
        "A-Za-z0-9+/=",
        "A-Za-z0-9-_",
        "A-Za-z0-9+\\-=",
        "./0-9A-Za-z=",
        "A-Za-z0-9_.",
        "A-Za-z0-9._-",
        "0-9a-zA-Z+/=",
        "0-9A-Za-z+/=",
        " -_",
        "+\\-0-9A-Za-z",
        "!-,-0-689@A-NP-VX-Z[`a-fh-mp-r",
        "N-ZA-Mn-za-m0-9+/=",
        "./0-9A-Za-z",
        "/128GhIoPQROSTeUbADfgHijKLM+n0pFWXY456xyzB7=39VaqrstJklmNuZvwcdEC",
        "3GHIJKLMNOPQRSTUb=cdefghijklmnopWXYZ/12+406789VaqrstuvwxyzABCDEF5",
        "ZKj9n+yf0wDVX1s/5YbdxSo=ILaUpPBCHg8uvNO4klm6iJGhQ7eFrWczAMEq3RTt2",
        "HNO4klm6ij9n+J2hyf0gzA8uvwDEq3X1Q7ZKeFrWcVTts/MRGYbdxSo=ILaUpPBC5",
    ] {
        let arguments = support::argument("alphabet", ArgumentValue::Text(alphabet.into()));
        let encoded = support::output_text(support::run(
            "encoding.base64.encode@1",
            arguments.clone(),
            Value::Bytes(input.clone()),
        ));
        let decoded = support::output_bytes(support::run(
            "encoding.base64.decode@1",
            arguments,
            support::text(&encoded),
        ));
        assert_eq!(decoded, input, "{alphabet}");
    }
}

#[test]
fn codecs_round_trip_deterministic_lengths() {
    for length in 0_usize..=257 {
        let input: Vec<_> = (0..length)
            .map(|index| {
                u8::try_from((index * 73 + length * 19) & 0xff)
                    .expect("masked value fits in a byte")
            })
            .collect();
        for (encode, decode) in [
            ("encoding.hex.encode@1", "encoding.hex.decode@1"),
            ("encoding.base64.encode@1", "encoding.base64.decode@1"),
        ] {
            let encoded = support::run(encode, Arguments::new(), Value::Bytes(input.clone()));
            let decoded = support::run(decode, Arguments::new(), encoded.value);
            assert_eq!(
                decoded.value,
                Value::Bytes(input.clone()),
                "{encode}/{length}"
            );
        }
    }
}

#[test]
fn base64_can_remove_non_alphabet_characters() {
    let result = support::run(
        "encoding.base64.decode@1",
        Arguments::new(),
        support::text(" Zm9v\n"),
    );
    assert_eq!(support::output_bytes(result), b"foo");
}

#[test]
fn real_operations_compose_through_the_executor() {
    let registry = support::registry();
    let recipe = Recipe::new(
        vec![
            step("encode", "encoding.base64.encode@1"),
            step("decode", "encoding.base64.decode@1"),
            step("hex", "encoding.hex.encode@1"),
            step("unhex", "encoding.hex.decode@1"),
        ],
        RecipeMetadata::default(),
    )
    .expect("valid recipe");
    let input = Value::Bytes(b"FerroSift".to_vec());
    let result = Executor::new(&registry)
        .execute(
            &recipe,
            input.clone(),
            support::budget(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .expect("real operation recipe must execute");

    assert_eq!(result.value, input);
    assert_eq!(result.trace.events.len(), 8);
}

fn step(id: &str, operation: &str) -> RecipeStep {
    RecipeStep {
        id: StepId::new(id).expect("valid step ID"),
        operation: OperationId::new(operation).expect("valid operation ID"),
        arguments: Arguments::new(),
        disabled: false,
        breakpoint: false,
    }
}
