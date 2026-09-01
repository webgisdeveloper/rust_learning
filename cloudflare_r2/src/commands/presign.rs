use crate::cli::PresignArgs;
use crate::client::{build_client, endpoint_for};
use anyhow::{Context, bail};
use aws_sdk_s3::presigning::PresigningConfig;
use std::time::Duration;

pub async fn run_presign(args: PresignArgs, verbose: bool) -> anyhow::Result<()> {
    let key = args.key.trim();
    if key.is_empty() {
        bail!("object key must not be empty");
    }

    let endpoint_url = endpoint_for(&args.r2)?;
    if verbose {
        eprintln!(
            "Endpoint: {}\nBucket: {}\nKey: {}\nExpires: {}s\nMethod: {}",
            endpoint_url, args.r2.bucket, key, args.expires, args.method
        );
    }

    let client = build_client(&endpoint_url, &args.r2.access_key, &args.r2.secret_key).await;
    let cfg = PresigningConfig::builder()
        .expires_in(Duration::from_secs(args.expires))
        .build()
        .context("invalid expires")?;

    let url = match args.method.as_str() {
        "get" => client
            .get_object()
            .bucket(&args.r2.bucket)
            .key(key)
            .presigned(cfg)
            .await
            .context("presign failed — check bucket, key, credentials, endpoint")?
            .uri()
            .to_string(),
        "put" => {
            let mut req = client.put_object().bucket(&args.r2.bucket).key(key);
            if let Some(ct) = args.content_type {
                req = req.content_type(ct);
            }
            req.presigned(cfg)
                .await
                .context("presign failed — check bucket, key, credentials, endpoint")?
                .uri()
                .to_string()
        }
        _ => unreachable!("clap restricts method to get|put"),
    };

    println!("{url}");
    if verbose {
        eprintln!(
            "Presigned {} s3://{}/{} ({}s)",
            args.method.to_uppercase(),
            args.r2.bucket,
            key,
            args.expires
        );
    }
    Ok(())
}
