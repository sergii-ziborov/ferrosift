//! What reading a bcrypt hash refuses, and the one place it cannot agree.
//!
//! The corpus pins what the operation *prints*, which it can only do for input
//! the reference accepts — a bake that throws is a generation failure rather
//! than a recorded case. So the refusals live here.
//!
//! There is exactly one rule behind all of them: the reference's library checks
//! the total length and nothing else, and checks it in UTF-16 code units. Every
//! other malformation is printed rather than refused, which the corpus covers.

use ferrosift_model::{Arguments, Value};

mod support;

const OPERATION: &str = "hash.bcrypt.parse@1";

/// A real hash, to keep the negative cases honest about what they change.
const VALID: &str = "$2a$10$k1wbIrmNyFAPwPVPSVa/zecw2BCEnBwVS2GbrmgzxFUOqW9dk4TCW";

fn run(input: &str) -> Result<ferrosift_core::ExecutionResult, ferrosift_core::ExecutionError> {
    support::run_with_budget(
        OPERATION,
        Arguments::new(),
        support::text(input),
        support::budget(),
    )
}

#[test]
fn a_hash_of_the_published_length_is_read() {
    let result = run(VALID).expect("a sixty-character hash is accepted");
    assert_eq!(
        support::output_text(result),
        expected_output(),
        "the four printed fields are the reference's own"
    );
}

fn expected_output() -> String {
    format!(
        "Rounds: 10\nSalt: {}\nPassword hash: {}\nFull hash: {VALID}",
        &VALID[..29],
        &VALID[29..],
    )
}

#[test]
fn any_other_length_is_refused() {
    // One short, one long, nothing at all, and a hash with a trailing newline —
    // which is the shape a file gives you and which the reference rejects.
    for input in [
        String::new(),
        VALID[..59].to_owned(),
        format!("{VALID}x"),
        format!("{VALID}\n"),
        "$2a$10$".to_owned(),
    ] {
        assert!(
            run(&input).is_err(),
            "{} characters must be refused",
            input.chars().count()
        );
    }
}

/// Length is counted in UTF-16 code units, not characters.
///
/// This is the check being reproduced rather than an approximation of it:
/// fifty-nine ASCII characters and one astral character are sixty characters
/// and sixty-one code units, and the reference refuses them.
#[test]
fn the_length_is_counted_in_code_units() {
    let sixty_characters = format!("{}😀", &VALID[..59]);
    assert_eq!(sixty_characters.chars().count(), 60);
    assert_eq!(sixty_characters.encode_utf16().count(), 61);
    assert!(
        run(&sixty_characters).is_err(),
        "sixty characters that are sixty-one code units must be refused"
    );

    // And the converse: fifty-eight ASCII characters plus one astral character
    // is fifty-nine characters and sixty code units, which is accepted.
    let sixty_units = format!("{}😀", &VALID[..58]);
    assert_eq!(sixty_units.chars().count(), 59);
    assert_eq!(sixty_units.encode_utf16().count(), 60);
    assert!(
        run(&sixty_units).is_ok(),
        "fifty-nine characters that are sixty code units must be accepted"
    );
}

/// The one input where this cannot answer what the reference answers.
///
/// The salt is the first twenty-nine code units, and that cut can fall between
/// the halves of a surrogate pair. JavaScript hands back a string holding one
/// half; a Rust string cannot hold one at all. So this refuses rather than
/// substituting a replacement character and calling the result the reference's
/// output — the same choice `Offset Checker` makes for the same reason.
///
/// The divergence is total rather than partial: there is no input where the two
/// both succeed and disagree.
#[test]
fn a_salt_cut_through_a_surrogate_pair_is_refused() {
    // Twenty-eight ASCII characters, then an astral character whose two code
    // units straddle the cut at twenty-nine, then enough to reach sixty.
    let split = format!("{}😀{}", &VALID[..28], &VALID[..30]);
    assert_eq!(split.encode_utf16().count(), 60);
    assert!(
        run(&split).is_err(),
        "a salt ending inside a surrogate pair must be refused"
    );

    // The same astral character one place later, where the pair falls wholly
    // inside the salt and there is nothing to refuse.
    let whole = format!("{}😀{}", &VALID[..27], &VALID[..31]);
    assert_eq!(whole.encode_utf16().count(), 60);
    assert!(
        run(&whole).is_ok(),
        "a surrogate pair inside the salt is ordinary input"
    );
}

/// Bytes reaching a text operation are converted, not refused.
#[test]
fn byte_input_is_read_as_the_reference_reads_it() {
    let result = run_bytes(VALID.as_bytes()).expect("bytes carrying a hash are accepted");
    assert_eq!(support::output_text(result), expected_output());
}

fn run_bytes(
    input: &[u8],
) -> Result<ferrosift_core::ExecutionResult, ferrosift_core::ExecutionError> {
    support::run_with_budget(
        OPERATION,
        Arguments::new(),
        Value::Bytes(input.to_vec()),
        support::budget(),
    )
}
