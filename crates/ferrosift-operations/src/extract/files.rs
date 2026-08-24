//! File-oriented extractors: paths, hashes, and printable strings.

use alloc::vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::args::{
    boolean_argument, boolean_value, integer_argument, integer_value, text_argument, text_value,
};
use crate::spec::{SpecDefinition, build};

use super::operation::{extract_flags, present_flags, require_text, text_out};
use super::{regexes, strings};

/// Extracts Windows and UNIX file paths.
pub struct ExtractFilePaths {
    spec: OperationSpec,
}

impl ExtractFilePaths {
    /// Creates the file path extract operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "extract.file_paths@1",
                display_name: "Extract file paths",
                category: "Extractors",
                description: "Extracts Windows and UNIX file paths from text.",
                cyberchef_alias: Some("Extract file paths"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: {
                    let mut args = vec![
                        boolean_argument("windows", "Include Windows paths.", true),
                        boolean_argument("unix", "Include UNIX paths.", true),
                    ];
                    args.extend(extract_flags!());
                    args
                },
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for ExtractFilePaths {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ExtractFilePaths {
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
        let input = require_text(input)?;
        Ok(text_out(regexes::extract_file_paths(
            &input,
            boolean_value(arguments, "windows")?,
            boolean_value(arguments, "unix")?,
            present_flags(arguments)?,
            context,
        )?))
    }
}

/// Extracts fixed-length lowercase hex hash candidates.
pub struct ExtractHashes {
    spec: OperationSpec,
}

impl ExtractHashes {
    /// Creates the hash extract operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "extract.hashes@1",
                display_name: "Extract hashes",
                category: "Extractors",
                description: "Extracts potential hashes by fixed hex character length.",
                cyberchef_alias: Some("Extract hashes"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    integer_argument(
                        "hash_character_length",
                        "Hex character length (for example 32 or 40).",
                        40,
                    ),
                    boolean_argument(
                        "all_hashes",
                        "Search a fixed set of common hash bit lengths instead.",
                        false,
                    ),
                    boolean_argument(
                        "display_total",
                        "Prefix the result with a total count.",
                        false,
                    ),
                ],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for ExtractHashes {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ExtractHashes {
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
        let input = require_text(input)?;
        Ok(text_out(regexes::extract_hashes(
            &input,
            integer_value(arguments, "hash_character_length")?,
            boolean_value(arguments, "all_hashes")?,
            boolean_value(arguments, "display_total")?,
            context,
        )?))
    }
}

/// Extracts printable strings from the input.
pub struct Strings {
    spec: OperationSpec,
}

impl Strings {
    /// Creates the strings operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "extract.strings@1",
                display_name: "Strings",
                category: "Extractors",
                description: "Extracts printable strings from the input.",
                cyberchef_alias: Some("Strings"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![
                    text_argument(
                        "encoding",
                        "Single byte, 16-bit littleendian, 16-bit bigendian, or All.",
                        "Single byte",
                    ),
                    integer_argument("minimum_length", "Minimum string length.", 4),
                    text_argument(
                        "match",
                        "Character class selector (ASCII/Unicode printable variants).",
                        "All printable chars (A)",
                    ),
                    boolean_argument(
                        "display_total",
                        "Prefix the result with a total count.",
                        false,
                    ),
                    boolean_argument("sort", "Sort matches.", false),
                    boolean_argument("unique", "Deduplicate matches.", false),
                ],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for Strings {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Strings {
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
        let input = require_text(input)?;
        Ok(text_out(strings::extract(
            &input,
            text_value(arguments, "encoding")?,
            integer_value(arguments, "minimum_length")?,
            text_value(arguments, "match")?,
            present_flags(arguments)?,
            context,
        )?))
    }
}
