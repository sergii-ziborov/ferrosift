//! Operations whose output comes from their arguments, not their input.
//!
//! These are the first users of [`ferrosift_model::OutputBehavior`]'s
//! `InputIndependent` variant. The executor's expansion ratio does not apply
//! to them, so each one is responsible for bounding its own output — which
//! every operation here does before allocating anything.

mod codec;
mod operation;

pub use operation::GenerateDeBruijnSequence;
