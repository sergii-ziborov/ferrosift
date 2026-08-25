mod alphabet;
mod codec;
mod operation;

pub use operation::{FromBase64, ToBase64};

use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

/// The alphabet every caller outside this module wants.
const STANDARD: &str = "A-Za-z0-9+/=";

/// Base64 with the standard alphabet, for operations that embed it.
///
/// PEM is the caller this exists for: it is base64 in a wrapper, and having it
/// grow a second encoder would be two implementations of one thing that could
/// then disagree about padding.
///
/// # Errors
///
/// Returns an error if the output would exceed the execution budget.
pub(crate) fn encode_standard(
    input: &[u8],
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    codec::encode(input, &alphabet::Alphabet::parse(STANDARD)?, context)
}

/// Base64 decoding that drops anything outside the alphabet.
///
/// Lenient because the callers are parsing text that is expected to carry line
/// breaks and framing — refusing those would refuse every real PEM file.
///
/// # Errors
///
/// Returns an error if what remains is not a whole number of base64 groups.
pub(crate) fn decode_standard(
    input: &str,
    context: &OperationContext<'_>,
) -> Result<Vec<u8>, OperationError> {
    codec::decode(
        input,
        &alphabet::Alphabet::parse(STANDARD)?,
        true,
        false,
        context,
    )
}
