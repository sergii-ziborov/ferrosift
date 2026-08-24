use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;
use crate::jsobject::KeySet;

const WRONG_SAMPLE_COUNT: &str = "sets.wrong_sample_count";

/// Which set operation to run.
#[derive(Clone, Copy)]
pub(super) enum Kind {
    Union,
    Intersection,
    Difference,
    SymmetricDifference,
    CartesianProduct,
}

impl Kind {
    /// How many samples the operation needs.
    const fn requires_pair(self) -> bool {
        !matches!(self, Self::CartesianProduct)
    }
}

/// Splits the input into samples, then each sample into items.
fn samples<'a>(input: &'a str, sample_delimiter: &str, kind: Kind) -> Option<Vec<&'a str>> {
    let parts: Vec<&str> = if sample_delimiter.is_empty() {
        alloc::vec![input]
    } else {
        input.split(sample_delimiter).collect()
    };
    let acceptable = if kind.requires_pair() {
        parts.len() == 2
    } else {
        parts.len() >= 2
    };
    acceptable.then_some(parts)
}

fn items<'a>(sample: &'a str, delimiter: &str) -> Vec<&'a str> {
    if delimiter.is_empty() {
        return alloc::vec![sample];
    }
    sample.split(delimiter).collect()
}

/// Runs one of the six set operations.
///
/// Both delimiters are taken literally. The reference's interface unescapes
/// them before the operation sees them, but its recipe API does not, and a
/// typed argument in this library carries a value rather than a spelling of
/// one — so `\n` here is a newline, not a backslash and an `n`.
pub(super) fn run(
    input: &str,
    kind: Kind,
    sample_delimiter: &str,
    item_delimiter: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let parts = samples(input, sample_delimiter, kind).ok_or_else(|| failed(WRONG_SAMPLE_COUNT))?;
    let sets: Vec<Vec<&str>> = parts
        .iter()
        .map(|sample| items(sample, item_delimiter))
        .collect();
    context.ensure_active()?;

    Ok(match kind {
        Kind::Union => union(&sets[0], &sets[1], item_delimiter),
        Kind::Intersection => keep(&sets[0], &sets[1], true, item_delimiter),
        Kind::Difference => keep(&sets[0], &sets[1], false, item_delimiter),
        Kind::SymmetricDifference => symmetric(&sets[0], &sets[1], item_delimiter),
        Kind::CartesianProduct => cartesian(&sets, item_delimiter, context)?,
    })
}

/// Union, built through an object literal — see `jsobject` for what that costs.
fn union(left: &[&str], right: &[&str], delimiter: &str) -> String {
    let mut keys = KeySet::new();
    for item in left.iter().chain(right) {
        keys.insert(item);
    }
    keys.keys().join(delimiter)
}

/// Intersection and difference share a shape: walk the first sample once,
/// keeping items by membership in the second and dropping later duplicates.
fn keep(left: &[&str], right: &[&str], included: bool, delimiter: &str) -> String {
    let mut seen: Vec<&str> = Vec::new();
    let mut kept: Vec<&str> = Vec::new();
    for item in left {
        if right.contains(item) != included || seen.contains(item) {
            continue;
        }
        seen.push(item);
        kept.push(item);
    }
    kept.join(delimiter)
}

/// Symmetric difference keeps duplicates: unlike the two above it filters by
/// membership alone, with no record of what it has already emitted.
fn symmetric(left: &[&str], right: &[&str], delimiter: &str) -> String {
    let mut result: Vec<&str> = left
        .iter()
        .filter(|item| !right.contains(item))
        .copied()
        .collect();
    result.extend(right.iter().filter(|item| !left.contains(item)).copied());
    result.join(delimiter)
}

/// Cartesian product across every sample, one line per tuple.
fn cartesian(
    sets: &[Vec<&str>],
    delimiter: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    let mut tuples: Vec<Vec<&str>> = alloc::vec![Vec::new()];
    for set in sets {
        let mut next = Vec::new();
        for tuple in &tuples {
            for item in set {
                let mut extended = tuple.clone();
                extended.push(item);
                next.push(extended);
            }
        }
        context.ensure_active()?;
        tuples = next;
    }
    Ok(tuples
        .into_iter()
        .map(|tuple| alloc::format!("({})", tuple.join(delimiter)))
        .collect::<Vec<_>>()
        .join(delimiter))
}

const NOT_TWO_SAMPLES: &str = "distance.wrong_sample_count";
const LENGTH_MISMATCH: &str = "distance.length_mismatch";
const NEGATIVE_COST: &str = "distance.negative_cost";

/// Hamming distance, by byte or by bit.
///
/// The equal-length check runs on the strings, before any conversion, so two
/// samples can pass it and still produce byte arrays of different lengths —
/// the loop then reads past the end of the shorter one, and the reference
/// treats that as zero.
pub(super) fn hamming(
    input: &str,
    delimiter: &str,
    by_byte: bool,
    hex_input: bool,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let parts: Vec<&str> = input.split(delimiter).collect();
    if parts.len() != 2 {
        return Err(failed(NOT_TWO_SAMPLES));
    }
    if parts[0].encode_utf16().count() != parts[1].encode_utf16().count() {
        return Err(failed(LENGTH_MISMATCH));
    }
    let left = decode_sample(parts[0], hex_input);
    let right = decode_sample(parts[1], hex_input);

    let mut distance: u64 = 0;
    for (index, byte) in left.iter().enumerate() {
        if index.is_multiple_of(4096) {
            context.ensure_active()?;
        }
        let other = right.get(index).copied().unwrap_or(0);
        if by_byte {
            distance += u64::from(*byte != other);
        } else {
            distance += u64::from((byte ^ other).count_ones());
        }
    }
    context.ensure_active()?;
    Ok(distance.to_string())
}

fn decode_sample(sample: &str, hex_input: bool) -> Vec<u8> {
    if hex_input {
        crate::hex_util::from_hex_auto(sample)
    } else {
        crate::jsstr::str_to_byte_array(sample)
    }
}

/// Levenshtein distance with configurable insertion, deletion, and
/// substitution costs.
pub(super) fn levenshtein(
    input: &str,
    delimiter: &str,
    costs: (i128, i128, i128),
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let (insertion, deletion, substitution) = costs;
    if insertion < 0 || deletion < 0 || substitution < 0 {
        return Err(failed(NEGATIVE_COST));
    }
    let parts: Vec<&str> = input.split(delimiter).collect();
    if parts.len() != 2 {
        return Err(failed(NOT_TWO_SAMPLES));
    }
    let source: Vec<u16> = parts[0].encode_utf16().collect();
    let target: Vec<u16> = parts[1].encode_utf16().collect();

    let mut current: Vec<i128> = (0..=source.len())
        .map(|index| i128::try_from(index).unwrap_or(0) * deletion)
        .collect();
    let mut next = alloc::vec![0i128; source.len() + 1];
    for (row, letter) in target.iter().enumerate() {
        if row.is_multiple_of(1024) {
            context.ensure_active()?;
        }
        next[0] = current[0] + insertion;
        for column in 0..source.len() {
            let mut best = current[column + 1] + insertion;
            let candidate = next[column] + deletion;
            if candidate < best {
                best = candidate;
            }
            let mut replace = current[column];
            if source[column] != *letter {
                replace += substitution;
            }
            if replace < best {
                best = replace;
            }
            next[column + 1] = best;
        }
        core::mem::swap(&mut current, &mut next);
    }
    context.ensure_active()?;
    Ok(current[source.len()].to_string())
}
