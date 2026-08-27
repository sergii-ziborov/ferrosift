//! Where a decompressor stops, and what it says when it does.
//!
//! Inflate is the one place in the catalog where a small input asks for a large
//! allocation, so the ceiling has to be handed to the decompressor rather than
//! applied to what it hands back: a hundred kilobytes of deflate can expand past
//! any budget worth setting, and checking afterwards means the allocation the
//! budget forbids has already happened.
//!
//! Bounding it moves the size refusal into `miniz`'s own error path, which is
//! also where a truncated stream comes out, so the tests here are mostly about
//! keeping those two apart. A bomb has to stay `output_limit_exceeded` and a
//! broken stream has to stay its own code, or the ceiling has been bought by
//! making every malformed input look like a bomb.
//!
//! The exact-fit cases are here for the other direction. `miniz` stops as soon
//! as its buffer reaches the limit, and an output that is *exactly* the ceiling
//! reaches it too — so the boundary is checked in both directions rather than
//! assumed to fall the friendly way.

#![cfg(feature = "compression")]

use ferrosift_core::ExecutionBudget;
use ferrosift_model::{Arguments, Value};

mod support;

/// What the bomb expands to: far enough above the ceiling to be unambiguous,
/// small enough that building it costs nothing.
const BOMB_PLAIN_BYTES: usize = 4 * 1024 * 1024;

/// What the recipe running the bomb is allowed to produce.
const CEILING: u64 = 64 * 1024;

/// A compressed form and the two operations that speak it.
struct Codec {
    compress: &'static str,
    decompress: &'static str,
    /// The code the decompressor reports for a stream it cannot read, which is
    /// the one thing a size refusal must never be relabelled as.
    invalid: &'static str,
}

const CODECS: [Codec; 3] = [
    Codec {
        compress: "compression.gzip@1",
        decompress: "compression.gunzip@1",
        invalid: "compression.gzip.invalid",
    },
    Codec {
        compress: "compression.zlib.deflate@1",
        decompress: "compression.zlib.inflate@1",
        invalid: "compression.zlib.invalid",
    },
    Codec {
        compress: "compression.raw.deflate@1",
        decompress: "compression.raw.inflate@1",
        invalid: "compression.raw.invalid",
    },
];

#[test]
fn a_bomb_is_refused_as_oversized_rather_than_as_malformed() {
    for codec in &CODECS {
        let bomb = compressed(codec, BOMB_PLAIN_BYTES);
        assert!(
            u64::try_from(bomb.len()).expect("length fits") < CEILING,
            "{}: the bomb must be smaller than the ceiling it defeats",
            codec.decompress
        );

        let error = support::run_with_budget(
            codec.decompress,
            Arguments::new(),
            Value::Bytes(bomb),
            budget(CEILING),
        )
        .expect_err("a stream expanding past the ceiling must fail");
        assert_eq!(
            error.code(),
            "core.operation.output_limit_exceeded",
            "{}",
            codec.decompress
        );
    }
}

#[test]
fn a_truncated_stream_keeps_its_own_failure_code() {
    for codec in &CODECS {
        let mut stream = compressed(codec, 4096);
        // Gzip carries an eight-byte trailer the header skip removes before
        // inflating, so the cut has to be deep enough to reach the deflate data
        // itself in every one of the three.
        stream.truncate(stream.len() / 2);

        let error = support::run_with_budget(
            codec.decompress,
            Arguments::new(),
            Value::Bytes(stream),
            budget(1024 * 1024),
        )
        .expect_err("a truncated stream must fail");
        assert_eq!(error.code(), codec.invalid, "{}", codec.decompress);
    }
}

#[test]
fn an_output_of_exactly_the_ceiling_is_still_produced() {
    let plain = usize::try_from(CEILING).expect("ceiling fits");
    for codec in &CODECS {
        let stream = compressed(codec, plain);
        let result = support::run_with_budget(
            codec.decompress,
            Arguments::new(),
            Value::Bytes(stream),
            budget(CEILING),
        )
        .unwrap_or_else(|error| {
            panic!(
                "{}: an output of exactly the ceiling must succeed, got {}",
                codec.decompress,
                error.code()
            )
        });
        assert_eq!(
            support::output_bytes(result).len(),
            plain,
            "{}",
            codec.decompress
        );
    }
}

#[test]
fn an_output_one_byte_over_the_ceiling_is_refused() {
    let plain = usize::try_from(CEILING).expect("ceiling fits") + 1;
    for codec in &CODECS {
        let stream = compressed(codec, plain);
        let error = support::run_with_budget(
            codec.decompress,
            Arguments::new(),
            Value::Bytes(stream),
            budget(CEILING),
        )
        .expect_err("one byte over the ceiling must fail");
        assert_eq!(
            error.code(),
            "core.operation.output_limit_exceeded",
            "{}",
            codec.decompress
        );
    }
}

/// `length` zero bytes in this codec's compressed form.
///
/// Zeros because the point is the ratio: the whole run costs a few dozen bytes,
/// so the compressed stream stays far below the ceiling its expansion breaks.
fn compressed(codec: &Codec, length: usize) -> Vec<u8> {
    let result = support::run_with_budget(
        codec.compress,
        Arguments::new(),
        Value::Bytes(vec![0; length]),
        budget(64 * 1024 * 1024),
    )
    .expect("compression should succeed");
    support::output_bytes(result)
}

/// A budget that constrains only the output size, so a failure here names the
/// ceiling rather than some other limit that happened to bite first.
const fn budget(max_output_bytes: u64) -> ExecutionBudget {
    ExecutionBudget {
        max_steps: 4,
        max_input_bytes: 64 * 1024 * 1024,
        max_output_bytes,
        max_expansion_ratio: u32::MAX,
        max_branches: 16,
        max_flow_depth: 4,
        max_operation_invocations: 16,
        max_total_bytes_processed: 256 * 1024 * 1024,
    }
}
