use anyhow::{Context, bail};
use aws_sdk_s3::primitives::ByteStream;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "cloudflare-r2",
    version,
    about = "Upload and list files on Cloudflare R2",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Upload a file to R2
    Upload(UploadArgs),
    /// List objects in a bucket
    List(ListArgs),
}

#[derive(Parser, Debug)]
struct UploadArgs {
    /// Local file to upload
    #[arg(value_name = "FILE")]
    file: PathBuf,

    /// Object key in bucket (defaults to filename)
    #[arg(short, long)]
    key: Option<String>,

    /// R2 bucket name (or env R2_BUCKET)
    #[arg(short, long, env = "R2_BUCKET")]
    bucket: String,

    /// Cloudflare Account ID (or env R2_ACCOUNT_ID)
    #[arg(long, env = "R2_ACCOUNT_ID")]
    account_id: Option<String>,

    /// Full R2 endpoint URL (or env R2_ENDPOINT); if omitted, https://{account_id}.r2.cloudflarestorage.com
    #[arg(long, env = "R2_ENDPOINT")]
    endpoint: Option<String>,

    /// Access Key ID (or env R2_ACCESS_KEY_ID)
    #[arg(long, env = "R2_ACCESS_KEY_ID", hide_env_values = true)]
    access_key: String,

    /// Secret Access Key (or env R2_SECRET_ACCESS_KEY)
    #[arg(long, env = "R2_SECRET_ACCESS_KEY", hide_env_values = true)]
    secret_key: String,

    /// Content-Type override (auto-detected if absent)
    #[arg(long)]
    content_type: Option<String>,
}

#[derive(Parser, Debug)]
struct ListArgs {
    /// R2 bucket name (or env R2_BUCKET)
    #[arg(short, long, env = "R2_BUCKET")]
    bucket: String,

    /// Only list keys with this prefix (e.g. "images/")
    #[arg(long)]
    prefix: Option<String>,

    /// Show detailed output (size, last modified)
    #[arg(short, long)]
    long: bool,

    /// Cloudflare Account ID (or env R2_ACCOUNT_ID)
    #[arg(long, env = "R2_ACCOUNT_ID")]
    account_id: Option<String>,

    /// Full R2 endpoint URL (or env R2_ENDPOINT); if omitted, https://{account_id}.r2.cloudflarestorage.com
    #[arg(long, env = "R2_ENDPOINT")]
    endpoint: Option<String>,

    /// Access Key ID (or env R2_ACCESS_KEY_ID)
    #[arg(long, env = "R2_ACCESS_KEY_ID", hide_env_values = true)]
    access_key: String,

    /// Secret Access Key (or env R2_SECRET_ACCESS_KEY)
    #[arg(long, env = "R2_SECRET_ACCESS_KEY", hide_env_values = true)]
    secret_key: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env if present (for local development)
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();

    match cli.command {
        Commands::Upload(args) => {
            // Validate file
            if !args.file.exists() {
                bail!("file not found: {}", args.file.display());
            }
            if !args.file.is_file() {
                bail!(
                    "not a file: {} (directories not supported)",
                    args.file.display()
                );
            }

            // Derive endpoint or account_id
            let endpoint_url =
                derive_endpoint(args.endpoint.as_deref(), args.account_id.as_deref()).context(
                    "must provide --endpoint / R2_ENDPOINT or --account-id / R2_ACCOUNT_ID",
                )?;

            if cli.verbose {
                eprintln!(
                    "Endpoint: {}\nBucket: {}\nFile: {}",
                    endpoint_url,
                    args.bucket,
                    args.file.display()
                );
            }

            let key = args.key.clone().unwrap_or_else(|| derive_key(&args.file));

            if key.is_empty() {
                bail!("derived key is empty; please provide --key");
            }

            let client = build_client(&endpoint_url, &args.access_key, &args.secret_key).await;

            upload(
                &client,
                &args.bucket,
                &key,
                &args.file,
                args.content_type,
                cli.verbose,
            )
            .await?;
        }
        Commands::List(args) => {
            let endpoint_url =
                derive_endpoint(args.endpoint.as_deref(), args.account_id.as_deref()).context(
                    "must provide --endpoint / R2_ENDPOINT or --account-id / R2_ACCOUNT_ID",
                )?;

            if cli.verbose {
                eprintln!(
                    "Endpoint: {}\nBucket: {}{}",
                    endpoint_url,
                    args.bucket,
                    args.prefix
                        .as_ref()
                        .map(|p| format!(" (prefix: {p})"))
                        .unwrap_or_default()
                );
            }

            let client = build_client(&endpoint_url, &args.access_key, &args.secret_key).await;

            list_objects(&client, &args.bucket, args.prefix.as_deref(), args.long, cli.verbose)
                .await?;
        }
    }

    Ok(())
}

fn derive_endpoint(endpoint: Option<&str>, account_id: Option<&str>) -> Option<String> {
    if let Some(e) = endpoint
        && !e.is_empty()
    {
        return Some(e.to_string());
    }
    account_id.map(|id| format!("https://{id}.r2.cloudflarestorage.com"))
}

fn derive_key(file: &Path) -> String {
    file.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "upload".to_string())
}

async fn build_client(
    endpoint_url: &str,
    access_key: &str,
    secret_key: &str,
) -> aws_sdk_s3::Client {
    let credentials = aws_sdk_s3::config::Credentials::new(
        access_key.to_owned(),
        secret_key.to_owned(),
        None,
        None,
        "r2",
    );

    // Load base config from environment (region, etc.), then override
    let base_config = aws_config::load_from_env().await;
    let s3_config = aws_sdk_s3::config::Builder::from(&base_config)
        .endpoint_url(endpoint_url)
        .credentials_provider(credentials)
        .region(aws_config::Region::new("auto"))
        .build();

    aws_sdk_s3::Client::from_conf(s3_config)
}

async fn upload(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    file: &Path,
    content_type: Option<String>,
    verbose: bool,
) -> anyhow::Result<()> {
    // Detect content-type if not provided
    let ct = content_type.or_else(|| mime_guess::from_path(file).first().map(|m| m.to_string()));

    let body = ByteStream::from_path(file)
        .await
        .with_context(|| format!("failed to read {}", file.display()))?;

    if verbose {
        let len = tokio::fs::metadata(file)
            .await
            .map(|m| m.len())
            .unwrap_or(0);
        eprintln!(
            "Uploading {} ({} bytes) -> s3://{}/{} as {}",
            file.display(),
            len,
            bucket,
            key,
            ct.as_deref().unwrap_or("application/octet-stream")
        );
    }

    let mut req = client.put_object().bucket(bucket).key(key).body(body);
    if let Some(ct) = ct {
        req = req.content_type(ct);
    }

    req.send()
        .await
        .context("put_object failed — check bucket, credentials, endpoint and network")?;

    println!("Uploaded {} to s3://{}/{}", file.display(), bucket, key);
    Ok(())
}

async fn list_objects(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    prefix: Option<&str>,
    long: bool,
    verbose: bool,
) -> anyhow::Result<()> {
    let mut continuation_token: Option<String> = None;
    let mut total = 0usize;
    let mut first_page = true;

    loop {
        let mut req = client.list_objects_v2().bucket(bucket);
        if let Some(p) = prefix {
            req = req.prefix(p);
        }
        if let Some(token) = continuation_token.take() {
            req = req.continuation_token(token);
        }

        let resp = req
            .send()
            .await
            .context("list_objects failed — check bucket, credentials, endpoint and network")?;

        let contents = resp.contents();
        if first_page && contents.is_empty() && verbose {
            if let Some(p) = prefix {
                eprintln!("No objects found in s3://{bucket}/ with prefix \"{p}\"");
            } else {
                eprintln!("No objects found in s3://{bucket}/");
            }
        }
        first_page = false;

        for obj in contents {
            total += 1;
            let key = obj.key().unwrap_or("<no-key>");
            if long {
                let size = obj.size().unwrap_or(0);
                let modified = obj
                    .last_modified()
                    .map(|d| d.fmt(aws_smithy_types::date_time::Format::DateTime).unwrap_or_default())
                    .unwrap_or_else(|| "-".to_string());
                println!("{key}\t{size}\t{modified}");
            } else {
                println!("{key}");
            }
        }

        if resp.is_truncated() == Some(true) {
            continuation_token = resp.next_continuation_token().map(|s| s.to_string());
            if continuation_token.is_none() {
                break;
            }
        } else {
            break;
        }
    }

    if verbose {
        eprintln!("Listed {total} object(s) from s3://{bucket}/");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_derive_endpoint_from_account_id() {
        assert_eq!(
            derive_endpoint(None, Some("abc123")),
            Some("https://abc123.r2.cloudflarestorage.com".to_string())
        );
    }

    #[test]
    fn test_derive_endpoint_prefers_explicit() {
        assert_eq!(
            derive_endpoint(Some("https://custom.example.com"), Some("abc123")),
            Some("https://custom.example.com".to_string())
        );
    }

    #[test]
    fn test_derive_endpoint_none() {
        assert_eq!(derive_endpoint(None, None), None);
    }

    #[test]
    fn test_derive_key_basename() {
        assert_eq!(derive_key(Path::new("a/b/photo.jpg")), "photo.jpg");
        assert_eq!(derive_key(Path::new("/tmp/README.md")), "README.md");
    }

    #[test]
    fn test_mime_guess() {
        assert_eq!(
            mime_guess::from_path(Path::new("photo.jpg"))
                .first()
                .unwrap()
                .to_string(),
            "image/jpeg"
        );
        // unknown extension should be None
        assert!(
            mime_guess::from_path(Path::new("file.unknownext123"))
                .first()
                .is_none()
        );
    }

    #[test]
    fn test_cli_parses_upload() {
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
        match cli.command {
            Commands::Upload(args) => {
                assert_eq!(args.file, PathBuf::from("./photo.jpg"));
                assert_eq!(args.bucket, "my-bucket");
                assert_eq!(args.account_id, Some("acc123".to_string()));
            }
            _ => panic!("expected upload"),
        }
    }

    #[test]
    fn test_cli_parses_list() {
        let cli = Cli::try_parse_from([
            "cloudflare_r2",
            "list",
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
        match cli.command {
            Commands::List(args) => {
                assert_eq!(args.bucket, "my-bucket");
                assert_eq!(args.prefix, None);
                assert!(!args.long);
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn test_cli_parses_list_with_prefix_and_long() {
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
        match cli.command {
            Commands::List(args) => {
                assert_eq!(args.prefix, Some("images/".to_string()));
                assert!(args.long);
                assert_eq!(
                    args.endpoint,
                    Some("https://custom.example.com".to_string()),
                );
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn test_cli_missing_subcommand_fails() {
        let err = Cli::try_parse_from(["cloudflare_r2"]).unwrap_err();
        // arg_required_else_help should produce error
        assert!(err.to_string().contains("Usage"));
    }
}
