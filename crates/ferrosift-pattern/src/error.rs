use alloc::string::String;
use core::fmt;

/// Position of a byte inside the pattern source, for diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    /// One-based line number.
    pub line: u32,
    /// One-based column number, counted in characters.
    pub column: u32,
}

impl fmt::Display for Position {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.line, self.column)
    }
}

/// A pattern-source failure carrying a stable code and a source position.
///
/// Codes are stable identifiers, not prose: callers may match on them and
/// they never change meaning between releases.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternError {
    code: &'static str,
    position: Position,
    detail: String,
}

impl PatternError {
    /// Creates an error with a stable code, a source position, and detail.
    #[must_use]
    pub fn new(code: &'static str, position: Position, detail: impl Into<String>) -> Self {
        Self {
            code,
            position,
            detail: detail.into(),
        }
    }

    /// The stable failure code, such as `pattern.parse.unexpected_token`.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code
    }

    /// Where in the source the failure was detected.
    #[must_use]
    pub const fn position(&self) -> Position {
        self.position
    }

    /// Human-facing explanation; never load-bearing for control flow.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for PatternError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: {} at {}",
            self.code, self.detail, self.position
        )
    }
}
