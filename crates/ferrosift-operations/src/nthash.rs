//! The NT hash, as Windows stores a password.
//!
//! MD4 over the password's UTF-16LE bytes, in upper-case hex. There is no
//! salt, no iteration count, and no work factor — the digest of a password is
//! a function of the password alone, so two accounts with the same password
//! have the same hash and a precomputed table covers every password anyone has
//! thought of. It is here because incident responders read these out of memory
//! dumps and registry hives every day, not because it is a reasonable way to
//! store a credential.
//!
//! The encoding is the part a port gets wrong. Hashing UTF-8 instead of
//! UTF-16LE agrees on every ASCII password and disagrees on every other one,
//! which is a bug that passes a careless test suite.

use alloc::vec;

use digest::Digest as _;
use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::hex_util::to_hex_lower;
use crate::jscompat::string::to_utf16le;
use crate::spec::{SpecDefinition, build};
use crate::value::{take_text, text as text_output};

/// Computes the NT hash of a password.
pub struct NtHash {
    spec: OperationSpec,
}

impl NtHash {
    /// Creates the NT hash operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "hash.nt@1",
                display_name: "NT Hash",
                category: "Hashing",
                description: "Computes an NT (NTLM) password hash: MD4 over the UTF-16LE encoding, as upper-case hex.",
                cyberchef_alias: Some("NT Hash"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for NtHash {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for NtHash {
    fn spec(&self) -> &OperationSpec {
        &self.spec
    }

    fn execute(
        &self,
        input: Value,
        _arguments: &Arguments,
        context: &mut OperationContext<'_>,
    ) -> Result<Value, OperationError> {
        context.ensure_active()?;
        let input = take_text(input)?;
        let digest = md4::Md4::digest(to_utf16le(&input));
        context.ensure_active()?;
        // Upper case, which is how Windows tooling prints it and what the
        // reference emits; every other digest in this catalog is lower case.
        Ok(text_output(to_hex_lower(&digest).to_uppercase()))
    }
}
