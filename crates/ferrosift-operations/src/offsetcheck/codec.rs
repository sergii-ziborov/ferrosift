//! Highlighting the characters that every sample shares at the same offset.

use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

use crate::failure::failed;

/// The wrapper the reference opens a run of matching characters with.
const OPEN: &str = "<span class='hl5'>";
const CLOSE: &str = "</span>";

/// Compares samples and marks the characters common to all of them.
///
/// Everything runs on UTF-16 code units because that is what the reference
/// indexes. A surrogate pair is two positions there and one `char` here, so
/// working in `char` would move every highlight boundary after the first
/// astral character in the input.
pub(super) fn check(
    input: &str,
    delimiter: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let units: Vec<u16> = input.encode_utf16().collect();
    let separator: Vec<u16> = delimiter.encode_utf16().collect();
    let samples = split(&units, &separator);
    if samples.len() < 2 {
        return Err(failed("analysis.offset_checker.not_enough_samples"));
    }

    let mut outputs: Vec<Vec<u16>> = alloc::vec![Vec::new(); samples.len()];
    // One flag for all samples rather than one per sample. The reference keeps
    // a single `inMatch` and only updates it while writing the *last* sample,
    // so the earlier samples in a row act on a run the last one has not opened
    // yet -- which is where the stray closing tag at the end of a run comes
    // from. Reproducing the output means reproducing the single flag, not
    // repairing it into the per-sample one it looks like it should be.
    let mut in_match = false;
    let last = samples.len() - 1;

    for index in 0..samples[0].len() {
        if index % 4096 == 0 {
            context.ensure_active()?;
        }
        let unit = samples[0][index];
        let matched = samples[1..]
            .iter()
            .all(|sample| sample.get(index) == Some(&unit));

        for (position, sample) in samples.iter().enumerate() {
            let output = &mut outputs[position];
            if sample.len() <= index {
                if in_match {
                    push_str(output, CLOSE);
                }
                if position == last {
                    in_match = false;
                }
                continue;
            }

            let current = sample[index];
            if matched && !in_match {
                push_str(output, OPEN);
                push_escaped(output, current);
                if sample.len() == index + 1 {
                    push_str(output, CLOSE);
                }
                if position == last {
                    in_match = true;
                }
            } else if !matched && in_match {
                push_str(output, CLOSE);
                push_escaped(output, current);
                if position == last {
                    in_match = false;
                }
            } else {
                push_escaped(output, current);
                if in_match && sample.len() == index + 1 {
                    push_str(output, CLOSE);
                    // The guard is dead: this arm has already established that
                    // the sample ends at `index`, so the test is always false.
                    // It is kept because the reference has it, and removing a
                    // condition that never fires still changes what the code
                    // claims about when the flag should clear.
                    if sample.len() - 1 != index {
                        in_match = false;
                    }
                }
            }

            // The tail is appended while walking the *first* sample's last
            // position, so a sample longer than the first has its remainder
            // emitted here rather than by a loop of its own.
            if samples[0].len() - 1 == index {
                if in_match {
                    push_str(output, CLOSE);
                }
                for unit in &sample[index + 1..] {
                    push_escaped(output, *unit);
                }
            }
        }
    }

    context.ensure_active()?;
    let mut escaped_separator: Vec<u16> = Vec::new();
    for unit in &separator {
        push_escaped(&mut escaped_separator, *unit);
    }
    let mut joined: Vec<u16> = Vec::new();
    for (position, output) in outputs.iter().enumerate() {
        if position > 0 {
            joined.extend_from_slice(&escaped_separator);
        }
        joined.extend_from_slice(output);
    }
    String::from_utf16(&joined).map_err(|_| failed("analysis.offset_checker.unpaired_surrogate"))
}

/// Splits on a separator the way `String.prototype.split` does.
///
/// An empty separator is not a split at all in the reference -- it yields one
/// element per code unit -- but the delimiter is never empty by the time this
/// is reached, so the simple scan is enough.
fn split<'a>(units: &'a [u16], separator: &[u16]) -> Vec<&'a [u16]> {
    if separator.is_empty() {
        return alloc::vec![units];
    }
    let mut pieces = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while index + separator.len() <= units.len() {
        if &units[index..index + separator.len()] == separator {
            pieces.push(&units[start..index]);
            index += separator.len();
            start = index;
        } else {
            index += 1;
        }
    }
    pieces.push(&units[start..]);
    pieces
}

/// Appends an ASCII string as code units.
fn push_str(output: &mut Vec<u16>, text: &str) {
    output.extend(text.encode_utf16());
}

/// Appends one code unit, escaped for HTML.
///
/// The table is the reference's own and is not the usual one: an apostrophe
/// becomes `&#x27;` rather than `&apos;`, a backtick is escaped as well, and a
/// NUL becomes a private-use character instead of an entity -- so that a zero
/// byte survives being placed in markup at all.
fn push_escaped(output: &mut Vec<u16>, unit: u16) {
    let replacement = match unit {
        0x26 => Some("&amp;"),
        0x3c => Some("&lt;"),
        0x3e => Some("&gt;"),
        0x22 => Some("&quot;"),
        0x27 => Some("&#x27;"),
        0x60 => Some("&#x60;"),
        0x00 => Some("\u{e000}"),
        _ => None,
    };
    match replacement {
        Some(text) => push_str(output, text),
        None => output.push(unit),
    }
}
