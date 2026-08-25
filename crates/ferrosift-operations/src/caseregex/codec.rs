use alloc::string::String;
use alloc::vec::Vec;

use ferrosift_core::{OperationContext, OperationError};

/// Rewrites a regular expression so it matches either case without the `i`
/// flag.
///
/// Two stages. First every letter that is not part of a range becomes a
/// two-element class: `a` becomes `[aA]`. Then nine passes widen the ranges
/// that survived, one shape per pass — `A-Z` gains `a-z`, `H-d` gains the
/// letters on both sides, and so on.
///
/// The passes run in sequence over each other's output and each replaces every
/// match, left to right. That order is not decoration: a later pass can match
/// text an earlier one produced, and reordering them changes the result.
pub(super) fn to_case_insensitive(
    input: &str,
    context: &OperationContext<'_>,
) -> Result<String, OperationError> {
    context.ensure_active()?;
    let mut characters = pre_process(input);
    for (index, pass) in PASSES.iter().enumerate() {
        if index.is_multiple_of(4) {
            context.ensure_active()?;
        }
        characters = apply(&characters, *pass);
    }
    context.ensure_active()?;
    Ok(characters.into_iter().collect())
}

/// Wraps each standalone letter in a class holding both of its cases.
///
/// A letter next to a hyphen is left alone: it is one end of a range, and
/// bracketing it would break the range rather than widen it. The reference
/// checks the neighbours directly because JavaScript had no look-behind when
/// this was written.
fn pre_process(input: &str) -> Vec<char> {
    let source: Vec<char> = input.chars().collect();
    let mut output = Vec::with_capacity(source.len());
    for (index, character) in source.iter().enumerate() {
        let before = index.checked_sub(1).and_then(|i| source.get(i));
        let after = source.get(index + 1);
        let bounded = before == Some(&'-') || after == Some(&'-');
        if character.is_ascii_alphabetic() && !bounded {
            output.push('[');
            output.extend(character.to_lowercase());
            output.extend(character.to_uppercase());
            output.push(']');
        } else {
            output.push(*character);
        }
    }
    output
}

/// One widening pass: a shape to recognise and a rule for what to emit.
#[derive(Clone, Copy)]
struct Pass {
    /// Whether a literal backslash may lead the match.
    escapable: bool,
    /// The class the range's low end must fall in.
    low: Class,
    /// The class the range's high end must fall in.
    high: Class,
    /// Whether the high end may itself be led by a backslash.
    high_escapable: bool,
    /// What to append, and in which order.
    rule: Rule,
}

/// The character ranges the passes discriminate on.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Upper,
    Lower,
    /// `[ -@]`: space through `@`, everything before the upper-case letters.
    BeforeUpper,
    /// ``[[-`]``: the six characters between the two letter blocks.
    BetweenLetters,
    /// `[{-~]`: the four characters after the lower-case letters.
    AfterLower,
    /// Either letter class, used only by the first pass's alternation.
    EitherLetter,
}

impl Class {
    fn holds(self, character: char) -> bool {
        match self {
            Self::Upper => character.is_ascii_uppercase(),
            Self::Lower => character.is_ascii_lowercase(),
            Self::BeforeUpper => (' '..='@').contains(&character),
            Self::BetweenLetters => ('['..='`').contains(&character),
            Self::AfterLower => ('{'..='~').contains(&character),
            Self::EitherLetter => character.is_ascii_alphabetic(),
        }
    }
}

/// What a pass appends once it has matched.
///
/// `first` and `third` name the match's first and third characters, which is
/// how the reference indexes them — and where an optional backslash is present
/// those indices slide, so `\!-D` is read as first `\` and third `-`. That is
/// reproduced rather than corrected: it is what the reference emits.
#[derive(Clone, Copy)]
enum Rule {
    /// `A-Z` to `A-Za-z`: both cases of the same range.
    BothCases,
    /// `H-d` to `A-DH-dh-z`: the range plus the letters outside it.
    SpanLetters,
    /// `!-D` to `!-Da-d`: keep the match, add the lower-case tail.
    AddLowerTail,
    /// `%-^` to `%-^a-z`: keep the match, add the whole lower-case block.
    AddLowerBlock,
    /// ``K-` `` to ``K-`k-z``: keep the match, lower-case from its start.
    AddLowerFromFirst,
    /// `[-}` to `[-}A-Z`: keep the match, add the whole upper-case block.
    AddUpperBlock,
    /// `b-}` to `b-}B-Z`: keep the match, upper-case from its start.
    AddUpperFromFirst,
    /// `<-j` to `<-z`: replace the match's tail with the lower-case end.
    ExtendToLower,
    /// `^-j` to `A-J^-j`: prepend the upper-case head.
    PrependUpper,
}

/// The nine widening passes, in the order the reference applies them.
const PASSES: &[Pass] = &[
    Pass {
        escapable: false,
        low: Class::EitherLetter,
        high: Class::EitherLetter,
        high_escapable: false,
        rule: Rule::BothCases,
    },
    Pass {
        escapable: false,
        low: Class::Upper,
        high: Class::Lower,
        high_escapable: false,
        rule: Rule::SpanLetters,
    },
    Pass {
        escapable: true,
        low: Class::BeforeUpper,
        high: Class::Upper,
        high_escapable: false,
        rule: Rule::AddLowerTail,
    },
    Pass {
        escapable: true,
        low: Class::BeforeUpper,
        high: Class::BetweenLetters,
        high_escapable: true,
        rule: Rule::AddLowerBlock,
    },
    Pass {
        escapable: false,
        low: Class::Upper,
        high: Class::BetweenLetters,
        high_escapable: true,
        rule: Rule::AddLowerFromFirst,
    },
    Pass {
        escapable: true,
        low: Class::BetweenLetters,
        high: Class::AfterLower,
        high_escapable: true,
        rule: Rule::AddUpperBlock,
    },
    Pass {
        escapable: false,
        low: Class::Lower,
        high: Class::AfterLower,
        high_escapable: true,
        rule: Rule::AddUpperFromFirst,
    },
    Pass {
        escapable: true,
        low: Class::BeforeUpper,
        high: Class::Lower,
        high_escapable: false,
        rule: Rule::ExtendToLower,
    },
    Pass {
        escapable: true,
        low: Class::BetweenLetters,
        high: Class::Lower,
        high_escapable: false,
        rule: Rule::PrependUpper,
    },
];

/// Runs one pass over the whole string, replacing every match left to right.
fn apply(source: &[char], pass: Pass) -> Vec<char> {
    let mut output: Vec<char> = Vec::with_capacity(source.len());
    let mut index = 0;
    while index < source.len() {
        if let Some(length) = match_at(source, index, pass) {
            let matched = &source[index..index + length];
            emit(&mut output, matched, pass.rule);
            index += length;
        } else {
            output.push(source[index]);
            index += 1;
        }
    }
    output
}

/// Reports the length of a match starting at `index`, if there is one.
///
/// The first pass is the one alternation in the set: it accepts two upper-case
/// ends or two lower-case ends, but never one of each. Expressing that as a
/// same-case check keeps the table flat instead of adding a second entry that
/// differs only in case.
fn match_at(source: &[char], index: usize, pass: Pass) -> Option<usize> {
    let mut cursor = index;
    let mut length = 0;
    if pass.escapable && source.get(cursor) == Some(&'\\') {
        cursor += 1;
        length += 1;
    }
    let low = *source.get(cursor)?;
    if !pass.low.holds(low) {
        return None;
    }
    if source.get(cursor + 1) != Some(&'-') {
        return None;
    }
    cursor += 2;
    length += 2;
    if pass.high_escapable && source.get(cursor) == Some(&'\\') {
        cursor += 1;
        length += 1;
    }
    let high = *source.get(cursor)?;
    if !pass.high.holds(high) {
        return None;
    }
    if pass.low == Class::EitherLetter
        && low.is_ascii_uppercase() != high.is_ascii_uppercase()
    {
        return None;
    }
    Some(length + 1)
}

/// Appends one match's replacement.
fn emit(output: &mut Vec<char>, matched: &[char], rule: Rule) {
    let first = matched[0];
    // The reference indexes the third character of the match, which slides
    // when an optional backslash was consumed. Reading the same position keeps
    // the agreement.
    let third = matched.get(2).copied().unwrap_or(first);
    match rule {
        Rule::BothCases => {
            output.extend(first.to_uppercase());
            output.push('-');
            output.extend(third.to_uppercase());
            output.extend(first.to_lowercase());
            output.push('-');
            output.extend(third.to_lowercase());
        }
        Rule::SpanLetters => {
            output.push('A');
            output.push('-');
            output.extend(third.to_uppercase());
            output.extend_from_slice(matched);
            output.extend(first.to_lowercase());
            output.push('-');
            output.push('z');
        }
        Rule::AddLowerTail => {
            output.extend_from_slice(matched);
            output.push('a');
            output.push('-');
            output.extend(third.to_lowercase());
        }
        Rule::AddLowerBlock => {
            output.extend_from_slice(matched);
            output.extend(['a', '-', 'z']);
        }
        Rule::AddLowerFromFirst => {
            output.extend_from_slice(matched);
            output.extend(first.to_lowercase());
            output.push('-');
            output.push('z');
        }
        Rule::AddUpperBlock => {
            output.extend_from_slice(matched);
            output.extend(['A', '-', 'Z']);
        }
        Rule::AddUpperFromFirst => {
            output.extend_from_slice(matched);
            output.extend(first.to_uppercase());
            output.push('-');
            output.push('Z');
        }
        Rule::ExtendToLower => {
            output.push(first);
            output.push('-');
            output.push('z');
        }
        Rule::PrependUpper => {
            output.push('A');
            output.push('-');
            output.extend(third.to_uppercase());
            output.extend_from_slice(matched);
        }
    }
}
