//! Operations that return their input unchanged.
//!
//! `HTML To Text` does its work in a presentation layer, which `FerroSift`
//! does not have, so the data transformation is genuinely the identity. Saying
//! so is more honest than inventing a text extraction the reference never
//! performs.
//!
//! It is here rather than aliased onto `Identity` because a recipe naming it
//! has to import, run, and export as the operation it named — folding them
//! together would lose the name on the way back out.
//!
//! `Comment` and `Label` have the same shape and are deliberately absent. The
//! reference refuses to run them outside a browser, so their behaviour cannot
//! be pinned, and an operation this project cannot measure is one it does not
//! claim.

mod operation;

pub use operation::HtmlToText;
