// Both the `differential` and `corpus` test binaries include this module via
// `#[path]`, and each uses a different subset of the loaders.
#![allow(dead_code)]

mod fixture;
mod runner;

pub use fixture::{load_corpus, load_suite};
pub use runner::{assert_supported_case, assert_unsupported_case};
