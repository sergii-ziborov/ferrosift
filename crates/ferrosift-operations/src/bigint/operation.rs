use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use ferrosift_model::CompatibilityProfile;

use crate::args::{text_argument, text_value};
use crate::spec::{SpecDefinition, build, build_since};
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
        let a = codec::parse_integer(a, "math.egcd.invalid_a", context)?;
        let b = codec::parse_integer(b, "math.egcd.invalid_b", context)?;

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
        let a = codec::parse_integer(a, "math.modinv.invalid_a", context)?;
        let m = codec::parse_integer(m, "math.modinv.invalid_m", context)?;

        context.ensure_active()?;
        Ok(text_output(codec::modular_inverse(&a, &m)?))
    }
}

/// `base ^ exponent` modulo a modulus, as used in Diffie-Hellman and RSA.
///
/// The first operation in the catalog the reference did not always have: 11.4
/// introduced it, so its alias starts there and 11.3 answers to no such name.
pub struct ModularExponentiation {
    spec: OperationSpec,
}

impl ModularExponentiation {
    /// Creates the modular-exponentiation operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build_since(
                CompatibilityProfile::CyberChefV11_4,
                SpecDefinition {
                    id: "math.modexp@1",
                    display_name: "Modular Exponentiation",
                    category: "Arithmetic",
                    description: "Computes base raised to exponent, modulo a modulus.",
                    cyberchef_alias: Some("Modular Exponentiation"),
                    input: ValueConstraint::Exact(ValueKind::Text),
                    output: ValueConstraint::Exact(ValueKind::Text),
                    // Argument order is the reference's, which is not the order
                    // the name reads in: the modulus sits between the base and
                    // the exponent. A positional CyberChef recipe carries them
                    // this way round, so reordering them for legibility would
                    // silently swap two of a caller's three numbers.
                    arguments: vec![
                        text_argument("base", "The base; empty takes it from the input.", ""),
                        text_argument("modulus", "The modulus, which must not be zero.", "1"),
                        text_argument(
                            "exponent",
                            "The exponent; empty takes it from the input.",
                            "",
                        ),
                    ],
                    inverse: None,
                    classifications: None,
                },
            ),
        }
    }
}

impl Default for ModularExponentiation {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ModularExponentiation {
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
        let base = text_value(arguments, "base")?;
        let modulus = text_value(arguments, "modulus")?;
        let exponent = text_value(arguments, "exponent")?;
        let input = take_text(input)?;

        // The modulus is checked for presence before either operand is placed,
        // so an empty modulus is reported even when the base and exponent are
        // also unusable. Its *value* is checked after all three parse, so a
        // modulus of zero is reported after a base that is not a number.
        if codec::js_trim(modulus).is_empty() {
            return Err(crate::failure::failed("math.modexp.modulus_missing"));
        }
        let (base, exponent) = codec::resolve_base_and_exponent(base, exponent, &input)?;
        let base = codec::parse_integer(base, "math.modexp.invalid_base", context)?;
        let exponent = codec::parse_integer(exponent, "math.modexp.invalid_exponent", context)?;
        let modulus = codec::parse_integer(modulus, "math.modexp.invalid_modulus", context)?;
        if modulus == num_bigint::BigInt::from(0) {
            return Err(crate::failure::failed("math.modexp.modulus_zero"));
        }

        context.ensure_active()?;
        Ok(text_output(codec::modular_exponentiation(
            &base, &exponent, &modulus, context,
        )?))
    }
}
