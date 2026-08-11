//! Explicit, loss-aware external recipe compatibility for `FerroSift`.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod arguments;
mod error;
mod export;
mod finding;
mod import;
mod json_writer;
mod profile;
mod source;
mod step;

pub mod cyberchef;
