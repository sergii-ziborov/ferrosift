use alloc::string::String;
use core::fmt;

use ferrosift_core::{ExecutionError, RegistryError};
use ferrosift_pattern::PatternError;

/// Every way a `FerroSift` pipeline can fail, behind one stable code space.
///
/// The point of the facade is that callers do not glue several error models
/// together: transformation failures, engine failures, and pattern failures
/// all answer [`Error::code`] with a matchable identifier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// The built-in operation registry did not validate.
    Registry(RegistryError),
    /// A step named an operation the registry does not contain.
    UnknownOperation(String),
    /// The assembled recipe was not valid.
    InvalidRecipe,
    /// A step failed, or a budget was exceeded, while executing.
    Execution(ExecutionError),
    /// The pipeline produced a value the requested output cannot represent.
    UnexpectedOutput,
    /// A pattern failed to lex, parse, or evaluate.
    Pattern(PatternError),
}

impl Error {
    /// A stable, matchable failure code.
    ///
    /// Codes never change meaning between releases, so callers may branch on
    /// them instead of on message text.
    #[must_use]
    pub fn code(&self) -> &str {
        match self {
            Self::Registry(_) => "ferrosift.registry.invalid",
            Self::UnknownOperation(_) => "ferrosift.operation.unknown",
            Self::InvalidRecipe => "ferrosift.recipe.invalid",
            Self::Execution(error) => error.code(),
            Self::UnexpectedOutput => "ferrosift.output.unexpected_kind",
            Self::Pattern(error) => error.code(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Registry(_) => formatter.write_str("built-in operation registry is invalid"),
            Self::UnknownOperation(name) => {
                write!(formatter, "unknown operation: {name}")
            }
            Self::InvalidRecipe => formatter.write_str("pipeline does not form a valid recipe"),
            Self::Execution(error) => write!(formatter, "{error}"),
            Self::UnexpectedOutput => {
                formatter.write_str("pipeline output is not of the requested kind")
            }
            Self::Pattern(error) => write!(formatter, "{error}"),
        }
    }
}

impl From<RegistryError> for Error {
    fn from(error: RegistryError) -> Self {
        Self::Registry(error)
    }
}

impl From<ExecutionError> for Error {
    fn from(error: ExecutionError) -> Self {
        Self::Execution(error)
    }
}

impl From<PatternError> for Error {
    fn from(error: PatternError) -> Self {
        Self::Pattern(error)
    }
}
