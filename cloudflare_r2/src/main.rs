mod cli;
mod r2;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

/// The entry point of the CLI tool.
/// #[tokio::main] initializes the Tokio async runtime, allowing us to use `async fn main`.
#[tokio::main]
async fn main() -> Result<()> {
    // dotenvy::dotenv() looks for a .env file in the current directory
    // and loads its contents into environment variables.
    // .ok() converts the Result to Option, ignoring errors if the file is missing.
    let _ = dotenvy::dotenv();

    // Parse command line arguments into the Cli struct defined in cli.rs
    let cli = Cli::parse();

    // match is a powerful Rust control flow operator that ensures
    // we handle every possible variant of the Commands enum.
    match cli.command {
        // Dispatch to the specific logic in r2.rs based on the subcommand chosen.
        Commands::Upload(args) => r2::run_upload(args, cli.verbose).await,
        Commands::List(args) => r2::run_list(args, cli.verbose).await,
        Commands::Download(args) => r2::run_download(args, cli.verbose).await,
        Commands::Delete(args) => r2::run_delete(args, cli.verbose).await,
        Commands::Stat(args) => r2::run_stat(args, cli.verbose).await,
        Commands::Presign(args) => r2::run_presign(args, cli.verbose).await,
    }
}
