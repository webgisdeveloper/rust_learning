use crate::cli::StatArgs;
use crate::client::{build_client, endpoint_for};
use crate::format::{StatInfo, format_stat_human, format_stat_json};
use anyhow::{Context, bail};

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
        println!("{}", format_stat_json(&info)?);
    } else {
        println!("{}", format_stat_human(&info));
    }

    if verbose {
        eprintln!("Stat s3://{}/{}", args.r2.bucket, key);
    }
    Ok(())
}

pub async fn head_stat(
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
