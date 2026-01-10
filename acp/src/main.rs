//! ACP CLI - Agent Credential Proxy command-line interface
//!
//! This binary provides the CLI for managing the ACP server,
//! including initialization, plugin management, credential storage,
//! and agent token management.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "acp")]
#[command(author, version, about = "Agent Credential Proxy CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Display version information
    Version,
    /// Display status (placeholder for Phase 7)
    Status,
}

fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Version) => {
            println!("acp {}", env!("CARGO_PKG_VERSION"));
        }
        Some(Commands::Status) => {
            println!("Status command not yet implemented (Phase 7)");
        }
        None => {
            println!("ACP CLI - use --help for usage information");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_version_parses() {
        // Verify CLI structure compiles and basic commands exist
        let _cli = Cli::parse_from(["acp", "version"]);
    }

    #[test]
    fn test_cli_status_parses() {
        let _cli = Cli::parse_from(["acp", "status"]);
    }
}
