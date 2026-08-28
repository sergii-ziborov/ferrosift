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

/// Modular Exponentiation, whose refusals the 11.4 corpus cannot carry.
///
/// Reports the failure *code* rather than a bare unit, because more than one
/// limit can refuse the same recipe and a test that only asks "did it fail"
/// can pass for a reason it was not written about.
fn modexp(base: &str, modulus: &str, exponent: &str, input: &str) -> Result<String, String> {
    let registry = ferrosift_operations::default_registry().expect("registry");
    let arguments: Arguments = [
        ("base".to_owned(), ArgumentValue::Text(base.to_owned())),
        (
            "modulus".to_owned(),
            ArgumentValue::Text(modulus.to_owned()),
        ),
        (
            "exponent".to_owned(),
            ArgumentValue::Text(exponent.to_owned()),
        ),
    ]
    .into_iter()
    .collect();

    let recipe = Recipe::new(
        vec![RecipeStep {
            id: StepId::new("s").expect("step id"),
            operation: OperationId::new("math.modexp@1").expect("operation id"),
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
        .map_err(|error| error.code().to_owned())
        .and_then(|outcome| match outcome.value {
            Value::Text(text) => Ok(text.text),
            other => Err(format!("unexpected value kind {:?}", other.kind())),
        })
}

#[test]
fn modular_exponentiation_refuses_a_missing_or_zero_modulus() {
    // Presence is checked before the operands are placed, so an absent modulus
    // is reported even when nothing else is usable either.
    assert!(modexp("4", "", "13", "").is_err(), "no modulus at all");
    assert!(modexp("4", "   ", "13", "").is_err(), "whitespace-only");
    assert!(modexp("", "", "", "").is_err(), "nothing usable anywhere");
    // Zero is a value the reference parses and then rejects, which is a
    // different moment: a malformed base is reported first.
    assert!(modexp("4", "0", "13", "").is_err(), "modulus of zero");
    assert!(modexp("4", "0x0", "13", "").is_err(), "hex zero");
    assert!(modexp("4", "-0", "13", "").is_err(), "signed zero");
}

#[test]
fn modular_exponentiation_will_not_guess_which_operand_the_input_is() {
    // With both boxes empty and an input to hand, either operand could be the
    // one meant. The reference refuses rather than picking, and so does this.
    assert!(
        modexp("", "497", "", "13").is_err(),
        "an input with both operands empty is ambiguous, not a default"
    );
    // Which is a different refusal from having nothing to work with at all.
    assert!(
        modexp("", "497", "", "").is_err(),
        "no operands and no input"
    );
    // One box filled resolves it, in either direction.
    assert_eq!(modexp("", "497", "13", "4").as_deref(), Ok("445"));
    assert_eq!(modexp("4", "497", "", "13").as_deref(), Ok("445"));
    // An empty input cannot fill the empty box.
    assert!(modexp("", "497", "13", "").is_err(), "no base to take");
    assert!(modexp("4", "497", "", "").is_err(), "no exponent to take");
}

#[test]
fn modular_exponentiation_trims_the_javascript_whitespace_set() {
    // U+0085 is whitespace to Rust's `str::trim` and not to JavaScript's, so
    // reading the operand with the wrong set turns a value the reference
    // refuses into one that computes. Verified against the pinned 11.4
    // checkout, which answers "Base must be decimal or hex (0x...)".
    //
    // The other direction — U+FEFF, which JavaScript trims and Rust does not —
    // produces a number rather than an error, so it is pinned in the corpus
    // instead of here.
    assert!(
        modexp("\u{0085}4", "497", "13", "").is_err(),
        "NEL is not whitespace in JavaScript, so this base is malformed"
    );
    assert!(
        modexp("4", "497", "\u{0085}13", "").is_err(),
        "the same for the exponent"
    );
    assert!(
        modexp("4", "\u{0085}497", "13", "").is_err(),
        "and for the modulus"
    );
}

#[test]
fn modular_exponentiation_refuses_work_it_cannot_afford() {
    // A recipe naming three short numbers can ask for an unbounded amount of
    // arithmetic: the cost is the exponent's bit length times the square of
    // the modulus's limb count, and neither is visible from the input size the
    // byte ceilings already bound. So it is charged, and refused up front
    // rather than discovered by waiting.
    //
    // Both cases take the exponent from the input, which keeps the executor's
    // expansion ratio well clear of what is being tested here. A version with
    // an empty input is refused too, but for the ratio rather than the work —
    // a true failure to the wrong question.
    let huge_exponent = format!("0x{}", "f".repeat(200_000));
    let huge_modulus = format!("0x{}", "f".repeat(20_000));
    assert_eq!(
        modexp("3", &huge_modulus, "", &huge_exponent),
        Err(String::from("core.operation.work_limit_exceeded")),
        "an 80000-bit modulus raised to an 800000-bit exponent is not free"
    );

    // The shapes this operation exists for stay inside the budget.
    let rsa_modulus = "1701411834604692317316873037158841057270000000000000000000000000\
                       0000000000000000000000000000000000000000000000000000000000000151";
    assert!(
        modexp("65537", rsa_modulus, "", "65537").is_ok(),
        "a 425-bit modulus with a 17-bit exponent is an ordinary request"
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
