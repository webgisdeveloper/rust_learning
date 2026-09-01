use crate::cli::ListArgs;
use crate::client::{build_client, endpoint_for};
use crate::commands::stat::head_stat;
use crate::format::{LongListItem, format_date, print_long_table};
use anyhow::{Context, bail};
use futures::stream::{self, StreamExt};

const MAX_CONCURRENT_METADATA_FETCHES: usize = 10;

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

    list_objects(
        &client,
        &args.r2.bucket,
        args.prefix.as_deref(),
        args.long,
        verbose,
    )
    .await
}

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

async fn list_objects(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    prefix: Option<&str>,
    long: bool,
    verbose: bool,
) -> anyhow::Result<()> {
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

        let response = request
            .send()
            .await
            .context("list_objects failed — check bucket, credentials, endpoint and network")?;

        let contents = response.contents();

        if first_page && contents.is_empty() && verbose {
            if let Some(prefix) = prefix {
                eprintln!("No objects found in s3://{bucket}/ with prefix \"{prefix}\"");
            } else {
                eprintln!("No objects found in s3://{bucket}/");
            }
        }
        first_page = false;

        if long {
            let page_items: Vec<_> = contents
                .iter()
                .map(|object| {
                    let key = object.key().unwrap_or("<no-key>").to_string();
                    let size = object.size().unwrap_or(0) as u64;
                    let modified = object
                        .last_modified()
                        .map(format_date)
                        .unwrap_or_else(|| "-".to_string());
                    (key, size, modified)
                })
                .collect();

            total += page_items.len();

            let client_ref = client;
            let fetched = stream::iter(page_items)
                .map(|(key, size, modified)| async move {
                    let (host, description) = fetch_object_metadata(client_ref, bucket, &key).await;
                    LongListItem {
                        key,
                        size,
                        modified,
                        host,
                        description,
                    }
                })
                .buffer_unordered(MAX_CONCURRENT_METADATA_FETCHES)
                .collect::<Vec<_>>()
                .await;

            long_items.extend(fetched);
        } else {
            for object in contents {
                total += 1;
                let key = object.key().unwrap_or("<no-key>");
                println!("{key}");
            }
        }

        if !response.is_truncated().unwrap_or(false) {
            break;
        }

        continuation_token = response.next_continuation_token().map(str::to_owned);

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
