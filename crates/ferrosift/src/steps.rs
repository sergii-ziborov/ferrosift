use alloc::collections::BTreeMap;
use alloc::string::String;

use ferrosift_model::{ArgumentValue, Arguments};

use crate::pipeline::Pipeline;

/// Typed convenience steps for the most common operations.
///
/// Each one is exactly [`Pipeline::step`] with the operation's canonical ID
/// and its default arguments, so anything reachable here is also reachable
/// through the escape hatch, and nothing here changes execution semantics.
impl Pipeline {
    /// Decodes Base64 text.
    #[must_use]
    pub fn from_base64(self) -> Self {
        self.step("encoding.base64.decode@1", Arguments::new())
    }

    /// Encodes bytes as Base64 text.
    #[must_use]
    pub fn to_base64(self) -> Self {
        self.step("encoding.base64.encode@1", Arguments::new())
    }

    /// Decodes Base32 text.
    #[must_use]
    pub fn from_base32(self) -> Self {
        self.step("encoding.base32.decode@1", Arguments::new())
    }

    /// Decodes Base58 text.
    #[must_use]
    pub fn from_base58(self) -> Self {
        self.step("encoding.base58.decode@1", Arguments::new())
    }

    /// Decodes Base85 text.
    #[must_use]
    pub fn from_base85(self) -> Self {
        self.step("encoding.base85.decode@1", Arguments::new())
    }

    /// Decodes hexadecimal text, detecting the delimiter automatically.
    #[must_use]
    pub fn from_hex(self) -> Self {
        self.step("encoding.hex.decode@1", Arguments::new())
    }

    /// Encodes bytes as space-delimited hexadecimal text.
    #[must_use]
    pub fn to_hex(self) -> Self {
        self.step("encoding.hex.encode@1", Arguments::new())
    }

    /// Decodes percent-encoded URL text.
    #[must_use]
    pub fn url_decode(self) -> Self {
        self.step("encoding.url.decode@1", Arguments::new())
    }

    /// Percent-encodes bytes as URL text.
    #[must_use]
    pub fn url_encode(self) -> Self {
        self.step("encoding.url.encode@1", Arguments::new())
    }

    /// Decompresses a gzip stream.
    #[cfg(feature = "compression")]
    #[must_use]
    pub fn gunzip(self) -> Self {
        self.step("compression.gunzip@1", Arguments::new())
    }

    /// Compresses bytes into a gzip stream.
    #[cfg(feature = "compression")]
    #[must_use]
    pub fn gzip(self) -> Self {
        self.step("compression.gzip@1", Arguments::new())
    }

    /// Decompresses a zlib stream.
    #[cfg(feature = "compression")]
    #[must_use]
    pub fn zlib_inflate(self) -> Self {
        self.step("compression.zlib.inflate@1", Arguments::new())
    }

    /// Decompresses a raw DEFLATE stream.
    #[cfg(feature = "compression")]
    #[must_use]
    pub fn raw_inflate(self) -> Self {
        self.step("compression.raw.inflate@1", Arguments::new())
    }

    /// Decompresses a bzip2 stream.
    #[cfg(feature = "compression")]
    #[must_use]
    pub fn bzip2_decompress(self) -> Self {
        self.step("compression.bzip2.decompress@1", Arguments::new())
    }

    /// XORs the input with a repeating key using the standard scheme.
    #[must_use]
    pub fn xor(self, key: &[u8]) -> Self {
        let arguments = Arguments::from([("key".into(), toggle_string("Hex", &hex(key)))]);
        self.step("logic.xor@1", arguments)
    }

    /// Rotates alphabetic characters by 13 places.
    #[must_use]
    pub fn rot13(self) -> Self {
        self.step("encoding.rot13@1", Arguments::new())
    }

    /// Hashes the input with MD5, producing lower-case hexadecimal text.
    #[cfg(feature = "hash")]
    #[must_use]
    pub fn md5(self) -> Self {
        self.step("hash.md5@1", Arguments::new())
    }

    /// Hashes the input with SHA-1, producing lower-case hexadecimal text.
    #[cfg(feature = "hash")]
    #[must_use]
    pub fn sha1(self) -> Self {
        self.step("hash.sha1@1", Arguments::new())
    }

    /// Hashes the input with SHA-2, producing lower-case hexadecimal text.
    #[cfg(feature = "hash")]
    #[must_use]
    pub fn sha2(self) -> Self {
        self.step("hash.sha2@1", Arguments::new())
    }

    /// Passes the value through unchanged.
    #[must_use]
    pub fn identity(self) -> Self {
        self.step("core.identity@1", Arguments::new())
    }
}

/// Builds a `CyberChef` toggleString argument.
fn toggle_string(option: &str, value: &str) -> ArgumentValue {
    ArgumentValue::Map(BTreeMap::from([
        (
            String::from("option"),
            ArgumentValue::Text(String::from(option)),
        ),
        (
            String::from("string"),
            ArgumentValue::Text(String::from(value)),
        ),
    ]))
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}
