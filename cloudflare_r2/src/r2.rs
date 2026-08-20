use crate::cli::{DeleteArgs, DownloadArgs, ListArgs, R2Args, StatArgs, UploadArgs};
use anyhow::{Context, bail};
use aws_sdk_s3::primitives::ByteStream;
use std::path::{Path, PathBuf};

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
        args.description,
        verbose,
    )
    .await
}

/// Entry point for the download command.
pub async fn run_download(args: DownloadArgs, verbose: bool) -> anyhow::Result<()> {
    let key = args.key.trim();
    if key.is_empty() {
        bail!("object key must not be empty");
    }
    if key.ends_with('/') {
        bail!("object key must identify a file, not a directory: {key}");
    }

    let output = derive_output_path(key, args.output)?;
    if output.exists() && !args.force {
        bail!(
            "destination already exists: {}; use --force to overwrite",
            output.display()
        );
    }

    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }

    let endpoint_url = endpoint_for(&args.r2)?;
    if verbose {
        eprintln!(
            "Endpoint: {}\nBucket: {}\nKey: {}\nOutput: {}",
            endpoint_url,
            args.r2.bucket,
            key,
            output.display()
        );
    }

    let client = build_client(&endpoint_url, &args.r2.access_key, &args.r2.secret_key).await;
    download(&client, &args.r2.bucket, key, &output, verbose).await
}

/// Entry point for the delete command.
pub async fn run_delete(args: DeleteArgs, verbose: bool) -> anyhow::Result<()> {
    let key = args.key.trim();
    if key.is_empty() {
        bail!("object key must not be empty");
    }

    let endpoint_url = endpoint_for(&args.r2)?;
    if verbose {
        eprintln!(
            "Endpoint: {}\nBucket: {}\nKey: {}",
            endpoint_url, args.r2.bucket, key
        );
    }

    let client = build_client(&endpoint_url, &args.r2.access_key, &args.r2.secret_key).await;
    delete(&client, &args.r2.bucket, key, verbose).await
}

/// Deletes an object from R2.
// Note: HeadObject 404 maps to is_not_found(); GetObject 404 maps to is_no_such_key() — intentional.
async fn delete(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    verbose: bool,
) -> anyhow::Result<()> {
    match client.head_object().bucket(bucket).key(key).send().await {
        Ok(_) => {}
        Err(aws_sdk_s3::error::SdkError::ServiceError(error)) if error.err().is_not_found() => {
            bail!("file not found: s3://{bucket}/{key}");
        }
        Err(error) => {
            return Err(error).context(
                "head_object failed — check bucket, key, credentials, endpoint and network",
            );
        }
    }

    client
        .delete_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .context("delete_object failed — check bucket, key, credentials, endpoint and network")?;

    if verbose {
        eprintln!("Deleted s3://{bucket}/{key}");
    }
    println!("Deleted s3://{bucket}/{key}");
    Ok(())
}

/// Entry point for the stat command.
pub async fn run_stat(args: StatArgs, verbose: bool) -> anyhow::Result<()> {
    let key = args.key.trim();
    if key.is_empty() {
        bail!("object key must not be empty");
    }

    let endpoint_url = endpoint_for(&args.r2)?;
    if verbose {
        eprintln!(
            "Endpoint: {}\nBucket: {}\nKey: {}",
            endpoint_url, args.r2.bucket, key
        );
    }

    let client = build_client(&endpoint_url, &args.r2.access_key, &args.r2.secret_key).await;
    let info = head_stat(&client, &args.r2.bucket, key).await?;

    if args.json {
        println!("{}", format_stat_json(&info));
    } else {
        println!("{}", format_stat_human(&info));
    }

    if verbose {
        eprintln!("Stat s3://{}/{}", args.r2.bucket, key);
    }
    Ok(())
}

/// Choose the requested output path or safely derive a local filename from the key.
fn derive_output_path(key: &str, output: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    if let Some(output) = output {
        return Ok(output);
    }

    Path::new(key)
        .file_name()
        .filter(|name| !name.is_empty())
        .map(PathBuf::from)
        .context("object key does not contain a filename; provide --output")
}

/// Stream one R2 object to a local file.
// Note: GetObject 404 maps to is_no_such_key(); HeadObject 404 maps to is_not_found() — intentional.
async fn download(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    output: &Path,
    verbose: bool,
) -> anyhow::Result<()> {
    let response = match client.get_object().bucket(bucket).key(key).send().await {
        Ok(response) => response,
        Err(aws_sdk_s3::error::SdkError::ServiceError(error)) if error.err().is_no_such_key() => {
            bail!("object not found in s3://{bucket}/{key}");
        }
        Err(error) => {
            return Err(error).context(
                "get_object failed — check bucket, key, credentials, endpoint and network",
            );
        }
    };

    // Convert the SDK body into an async reader and copy it in chunks.
    // This avoids loading the complete object into memory.
    let mut body = response.body.into_async_read();
    let mut file = tokio::fs::File::create(output)
        .await
        .with_context(|| format!("failed to create {}", output.display()))?;
    tokio::io::copy(&mut body, &mut file)
        .await
        .with_context(|| format!("failed to write {}", output.display()))?;

    if verbose {
        let length = tokio::fs::metadata(output)
            .await
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        eprintln!(
            "Downloaded s3://{bucket}/{key} -> {} ({} bytes)",
            output.display(),
            length
        );
    }
    println!("Downloaded s3://{bucket}/{key} to {}", output.display());
    Ok(())
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

/// Helper to obtain the hostname of the current machine.
fn get_hostname() -> String {
    hostname::get()
        .map(|h| h.to_string_lossy().into_owned())
        .or_else(|_| std::env::var("HOSTNAME"))
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

/// Handles the actual streaming upload to R2.
async fn upload(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    file: &Path,
    content_type: Option<String>,
    description: Option<String>,
    verbose: bool,
) -> anyhow::Result<()> {
    // mime_guess helps us set the correct Content-Type based on the file extension.
    let content_type = content_type.or_else(|| {
        mime_guess::from_path(file)
            .first()
            .map(|mime| mime.to_string())
    });

    // ByteStream::from_path reads the file asynchronously and streams it
    // to the network, preventing the need to load the whole file into RAM.
    let body = ByteStream::from_path(file)
        .await
        .with_context(|| format!("failed to read {}", file.display()))?;

    let host = get_hostname();

    if verbose {
        let length = tokio::fs::metadata(file)
            .await
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        let desc_info = description
            .as_deref()
            .map(|d| format!(", description: \"{d}\""))
            .unwrap_or_default();
        eprintln!(
            "Uploading {} ({} bytes) -> s3://{}/{} as {} (host: {}{})",
            file.display(),
            length,
            bucket,
            key,
            content_type
                .as_deref()
                .unwrap_or("application/octet-stream"),
            host,
            desc_info
        );
    }

    // Build the put_object request and automatically attach the host metadata.
    let mut request = client
        .put_object()
        .bucket(bucket)
        .key(key)
        .metadata("host", host)
        .body(body);
    if let Some(content_type) = content_type {
        request = request.content_type(content_type);
    }
    if let Some(description) = description {
        request = request.metadata("description", description);
    }

    request
        .send()
        .await
        .context("put_object failed — check bucket, credentials, endpoint and network")?;

    println!("Uploaded {} to s3://{}/{}", file.display(), bucket, key);
    Ok(())
}

/// Full metadata for a single R2 object (used by `stat`).
#[derive(Debug, Clone)]
struct StatInfo {
    key: String,
    bucket: String,
    size: i64,
    last_modified: Option<aws_smithy_types::DateTime>,
    etag: Option<String>,
    content_type: Option<String>,
    content_encoding: Option<String>,
    storage_class: Option<String>,
    host: String,
    description: String,
    metadata: std::collections::HashMap<String, String>,
}

/// Escape a string for JSON output without external dependencies.
fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            _ => out.push(c),
        }
    }
    out
}

/// Format a `StatInfo` as human-readable aligned text.
fn format_stat_human(info: &StatInfo) -> String {
    let last_modified = info
        .last_modified
        .as_ref()
        .map(format_date)
        .unwrap_or_else(|| "-".to_string());
    let etag = info.etag.as_deref().unwrap_or("-");
    let content_type = info.content_type.as_deref().unwrap_or("-");
    let content_encoding = info.content_encoding.as_deref().unwrap_or("-");
    let storage_class = info.storage_class.as_deref().unwrap_or("-");

    let mut out = format!(
        "Key:            {}\nBucket:         {}\nSize:           {} bytes\nLast-Modified:  {}\nETag:           {}\nContent-Type:   {}\nContent-Encoding: {}\nStorage-Class:  {}\nHost:           {}\nDescription:    {}",
        info.key,
        info.bucket,
        info.size,
        last_modified,
        etag,
        content_type,
        content_encoding,
        storage_class,
        info.host,
        info.description
    );

    // Show extra user metadata beyond host/description if present.
    let extra: Vec<_> = info
        .metadata
        .iter()
        .filter(|(k, _)| *k != "host" && *k != "description")
        .collect();
    if !extra.is_empty() {
        out.push_str("\nMetadata:");
        for (k, v) in extra {
            out.push_str(&format!("\n  {k}: {v}"));
        }
    }
    out
}

/// Format a `StatInfo` as single-line JSON.
fn format_stat_json(info: &StatInfo) -> String {
    let last_modified = info
        .last_modified
        .as_ref()
        .and_then(|d| d.fmt(aws_smithy_types::date_time::Format::DateTime).ok())
        .unwrap_or_else(|| "-".to_string());
    let etag = info.etag.as_deref().unwrap_or("-");
    let content_type = info.content_type.as_deref().unwrap_or("-");
    let content_encoding = info.content_encoding.as_deref().unwrap_or("-");
    let storage_class = info.storage_class.as_deref().unwrap_or("-");

    // Build metadata JSON object string.
    let mut meta_parts = Vec::new();
    for (k, v) in &info.metadata {
        meta_parts.push(format!("\"{}\":\"{}\"", escape_json(k), escape_json(v)));
    }
    let metadata_json = format!("{{{}}}", meta_parts.join(","));

    format!(
        "{{\"key\":\"{}\",\"bucket\":\"{}\",\"size\":{},\"lastModified\":\"{}\",\"eTag\":\"{}\",\"contentType\":\"{}\",\"contentEncoding\":\"{}\",\"storageClass\":\"{}\",\"host\":\"{}\",\"description\":\"{}\",\"metadata\":{}}}",
        escape_json(&info.key),
        escape_json(&info.bucket),
        info.size,
        escape_json(&last_modified),
        escape_json(etag),
        escape_json(content_type),
        escape_json(content_encoding),
        escape_json(storage_class),
        escape_json(&info.host),
        escape_json(&info.description),
        metadata_json
    )
}

/// Perform HeadObject and map into `StatInfo`. Handles NotFound (HeadObject only has is_not_found()).
// Note: HeadObject 404 is is_not_found(); do not use is_no_such_key() here (GetObject only).
async fn head_stat(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
) -> anyhow::Result<StatInfo> {
    let res = match client.head_object().bucket(bucket).key(key).send().await {
        Ok(res) => res,
        Err(aws_sdk_s3::error::SdkError::ServiceError(err)) if err.err().is_not_found() => {
            bail!("file not found: s3://{bucket}/{key}");
        }
        Err(err) => {
            return Err(err).context(
                "head_object failed — check bucket, key, credentials, endpoint and network",
            );
        }
    };

    let size = res.content_length().unwrap_or(0);
    let last_modified = res.last_modified().cloned();
    let etag = res.e_tag().map(|s| s.to_string());
    let content_type = res.content_type().map(|s| s.to_string());
    let content_encoding = res.content_encoding().map(|s| s.to_string());
    let storage_class = res.storage_class().map(|s| s.as_str().to_string());

    let meta_map = res.metadata().cloned().unwrap_or_default();
    let host = meta_map
        .get("host")
        .cloned()
        .filter(|h| !h.trim().is_empty())
        .unwrap_or_else(|| "-".to_string());
    let description = meta_map
        .get("description")
        .cloned()
        .filter(|d| !d.trim().is_empty())
        .unwrap_or_else(|| "-".to_string());

    Ok(StatInfo {
        key: key.to_string(),
        bucket: bucket.to_string(),
        size,
        last_modified,
        etag,
        content_type,
        content_encoding,
        storage_class,
        host,
        description,
        metadata: meta_map,
    })
}

/// Attempts to fetch the `host` and `description` user metadata fields for a given object using HeadObject.
/// Delegates to `head_stat` to avoid duplication; returns "-" pair on any failure.
async fn fetch_object_metadata(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
) -> (String, String) {
    head_stat(client, bucket, key)
        .await
        .map(|info| (info.host, info.description))
        .unwrap_or_else(|_| ("-".to_string(), "-".to_string()))
}

/// Formats a DateTime into a clean, human-readable string (YYYY-MM-DD HH:MM:SS).
fn format_date(date: &aws_smithy_types::DateTime) -> String {
    let Ok(s) = date.fmt(aws_smithy_types::date_time::Format::DateTime) else {
        return "-".to_string();
    };
    if s.len() >= 19 {
        s[..19].replace('T', " ")
    } else {
        s.replace('T', " ").replace('Z', "")
    }
}

/// Item representation for long list format table output.
#[derive(Debug)]
struct LongListItem {
    key: String,
    size: u64,
    modified: String,
    host: String,
    description: String,
}

/// Formats long list items into an aligned tabular string.
fn format_long_table(items: &[LongListItem]) -> String {
    if items.is_empty() {
        return String::new();
    }

    let w_key = items.iter().map(|i| i.key.len()).max().unwrap_or(0).max(3);
    let w_size = items
        .iter()
        .map(|i| i.size.to_string().len())
        .max()
        .unwrap_or(0)
        .max(4);
    let w_mod = items
        .iter()
        .map(|i| i.modified.len())
        .max()
        .unwrap_or(0)
        .max(13);
    let w_host = items.iter().map(|i| i.host.len()).max().unwrap_or(0).max(4);
    let w_desc = items
        .iter()
        .map(|i| i.description.len())
        .max()
        .unwrap_or(0)
        .max(11);

    let mut out = Vec::with_capacity(items.len() + 1);
    out.push(format!(
        "{:<w_key$}  {:>w_size$}  {:<w_mod$}  {:<w_host$}  {:<w_desc$}",
        "KEY", "SIZE", "LAST_MODIFIED", "HOST", "DESCRIPTION"
    ));

    for item in items {
        out.push(format!(
            "{:<w_key$}  {:>w_size$}  {:<w_mod$}  {:<w_host$}  {:<w_desc$}",
            item.key, item.size, item.modified, item.host, item.description
        ));
    }

    out.join("\n")
}

/// Prints long list items formatted as an aligned table.
fn print_long_table(items: &[LongListItem]) {
    let table = format_long_table(items);
    if !table.is_empty() {
        println!("{table}");
    }
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
    let mut long_items = Vec::new();

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
                let size = object.size().unwrap_or(0) as u64;
                let modified = object
                    .last_modified()
                    .map(format_date)
                    .unwrap_or_else(|| "-".to_string());
                let (host, description) = fetch_object_metadata(client, bucket, key).await;
                long_items.push(LongListItem {
                    key: key.to_string(),
                    size,
                    modified,
                    host,
                    description,
                });
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

    if long {
        print_long_table(&long_items);
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

    #[test]
    fn derives_output_path_from_key() {
        assert_eq!(
            derive_output_path("images/photo.jpg", None).unwrap(),
            PathBuf::from("photo.jpg")
        );
    }

    #[test]
    fn preserves_explicit_output_path() {
        let output = PathBuf::from("downloads/photo.jpg");
        assert_eq!(
            derive_output_path("images/photo.jpg", Some(output.clone())).unwrap(),
            output
        );
    }

    #[test]
    fn returns_hostname() {
        let host = get_hostname();
        assert!(!host.is_empty());
    }

    #[test]
    fn formats_date_naturally() {
        let dt = aws_smithy_types::DateTime::from_secs_and_nanos(1786121765, 613_000_000);
        assert_eq!(format_date(&dt), "2026-08-07 16:56:05");
    }

    #[test]
    fn formats_long_table_aligned() {
        let items = vec![
            LongListItem {
                key: "photo.jpg".to_string(),
                size: 89201,
                modified: "2026-08-07 16:56:05".to_string(),
                host: "host1".to_string(),
                description: "Vacation".to_string(),
            },
            LongListItem {
                key: "test/README.md".to_string(),
                size: 4614,
                modified: "2026-08-07 17:00:00".to_string(),
                host: "host2".to_string(),
                description: "-".to_string(),
            },
        ];

        let table = format_long_table(&items);
        let lines: Vec<&str> = table.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("KEY"));
        assert!(lines[0].contains("SIZE"));
        assert!(lines[0].contains("LAST_MODIFIED"));
        assert!(lines[0].contains("HOST"));
        assert!(lines[0].contains("DESCRIPTION"));
    }

    #[test]
    fn escapes_json_special_chars() {
        assert_eq!(escape_json("a\"b"), "a\\\"b");
        assert_eq!(escape_json("a\\b"), "a\\\\b");
        assert_eq!(escape_json("a\nb"), "a\\nb");
        assert_eq!(escape_json("a\tb"), "a\\tb");
    }

    #[test]
    fn formats_stat_human_contains_labels() {
        let info = StatInfo {
            key: "images/photo.jpg".to_string(),
            bucket: "my-bucket".to_string(),
            size: 89201,
            last_modified: Some(aws_smithy_types::DateTime::from_secs_and_nanos(
                1786121765,
                613_000_000,
            )),
            etag: Some("\"abc123\"".to_string()),
            content_type: Some("image/jpeg".to_string()),
            content_encoding: None,
            storage_class: Some("STANDARD".to_string()),
            host: "my-host".to_string(),
            description: "Vacation".to_string(),
            metadata: std::collections::HashMap::new(),
        };
        let out = format_stat_human(&info);
        assert!(out.contains("Key:"));
        assert!(out.contains("images/photo.jpg"));
        assert!(out.contains("Bucket:"));
        assert!(out.contains("my-bucket"));
        assert!(out.contains("89201 bytes"));
        assert!(out.contains("2026-08-07 16:56:05"));
        assert!(out.contains("Host:"));
        assert!(out.contains("my-host"));
        assert!(out.contains("Description:"));
        assert!(out.contains("Vacation"));
    }

    #[test]
    fn formats_stat_json_roundtrip() {
        let mut meta = std::collections::HashMap::new();
        meta.insert("host".to_string(), "my-host".to_string());
        meta.insert("description".to_string(), "A \"quoted\" desc".to_string());
        let info = StatInfo {
            key: "images/photo.jpg".to_string(),
            bucket: "my-bucket".to_string(),
            size: 1234,
            last_modified: None,
            etag: None,
            content_type: Some("image/jpeg".to_string()),
            content_encoding: None,
            storage_class: None,
            host: "my-host".to_string(),
            description: "A \"quoted\" desc".to_string(),
            metadata: meta,
        };
        let json = format_stat_json(&info);
        assert!(json.contains("\"key\":\"images/photo.jpg\""));
        assert!(json.contains("\"size\":1234"));
        assert!(json.contains("\"host\":\"my-host\""));
        // Ensure quotes are escaped
        assert!(json.contains("A \\\"quoted\\\" desc"));
        // Must be valid-ish: starts with { ends with }
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
    }

    #[test]
    fn formats_stat_human_shows_extra_metadata() {
        let mut meta = std::collections::HashMap::new();
        meta.insert("host".to_string(), "h".to_string());
        meta.insert("description".to_string(), "d".to_string());
        meta.insert("custom".to_string(), "val".to_string());
        let info = StatInfo {
            key: "k".to_string(),
            bucket: "b".to_string(),
            size: 0,
            last_modified: None,
            etag: None,
            content_type: None,
            content_encoding: None,
            storage_class: None,
            host: "h".to_string(),
            description: "d".to_string(),
            metadata: meta,
        };
        let out = format_stat_human(&info);
        assert!(out.contains("Metadata:"));
        assert!(out.contains("custom: val"));
    }

    #[test]
    fn formats_stat_human_missing_fields() {
        let info = StatInfo {
            key: "k".to_string(),
            bucket: "b".to_string(),
            size: 0,
            last_modified: None,
            etag: None,
            content_type: None,
            content_encoding: None,
            storage_class: None,
            host: "-".to_string(),
            description: "-".to_string(),
            metadata: std::collections::HashMap::new(),
        };
        let out = format_stat_human(&info);
        // All optional fields should render as "-"
        assert!(out.contains("Last-Modified:  -"));
        assert!(out.contains("ETag:           -"));
        assert!(out.contains("Content-Type:   -"));
        assert!(out.contains("Content-Encoding: -"));
        assert!(out.contains("Storage-Class:  -"));
        let json = format_stat_json(&info);
        assert!(json.contains("\"lastModified\":\"-\""));
        assert!(json.contains("\"eTag\":\"-\""));
    }
}
