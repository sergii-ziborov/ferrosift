//! Bit-level behaviour the automatic corpus cannot cover.
//!
//! The corpus pins outputs for recipes the reference accepts. These two cases
//! sit outside that: one is an argument the reference rejects before running,
//! so there is no output to pin, and the other is a shape where two operations
//! that look symmetrical disagree.

use ferrosift_core::ExecutionStatus;
use ferrosift_model::{ArgumentValue, Arguments, Value};

mod support;

fn shift_left(
    amount: i128,
    input: &[u8],
) -> Result<ferrosift_core::ExecutionResult, ferrosift_core::ExecutionError> {
    support::run_with_budget(
        "logic.shift.left@1",
        support::argument("amount", ArgumentValue::Integer(amount)),
        Value::Bytes(input.to_vec()),
        support::budget(),
    )
}

/// `Bit shift left` declares `max: 7` on its amount, so the reference refuses
/// an eight-bit shift outright — while `Bit shift right`, which declares no
/// bound, masks the count and shifts. Accepting 8 here would look harmless and
/// would silently disagree with the reference.
#[test]
fn bit_shift_left_rejects_an_amount_the_reference_will_not_accept() {
    for amount in [-1, 8, 32, 1000] {
        assert!(
            shift_left(amount, b"\x01\x02\x03").is_err(),
            "amount {amount} must be rejected"
        );
    }
}

#[test]
fn bit_shift_left_accepts_the_whole_declared_range() {
    for amount in 0..=7 {
        let result = shift_left(amount, b"\x01").expect("amount is in range");
        assert_eq!(result.status, ExecutionStatus::Completed);
        let expected = u8::try_from((1i32 << amount) & 0xff).expect("byte");
        assert_eq!(support::output_bytes(result), [expected]);
    }
}

/// Rotating an empty buffer with carry is not symmetric. Rotating right ends
/// with `result[0] |= carry`, which creates index 0 and returns one zero byte;
/// rotating left ends with `result[length - 1] |= carry`, which writes to
/// index `-1` and leaves the array empty.
#[test]
fn carrying_rotation_of_empty_input_is_asymmetric() {
    let arguments = Arguments::from([
        ("amount".into(), ArgumentValue::Integer(3)),
        ("carry_through".into(), ArgumentValue::Boolean(true)),
    ]);

    let right = support::run(
        "logic.rotate.right@1",
        arguments.clone(),
        Value::Bytes(Vec::new()),
    );
    assert_eq!(support::output_bytes(right), [0]);

    let left = support::run("logic.rotate.left@1", arguments, Value::Bytes(Vec::new()));
    assert!(support::output_bytes(left).is_empty());
}
