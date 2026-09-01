use crate::cli::DownloadArgs;
use crate::client::{build_client, endpoint_for};
use crate::glob::{expand_remote_keys, shell_expansion_hint};
use anyhow::{Context, bail};
use futures::stream::{self, StreamExt};
use std::path::{Path, PathBuf};

const MAX_CONCURRENT_DOWNLOADS: usize = 8;

pub async fn run_download(args: DownloadArgs, verbose: bool) -> anyhow::Result<()> {
    if args.keys.is_empty() {
        bail!("object key must not be empty");
    }

    let endpoint_url = endpoint_for(&args.r2)?;
    if verbose {
        eprintln!("Endpoint: {}\nBucket: {}", endpoint_url, args.r2.bucket);
    }
    let client = build_client(&endpoint_url, &args.r2.access_key, &args.r2.secret_key).await;

    let keys = expand_remote_keys(&client, &args.r2.bucket, &args.keys).await?;

    if keys.is_empty() {
        bail!("no objects to download");
    }

    if keys.len() > 1 {
        if let Some(out) = &args.output {
            if out.exists() && out.is_file() {
                bail!(
                    "--output must be a directory when downloading multiple files: {}",
                    out.display()
                );
            }
            if verbose {
                eprintln!(
                    "Keys: {} ({} matched)\nOutput dir: {}",
                    keys.join(", "),
                    keys.len(),
                    out.display()
                );
            }
        } else if verbose {
            eprintln!("Keys: {} ({} matched)", keys.join(", "), keys.len());
        }
    } else if verbose {
        eprintln!("Key: {}", keys[0]);
    }

    // Resolve local destinations
    let mut tasks = Vec::with_capacity(keys.len());
    for key in &keys {
        let output: PathBuf = if keys.len() == 1 {
            if let Some(p) = &args.output {
                if p.is_dir() {
                    let fname = Path::new(key).file_name().with_context(|| {
                        format!("object key does not contain a filename: {key}")
                    })?;
                    p.join(fname)
                } else {
                    derive_output_path(key, Some(p.clone()))?
                }
            } else {
                derive_output_path(key, None)?
            }
        } else {
            let dir = args.output.as_deref().unwrap_or(Path::new("."));
            let fname = Path::new(key)
                .file_name()
                .with_context(|| format!("object key does not contain a filename: {key}"))?;
            dir.join(fname)
        };
        tasks.push((key.clone(), output));
    }

    let client_ref = &client;
    let bucket_ref = &args.r2.bucket;
    let force = args.force;

    let results = stream::iter(tasks)
        .map(|(key, output)| async move {
            if output.exists() && !force {
                let msg = format!(
                    "destination already exists: {}; use --force to overwrite",
                    output.display()
                );
                return (key, Err(anyhow::anyhow!(msg)));
            }

            if let Some(parent) = output.parent()
                && !parent.as_os_str().is_empty()
                && let Err(e) = tokio::fs::create_dir_all(parent).await
            {
                return (
                    key,
                    Err(anyhow::anyhow!(
                        "failed to create directory {}: {e}",
                        parent.display()
                    )),
                );
            }

            if verbose {
                eprintln!(
                    "Downloading s3://{}/{} -> {}",
                    bucket_ref,
                    key,
                    output.display()
                );
            }

            let res = download(client_ref, bucket_ref, &key, &output, verbose).await;
            (key, res)
        })
        .buffer_unordered(MAX_CONCURRENT_DOWNLOADS)
        .collect::<Vec<_>>()
        .await;

    let mut downloaded = 0usize;
    let mut failures = Vec::new();

    for (key, res) in results {
        match res {
            Ok(_) => downloaded += 1,
            Err(e) => {
                let msg = format!("{key}: {e:#}");
                eprintln!("Failed to download {msg}");
                failures.push(msg.clone());
                if keys.len() == 1 {
                    bail!("{msg}");
                }
            }
        }
    }

    if !failures.is_empty() {
        bail!(
            "downloaded {downloaded}/{} file(s); failures:\n{}",
            keys.len(),
            failures.join("\n")
        );
    }

    if verbose && keys.len() > 1 {
        eprintln!("Downloaded {downloaded} file(s)");
    }

    Ok(())
}

pub fn derive_output_path(key: &str, output: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    if let Some(output) = output {
        return Ok(output);
    }

    Path::new(key)
        .file_name()
        .filter(|name| !name.is_empty())
        .map(PathBuf::from)
        .context("object key does not contain a filename; provide --output")
}

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
            let hint = shell_expansion_hint(key);
            bail!("object not found in s3://{bucket}/{key}{hint}");
        }
        Err(error) => {
            return Err(error).context(
                "get_object failed — check bucket, key, credentials, endpoint and network",
            );
        }
    };

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
