//! Bounded process and filesystem I/O.

use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

use crate::error::CliError;

pub fn read_limited(
    path: &Path,
    standard_input: &mut dyn Read,
    limit: u64,
    too_large_code: &'static str,
) -> Result<Vec<u8>, CliError> {
    if path == Path::new("-") {
        read_from(standard_input, limit, too_large_code)
    } else {
        let mut file = File::open(path).map_err(|error| {
            CliError::new("cli.io.read", format!("{}: {error}", path.display()))
        })?;
        read_from(&mut file, limit, too_large_code)
    }
}

fn read_from(
    reader: &mut dyn Read,
    limit: u64,
    too_large_code: &'static str,
) -> Result<Vec<u8>, CliError> {
    let mut bytes = Vec::new();
    reader
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| CliError::new("cli.io.read", error.to_string()))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limit {
        Err(CliError::new(too_large_code, format!("limit={limit}")))
    } else {
        Ok(bytes)
    }
}

pub fn write_line(output: &mut dyn Write, line: &str) -> Result<(), CliError> {
    writeln!(output, "{line}").map_err(CliError::write)
}

pub fn write_output(
    path: &Path,
    standard_output: &mut dyn Write,
    bytes: &[u8],
) -> Result<(), CliError> {
    if path == Path::new("-") {
        standard_output.write_all(bytes).map_err(CliError::write)?;
        standard_output.flush().map_err(CliError::write)
    } else {
        let mut file = File::create(path).map_err(|error| {
            CliError::new("cli.io.write", format!("{}: {error}", path.display()))
        })?;
        file.write_all(bytes).map_err(|error| {
            CliError::new("cli.io.write", format!("{}: {error}", path.display()))
        })?;
        file.flush()
            .map_err(|error| CliError::new("cli.io.write", format!("{}: {error}", path.display())))
    }
}
