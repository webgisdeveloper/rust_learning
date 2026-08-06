use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "cloudflare-r2",
    version,
    about = "Upload and list files on Cloudflare R2",
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Upload a file to R2
    Upload(UploadArgs),
    /// List objects in a bucket
    List(ListArgs),
}

#[derive(Args, Debug)]
pub struct R2Args {
    /// R2 bucket name (or env R2_BUCKET)
    #[arg(short, long, env = "R2_BUCKET")]
    pub bucket: String,

    /// Cloudflare Account ID (or env R2_ACCOUNT_ID)
    #[arg(long, env = "R2_ACCOUNT_ID")]
    pub account_id: Option<String>,

    /// Full R2 endpoint URL (or env R2_ENDPOINT); if omitted, https://{account_id}.r2.cloudflarestorage.com
    #[arg(long, env = "R2_ENDPOINT")]
    pub endpoint: Option<String>,

    /// Access Key ID (or env R2_ACCESS_KEY_ID)
    #[arg(long, env = "R2_ACCESS_KEY_ID", hide_env_values = true)]
    pub access_key: String,

    /// Secret Access Key (or env R2_SECRET_ACCESS_KEY)
    #[arg(long, env = "R2_SECRET_ACCESS_KEY", hide_env_values = true)]
    pub secret_key: String,
}

#[derive(Args, Debug)]
pub struct UploadArgs {
    #[command(flatten)]
    pub r2: R2Args,

    /// Local file to upload
    #[arg(value_name = "FILE")]
    pub file: PathBuf,

    /// Object key in bucket (defaults to filename)
    #[arg(short, long)]
    pub key: Option<String>,

    /// Content-Type override (auto-detected if absent)
    #[arg(long)]
    pub content_type: Option<String>,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    #[command(flatten)]
    pub r2: R2Args,

    /// Only list keys with this prefix (e.g. "images/")
    #[arg(long)]
    pub prefix: Option<String>,

    /// Show detailed output (size, last modified)
    #[arg(short, long)]
    pub long: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_upload() {
        let cli = Cli::try_parse_from([
            "cloudflare_r2",
            "upload",
            "./photo.jpg",
            "--bucket",
            "my-bucket",
            "--access-key",
            "ak",
            "--secret-key",
            "sk",
            "--account-id",
            "acc123",
        ])
        .expect("parse should succeed");

        let Commands::Upload(args) = cli.command else {
            panic!("expected upload");
        };
        assert_eq!(args.file, PathBuf::from("./photo.jpg"));
        assert_eq!(args.r2.bucket, "my-bucket");
        assert_eq!(args.r2.account_id, Some("acc123".to_string()));
    }

    #[test]
    fn parses_list_with_prefix_and_long() {
        let cli = Cli::try_parse_from([
            "cloudflare_r2",
            "list",
            "--bucket",
            "my-bucket",
            "--prefix",
            "images/",
            "--long",
            "--access-key",
            "ak",
            "--secret-key",
            "sk",
            "--endpoint",
            "https://custom.example.com",
        ])
        .expect("parse should succeed");

        let Commands::List(args) = cli.command else {
            panic!("expected list");
        };
        assert_eq!(args.r2.bucket, "my-bucket");
        assert_eq!(args.prefix, Some("images/".to_string()));
        assert!(args.long);
    }

    #[test]
    fn rejects_missing_subcommand() {
        let err = Cli::try_parse_from(["cloudflare_r2"]).unwrap_err();
        assert!(err.to_string().contains("Usage"));
    }
}
