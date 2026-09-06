//! Local stdio MCP adapter for `FerroSift`.

#![forbid(unsafe_code)]

mod server;

use std::{env, path::PathBuf, process::ExitCode, sync::Arc};

use ferrosift_host::{HostConfig, HostService};
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::EnvFilter;

fn main() -> ExitCode {
    if let Err(error) = run() {
        eprintln!("ferrosift-mcp: {error}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

#[tokio::main]
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("ferrosift_mcp=info".parse()?))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let roots = parse_roots(env::args().skip(1))?;
    let service = HostService::new(HostConfig {
        allowed_roots: roots,
        ..HostConfig::default()
    })?;
    let server = server::FerroSiftMcp::new(Arc::new(service));
    tracing::info!("ferrosift-mcp listening on stdio");
    let running = server.serve(stdio()).await?;
    running.waiting().await?;
    Ok(())
}

fn parse_roots(
    args: impl IntoIterator<Item = String>,
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut roots = Vec::new();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => {
                let path = args
                    .next()
                    .ok_or("missing path after --root")?;
                roots.push(PathBuf::from(path));
            }
            "--help" | "-h" => {
                eprintln!(
                    "Usage: ferrosift-mcp [--root DIR]...\n\
                     Local stdio MCP adapter. Repeat --root for each allowlisted sample directory.\n\
                     Logs go to stderr; stdout is the MCP stream."
                );
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}").into()),
        }
    }
    Ok(roots)
}
