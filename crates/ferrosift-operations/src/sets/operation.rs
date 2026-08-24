//! Operation wrappers for the set operations and edit distances.

use alloc::{vec, vec::Vec};

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{ArgumentSpec, Arguments, OperationSpec, Value, ValueKind};

use crate::args::{integer_argument, integer_value, text_argument, text_value};
use crate::spec::{UniformSpec, build_uniform};
use crate::value::{take_text, text};

use super::codec::{self, Kind};

fn text_spec(
    id: &'static str,
    display_name: &'static str,
    description: &'static str,
    category: &'static str,
    arguments: Vec<ArgumentSpec>,
) -> OperationSpec {
    build_uniform(
        ValueKind::Text,
        UniformSpec {
            id,
            display_name,
            category,
            description,
            cyberchef_alias: display_name,
            arguments,
        },
    )
}

/// The two delimiters every set operation takes, as binary strings.
fn set_arguments() -> Vec<ArgumentSpec> {
    vec![
        text_argument("sample_delimiter", "Between samples.", "\n\n"),
        text_argument("item_delimiter", "Between items.", ","),
    ]
}

/// One of the six set operations over delimited samples.
pub struct SetOperation {
    spec: OperationSpec,
    kind: Kind,
}

impl SetOperation {
    /// Union of two samples.
    #[must_use]
    pub fn union() -> Self {
        Self::build(
            Kind::Union,
            "sets.union@1",
            "Set Union",
            "Every item in either sample, without duplicates.",
        )
    }

    /// Intersection of two samples.
    #[must_use]
    pub fn intersection() -> Self {
        Self::build(
            Kind::Intersection,
            "sets.intersection@1",
            "Set Intersection",
            "Items present in both samples.",
        )
    }

    /// Items in the first sample only.
    #[must_use]
    pub fn difference() -> Self {
        Self::build(
            Kind::Difference,
            "sets.difference@1",
            "Set Difference",
            "Items in the first sample that are not in the second.",
        )
    }

    /// Items in exactly one sample.
    #[must_use]
    pub fn symmetric_difference() -> Self {
        Self::build(
            Kind::SymmetricDifference,
            "sets.symmetric_difference@1",
            "Symmetric Difference",
            "Items present in exactly one of the two samples.",
        )
    }

    /// Every tuple across all samples.
    #[must_use]
    pub fn cartesian_product() -> Self {
        Self::build(
            Kind::CartesianProduct,
            "sets.cartesian_product@1",
            "Cartesian Product",
            "Every combination taking one item from each sample.",
        )
    }

    fn build(
        kind: Kind,
        id: &'static str,
        display_name: &'static str,
        description: &'static str,
    ) -> Self {
        Self {
            spec: text_spec(id, display_name, description, "Sets", set_arguments()),
            kind,
        }
    }
}

impl Operation for SetOperation {
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
        let sample = text_value(arguments, "sample_delimiter")?;
        let item = text_value(arguments, "item_delimiter")?;
        let input = take_text(input)?;
        Ok(text(codec::run(&input, self.kind, sample, item, context)?))
    }
}

/// Hamming distance between two equal-length samples.
pub struct HammingDistance {
    spec: OperationSpec,
}

impl HammingDistance {
    /// Creates the Hamming distance operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: text_spec(
                "distance.hamming@1",
                "Hamming Distance",
                "Counts the differing bytes or bits between two samples.",
                "Distance",
                vec![
                    text_argument("delimiter", "Between samples.", "\n\n"),
                    text_argument("unit", "Byte or Bit.", "Byte"),
                    text_argument("input_type", "Raw string or Hex.", "Raw string"),
                ],
            ),
        }
    }
}

impl Default for HammingDistance {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for HammingDistance {
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
        let delimiter = text_value(arguments, "delimiter")?;
        let by_byte = text_value(arguments, "unit")? == "Byte";
        let hex = text_value(arguments, "input_type")? == "Hex";
        let input = take_text(input)?;
        Ok(text(codec::hamming(
            &input, delimiter, by_byte, hex, context,
        )?))
    }
}

/// Levenshtein edit distance between two samples.
pub struct LevenshteinDistance {
    spec: OperationSpec,
}

impl LevenshteinDistance {
    /// Creates the Levenshtein distance operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: text_spec(
                "distance.levenshtein@1",
                "Levenshtein Distance",
                "Computes the edit distance between two samples.",
                "Distance",
                vec![
                    text_argument("delimiter", "Between samples.", "\n"),
                    integer_argument("insertion_cost", "Cost of an insertion.", 1),
                    integer_argument("deletion_cost", "Cost of a deletion.", 1),
                    integer_argument("substitution_cost", "Cost of a substitution.", 1),
                ],
            ),
        }
    }
}

impl Default for LevenshteinDistance {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for LevenshteinDistance {
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
        let delimiter = text_value(arguments, "delimiter")?;
        let costs = (
            integer_value(arguments, "insertion_cost")?,
            integer_value(arguments, "deletion_cost")?,
            integer_value(arguments, "substitution_cost")?,
        );
        let input = take_text(input)?;
        Ok(text(codec::levenshtein(&input, delimiter, costs, context)?))
    }
}
