//! JavaScript semantics, checked against Node rather than against intuition.
//!
//! The `CyberChef` corpus pins whole operations. This pins the language
//! underneath them, which is a narrower and more useful question: not "does
//! From Decimal agree" but "does our `parseInt` agree". A divergence at this
//! level surfaces as a handful of unrelated operations failing for reasons
//! nobody would connect, so it is worth catching where it starts.
//!
//! Regenerate the fixture with `cargo xtask jscompat generate`.

use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    node: String,
    parse_int: Vec<ParseIntCase>,
    whitespace: Vec<WhitespaceCase>,
    utf16: Vec<Utf16Case>,
    key_order: Vec<KeyOrderCase>,
    number_format: Vec<NumberFormatCase>,
}

#[derive(Deserialize)]
struct NumberFormatCase {
    /// The double being formatted, as its big-endian bit pattern.
    ///
    /// Bits rather than a decimal literal, so the fixture states exactly which
    /// double Node formatted with no parsing step in between that could itself
    /// be the thing that disagrees.
    bits: String,
    text: String,
}

#[derive(Deserialize)]
struct ParseIntCase {
    token: String,
    radix: u32,
    nan: bool,
    value: Option<String>,
}

#[derive(Deserialize)]
struct WhitespaceCase {
    code_point: u32,
    whitespace: bool,
}

#[derive(Deserialize)]
struct Utf16Case {
    code_units: Vec<u16>,
    well_formed: bool,
    char_count: Option<usize>,
}

#[derive(Deserialize)]
struct KeyOrderCase {
    inserted: Vec<String>,
    ordered: Vec<String>,
}

fn fixture() -> Fixture {
    let raw = include_str!("fixtures/jscompat.json");
    serde_json::from_str(raw).expect("jscompat fixture must parse")
}

#[test]
fn the_fixture_records_which_node_produced_it() {
    let fixture = fixture();
    assert!(
        fixture.node.starts_with('v'),
        "the fixture should name the Node that generated it, got {:?}",
        fixture.node
    );
}

#[test]
fn parse_int_matches_node() {
    let fixture = fixture();
    let mut mismatches = Vec::new();

    for case in &fixture.parse_int {
        // `parseInt` saturates in this port, because every caller only needs
        // to tell "not a number" from "a byte" from "too large". A case whose
        // true value is outside that range is therefore compared on the
        // classification rather than the digits.
        let parsed = ferrosift_operations::jscompat_testing::parse_int(&case.token, case.radix);
        let expected_nan = case.nan;
        let actual_nan = parsed.is_none();

        if expected_nan != actual_nan {
            mismatches.push(format!(
                "parseInt({:?}, {}) — node says {}, we say {}",
                case.token,
                case.radix,
                if expected_nan { "NaN" } else { "a number" },
                if actual_nan { "NaN" } else { "a number" },
            ));
            continue;
        }
        let (Some(actual), Some(expected)) = (parsed, case.value.as_deref()) else {
            continue;
        };
        let Ok(expected) = expected.parse::<i64>() else {
            // Past i64, which no caller in this crate reaches.
            continue;
        };
        // Only compare inside the saturation window; outside it the port
        // deliberately reports a bound rather than the digits.
        if expected.abs() <= 1_000_000 && actual != expected {
            mismatches.push(format!(
                "parseInt({:?}, {}) — node says {expected}, we say {actual}",
                case.token, case.radix,
            ));
        }
    }

    assert!(
        mismatches.is_empty(),
        "{} of {} parseInt cases disagree with Node:\n{}",
        mismatches.len(),
        fixture.parse_int.len(),
        mismatches.join("\n")
    );
}

#[test]
fn the_whitespace_definition_matches_node() {
    let fixture = fixture();
    let mut mismatches = Vec::new();

    for case in &fixture.whitespace {
        let Some(character) = char::from_u32(case.code_point) else {
            continue;
        };
        let actual = ferrosift_operations::jscompat_testing::is_js_whitespace(character);
        if actual != case.whitespace {
            mismatches.push(format!(
                "U+{:04X} — node says {}, we say {}",
                case.code_point, case.whitespace, actual
            ));
        }
    }

    assert!(
        mismatches.is_empty(),
        "{} of {} whitespace cases disagree with Node:\n{}\n\
         JavaScript's \\s is wider than Rust's char::is_whitespace — it \
         includes the byte-order mark and excludes some things Rust counts.",
        mismatches.len(),
        fixture.whitespace.len(),
        mismatches.join("\n")
    );
}

#[test]
fn the_utf16_view_matches_node() {
    let fixture = fixture();
    for case in &fixture.utf16 {
        let rebuilt = String::from_utf16(&case.code_units);

        assert_eq!(
            rebuilt.is_ok(),
            case.well_formed,
            "units {:?} — Node says well-formed={}, Rust {} hold them as a string",
            case.code_units,
            case.well_formed,
            if rebuilt.is_ok() { "can" } else { "cannot" }
        );

        let Ok(value) = rebuilt else {
            // A lone surrogate. JavaScript keeps it; a FerroSift text value
            // cannot, which is why operations that index by code unit
            // round-trip through `encode_utf16` rather than through `chars`.
            continue;
        };

        assert_eq!(
            value.encode_utf16().collect::<Vec<u16>>(),
            case.code_units,
            "re-encoding {value:?} does not give back the units Node saw"
        );
        assert_eq!(
            Some(value.chars().count()),
            case.char_count,
            "character count for {value:?} disagrees with Node"
        );
    }
}

#[test]
fn object_key_order_matches_node() {
    let fixture = fixture();
    let mut mismatches = Vec::new();

    for case in &fixture.key_order {
        let mut keys = ferrosift_operations::jscompat_testing::KeySet::default();
        for key in &case.inserted {
            keys.insert(key);
        }
        let actual = keys.into_keys();
        if actual != case.ordered {
            mismatches.push(format!(
                "inserting {:?} — node gives {:?}, we give {:?}",
                case.inserted, case.ordered, actual
            ));
        }
    }

    assert!(
        mismatches.is_empty(),
        "{} of {} key-order cases disagree with Node:\n{}\n\
         JavaScript objects hand back integer-like keys first in ascending \
         numeric order, then everything else in insertion order.",
        mismatches.len(),
        fixture.key_order.len(),
        mismatches.join("\n")
    );
}

#[test]
fn number_formatting_matches_node() {
    let fixture = fixture();
    let mut mismatches = Vec::new();

    for case in &fixture.number_format {
        let bits = u64::from_str_radix(&case.bits, 16).expect("fixture bits must be hexadecimal");
        let value = f64::from_bits(bits);
        let actual = ferrosift_operations::jscompat_testing::format_double(value);
        if actual != case.text {
            mismatches.push(format!(
                "0x{} — node gives {:?}, we give {:?}",
                case.bits, case.text, actual
            ));
        }
    }

    assert!(
        mismatches.is_empty(),
        "{} of {} number-format cases disagree with Node:\n{}\n\
         JavaScript takes the shortest round-tripping digits, then switches to \
         exponential notation above 1e21 and below 1e-6. Rust agrees on the \
         digits and on neither threshold.",
        mismatches.len(),
        fixture.number_format.len(),
        mismatches.join("\n")
    );
}

/// A guard against the fixture quietly emptying out.
#[test]
fn the_fixture_is_not_empty() {
    let fixture = fixture();
    let counts: BTreeMap<&str, usize> = BTreeMap::from([
        ("parse_int", fixture.parse_int.len()),
        ("whitespace", fixture.whitespace.len()),
        ("utf16", fixture.utf16.len()),
        ("key_order", fixture.key_order.len()),
        ("number_format", fixture.number_format.len()),
    ]);
    for (name, count) in counts {
        assert!(count > 0, "the {name} section of the fixture is empty");
    }
}
