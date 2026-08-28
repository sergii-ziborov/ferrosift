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

impl Position {
    /// The position of something that has none.
    ///
    /// Line and column are one-based, so zero is a value no real position can
    /// take — which is what lets "not known" be represented without an
    /// `Option` on every error and every declaration. It is rendered as `?:?`
    /// rather than as `0:0`, because a reader who saw `0:0` would go looking
    /// for a line zero.
    pub const UNKNOWN: Self = Self { line: 0, column: 0 };

    /// Whether this position names a place in the source.
    #[must_use]
    pub const fn is_known(self) -> bool {
        self.line != 0
    }
}

impl fmt::Display for Position {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_known() {
            write!(formatter, "{}:{}", self.line, self.column)
        } else {
            formatter.write_str("?:?")
        }
    }
}

/// A pattern failure carrying a stable code and where it happened.
///
/// Codes are stable identifiers, not prose: callers may match on them and
/// they never change meaning between releases.
///
/// A failure has up to *two* locations, and the difference is the whole point
/// of carrying both. [`position`](Self::position) is a place in the pattern
/// source — the line and column of the declaration being read.
/// [`data_offset`](Self::data_offset) is a place in the bytes. A parse failure
/// has only the first; an evaluation failure usually has both, and neither one
/// answers the other's question. "The read left the data" is not useful without
/// knowing which byte was wanted, and knowing the byte is not useful without
/// knowing which line asked for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternError {
    code: &'static str,
    position: Position,
    data_offset: Option<u64>,
    detail: String,
}

impl PatternError {
    /// Creates an error with a stable code, a source position, and detail.
    #[must_use]
    pub fn new(code: &'static str, position: Position, detail: impl Into<String>) -> Self {
        Self {
            code,
            position,
            data_offset: None,
            detail: detail.into(),
        }
    }

    /// Records the byte offset the failure concerns.
    #[must_use]
    pub fn at_offset(mut self, offset: u64) -> Self {
        self.data_offset = Some(offset);
        self
    }

    /// Records where in the pattern source the failure was detected.
    ///
    /// Applied on the way out rather than at the point of failure. The
    /// evaluator is several types deep when a read fails, and the only place
    /// that knows which line a caller should look at is the declaration being
    /// read — so the location is attached as the error passes it.
    #[must_use]
    pub fn at_position(mut self, position: Position) -> Self {
        self.position = position;
        self
    }

    /// Fills in a source position only where none was recorded.
    ///
    /// So an inner declaration keeps the position it reported and an outer one
    /// supplies the position for everything that had none — which is what puts
    /// a struct member's own line on the error rather than the line of the
    /// placement that reached it.
    #[must_use]
    pub fn or_position(self, position: Position) -> Self {
        if self.position == Position::UNKNOWN {
            self.at_position(position)
        } else {
            self
        }
    }

    /// Where in the data the failure happened, when it concerns a byte at all.
    #[must_use]
    pub const fn data_offset(&self) -> Option<u64> {
        self.data_offset
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
        )?;
        // Hexadecimal, because a byte offset is read against a hex view and
        // nobody converts in their head to find it there.
        if let Some(offset) = self.data_offset {
            write!(formatter, " (data offset 0x{offset:x})")?;
        }
        Ok(())
    }
}
