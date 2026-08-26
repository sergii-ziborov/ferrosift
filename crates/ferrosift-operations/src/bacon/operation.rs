use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{
    Arguments, OperationSpec, TextEncoding, TextValue, Value, ValueConstraint, ValueKind,
};

use crate::args::{boolean_argument, boolean_value, text_argument, text_value};
use crate::spec::{SpecDefinition, build};

use super::codec::{self, Alphabet, Translation};

/// Encodes text as a Bacon cipher.
pub struct BaconEncode {
    spec: OperationSpec,
}

impl BaconEncode {
    /// Creates the Bacon encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "cipher.bacon.encode@1",
                display_name: "Bacon Cipher Encode",
                category: "Ciphers",
                description: "Encodes letters as five-symbol Bacon codes.",
                cyberchef_alias: Some("Bacon Cipher Encode"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    text_argument(
                        "alphabet",
                        "\"Standard (I=J and U=V)\" or \"Complete\".",
                        "Standard (I=J and U=V)",
                    ),
                    text_argument(
                        "translation",
                        "Symbols to write: \"0/1\" or \"A/B\".",
                        "0/1",
                    ),
                    boolean_argument("keep", "Keep characters that are not letters.", false),
                    boolean_argument("invert", "Swap the two symbols.", false),
                ],
                inverse: Some("cipher.bacon.decode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for BaconEncode {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for BaconEncode {
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
        let alphabet = Alphabet::parse(text_value(arguments, "alphabet")?)?;
        let translation = Translation::parse(text_value(arguments, "translation")?)?.encodable()?;
        let keep = boolean_value(arguments, "keep")?;
        let invert = boolean_value(arguments, "invert")?;
        let input = crate::value::take_text_value(input)?;
        Ok(Value::Text(TextValue {
            text: codec::encode(&input.text, alphabet, translation, keep, invert, context)?,
            encoding: TextEncoding::Utf8,
        }))
    }
}

/// Decodes a Bacon cipher back to letters.
pub struct BaconDecode {
    spec: OperationSpec,
}

impl BaconDecode {
    /// Creates the Bacon decoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "cipher.bacon.decode@1",
                display_name: "Bacon Cipher Decode",
                category: "Ciphers",
                description: "Reads five-symbol Bacon codes back into letters.",
                cyberchef_alias: Some("Bacon Cipher Decode"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    text_argument(
                        "alphabet",
                        "\"Standard (I=J and U=V)\" or \"Complete\".",
                        "Standard (I=J and U=V)",
                    ),
                    text_argument(
                        "translation",
                        "How the symbols are written: \"0/1\", \"A/B\", \"Case\", \
                         or \"A-M/N-Z first letter\".",
                        "0/1",
                    ),
                    boolean_argument("invert", "Swap the two symbols.", false),
                ],
                inverse: Some("cipher.bacon.encode@1"),
                classifications: None,
            }),
        }
    }
}

impl Default for BaconDecode {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for BaconDecode {
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
        let alphabet = Alphabet::parse(text_value(arguments, "alphabet")?)?;
        let translation = Translation::parse(text_value(arguments, "translation")?)?;
        let invert = boolean_value(arguments, "invert")?;
        let input = crate::value::take_text_value(input)?;
        Ok(Value::Text(TextValue {
            text: codec::decode(&input.text, alphabet, translation, invert, context)?,
            encoding: TextEncoding::Utf8,
        }))
    }
}
