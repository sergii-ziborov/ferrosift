//! Stable process-facing failures.

use std::fmt;

use ferrosift_core::ExecutionError;

#[derive(Debug)]
pub struct CliError {
    code: String,
    detail: String,
}

impl CliError {
    pub fn new(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            detail: detail.into(),
        }
    }

    pub fn write(error: impl fmt::Display) -> Self {
        Self::new("cli.io.write", error.to_string())
    }

    pub fn execution(error: &ExecutionError) -> Self {
        let detail = error.location.as_ref().map_or_else(
            || String::from("recipe preflight"),
            |location| {
                format!(
                    "step={} id={} operation={}",
                    location.index,
                    location.step_id.as_str(),
                    location.operation.as_str()
                )
            },
        );
        Self::new(error.code(), detail)
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "ferrosift: {}: {}", self.code, self.detail)
    }
}
