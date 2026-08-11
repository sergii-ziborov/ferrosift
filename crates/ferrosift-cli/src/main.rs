//! Native command-line entry point for `FerroSift`.

#![forbid(unsafe_code)]

use std::{io as stdio, process::ExitCode};

use clap::Parser as _;

mod app;
mod args;
mod commands;
mod error;
mod io;
mod limits;
mod recipe;
mod value;

fn main() -> ExitCode {
    let arguments = args::Args::parse();
    let mut input = stdio::stdin().lock();
    let mut output = stdio::stdout().lock();
    match app::run(arguments, &mut input, &mut output) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
