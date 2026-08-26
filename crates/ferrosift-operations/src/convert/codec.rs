use ferrosift_model::DecimalValue;

use crate::jscompat::bignumber;

/// The factor for one unit, or `None` for a name the table has no entry for.
///
/// The reference's unit *lists* carry group markers -- `[Metric]`,
/// `[/Metric]` and their like -- which its interface renders as separators
/// and which have no factor. Naming one is therefore not a unit, and this
/// answers the same for a marker as for a misspelling: nothing.
pub(crate) fn factor(table: &[(&str, &str)], name: &str) -> Option<DecimalValue> {
    table
        .iter()
        .find(|(unit, _)| *unit == name)
        .map(|(_, value)| DecimalValue::parse(value))
}

/// Converts a quantity from one unit to another.
///
/// Two steps, in the reference's order: multiply by the input's factor, then
/// divide by the output's. Combining them into a single ratio first would
/// round once where this rounds once -- but at a different place, because the
/// intermediate is a different number. The multiplication is exact and only
/// the division rounds, so the answer is the reference's exactly when the
/// order is.
pub(crate) fn convert(
    value: &DecimalValue,
    from: &DecimalValue,
    to: &DecimalValue,
) -> DecimalValue {
    bignumber::divide(&bignumber::times(value, from), to)
}
