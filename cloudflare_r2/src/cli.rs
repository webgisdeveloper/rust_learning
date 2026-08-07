use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// Top-level CLI structure.
/// #[derive(Parser)] allows clap to generate the parsing logic automatically.
#[derive(Parser, Debug)]
#[command(
    name = "cloudflare-r2",
    version,
    about = "Upload, list and download files on Cloudflare R2",
    arg_required_else_help = true // If no subcommand is provided, show help automatically.
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output.
    /// global=true makes this flag available for all subcommands.
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

/// Defines the available subcommands for the tool.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Upload a file to R2
    Upload(UploadArgs),
    /// List objects in a bucket
    List(ListArgs),
    /// Download an object from a bucket
    Download(DownloadArgs),
    /// Delete an object from a bucket
    Delete(DeleteArgs),
}

/// Shared arguments used by Upload, List and Download.
/// Using #[derive(Args)] allows us to "flatten" this struct into other argument structs.
#[derive(Args, Debug)]
pub struct R2Args {
    /// R2 bucket name. The `env` attribute tells clap to check
    /// the environment variable R2_BUCKET if the flag is not provided.
    #[arg(short, long, env = "R2_BUCKET")]
    pub bucket: String,

    /// Cloudflare Account ID.
    #[arg(long, env = "R2_ACCOUNT_ID")]
    pub account_id: Option<String>,

    /// Full R2 endpoint URL.
    #[arg(long, env = "R2_ENDPOINT")]
    pub endpoint: Option<String>,

    /// Access Key ID. `hide_env_values=true` prevents the secret from being
    /// printed in the --help output.
    #[arg(long, env = "R2_ACCESS_KEY_ID", hide_env_values = true)]
    pub access_key: String,

    /// Secret Access Key.
    #[arg(long, env = "R2_SECRET_ACCESS_KEY", hide_env_values = true)]
    pub secret_key: String,
}

#[derive(Args, Debug)]
pub struct UploadArgs {
    /// Compose shared R2 arguments into this struct.
    #[command(flatten)]
    pub r2: R2Args,

    /// Local file to upload. PathBuf is used for cross-platform file path handling.
    #[arg(value_name = "FILE")]
    pub file: PathBuf,

    /// Object key in bucket (defaults to filename).
    #[arg(short, long)]
    pub key: Option<String>,

    /// Content-Type override (auto-detected if absent).
    #[arg(long)]
    pub content_type: Option<String>,

    /// Short description of the file to store in metadata.
    #[arg(short, long)]
    pub description: Option<String>,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Compose shared R2 arguments into this struct.
    #[command(flatten)]
    pub r2: R2Args,

    /// Only list keys with this prefix (e.g. "images/").
    #[arg(long)]
    pub prefix: Option<String>,

    /// Show detailed output (size, last modified).
    #[arg(short, long)]
    pub long: bool,
}

#[derive(Args, Debug)]
pub struct DownloadArgs {
    /// Compose shared R2 arguments into this struct.
    #[command(flatten)]
    pub r2: R2Args,

    /// Object key in R2 to download.
    #[arg(value_name = "KEY")]
    pub key: String,

    /// Local destination path. Defaults to the key's filename.
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Overwrite the destination if it already exists.
    #[arg(long)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct DeleteArgs {
    /// Compose shared R2 arguments into this struct.
    #[command(flatten)]
    pub r2: R2Args,

    /// Object key in R2 to delete.
    #[arg(value_name = "KEY")]
    pub key: String,
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

        // We use a 'let-else' statement here, a concise way to unwrap enum variants
        // while providing a fallback (panic in this case).
        let Commands::Upload(args) = cli.command else {
            panic!("expected upload");
        };
        assert_eq!(args.file, PathBuf::from("./photo.jpg"));
        assert_eq!(args.r2.bucket, "my-bucket");
        assert_eq!(args.r2.account_id, Some("acc123".to_string()));
    }

    #[test]
    fn parses_upload_with_description() {
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
            "-d",
            "A test description",
        ])
        .expect("parse should succeed");

        let Commands::Upload(args) = cli.command else {
            panic!("expected upload");
        };
        assert_eq!(args.description, Some("A test description".to_string()));
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
    fn parses_download_with_output() {
        let cli = Cli::try_parse_from([
            "cloudflare_r2",
            "download",
            "images/photo.jpg",
            "--bucket",
            "my-bucket",
            "--output",
            "./local.jpg",
            "--access-key",
            "ak",
            "--secret-key",
            "sk",
            "--account-id",
            "acc123",
        ])
        .expect("parse should succeed");

        let Commands::Download(args) = cli.command else {
            panic!("expected download");
        };
        assert_eq!(args.key, "images/photo.jpg");
        assert_eq!(args.output, Some(PathBuf::from("./local.jpg")));
        assert_eq!(args.r2.bucket, "my-bucket");
    }

    #[test]
    fn parses_delete() {
        let cli = Cli::try_parse_from([
            "cloudflare_r2",
            "delete",
            "images/photo.jpg",
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

        let Commands::Delete(args) = cli.command else {
            panic!("expected delete");
        };
        assert_eq!(args.key, "images/photo.jpg");
        assert_eq!(args.r2.bucket, "my-bucket");
    }

    #[test]
    fn rejects_missing_subcommand() {
        let err = Cli::try_parse_from(["cloudflare_r2"]).unwrap_err();
        assert!(err.to_string().contains("Usage"));
    }
}
