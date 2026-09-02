//! `CyberChef` JSON recipe interchange, for every profile the catalog claims.
//!
//! One parser, because the reference's recipe *format* is byte-identical
//! between 11.3.0 and 11.4.0. What the profile decides is which operation
//! *names* resolve: an operation 11.4 introduced is genuinely unknown to 11.3,
//! and export refuses rather than writing a name the requested version cannot
//! load.

pub use crate::error::{ExportError, ImportError};
pub use crate::export::{export_recipe, export_source};
pub use crate::finding::{CompatibilityFinding, FindingSeverity};
pub use crate::import::{ImportReport, import_recipe};
pub use crate::profile::{
    MAX_ARGUMENT_DEPTH, MAX_RECIPE_BYTES, MAX_RECIPE_STEPS, MAX_SAFE_INTEGER,
};
pub use crate::source::{SourceRecipe, SourceStep};
