//! PEM, the text wrapper around DER.
//!
//! A certificate on disk is base64 between two marker lines. Nothing here is
//! cryptography — it is framing — but the framing has details worth pinning
//! rather than reinventing: the reference writes CRLF line endings and a
//! trailing one, folds the body at sixty-four characters, and trims the fold's
//! trailing break so a body that divides evenly does not gain a blank line.
//!
//! Reading is deliberately more permissive than writing. A file can hold a
//! whole certificate chain with commentary between the blocks, so every block
//! is extracted and anything outside them is ignored — but a block that opens
//! and never closes is refused, because the bytes after it are not known to
//! belong to it.

mod codec;
mod operation;

pub use operation::{HexToPem, PemToHex};
