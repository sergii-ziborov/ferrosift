//! Computing an expression against the fields already read.
//!
//! Everything folds to a single signed integer. That is what the language this
//! grammar follows does -- comparisons yield 0 or 1, and a value is true when
//! it is not zero -- so a separate boolean type would have to be converted at
//! every operator rather than at the two places that actually ask.

use crate::ast::{BinaryOperator, Expression, SizeOfTarget, UnaryOperator};
use crate::error::{PatternError, Position};

use super::value::{Node, NodeValue};

pub(super) const UNKNOWN_FIELD: &str = "pattern.eval.unknown_field";
pub(super) const NOT_A_NUMBER: &str = "pattern.eval.not_a_number";
pub(super) const ARITHMETIC_OVERFLOW: &str = "pattern.eval.arithmetic_overflow";
pub(super) const DIVIDE_BY_ZERO: &str = "pattern.eval.divide_by_zero";

/// What an expression can see: the fields read so far, and where we are.
#[derive(Clone, Copy)]
pub(super) struct Scope<'a> {
    /// Fields already read in the enclosing composite, in layout order.
    ///
    /// Only *earlier* fields are present. A member cannot refer to one that
    /// comes after it, because the bytes have not been read and the offset is
    /// not yet known -- and that is a property of the layout, not a
    /// restriction this crate adds.
    pub(super) siblings: &'a [Node],
    /// The offset the field being evaluated begins at, for `$`.
    pub(super) offset: u64,
}

impl Scope<'_> {
    /// An empty scope, for expressions folded before any data is read.
    pub(super) const EMPTY: Scope<'static> = Scope {
        siblings: &[],
        offset: 0,
    };
}

/// Computes an expression that cannot refer to any field.
///
/// Enum values and bit widths are fixed by the source, so they are folded once
/// while parsing rather than re-computed per read. An expression that reaches
/// for a field here fails with the same `unknown_field` code it would at
/// evaluation time -- there simply is no field to find.
pub(crate) fn fold(expression: &Expression) -> Result<i128, PatternError> {
    evaluate(expression, Scope::EMPTY)
}

/// Computes `expression` in `scope`.
pub(super) fn evaluate(expression: &Expression, scope: Scope<'_>) -> Result<i128, PatternError> {
    match expression {
        Expression::Integer(value) => {
            i128::try_from(*value).map_err(|_| fail(ARITHMETIC_OVERFLOW, "literal exceeds 127 bits"))
        }
        Expression::Bool(value) => Ok(i128::from(*value)),
        Expression::Char(value) => Ok(i128::from(u32::from(*value))),
        Expression::Offset => Ok(i128::from(scope.offset)),
        Expression::Path(segments) => number(resolve(segments, scope)?),
        Expression::SizeOf(target) => size_of(target, scope),
        Expression::Unary { operator, operand } => {
            let value = evaluate(operand, scope)?;
            Ok(match operator {
                UnaryOperator::Negate => value
                    .checked_neg()
                    .ok_or_else(|| fail(ARITHMETIC_OVERFLOW, "negation overflows"))?,
                UnaryOperator::Complement => !value,
                UnaryOperator::Not => i128::from(value == 0),
            })
        }
        Expression::Binary {
            operator,
            left,
            right,
        } => binary(*operator, left, right, scope),
        Expression::Conditional {
            condition,
            when_true,
            when_false,
        } => {
            // Only the taken branch is computed. A pattern routinely guards a
            // division or a field reference with the test beside it, so
            // evaluating both would fail on expressions that are correct.
            if evaluate(condition, scope)? == 0 {
                evaluate(when_false, scope)
            } else {
                evaluate(when_true, scope)
            }
        }
    }
}

/// Applies one infix operator, short-circuiting the logical pair.
fn binary(
    operator: BinaryOperator,
    left: &Expression,
    right: &Expression,
    scope: Scope<'_>,
) -> Result<i128, PatternError> {
    // `&&` and `||` decide on the left operand alone when they can, which is
    // what makes `count != 0 && total / count > 4` safe to write.
    match operator {
        BinaryOperator::And => {
            return if evaluate(left, scope)? == 0 {
                Ok(0)
            } else {
                Ok(i128::from(evaluate(right, scope)? != 0))
            };
        }
        BinaryOperator::Or => {
            return if evaluate(left, scope)? == 0 {
                Ok(i128::from(evaluate(right, scope)? != 0))
            } else {
                Ok(1)
            };
        }
        _ => {}
    }

    let a = evaluate(left, scope)?;
    let b = evaluate(right, scope)?;
    let overflow = || fail(ARITHMETIC_OVERFLOW, "arithmetic overflows 128 bits");
    Ok(match operator {
        BinaryOperator::Multiply => a.checked_mul(b).ok_or_else(overflow)?,
        BinaryOperator::Divide => a
            .checked_div(b)
            .ok_or_else(|| divide_failure(b))?,
        BinaryOperator::Remainder => a
            .checked_rem(b)
            .ok_or_else(|| divide_failure(b))?,
        BinaryOperator::Add => a.checked_add(b).ok_or_else(overflow)?,
        BinaryOperator::Subtract => a.checked_sub(b).ok_or_else(overflow)?,
        BinaryOperator::ShiftLeft => shift(a, b, true)?,
        BinaryOperator::ShiftRight => shift(a, b, false)?,
        BinaryOperator::Less => i128::from(a < b),
        BinaryOperator::LessEqual => i128::from(a <= b),
        BinaryOperator::Greater => i128::from(a > b),
        BinaryOperator::GreaterEqual => i128::from(a >= b),
        BinaryOperator::Equal => i128::from(a == b),
        BinaryOperator::NotEqual => i128::from(a != b),
        BinaryOperator::BitAnd => a & b,
        BinaryOperator::BitXor => a ^ b,
        BinaryOperator::BitOr => a | b,
        BinaryOperator::And | BinaryOperator::Or => unreachable!("handled above"),
    })
}

/// Distinguishes a zero divisor from the one overflowing division.
fn divide_failure(divisor: i128) -> PatternError {
    if divisor == 0 {
        fail(DIVIDE_BY_ZERO, "division by zero")
    } else {
        fail(ARITHMETIC_OVERFLOW, "division overflows")
    }
}

/// Shifts, rejecting a distance the width cannot express.
///
/// A shift of 128 or more is undefined in the C this follows, so it is a
/// failure here rather than a silently wrapped distance.
fn shift(value: i128, distance: i128, left: bool) -> Result<i128, PatternError> {
    let distance = u32::try_from(distance)
        .ok()
        .filter(|amount| *amount < 128)
        .ok_or_else(|| fail(ARITHMETIC_OVERFLOW, "shift distance is out of range"))?;
    if left {
        value
            .checked_shl(distance)
            .ok_or_else(|| fail(ARITHMETIC_OVERFLOW, "shift overflows"))
    } else {
        value
            .checked_shr(distance)
            .ok_or_else(|| fail(ARITHMETIC_OVERFLOW, "shift overflows"))
    }
}

/// The byte width a `sizeof` asks about.
fn size_of(target: &SizeOfTarget, scope: Scope<'_>) -> Result<i128, PatternError> {
    Ok(match target {
        SizeOfTarget::Builtin(builtin) => i128::from(builtin.size()),
        SizeOfTarget::Path(segments) => i128::from(resolve(segments, scope)?.size),
    })
}

/// Walks a dotted path through the fields already read.
fn resolve<'a>(segments: &[alloc::string::String], scope: Scope<'a>) -> Result<&'a Node, PatternError> {
    let mut names = segments.iter();
    let first = names
        .next()
        .ok_or_else(|| fail(UNKNOWN_FIELD, "empty field path"))?;
    let mut node = scope
        .siblings
        .iter()
        .find(|sibling| sibling.name == *first)
        .ok_or_else(|| fail(UNKNOWN_FIELD, "field is not readable from here"))?;
    for name in names {
        node = node
            .child(name)
            .ok_or_else(|| fail(UNKNOWN_FIELD, "no such member"))?;
    }
    Ok(node)
}

/// Reads a node as a number, or explains why it is not one.
fn number(node: &Node) -> Result<i128, PatternError> {
    match &node.value {
        NodeValue::Unsigned(value) | NodeValue::Enumerator { value, .. } => i128::try_from(*value)
            .map_err(|_| fail(ARITHMETIC_OVERFLOW, "value exceeds 127 bits")),
        NodeValue::Signed(value) => Ok(*value),
        NodeValue::Bool(value) => Ok(i128::from(*value)),
        NodeValue::Char(value) => Ok(i128::from(u32::from(*value))),
        NodeValue::Float(_) | NodeValue::Double(_) => Err(fail(
            NOT_A_NUMBER,
            "floating-point values cannot be used in a pattern expression",
        )),
        NodeValue::Group(_) => Err(fail(NOT_A_NUMBER, "a composite has no numeric value")),
    }
}

fn fail(code: &'static str, detail: &'static str) -> PatternError {
    PatternError::new(code, Position { line: 0, column: 0 }, detail)
}
