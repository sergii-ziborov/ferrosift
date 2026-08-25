// Both the `differential` and `corpus` test binaries include this module via
// `#[path]`, and each uses a different subset of the loaders, so the unused
// half is expected in either binary.
#![allow(dead_code, unused_imports)]

mod fixture;
mod runner;

pub use fixture::{
    Case, apply_overlay, load_corpus, load_corpus_overlay_11_4, load_suite, load_suite_overlay_11_4,
};
pub use runner::{assert_supported_case, assert_unsupported_case};
