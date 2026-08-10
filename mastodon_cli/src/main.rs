mod api;
mod cli;
mod format;

use api::{api_url, Account, Status, StatusRequest};
use clap::Parser;
use cli::Args;
use format::replace_emojis;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};

const DEFAULT_INSTANCE: &str = "https://mastodon.social";

fn resolve_instance(arg: Option<String>) -> String {
    let configured = arg
        .or_else(|| std::env::var("MASTODON_INSTANCE").ok())
        .unwrap_or_else(|| DEFAULT_INSTANCE.to_string());
    api::normalize_instance(&configured)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let token = args.token
        .or_else(|| std::env::var("MASTODON_TOKEN").ok())
        .ok_or("Mastodon token not provided. Please use --token or set MASTODON_TOKEN env var.")?;
    let instance = resolve_instance(args.instance);
    let client = reqwest::Client::new();
    let auth_header = format!("Bearer {token}");

    if let Some(msg) = args.message {
        let mut media_ids = None;
        if let Some(image_path) = &args.image {
            println!("Uploading image: {image_path}...");
            let media_id = api::upload_media(&client, &token, &instance, image_path).await?;
            media_ids = Some(vec![media_id]);
            println!("Image uploaded successfully.");
        }

        let body = StatusRequest { status: replace_emojis(&msg), media_ids };
        let response = client
            .post(api_url(&instance, "/api/v1/statuses"))
            .header(AUTHORIZATION, &auth_header)
            .header(CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await?;

        if response.status().is_success() {
            println!("Successfully posted message to Mastodon!");
        } else {
            let status = response.status();
            let error_text = response.text().await?;
            eprintln!("Error posting message: {status} - {error_text}");
            std::process::exit(1);
        }
    } else {
        let account_resp = client
            .get(api_url(&instance, "/api/v1/accounts/verify_credentials"))
            .header(AUTHORIZATION, &auth_header)
            .send()
            .await?;
        if !account_resp.status().is_success() {
            let status = account_resp.status();
            let error_text = account_resp.text().await?;
            eprintln!("Error verifying credentials: {status} - {error_text}");
            std::process::exit(1);
        }
        let account_id = account_resp.json::<Account>().await?.id;
        let statuses_url = api_url(&instance, &format!("/api/v1/accounts/{account_id}/statuses?limit={}", args.list));
        let statuses_resp = client
            .get(statuses_url)
            .header(AUTHORIZATION, &auth_header)
            .send()
            .await?;
        if !statuses_resp.status().is_success() {
            let status = statuses_resp.status();
            let error_text = statuses_resp.text().await?;
            eprintln!("Error fetching statuses: {status} - {error_text}");
            std::process::exit(1);
        }

        let statuses = statuses_resp.json::<Vec<Status>>().await?;
        if statuses.is_empty() {
            println!("No recent statuses found.");
        } else {
            println!("Recent {} statuses:\n", args.list);
            for (i, status) in statuses.iter().enumerate() {
                println!("{}\n", format::format_status(i, status));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_default_instance() {
        assert_eq!(api::normalize_instance("mastodon.social/"), "https://mastodon.social");
        assert_eq!(api_url("https://example.social/", "/api/v1/statuses"), "https://example.social/api/v1/statuses");
    }
}
