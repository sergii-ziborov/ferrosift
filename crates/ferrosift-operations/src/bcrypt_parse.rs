use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use ferrosift_core::{Operation, OperationContext, OperationError};
use ferrosift_model::{Arguments, OperationSpec, Value, ValueConstraint, ValueKind};

use crate::failure::failed;
use crate::spec::{SpecDefinition, build};
use crate::value::{take_text, text as text_output};

/// Where the salt ends in a bcrypt hash.
///
/// The reference's library takes the first twenty-nine characters and calls
/// them the salt: `$2a$10$` and the twenty-two that encode the sixteen salt
/// bytes. It does not parse them — it counts — which is why a hash whose
/// *total* length is wrong is refused while one whose contents are nonsense is
/// not.
const SALT_LENGTH: usize = 29;

/// How long a bcrypt hash is, in UTF-16 code units, exactly.
const HASH_LENGTH: usize = 60;

/// What a hash of the wrong length is refused with.
///
/// The only refusal the reference has: everything it can throw here comes from
/// the length check, and every other malformation is printed rather than
/// rejected. The one below it is this port's own, and says why.
const MALFORMED: &str = "hash.bcrypt.parse.malformed";

/// Splits a bcrypt hash into the parts it is made of.
///
/// No bcrypt is computed here and none is needed: the operation reads a hash
/// rather than producing one, so this is string arithmetic on a fixed layout.
/// That is also why it is not behind the `hash` pack — there is no digest to
/// pull a dependency in for.
///
/// Three details of that arithmetic are the reference's rather than the
/// obvious ones, and each is a place a plausible port would differ:
///
/// * The length is counted in UTF-16 code units, because that is what
///   JavaScript's `length` counts. Fifty-nine ASCII characters and one emoji
///   is sixty characters and sixty-one units, and the reference refuses it.
/// * The rounds are read with `parseInt` and *printed*, not validated. A hash
///   with no third field prints `Rounds: NaN` and succeeds, so long as it is
///   sixty units long.
/// * The password hash is `input.split(salt)[1]`, which is not the same as
///   "everything after the salt". When the salt occurs a second time — which
///   fits, twice twenty-nine being under sixty — the second field ends where
///   the repeat begins, so sixty identical characters give an empty one.
pub struct BcryptParse {
    spec: OperationSpec,
}

impl BcryptParse {
    /// Creates the bcrypt-parse operation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: build(SpecDefinition {
                id: "hash.bcrypt.parse@1",
                display_name: "Bcrypt parse",
                category: "Hashing",
                description: "Splits a bcrypt hash into its rounds, salt, and digest.",
                cyberchef_alias: Some("Bcrypt parse"),
                input: ValueConstraint::Exact(ValueKind::Text),
                output: ValueConstraint::Exact(ValueKind::Text),
                arguments: vec![],
                inverse: None,
                classifications: None,
            }),
        }
    }
}

impl Default for BcryptParse {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for BcryptParse {
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

        // `parseInt` on the third dollar-separated field, and on nothing at all
        // when there is no third field. Neither refuses: both print.
        let rounds = crate::jscompat::double::format(
            input
                .split('$')
                .nth(2)
                .map_or(f64::NAN, crate::jscompat::number::parse_decimal),
        );

        let units: Vec<u16> = input.encode_utf16().collect();
        if units.len() != HASH_LENGTH {
            return Err(failed(MALFORMED));
        }
        let salt = &units[..SALT_LENGTH];
        let rest = &units[SALT_LENGTH..];
        // `String.prototype.split` cuts at every non-overlapping occurrence and
        // the second field is what lies between the first two. The first is at
        // zero by construction, so this looks for the next one.
        let digest = match find(rest, salt) {
            Some(at) => &rest[..at],
            None => rest,
        };

        let salt = decode(salt)?;
        let digest = decode(digest)?;
        context.ensure_active()?;
        Ok(text_output(alloc::format!(
            "Rounds: {rounds}\nSalt: {salt}\nPassword hash: {digest}\nFull hash: {input}"
        )))
    }
}

/// Where `needle` first occurs in `haystack`, if it does.
fn find(haystack: &[u16], needle: &[u16]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// The code units as a Rust string, or a refusal.
///
/// Cutting at a fixed offset can land between the halves of a surrogate pair,
/// and JavaScript would hand back a string holding one half. Rust has no such
/// string, so this reports that it cannot rather than substituting a
/// replacement character and calling the result the reference's output.
fn decode(units: &[u16]) -> Result<String, OperationError> {
    String::from_utf16(units).map_err(|_| failed("hash.bcrypt.parse.unpaired_surrogate"))
}
