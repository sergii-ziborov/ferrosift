//! Stable compatibility boundary errors.

use core::fmt;

/// A hard failure that prevents preserving a trustworthy source recipe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImportError {
    /// The serialized recipe exceeds the public byte ceiling.
    SourceTooLarge,
    /// The input is not valid JSON.
    MalformedJson,
    /// The top-level JSON value is not an array.
    ExpectedArray,
    /// The source contains more than the public step ceiling.
    TooManySteps,
    /// Generated portable recipe invariants could not be satisfied.
    InvalidRecipe,
}

impl ImportError {
    /// Returns the stable machine-readable error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::SourceTooLarge => "compat.cyberchef.source_too_large",
            Self::MalformedJson => "compat.cyberchef.malformed_json",
            Self::ExpectedArray => "compat.cyberchef.expected_array",
            Self::TooManySteps => "compat.cyberchef.too_many_steps",
            Self::InvalidRecipe => "compat.cyberchef.invalid_recipe",
        }
    }
}

impl fmt::Display for ImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl core::error::Error for ImportError {}

/// A failure while converting a portable recipe into `CyberChef` JSON.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportError {
    /// The portable recipe contains more than the public step ceiling.
    TooManySteps,
    /// The serialized recipe exceeds the public byte ceiling.
    RecipeTooLarge,
    /// A recipe step references an operation absent from the registry.
    UnknownOperation,
    /// A registered operation has no `CyberChef` 11.3 alias.
    MissingAlias,
    /// A registered operation has multiple `CyberChef` 11.3 aliases.
    AmbiguousAlias,
    /// A recipe supplies an argument not declared by the operation.
    UndeclaredArgument,
    /// A required positional value cannot be emitted.
    MissingArgument,
    /// A typed argument cannot be represented as `CyberChef` JSON.
    ArgumentValue,
    /// The source value could not be serialized.
    Serialization,
}

impl ExportError {
    /// Returns the stable machine-readable error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::TooManySteps => "compat.cyberchef.export_too_many_steps",
            Self::RecipeTooLarge => "compat.cyberchef.export_recipe_too_large",
            Self::UnknownOperation => "compat.cyberchef.export_unknown_operation",
            Self::MissingAlias => "compat.cyberchef.export_missing_alias",
            Self::AmbiguousAlias => "compat.cyberchef.export_ambiguous_alias",
            Self::UndeclaredArgument => "compat.cyberchef.export_undeclared_argument",
            Self::MissingArgument => "compat.cyberchef.export_missing_argument",
            Self::ArgumentValue => "compat.cyberchef.export_argument_value",
            Self::Serialization => "compat.cyberchef.serialization",
        }
    }
}

impl fmt::Display for ExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl core::error::Error for ExportError {}
