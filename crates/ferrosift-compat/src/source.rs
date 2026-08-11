//! Loss-aware source recipe representation.

use alloc::{string::ToString, vec::Vec};

use serde::de::{Deserializer as _, Error as _, IgnoredAny, SeqAccess, Visitor};
use serde_json::Value as JsonValue;

use crate::{error::ImportError, profile::MAX_RECIPE_STEPS};

const STEP_LIMIT_MARKER: &str = "ferrosift cyberchef step limit";

/// One original `CyberChef` recipe step.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceStep {
    raw: JsonValue,
}

impl SourceStep {
    pub(crate) const fn new(raw: JsonValue) -> Self {
        Self { raw }
    }

    /// Borrows the complete semantic JSON value for this step.
    #[must_use]
    pub const fn raw(&self) -> &JsonValue {
        &self.raw
    }
}

/// Complete source steps in their original order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceRecipe {
    steps: Vec<SourceStep>,
}

impl SourceRecipe {
    pub(crate) const fn new(steps: Vec<SourceStep>) -> Self {
        Self { steps }
    }

    /// Borrows preserved source steps.
    #[must_use]
    pub fn steps(&self) -> &[SourceStep] {
        &self.steps
    }

    pub(crate) fn raw_steps(&self) -> impl ExactSizeIterator<Item = &JsonValue> {
        self.steps.iter().map(SourceStep::raw)
    }
}

pub(crate) fn parse_source(bytes: &[u8]) -> Result<SourceRecipe, ImportError> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let source = deserializer
        .deserialize_seq(SourceRecipeVisitor)
        .map_err(|error| classify_error(&error))?;
    deserializer.end().map_err(|_| ImportError::MalformedJson)?;
    Ok(source)
}

struct SourceRecipeVisitor;

impl<'de> Visitor<'de> for SourceRecipeVisitor {
    type Value = SourceRecipe;

    fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("a CyberChef recipe array")
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut steps = Vec::new();
        while steps.len() < MAX_RECIPE_STEPS {
            let Some(raw) = sequence.next_element::<JsonValue>()? else {
                return Ok(SourceRecipe::new(steps));
            };
            steps.push(SourceStep::new(raw));
        }

        if sequence.next_element::<IgnoredAny>()?.is_some() {
            Err(A::Error::custom(STEP_LIMIT_MARKER))
        } else {
            Ok(SourceRecipe::new(steps))
        }
    }
}

fn classify_error(error: &serde_json::Error) -> ImportError {
    if error.to_string().starts_with(STEP_LIMIT_MARKER) {
        ImportError::TooManySteps
    } else if error.classify() == serde_json::error::Category::Data {
        ImportError::ExpectedArray
    } else {
        ImportError::MalformedJson
    }
}
