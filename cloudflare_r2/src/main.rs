mod cli;
mod r2;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    match cli.command {
        Commands::Upload(args) => r2::run_upload(args, cli.verbose).await,
        Commands::List(args) => r2::run_list(args, cli.verbose).await,
    }
}
