//! Local install smoke checks with no network and no sample files.

use std::io::Write;

use ferrosift_core::{ExecutionStatus, Executor, NeverCancelled, OperationRegistry};
use ferrosift_model::{CapabilitySet, Value};
use ferrosift_pattern::{EvalOptions, evaluate, parse};
use serde::Serialize;

use crate::{args::RecipeFormat, error::CliError, limits, recipe};

const SMOKE_RECIPE: &[u8] = br#"[{"op":"To Hex","args":["None",0]}]"#;
const SMOKE_INPUT: &[u8] = b"FS";
const SMOKE_HEX: &str = "4653";
const SMOKE_PATTERN: &str = "struct S { u8 v; };\nS s @ 0x00;";

#[derive(Debug, Serialize)]
struct DoctorReport {
    schema: &'static str,
    version: &'static str,
    ok: bool,
    checks: Vec<DoctorCheck>,
}

#[derive(Debug, Serialize)]
struct DoctorCheck {
    id: &'static str,
    ok: bool,
    detail: String,
}

/// Runs built-in smoke checks and writes `ferrosift.doctor.v1` JSON.
pub fn run(output: &mut dyn Write) -> Result<(), CliError> {
    let mut checks = Vec::new();
    let mut ok = true;

    checks.push(DoctorCheck {
        id: "cli.version",
        ok: true,
        detail: env!("CARGO_PKG_VERSION").to_owned(),
    });

    let registry = match ferrosift_operations::default_registry() {
        Ok(registry) => {
            let count = registry.catalog().count();
            checks.push(DoctorCheck {
                id: "registry.load",
                ok: true,
                detail: format!("operations={count}"),
            });
            Some(registry)
        }
        Err(error) => {
            ok = false;
            checks.push(DoctorCheck {
                id: "registry.load",
                ok: false,
                detail: error.to_string(),
            });
            None
        }
    };

    if let Some(registry) = registry.as_ref() {
        match smoke_recipe(registry) {
            Ok(detail) => checks.push(DoctorCheck {
                id: "recipe.smoke",
                ok: true,
                detail,
            }),
            Err(error) => {
                ok = false;
                checks.push(DoctorCheck {
                    id: "recipe.smoke",
                    ok: false,
                    detail: error.to_string(),
                });
            }
        }
    }

    match smoke_pattern() {
        Ok(detail) => checks.push(DoctorCheck {
            id: "pattern.smoke",
            ok: true,
            detail,
        }),
        Err(error) => {
            ok = false;
            checks.push(DoctorCheck {
                id: "pattern.smoke",
                ok: false,
                detail: error.to_string(),
            });
        }
    }

    let report = DoctorReport {
        schema: "ferrosift.doctor.v1",
        version: env!("CARGO_PKG_VERSION"),
        ok,
        checks,
    };
    let body = serde_json::to_vec_pretty(&report)
        .map_err(|error| CliError::new("cli.doctor.serialize", error.to_string()))?;
    output.write_all(&body).map_err(CliError::write)?;
    output.write_all(b"\n").map_err(CliError::write)?;
    if ok {
        Ok(())
    } else {
        Err(CliError::new(
            "cli.doctor.failed",
            "one or more install smoke checks failed",
        ))
    }
}

fn smoke_recipe(registry: &OperationRegistry) -> Result<String, CliError> {
    let loaded = recipe::load(SMOKE_RECIPE, RecipeFormat::CyberChefV11_3, registry)?;
    let execution = Executor::new(registry)
        .execute(
            &loaded,
            Value::Bytes(SMOKE_INPUT.to_vec()),
            limits::budget(),
            &NeverCancelled,
            CapabilitySet::new(),
        )
        .map_err(|error| CliError::execution(&error))?;
    if !matches!(execution.status, ExecutionStatus::Completed) {
        return Err(CliError::new(
            "cli.doctor.recipe_not_completed",
            format!("status={:?}", execution.status),
        ));
    }
    let Value::Text(text) = execution.value else {
        return Err(CliError::new(
            "cli.doctor.recipe_kind",
            "expected text hex output",
        ));
    };
    if text.text != SMOKE_HEX {
        return Err(CliError::new(
            "cli.doctor.recipe_mismatch",
            format!("expected={SMOKE_HEX} actual={}", text.text),
        ));
    }
    Ok(format!("to_hex({SMOKE_INPUT:?})={SMOKE_HEX}"))
}

fn smoke_pattern() -> Result<String, CliError> {
    let pattern =
        parse(SMOKE_PATTERN).map_err(|error| CliError::new(error.code(), error.to_string()))?;
    let nodes = evaluate(&pattern, &[0x2a], &EvalOptions::default())
        .map_err(|error| CliError::new(error.code(), error.to_string()))?;
    if nodes.is_empty() {
        return Err(CliError::new(
            "cli.doctor.pattern_empty",
            "pattern evaluation produced no nodes",
        ));
    }
    Ok("struct S { u8 v; }; S s @ 0x00".to_owned())
}
