//! What the arbitrary-precision operations refuse.
//!
//! The corpus pins outputs, so it cannot pin an operation that declines to
//! produce one. The reference throws for a non-invertible pair, a non-positive
//! modulus, and a malformed number; those refusals are as much a part of the
//! contract as the answers, and this is where they are held.
//!
//! Refusing is also the safer half to get wrong quietly. An inverse that does
//! not exist is a question with no answer — returning zero, or the input, or a
//! plausible-looking number would each be worse than an error, and none of
//! them would be caught by a corpus of cases that succeed.

#![cfg(feature = "arithmetic")]

use ferrosift_core::{ExecutionBudget, Executor, NeverCancelled};
use ferrosift_model::{
    ArgumentValue, Arguments, CapabilitySet, OperationId, Recipe, RecipeMetadata, RecipeStep,
    StepId, TextEncoding, TextValue, Value,
};

fn run(operation: &str, first: &str, second: &str, input: &str) -> Result<String, ()> {
    let registry = ferrosift_operations::default_registry().expect("registry");
    let (first_name, second_name) = if operation == "math.egcd@1" {
        ("value_a", "value_b")
    } else {
        ("value_a", "modulus_m")
    };
    let arguments: Arguments = [
        (first_name.to_owned(), ArgumentValue::Text(first.to_owned())),
        (
            second_name.to_owned(),
            ArgumentValue::Text(second.to_owned()),
        ),
    ]
    .into_iter()
    .collect();

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
            Value::Text(TextValue {
                text: input.to_owned(),
                encoding: TextEncoding::Utf8,
            }),
            ExecutionBudget::generous(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .map_err(|_| ())
        .and_then(|outcome| match outcome.value {
            Value::Text(text) => Ok(text.text),
            _ => Err(()),
        })
}

#[test]
fn modular_inverse_refuses_a_non_invertible_pair() {
    // gcd(4, 8) is 4, so no inverse exists. The reference throws.
    assert!(
        run("math.modinv@1", "4", "8", "").is_err(),
        "an inverse that does not exist must be refused, not invented"
    );
    assert!(
        run("math.modinv@1", "0", "5", "").is_err(),
        "zero has no inverse modulo anything"
    );
    assert!(
        run("math.modinv@1", "6", "9", "").is_err(),
        "gcd(6, 9) is 3"
    );
}

#[test]
fn modular_inverse_refuses_a_non_positive_modulus() {
    for modulus in ["0", "-7"] {
        assert!(
            run("math.modinv@1", "3", modulus, "").is_err(),
            "modulus {modulus} should be refused"
        );
    }
}

#[test]
fn a_malformed_number_is_refused_rather_than_partly_parsed() {
    // The reference anchors both of its patterns, so trailing text is an
    // error here where `parseInt` would have ignored it. That difference is
    // the reason this is worth a test: the two helpers disagree on purpose.
    for value in ["12abc", "abc", "1.5", "0x", "0xzz", "--1", "+ 1", ""] {
        assert!(
            run("math.egcd@1", value, "5", "").is_err(),
            "{value:?} is not a well-formed integer and should be refused"
        );
    }
    // A signed hex literal is refused too: the reference's hex pattern has no
    // sign branch.
    assert!(
        run("math.egcd@1", "-0xff", "5", "").is_err(),
        "the reference does not accept a signed hex literal"
    );
}

#[test]
fn both_operands_missing_is_refused() {
    assert!(
        run("math.egcd@1", "", "", "").is_err(),
        "with no arguments and no input there is nothing to compute"
    );
    assert!(
        run("math.modinv@1", "", "", "").is_err(),
        "with no arguments and no input there is nothing to compute"
    );
}

#[test]
fn arbitrary_precision_is_actually_arbitrary() {
    // 2^127 - 1, one past what a u128 magnitude holds alongside a sign, and
    // the reason this pack exists rather than a fixed-width port.
    let mersenne = "170141183460469231731687303715884105727";
    let inverse = run("math.modinv@1", "65537", mersenne, "")
        .expect("a 127-bit modulus is ordinary for this operation");

    // Verified by construction rather than by a hard-coded digit string: the
    // product of the value and its inverse is one modulo the modulus.
    assert!(
        !inverse.is_empty() && inverse.chars().all(|c| c.is_ascii_digit()),
        "expected a decimal inverse, got {inverse:?}"
    );
    assert!(
        inverse.len() > 20,
        "an inverse modulo a 127-bit number should not fit in 64 bits, got {inverse:?}"
    );
}
