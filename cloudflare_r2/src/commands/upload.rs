use crate::cli::UploadArgs;
use crate::client::{build_client, endpoint_for, get_hostname};
use crate::glob::expand_file_patterns;
use anyhow::{Context, bail};
use aws_sdk_s3::primitives::ByteStream;
use futures::stream::{self, StreamExt};
use std::path::Path;

const MAX_CONCURRENT_UPLOADS: usize = 8;

pub async fn run_upload(args: UploadArgs, verbose: bool) -> anyhow::Result<()> {
    let files = expand_file_patterns(&args.files)?;

    if files.is_empty() {
        bail!("no files to upload");
    }

    if args.key.is_some() && files.len() > 1 {
        bail!(
            "cannot use --key with multiple files / wildcard patterns; use --folder to set a prefix instead"
        );
    }

    for file in &files {
        if !file.exists() {
            bail!("file not found: {}", file.display());
        }
        if !file.is_file() {
            bail!("not a file: {} (directories not supported)", file.display());
        }
    }

    let endpoint_url = endpoint_for(&args.r2)?;
    if verbose {
        if files.len() == 1 {
            eprintln!(
                "Endpoint: {}\nBucket: {}\nFile: {}",
                endpoint_url,
                args.r2.bucket,
                files[0].display()
            );
        } else {
            eprintln!(
                "Endpoint: {}\nBucket: {}\nFiles: {} ({} matched)",
                endpoint_url,
                args.r2.bucket,
                files
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                files.len()
            );
        }
    }

    let client = build_client(&endpoint_url, &args.r2.access_key, &args.r2.secret_key).await;

    // Prepare upload tasks
    let mut tasks = Vec::with_capacity(files.len());
    for file in &files {
        let key = resolve_key(file, args.key.clone(), args.folder.as_deref())?;
        tasks.push((file.clone(), key));
    }

    let client_ref = &client;
    let bucket_ref = &args.r2.bucket;
    let content_type_ref = &args.content_type;
    let description_ref = &args.description;

    let results = stream::iter(tasks)
        .map(|(file, key)| async move {
            let ct = content_type_ref.clone();
            let desc = description_ref.clone();
            let res = upload(client_ref, bucket_ref, &key, &file, ct, desc, verbose).await;
            (file, res)
        })
        .buffer_unordered(MAX_CONCURRENT_UPLOADS)
        .collect::<Vec<_>>()
        .await;

    let mut uploaded = 0usize;
    let mut failures = Vec::new();
    for (file, res) in results {
        match res {
            Ok(_) => uploaded += 1,
            Err(e) => {
                let msg = format!("{}: {e:#}", file.display());
                eprintln!("Failed to upload {msg}");
                failures.push(msg);
            }
        }
    }

    if !failures.is_empty() {
        bail!(
            "uploaded {uploaded}/{} file(s); failures:\n{}",
            files.len(),
            failures.join("\n")
        );
    }

    if verbose && files.len() > 1 {
        eprintln!(
            "Uploaded {uploaded} file(s) to s3://{}/{}",
            args.r2.bucket,
            args.folder.as_deref().unwrap_or("")
        );
    }

    Ok(())
}

fn resolve_key(file: &Path, key: Option<String>, folder: Option<&str>) -> anyhow::Result<String> {
    let base = key.unwrap_or_else(|| derive_key(file));
    let base_trimmed = base.trim();
    if base_trimmed.is_empty() {
        bail!("object key must not be empty; provide --key");
    }
    let Some(folder) = folder else {
        let k = base_trimmed.trim_start_matches('/').to_string();
        if k.is_empty() {
            bail!("object key must not be empty; provide --key");
        }
        return Ok(k);
    };
    let folder_trimmed = folder.trim();
    if folder_trimmed.is_empty() {
        let k = base_trimmed.trim_start_matches('/').to_string();
        if k.is_empty() {
            bail!("object key must not be empty; provide --key");
        }
        return Ok(k);
    }
    let normalized = folder_trimmed
        .split('/')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("/");
    if normalized.is_empty() {
        bail!("folder must not be empty; provide a valid folder name");
    }
    let base = base_trimmed.trim_start_matches('/').trim().to_string();
    if base.is_empty() {
        bail!("object key must not be empty; provide --key");
    }
    Ok(format!("{}/{}", normalized, base))
}

fn derive_key(file: &Path) -> String {
    file.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "upload".to_string())
}

async fn upload(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    file: &Path,
    content_type: Option<String>,
    description: Option<String>,
    verbose: bool,
) -> anyhow::Result<()> {
    let content_type = content_type.or_else(|| {
        mime_guess::from_path(file)
            .first()
            .map(|mime| mime.to_string())
    });

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_key_from_file_name() {
        assert_eq!(derive_key(Path::new("a/b/photo.jpg")), "photo.jpg");
    }
}
