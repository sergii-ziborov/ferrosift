//! Stable failure codes shared by every AES mode.

use ferrosift_core::OperationError;

use crate::failure::failed;

pub(super) const INVALID_KEY: &str = "crypto.aes.invalid_key_length";
pub(super) const INVALID_MODE: &str = "crypto.aes.invalid_mode";
pub(super) const INVALID_LENGTH: &str = "crypto.aes.invalid_length";
pub(super) const DECRYPT_FAILED: &str = "crypto.aes.decrypt_failed";
pub(super) const INVALID_IV: &str = "crypto.aes.invalid_iv";

/// Maps any backend error to the stable length code.
pub(super) fn fail_len<E>(_: E) -> OperationError {
    failed(INVALID_LENGTH)
}

/// Maps any backend error to the stable decryption code.
///
/// Decryption failures are deliberately indistinguishable: a wrong key, a
/// wrong tag, and corrupt padding all report the same code so the error does
/// not become an oracle.
pub(super) fn fail_dec<E>(_: E) -> OperationError {
    failed(DECRYPT_FAILED)
}

pub(super) fn invalid_key() -> OperationError {
    failed(INVALID_KEY)
}
