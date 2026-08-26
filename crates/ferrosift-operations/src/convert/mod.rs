//! Converting a quantity between units.
//!
//! Five operations with one body between them: multiply by the input unit's
//! factor, divide by the output unit's. Both steps happen on a decimal rather
//! than a float, which is the whole reason these waited on the
//! arbitrary-precision layer -- a mass in nanograms and a mass in tonnes are
//! twenty-one orders of magnitude apart, and a double has fifteen digits.
//!
//! The order of the two steps is load-bearing and is the reference's. A single
//! combined ratio would be one rounding where this is two, and the two do not
//! always agree in the twentieth place.
//!
//! The tables in [`units`] are generated from the pinned checkout rather than
//! typed out. Each factor is a JavaScript number literal there, and what the
//! reference's arithmetic receives is the double's shortest decimal form --
//! which is not always the text in the source, and is what the generator
//! records.

mod codec;
mod operation;
mod units;

pub use operation::ConvertUnits;
