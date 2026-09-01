use crate::cli::DeleteArgs;
use crate::client::{build_client, endpoint_for};
use crate::glob::{expand_remote_keys, shell_expansion_hint};
use anyhow::{Context, bail};
use futures::stream::{self, StreamExt};

const MAX_CONCURRENT_DELETES: usize = 16;

pub async fn run_delete(args: DeleteArgs, verbose: bool) -> anyhow::Result<()> {
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
        bail!("no objects to delete");
    }

    if verbose {
        if keys.len() == 1 {
            eprintln!("Key: {}", keys[0]);
        } else {
            eprintln!("Keys: {} ({} matched)", keys.join(", "), keys.len());
        }
    }

    let client_ref = &client;
    let bucket_ref = &args.r2.bucket;

    let results = stream::iter(keys.clone())
        .map(|key| async move {
            let res = delete(client_ref, bucket_ref, &key, verbose).await;
            (key, res)
        })
        .buffer_unordered(MAX_CONCURRENT_DELETES)
        .collect::<Vec<_>>()
        .await;

    let mut deleted = 0usize;
    let mut failures = Vec::new();

    for (key, res) in results {
        match res {
            Ok(_) => deleted += 1,
            Err(e) => {
                let msg = format!("{key}: {e:#}");
                eprintln!("Failed to delete {msg}");
                failures.push(msg.clone());
                if keys.len() == 1 {
                    bail!("{msg}");
                }
            }
        }
    }

    if !failures.is_empty() {
        bail!(
            "deleted {deleted}/{} object(s); failures:\n{}",
            keys.len(),
            failures.join("\n")
        );
    }

    if verbose && keys.len() > 1 {
        eprintln!("Deleted {deleted} object(s) from s3://{}/", args.r2.bucket);
    }

    Ok(())
}

async fn delete(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    verbose: bool,
) -> anyhow::Result<()> {
    match client.head_object().bucket(bucket).key(key).send().await {
        Ok(_) => {}
        Err(aws_sdk_s3::error::SdkError::ServiceError(error)) if error.err().is_not_found() => {
            let hint = shell_expansion_hint(key);
            bail!("file not found: s3://{bucket}/{key}{hint}");
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
