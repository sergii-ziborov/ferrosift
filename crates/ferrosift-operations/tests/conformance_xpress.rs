//! What the XPRESS decoders refuse.
//!
//! The corpus pins what these two produce, so it cannot pin what they decline
//! to produce — and for a decompressor that is most of the contract. Every
//! stream a forensic tool meets is one someone else wrote, so the interesting
//! question is not only "does a good stream decode" but "does a bad one stop
//! at the right place, saying the right thing".
//!
//! Each expectation below was checked against the pinned `CyberChef` 11.4.0
//! checkout before it was written down: the same bytes through the reference
//! produce the same refusal, and a control case that decodes to `ab` proves
//! the harness was not simply refusing everything.

#![allow(clippy::cast_possible_truncation)]

use ferrosift_core::{ExecutionBudget, Executor, NeverCancelled};
use ferrosift_model::{
    ArgumentValue, Arguments, CapabilitySet, OperationId, Recipe, RecipeMetadata, RecipeStep,
    StepId, Value,
};

fn run(operation: &str, arguments: Arguments, input: &[u8]) -> Result<Vec<u8>, String> {
    run_with(operation, arguments, input, ExecutionBudget::generous())
}

fn run_with(
    operation: &str,
    arguments: Arguments,
    input: &[u8],
    budget: ExecutionBudget,
) -> Result<Vec<u8>, String> {
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
            Value::Bytes(input.to_vec()),
            budget,
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .map_err(|error| error.code().to_owned())
        .and_then(|outcome| match outcome.value {
            Value::Bytes(bytes) => Ok(bytes),
            other => Err(format!("unexpected value kind {:?}", other.kind())),
        })
}

fn plain(input: &[u8]) -> Result<Vec<u8>, String> {
    run("compression.xpress.decompress@1", Arguments::new(), input)
}

fn huffman(input: &[u8], declared: i128) -> Result<Vec<u8>, String> {
    run(
        "compression.xpress.huffman.decompress@1",
        size_argument(declared),
        input,
    )
}

#[test]
fn plain_refuses_a_stream_that_stops_mid_item() {
    // A flag group is four bytes and there is no short form.
    assert_eq!(
        plain(&[]),
        Err(String::from("compression.xpress.truncated_flag_group"))
    );
    assert_eq!(
        plain(&[0, 0, 0]),
        Err(String::from("compression.xpress.truncated_flag_group"))
    );
    // All-clear flags with nothing behind them: the first item is a literal
    // that is not there. Contrast the next test, where the same emptiness
    // after a *set* flag is the ordinary end of the stream.
    assert_eq!(
        plain(&[0, 0, 0, 0]),
        Err(String::from("compression.xpress.truncated_literal"))
    );
    // One byte after a match flag is a match word missing its second half.
    assert_eq!(
        plain(&[0, 0, 0, 0x80, 0x41]),
        Err(String::from("compression.xpress.truncated_match"))
    );
}

#[test]
fn plain_reads_a_set_flag_with_nothing_behind_it_as_the_end() {
    // The final flag group is padded with set bits, so this is how every
    // well-formed stream finishes rather than an error case.
    assert_eq!(plain(&[0xff, 0xff, 0xff, 0xff]), Ok(Vec::new()));
    assert_eq!(plain(&[0x00, 0x00, 0x00, 0x7f, 0x41]), Ok(vec![0x41]));
}

#[test]
fn plain_refuses_a_truncated_length_extension() {
    // A match whose low three bits are seven needs an extension nibble.
    assert_eq!(
        plain(&[0, 0, 0, 0x80, 0x07, 0x00]),
        Err(String::from("compression.xpress.truncated_shared_nibble"))
    );
    // A nibble of fifteen needs a raw length after it, in one of three widths.
    assert_eq!(
        plain(&[0, 0, 0, 0x80, 0x07, 0x00, 0x0f]),
        Err(String::from("compression.xpress.truncated_raw_length"))
    );
    assert_eq!(
        plain(&[0, 0, 0, 0x80, 0x07, 0x00, 0x0f, 0xff]),
        Err(String::from("compression.xpress.truncated_raw_length"))
    );
    assert_eq!(
        plain(&[
            0, 0, 0, 0x80, 0x07, 0x00, 0x0f, 0xff, 0x00, 0x00, 0x01, 0x02
        ]),
        Err(String::from("compression.xpress.truncated_raw_length"))
    );
}

#[test]
fn plain_refuses_a_length_the_short_form_could_have_said() {
    // The escaped form starts at 22. A smaller value is a stream that encoded
    // one length two ways, which the reference treats as corruption rather
    // than as a redundant but harmless encoding.
    assert_eq!(
        plain(&[0, 0, 0, 0x80, 0x07, 0x00, 0x0f, 0xff, 0x05, 0x00]),
        Err(String::from("compression.xpress.invalid_match_length"))
    );
    assert_eq!(
        plain(&[0, 0, 0, 0x80, 0x07, 0x00, 0x0f, 0xff, 0x15, 0x00]),
        Err(String::from("compression.xpress.invalid_match_length"))
    );
}

#[test]
fn plain_refuses_a_match_pointing_before_the_output() {
    // One literal, then a match at distance two. Reading a byte that was never
    // produced is the failure a decompressor has to refuse rather than
    // improvise: zero, or the input, or wrapping would each be plausible and
    // each would be wrong.
    assert_eq!(
        plain(&[0, 0, 0, 0x40, 0x41, 0x08, 0x00]),
        Err(String::from("compression.xpress.offset_out_of_range"))
    );
    // The distance field is thirteen bits, so the reference's own ceiling of
    // 8192 cannot be passed by any encoding — 0xffff means exactly 8192. The
    // check is kept because the reference keeps it, not because a stream can
    // reach it.
    assert_eq!(
        plain(&[0, 0, 0, 0x40, 0x41, 0xf8, 0xff]),
        Err(String::from("compression.xpress.offset_out_of_range"))
    );
}

/// A nine-bit uniform Huffman table, where symbol `s` has code `s`.
///
/// Every one of the 512 symbols at length nine fills the 2^15-entry table
/// exactly, and canonical assignment in (length, symbol) order then makes the
/// code equal the symbol. That is what lets these fixtures be written as a
/// list of symbols rather than as a bit pattern nobody can check by eye.
fn uniform_stream(symbols: &[u16]) -> Vec<u8> {
    let mut stream = vec![0x99_u8; 256];
    let mut accumulator: u32 = 0;
    let mut held = 0_u32;
    for symbol in symbols {
        for bit in (0..9).rev() {
            accumulator = (accumulator << 1) | u32::from((symbol >> bit) & 1);
            held += 1;
            if held == 16 {
                stream.extend_from_slice(&(accumulator as u16).to_le_bytes());
                accumulator = 0;
                held = 0;
            }
        }
    }
    if held > 0 {
        accumulator <<= 16 - held;
        stream.extend_from_slice(&(accumulator as u16).to_le_bytes());
    }
    // The decoder preloads thirty-two bits before it decodes anything, so a
    // stream shorter than two words is truncated no matter what it says.
    while stream.len() < 256 + 8 {
        stream.push(0);
    }
    stream
}

#[test]
fn huffman_decodes_the_uniform_table_fixture() {
    // The control: without this the refusals below would pass for a decoder
    // that rejected everything.
    assert_eq!(uniform_stream(&[0x61, 0x62, 256]).len(), 264);
    assert_eq!(
        huffman(&uniform_stream(&[0x61, 0x62, 256]), 2),
        Ok(b"ab".to_vec())
    );
}

#[test]
fn huffman_refuses_a_size_outside_the_reference_range() {
    // The stream does not carry its own length, so the size is an argument —
    // and an argument is where a caller's mistake arrives. Zero and negative
    // are refused, and so is anything past the 32 MiB one call may produce.
    let stream = uniform_stream(&[0x61, 256]);
    for size in [0, -1, -4096, 32 * 1024 * 1024 + 1] {
        assert_eq!(
            huffman(&stream, size),
            Err(String::from("compression.xpress.invalid_decompressed_size")),
            "size {size} should be refused"
        );
    }
}

#[test]
fn huffman_refuses_a_table_it_cannot_build() {
    // The table is the first 256 bytes, whatever else the stream holds.
    assert_eq!(
        huffman(&vec![0x99; 255], 4),
        Err(String::from("compression.xpress.truncated_huffman_table"))
    );
    // All-zero lengths assign no codes at all, so the table is empty rather
    // than full.
    let mut empty = vec![0_u8; 256];
    empty.extend_from_slice(&[0; 8]);
    assert_eq!(
        huffman(&empty, 4),
        Err(String::from("compression.xpress.invalid_code_lengths"))
    );
    // Three one-bit codes ask for one and a half tables. Two would be exactly
    // right, which is why three is the smallest over-subscription.
    let mut crowded = vec![0_u8; 256];
    crowded[0] = 0x11;
    crowded[1] = 0x01;
    crowded.extend_from_slice(&[0; 8]);
    assert_eq!(
        huffman(&crowded, 4),
        Err(String::from("compression.xpress.invalid_code_lengths"))
    );
}

#[test]
fn huffman_refuses_a_bit_stream_that_runs_out() {
    let mut short = vec![0x99_u8; 256];
    short.extend_from_slice(&[0, 0]);
    assert_eq!(
        huffman(&short, 4),
        Err(String::from("compression.xpress.truncated_bit_stream"))
    );
}

#[test]
fn huffman_refuses_output_past_the_declared_size() {
    assert_eq!(
        huffman(&uniform_stream(&[0x61, 0x62, 256]), 1),
        Err(String::from(
            "compression.xpress.output_exceeds_declared_size"
        ))
    );
}

#[test]
fn huffman_reads_the_end_symbol_as_a_match_away_from_the_declared_length() {
    // Symbol 256 ends the stream only where the output is already the declared
    // length. Anywhere else it is an ordinary three-byte match at distance
    // one, which is a quirk of the format rather than an error — so `ab`
    // followed by the end symbol and then the real end is `abbbb`.
    assert_eq!(
        huffman(&uniform_stream(&[0x61, 0x62, 256, 256]), 5),
        Ok(b"abbbb".to_vec())
    );
    // With fewer than three bytes of room left it cannot be that match either.
    assert_eq!(
        huffman(&uniform_stream(&[0x61, 256, 256]), 2),
        Err(String::from("compression.xpress.corrupt_end_marker"))
    );
    // And with nothing produced yet there is nothing to point back at.
    assert_eq!(
        huffman(&uniform_stream(&[256, 256]), 5),
        Err(String::from("compression.xpress.corrupt_end_marker"))
    );
}

#[test]
fn huffman_refuses_a_size_the_budget_will_not_hold() {
    // The declared size is what the output grows to, so a caller asking for
    // more than the budget allows is refused before any of it is built rather
    // than after — which is the difference between a limit and a report.
    //
    // This is FerroSift's limit and not the reference's: the reference has
    // only its own 32 MiB ceiling, so a caller who sets a smaller one gets an
    // answer the reference has no equivalent of. The stream here decodes
    // cleanly under a budget that allows it, which is what makes the refusal
    // attributable to the budget rather than to the bytes.
    let stream = uniform_stream(&[0x61, 0x62, 256]);
    let tight = ExecutionBudget {
        max_output_bytes: 8,
        ..ExecutionBudget::generous()
    };
    assert_eq!(
        run_with(
            "compression.xpress.huffman.decompress@1",
            size_argument(4096),
            &stream,
            tight,
        ),
        Err(String::from("core.operation.output_limit_exceeded"))
    );
    assert_eq!(huffman(&stream, 2), Ok(b"ab".to_vec()));
}

fn size_argument(declared: i128) -> Arguments {
    [(
        "decompressed_size".to_owned(),
        ArgumentValue::Integer(declared),
    )]
    .into_iter()
    .collect()
}
