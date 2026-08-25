use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{text_argument, text_value};
use crate::spec::{SpecDefinition, build};
use crate::value::{take_text, text as text_output};

use super::codec;

/// The extended Euclidean algorithm over arbitrary-precision integers.
pub struct ExtendedGcd {
    spec: OperationSpec,
}

impl ExtendedGcd {
    /// Creates the extended-GCD operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "math.egcd@1",
                display_name: "Extended GCD",
                category: "Arithmetic",
                description: "Finds the greatest common divisor of a and b with their Bezout coefficients.",
                cyberchef_alias: Some("Extended GCD"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    text_argument(
                        "value_a",
                        "First integer; empty takes it from the input.",
                        "",
                    ),
                    text_argument(
                        "value_b",
                        "Second integer; empty takes it from the input.",
                        "",
                    ),
                ],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for ExtendedGcd {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ExtendedGcd {
    fn spec(&self) -> &OperationSpec {
        &self.spec
    }

    fn execute(
        &self,
        input: Value,
        arguments: &Arguments,
        context: &mut OperationContext<'_>,
    ) -> Result<Value, OperationError> {
        context.ensure_active()?;
        let value_a = text_value(arguments, "value_a")?;
        let value_b = text_value(arguments, "value_b")?;
        let input = take_text(input)?;

        let (a, b) = codec::resolve_pair(
            value_a,
            value_b,
            &input,
            "math.egcd.missing_a",
            "math.egcd.missing_b",
            "math.egcd.missing_both",
        )?;
        let a = codec::parse_integer(a, "math.egcd.invalid_a")?;
        let b = codec::parse_integer(b, "math.egcd.invalid_b")?;

        context.ensure_active()?;
        Ok(text_output(codec::extended_gcd_report(&a, &b)))
    }
}

/// The modular multiplicative inverse.
pub struct ModularInverse {
    spec: OperationSpec,
}

impl ModularInverse {
    /// Creates the modular-inverse operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "math.modinv@1",
                display_name: "Modular Inverse",
                category: "Arithmetic",
                description: "Finds x such that a*x is congruent to 1 modulo m.",
                cyberchef_alias: Some("Modular Inverse"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    text_argument("value_a", "The value; empty takes it from the input.", ""),
                    text_argument(
                        "modulus_m",
                        "The modulus; empty takes it from the input.",
                        "",
                    ),
                ],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for ModularInverse {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ModularInverse {
    fn spec(&self) -> &OperationSpec {
        &self.spec
    }

    fn execute(
        &self,
        input: Value,
        arguments: &Arguments,
        context: &mut OperationContext<'_>,
    ) -> Result<Value, OperationError> {
        context.ensure_active()?;
        let value_a = text_value(arguments, "value_a")?;
        let modulus = text_value(arguments, "modulus_m")?;
        let input = take_text(input)?;

        let (a, m) = codec::resolve_pair(
            value_a,
            modulus,
            &input,
            "math.modinv.missing_a",
            "math.modinv.missing_m",
            "math.modinv.missing_both",
        )?;
        let a = codec::parse_integer(a, "math.modinv.invalid_a")?;
        let m = codec::parse_integer(m, "math.modinv.invalid_m")?;

        context.ensure_active()?;
        Ok(text_output(codec::modular_inverse(&a, &m)?))
    }
}
