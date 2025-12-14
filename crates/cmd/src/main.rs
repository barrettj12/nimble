mod commands;
mod types;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::commands::{build_get, build_list, deploy};

const DEFAULT_AGENT_URL: &str = "http://localhost:7080";

#[derive(Parser)]
#[command(name = "nimble")]
#[command(about = "The Nimble CLI")]
struct Cli {
    /// Agent API URL
    #[arg(long, default_value = DEFAULT_AGENT_URL)]
    agent_url: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new build from a directory of source files
    Deploy(deploy::DeployArgs),
    /// Manage builds
    Build {
        #[command(subcommand)]
        command: BuildCommands,
    },
}

#[derive(Subcommand)]
enum BuildCommands {
    /// List builds
    List(build_list::BuildListArgs),
    /// Get details about a specific build
    Get(build_get::BuildGetArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Deploy(args) => {
            deploy::execute(&cli.agent_url, args).await?;
        }
        Commands::Build { command } => match command {
            BuildCommands::List(args) => {
                build_list::execute(&cli.agent_url, args).await?;
            }
            BuildCommands::Get(args) => {
                build_get::execute(&cli.agent_url, args).await?;
            }
        },
    }

    Ok(())
}
