use crate::cli::{ListArgs, R2Args, UploadArgs};
use anyhow::{Context, bail};
use aws_sdk_s3::primitives::ByteStream;
use std::path::Path;

/// Entry point for the upload command.
pub async fn run_upload(args: UploadArgs, verbose: bool) -> anyhow::Result<()> {
    // Validate that the target path exists and is a file.
    if !args.file.exists() {
        bail!("file not found: {}", args.file.display());
    }
    if !args.file.is_file() {
        bail!(
            "not a file: {} (directories not supported)",
            args.file.display()
        );
    }

    // Derive the S3 endpoint based on account ID or an explicit URL.
    let endpoint_url = endpoint_for(&args.r2)?;
    if verbose {
        eprintln!(
            "Endpoint: {}\nBucket: {}\nFile: {}",
            endpoint_url,
            args.r2.bucket,
            args.file.display()
        );
    }

    // Use filename as the S3 key if no explicit key is provided.
    let key = args.key.unwrap_or_else(|| derive_key(&args.file));
    if key.is_empty() {
        bail!("object key must not be empty; provide --key");
    }

    // Initialize the S3 client with Cloudflare R2 credentials.
    let client = build_client(&endpoint_url, &args.r2.access_key, &args.r2.secret_key).await;
    
    // Perform the upload.
    upload(
        &client,
        &args.r2.bucket,
        &key,
        &args.file,
        args.content_type,
        verbose,
    )
    .await
}

/// Entry point for the list command.
pub async fn run_list(args: ListArgs, verbose: bool) -> anyhow::Result<()> {
    let endpoint_url = endpoint_for(&args.r2)?;
    if verbose {
        eprintln!(
            "Endpoint: {}\nBucket: {}{}",
            endpoint_url,
            args.r2.bucket,
            args.prefix
                .as_ref()
                .map(|prefix| format!(" (prefix: {prefix})"))
                .unwrap_or_default()
        );
    }

    let client = build_client(&endpoint_url, &args.r2.access_key, &args.r2.secret_key).await;
    
    // Perform the object listing.
    list_objects(
        &client,
        &args.r2.bucket,
        args.prefix.as_deref(),
        args.long,
        verbose,
    )
    .await
}

/// Helper to determine the R2 endpoint URL.
fn endpoint_for(args: &R2Args) -> anyhow::Result<String> {
    // .context() from anyhow allows us to wrap lower-level errors 
    // with higher-level explanations.
    derive_endpoint(args.endpoint.as_deref(), args.account_id.as_deref())
        .context("must provide --endpoint / R2_ENDPOINT or --account-id / R2_ACCOUNT_ID")
}

/// Logic to derive an endpoint URL. Returns None if neither is provided.
fn derive_endpoint(endpoint: Option<&str>, account_id: Option<&str>) -> Option<String> {
    // Try to use the explicit endpoint first.
    if let Some(endpoint) = endpoint.map(str::trim)
        && !endpoint.is_empty()
    {
        return Some(endpoint.to_string());
    }

    // Otherwise, construct the endpoint from the account ID.
    let account_id = account_id?.trim();
    (!account_id.is_empty()).then(|| format!("https://{account_id}.r2.cloudflarestorage.com"))
}

/// Extracts the filename from a path to use as a default S3 key.
fn derive_key(file: &Path) -> String {
    file.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "upload".to_string())
}

/// Configures and builds the AWS S3 Client for R2.
async fn build_client(
    endpoint_url: &str,
    access_key: &str,
    secret_key: &str,
) -> aws_sdk_s3::Client {
    // R2 uses static credentials (Access Key / Secret Key).
    let credentials = aws_sdk_s3::config::Credentials::new(
        access_key.to_owned(),
        secret_key.to_owned(),
        None,
        None,
        "r2",
    );

    // Use the recommended SDK loading pattern.
    let base_config = aws_config::load_from_env().await;
    let s3_config = aws_sdk_s3::config::Builder::from(&base_config)
        .endpoint_url(endpoint_url)
        .credentials_provider(credentials)
        .region(aws_config::Region::new("auto")) // Region "auto" is required by the SDK for R2.
        .build();

    aws_sdk_s3::Client::from_conf(s3_config)
}

/// Handles the actual streaming upload to R2.
async fn upload(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    file: &Path,
    content_type: Option<String>,
    verbose: bool,
) -> anyhow::Result<()> {
    // mime_guess helps us set the correct Content-Type based on the file extension.
    let content_type =
        content_type.or_else(|| mime_guess::from_path(file).first().map(|mime| mime.to_string()));
    
    // ByteStream::from_path reads the file asynchronously and streams it 
    // to the network, preventing the need to load the whole file into RAM.
    let body = ByteStream::from_path(file)
        .await
        .with_context(|| format!("failed to read {}", file.display()))?;

    if verbose {
        let length = tokio::fs::metadata(file)
            .await
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        eprintln!(
            "Uploading {} ({} bytes) -> s3://{}/{} as {}",
            file.display(),
            length,
            bucket,
            key,
            content_type.as_deref().unwrap_or("application/octet-stream")
        );
    }

    // Build the put_object request.
    let mut request = client.put_object().bucket(bucket).key(key).body(body);
    if let Some(content_type) = content_type {
        request = request.content_type(content_type);
    }

    request
        .send()
        .await
        .context("put_object failed — check bucket, credentials, endpoint and network")?;

    println!("Uploaded {} to s3://{}/{}", file.display(), bucket, key);
    Ok(())
}

/// Handles paginated object listing from R2.
async fn list_objects(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    prefix: Option<&str>,
    long: bool,
    verbose: bool,
) -> anyhow::Result<()> {
    // continuation_token is used to fetch the next page of results.
    let mut continuation_token = None;
    let mut total = 0usize;
    let mut first_page = true;

    loop {
        let mut request = client.list_objects_v2().bucket(bucket);
        if let Some(prefix) = prefix {
            request = request.prefix(prefix);
        }
        if let Some(token) = continuation_token.take() {
            request = request.continuation_token(token);
        }

        // Send the request and handle potential API errors.
        let response = request
            .send()
            .await
            .context("list_objects failed — check bucket, credentials, endpoint and network")?;

        let contents = response.contents();
        
        // If the first page is empty, we let the user know in verbose mode.
        if first_page && contents.is_empty() && verbose {
            if let Some(prefix) = prefix {
                eprintln!("No objects found in s3://{bucket}/ with prefix \"{prefix}\"");
            } else {
                eprintln!("No objects found in s3://{bucket}/");
            }
        }
        first_page = false;

        // Iterate over objects in the current page.
        for object in contents {
            total += 1;
            let key = object.key().unwrap_or("<no-key>");
            if long {
                let size = object.size().unwrap_or(0);
                // Formatting the AWS DateTime object into a readable string.
                let modified = object
                    .last_modified()
                    .map(|date| {
                        date.fmt(aws_smithy_types::date_time::Format::DateTime)
                            .unwrap_or_else(|_| "-".to_string())
                    })
                    .unwrap_or_else(|| "-".to_string());
                println!("{key}\t{size}\t{modified}");
            } else {
                println!("{key}");
            }
        }

        // Check if more objects are available.
        if !response.is_truncated().unwrap_or(false) {
            break;
        }

        // Retrieve the token for the next page.
        continuation_token = response.next_continuation_token().map(str::to_owned);
        
        // Fail if the API says more results exist but provides no way to get them.
        if continuation_token.is_none() {
            bail!("R2 returned a truncated object list without a continuation token");
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

    #[test]
    fn derives_endpoint_from_account_id() {
        assert_eq!(
            derive_endpoint(None, Some("abc123")),
            Some("https://abc123.r2.cloudflarestorage.com".to_string())
        );
    }

    #[test]
    fn explicit_endpoint_takes_precedence() {
        assert_eq!(
            derive_endpoint(Some("https://custom.example.com"), Some("abc123")),
            Some("https://custom.example.com".to_string())
        );
    }

    #[test]
    fn blank_endpoint_and_account_id_are_rejected() {
        assert_eq!(derive_endpoint(Some("  "), Some("\t")), None);
    }

    #[test]
    fn derives_key_from_file_name() {
        assert_eq!(derive_key(Path::new("a/b/photo.jpg")), "photo.jpg");
    }
}
