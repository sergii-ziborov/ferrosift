//! Deterministic recipe suggestions (Magic-as-advisor).
//!
//! This never mutates the input into a "best" decode. It ranks portable
//! operations already in the catalog and reports candidate recipes.
//!
//! The search itself lives in [`super::probes`], the ranking and report in
//! [`super::scoring`], and the shape detectors in [`super::detect`].

use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};
use ferrosift_model::Value;

use super::model::{MAX_DEPTH, MAX_RESULTS_CAP, Options, ensure_budget};
use super::probes::explore;
use super::scoring::render_report;
use crate::failure::failed;

const INVALID_DEPTH: &str = "analysis.suggest.invalid_depth";
const INVALID_MAX_RESULTS: &str = "analysis.suggest.invalid_max_results";

pub(super) fn suggest(
    input: Value,
    depth: i128,
    max_results: i128,
    intensive: bool,
    crib: &str,
    context: &mut OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let options = parse_options(depth, intensive)?;
    let max_results = parse_max_results(max_results)?;

    let (bytes, text) = normalize_input(input)?;
    let mut hits = Vec::new();
    explore(&bytes, text.as_deref(), options, &mut hits, context)?;

    super::scoring::rank(&mut hits, crib, max_results);

    let report = render_report(&bytes, &hits);
    ensure_budget(report.len(), context)?;
    context.ensure_active()?;
    Ok(report)
}

fn parse_options(depth: i128, intensive: bool) -> Result<Options, OperationError> {
    if !(1..=MAX_DEPTH).contains(&depth) {
        return Err(failed(INVALID_DEPTH));
    }
    Ok(Options {
        depth: usize::try_from(depth).unwrap_or(1),
        intensive,
    })
}

fn parse_max_results(max_results: i128) -> Result<usize, OperationError> {
    if max_results <= 0 || max_results > MAX_RESULTS_CAP {
        return Err(failed(INVALID_MAX_RESULTS));
    }
    usize::try_from(max_results).map_err(|_| failed(INVALID_MAX_RESULTS))
}

fn normalize_input(input: Value) -> Result<(Vec<u8>, Option<String>), OperationError> {
    match input {
        Value::Bytes(bytes) => {
            let text = core::str::from_utf8(&bytes).ok().map(String::from);
            Ok((bytes, text))
        }
        Value::Text(text) => Ok((text.text.as_bytes().to_vec(), Some(text.text))),
        _ => Err(OperationError::InvalidArguments),
    }
}
