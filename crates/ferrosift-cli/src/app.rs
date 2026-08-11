//! Command dispatch over the production operation registry.

use std::io::{Read, Write};

use ferrosift_operations::default_registry;

use crate::{
    args::{Args, Command},
    commands,
    error::CliError,
};

pub fn run(arguments: Args, input: &mut dyn Read, output: &mut dyn Write) -> Result<(), CliError> {
    let registry = default_registry()
        .map_err(|error| CliError::new("cli.registry.invalid", error.to_string()))?;
    match arguments.command {
        Command::Operations => commands::operations::run(&registry, output),
        Command::Describe { operation } => commands::describe::run(&registry, &operation, output),
        Command::Validate {
            format,
            input_kind,
            recipe,
        } => commands::validate::run(&registry, format, input_kind, &recipe, input, output),
        Command::Run {
            format,
            input_kind,
            recipe,
            input: input_path,
            output: output_path,
        } => {
            let request = commands::run::Request {
                format,
                input_kind,
                recipe_path: &recipe,
                input_path: &input_path,
                output_path: &output_path,
            };
            commands::run::run(&registry, &request, input, output)
        }
    }
}
