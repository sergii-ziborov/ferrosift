use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{text_argument, text_value};
use crate::failure::failed;
use crate::spec::{SpecDefinition, build};
use crate::value::take_decimal;

use super::codec;
use super::units;

/// A quantity converted between two units of the same kind.
///
/// Five operations that differ only in which table they consult. Written once
/// because the body is four lines and the difference between them is data --
/// writing it five times would be five chances to get the order of the
/// multiplication and the division wrong in one of them.
pub struct ConvertUnits {
    spec: OperationSpec,
    table: &'static [(&'static str, &'static str)],
    unknown_unit: &'static str,
}

/// What distinguishes one converter from another.
struct ConvertDefinition {
    id: &'static str,
    display_name: &'static str,
    description: &'static str,
    alias: &'static str,
    unknown_unit: &'static str,
    table: &'static [(&'static str, &'static str)],
    default_unit: &'static str,
}

impl ConvertUnits {
    fn from(definition: &ConvertDefinition) -> Self {
        Self {
            spec: build(SpecDefinition {
                id: definition.id,
                display_name: definition.display_name,
                category: "Arithmetic",
                description: definition.description,
                cyberchef_alias: Some(definition.alias),
                input: ValueConstraint::Exact(ValueKind::Decimal),
                output: ValueConstraint::Exact(ValueKind::Decimal),
                arguments: vec![
                    text_argument(
                        "input_units",
                        "The unit the input is in, named exactly as the reference names it.",
                        definition.default_unit,
                    ),
                    text_argument(
                        "output_units",
                        "The unit to answer in, named exactly as the reference names it.",
                        definition.default_unit,
                    ),
                ],
                inverse: None,
                classifications: None,
            }),
            table: definition.table,
            unknown_unit: definition.unknown_unit,
        }
    }

    /// Converts an area.
    #[must_use]
    pub fn area() -> Self {
        Self::from(&ConvertDefinition {
            id: "math.convert.area@1",
            display_name: "Convert area",
            description: "Converts an area between the reference's units.",
            alias: "Convert area",
            unknown_unit: "math.convert.area.unknown_unit",
            table: units::AREA,
            default_unit: "Square metre (sq m)",
        })
    }

    /// Converts a quantity of data.
    #[must_use]
    pub fn data() -> Self {
        Self::from(&ConvertDefinition {
            id: "math.convert.data@1",
            display_name: "Convert data units",
            description: "Converts a quantity of data between the reference's units.",
            alias: "Convert data units",
            unknown_unit: "math.convert.data.unknown_unit",
            table: units::DATA,
            default_unit: "Bytes (B)",
        })
    }

    /// Converts a distance.
    #[must_use]
    pub fn distance() -> Self {
        Self::from(&ConvertDefinition {
            id: "math.convert.distance@1",
            display_name: "Convert distance",
            description: "Converts a distance between the reference's units.",
            alias: "Convert distance",
            unknown_unit: "math.convert.distance.unknown_unit",
            table: units::DISTANCE,
            default_unit: "Metres (m)",
        })
    }

    /// Converts a mass.
    #[must_use]
    pub fn mass() -> Self {
        Self::from(&ConvertDefinition {
            id: "math.convert.mass@1",
            display_name: "Convert mass",
            description: "Converts a mass between the reference's units.",
            alias: "Convert mass",
            unknown_unit: "math.convert.mass.unknown_unit",
            table: units::MASS,
            default_unit: "Kilogram (kg)",
        })
    }

    /// Converts a speed.
    #[must_use]
    pub fn speed() -> Self {
        Self::from(&ConvertDefinition {
            id: "math.convert.speed@1",
            display_name: "Convert speed",
            description: "Converts a speed between the reference's units.",
            alias: "Convert speed",
            unknown_unit: "math.convert.speed.unknown_unit",
            table: units::SPEED,
            default_unit: "Metres per second (m/s)",
        })
    }
}

impl Operation for ConvertUnits {
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
        let from = codec::factor(self.table, text_value(arguments, "input_units")?)
            .ok_or_else(|| failed(self.unknown_unit))?;
        let to = codec::factor(self.table, text_value(arguments, "output_units")?)
            .ok_or_else(|| failed(self.unknown_unit))?;
        let value = take_decimal(input)?;

        context.ensure_active()?;
        Ok(Value::Decimal(codec::convert(&value, &from, &to)))
    }
}
