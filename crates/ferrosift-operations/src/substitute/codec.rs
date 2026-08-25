use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};

/// Applies a monoalphabetic substitution.
///
/// Mismatched alphabets are a warning rather than a refusal: the reference
/// prefixes a line to the output and carries on with as many pairs as the
/// shorter of the two provides. Refusing would be the better design and would
/// also be a different operation, so the warning is reproduced.
pub(super) fn substitute(
    input: &str,
    plaintext: &[char],
    ciphertext: &[char],
    ignore_case: bool,
) -> String {
    let mut output = String::with_capacity(input.len());
    if plaintext.len() != ciphertext.len() {
        output.push_str("Warning: Plaintext and Ciphertext lengths differ\n\n");
    }

    let mut dictionary: BTreeMap<char, char> = BTreeMap::new();
    for (from, to) in plaintext.iter().zip(ciphertext.iter()) {
        // A plaintext character listed twice keeps its *last* mapping: the
        // reference assigns into an object in order, so a later write wins,
        // and `insert` replaces for the same reason. `zip` also stops at the
        // shorter alphabet, which is the bound the reference computes
        // explicitly.
        dictionary.insert(*from, *to);
    }

    for character in input.chars() {
        output.push_str(&substitute_one(character, &dictionary, ignore_case));
    }
    output
}

/// One character, with the case rules the reference applies.
///
/// Case-insensitive mode decides the output case from the *input* character,
/// not from the alphabet, and it treats "equal to its own upper-casing" as
/// upper — so an uncased character counts as upper and takes that branch.
fn substitute_one(character: char, dictionary: &BTreeMap<char, char>, ignore_case: bool) -> String {
    if !ignore_case {
        return dictionary
            .get(&character)
            .copied()
            .unwrap_or(character)
            .to_string();
    }

    let upper: String = character.to_uppercase().collect();
    let is_upper = upper.chars().eq(core::iter::once(character));

    if let Some(mapped) = dictionary.get(&character) {
        return case_of(*mapped, is_upper);
    }
    // Try the other case of the input character.
    let other: String = if is_upper {
        character.to_lowercase().collect()
    } else {
        upper
    };
    if let Some(single) = single_char(&other)
        && let Some(mapped) = dictionary.get(&single)
    {
        return case_of(*mapped, is_upper);
    }
    character.to_string()
}

fn case_of(value: char, upper: bool) -> String {
    if upper {
        value.to_uppercase().collect()
    } else {
        value.to_lowercase().collect()
    }
}

/// The single character of a string, when its case mapping did not grow it.
fn single_char(value: &str) -> Option<char> {
    let mut characters = value.chars();
    let first = characters.next()?;
    characters.next().is_none().then_some(first)
}
