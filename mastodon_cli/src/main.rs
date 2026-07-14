/*
WORKFLOW OVERVIEW:
1. Argument Parsing: The program starts by parsing command-line arguments using `clap`.
2. Authentication: It resolves the API token, prioritizing the CLI flag over the 
   `MASTODON_TOKEN` environment variable.
3. Data Preparation: It creates a `StatusRequest` struct and uses `serde` to 
   serialize it into JSON.
4. Async Request: Using the `tokio` runtime and `reqwest` client, it sends an 
   asynchronous HTTP POST request to the Mastodon API.
5. Response Handling: It checks the HTTP status code and either confirms success 
   or prints the API error message.

LIBRARIES USED:
- `clap`: The standard for CLI argument parsing in Rust. Its "derive" feature 
   allows defining the interface as a simple Rust struct.
- `reqwest`: A powerful HTTP client. It is asynchronous by default, allowing 
   the program to handle I/O without blocking the main thread.
- `tokio`: The industry-standard asynchronous runtime. Since Rust's standard 
   library provides the `Future` trait but not the executor, `tokio` is used 
   to actually run the async code.
- `serde` (Serialization/Deserialization): A framework for efficiently 
   converting Rust data structures to other formats (like JSON) and vice versa.
*/

use clap::Parser;
use serde::{Serialize, Deserialize};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use regex::Regex;

/// Replaces :shortcodes: with actual emoji characters using a single-pass regex.
/// Uses the `emojis` crate for comprehensive Unicode support.
fn replace_emojis(text: &str) -> String {
    let re = Regex::new(r":([a-z0-9_]+):").unwrap();
    
    re.replace_all(text, |caps: &regex::Captures| {
        let shortcode = &caps[1];
        // Look up the shortcode using the `emojis` crate.
        // If found, return the emoji character; otherwise, keep the original text.
        match emojis::get_by_shortcode(shortcode) {
            Some(emoji) => emoji.as_str().to_string(),
            None => caps[0].to_string(),
        }
    }).into_owned()
}

/// Strips HTML tags and decodes HTML entities.
fn clean_html(text: &str) -> String {
    // 1. Remove HTML tags using regex
    let re = Regex::new(r"<[^>]*>").unwrap();
    let stripped = re.replace_all(text, "");
    
    // 2. Decode HTML entities (e.g., &gt; -> >)
    html_escape::decode_html_entities(&stripped).into_owned()
}

/// Uploads a media file to Mastodon and returns the media ID.
async fn upload_media(client: &reqwest::Client, token: &str, file_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let url = "https://mastodon.social/api/v1/media";
    
    // Create a multipart form
    let file_bytes = std::fs::read(file_path)
        .map_err(|e| format!("Failed to read image file {}: {}", file_path, e))?;
    
    let part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name("image.jpg"); // Simplification: assuming jpg, Mastodon handles mime type
    
    let form = reqwest::multipart::Form::new()
        .part("file", part);

    let response = client
        .post(url)
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        return Err(format!("Media upload failed: {} - {}", status, error_text).into());
    }

    let media_resp = response.json::<MediaResponse>().await?;
    Ok(media_resp.id)
}

/// Simple CLI tool to post a message to Mastodon
// #[derive(Parser)] allows clap to automatically generate the CLI argument parser 
// from this struct. It maps struct fields to command-line flags.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The message to post
    // #[arg(short, long)] defines both -m and --message as valid flags.
    #[arg(short, long)]
    message: Option<String>,

    /// Path to an image to upload
    #[arg(short, long)]
    image: Option<String>,

    /// The Mastodon access token
    // We make this optional so we can fall back to the environment variable.
    #[arg(short, long)]
    token: Option<String>,
}

/// Request body for creating a new status
// #[derive(Serialize)] allows serde to convert this Rust struct into a JSON object.
#[derive(Serialize)]
struct StatusRequest {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    media_ids: Option<Vec<String>>,
}

/// Response model for media upload
#[derive(Deserialize, Debug)]
struct MediaResponse {
    id: String,
}

/// Response model for account verification
#[derive(Deserialize, Debug)]
struct Account {
    id: String,
}

/// Response model for a single status
#[derive(Deserialize, Debug)]
struct Status {
    content: String,
    media_attachments: Vec<MediaAttachment>,
}

/// Response model for a media attachment
#[derive(Deserialize, Debug)]
struct MediaAttachment {}

// #[tokio::main] marks the main function as an asynchronous entry point, 
// setting up the Tokio runtime required by reqwest.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Args::parse() is provided by clap's Parser trait; it handles input validation 
    // and exits with a helpful message if required arguments are missing.
    let args = Args::parse();

    // Resolve the token: use CLI flag if provided, otherwise check environment variable.
    let token = args.token.or_else(|| std::env::var("MASTODON_TOKEN").ok())
        .ok_or("Mastodon token not provided. Please use --token or set MASTODON_TOKEN env var.")?;

    // reqwest::Client is designed to be reused across requests for efficiency 
    // (it manages a connection pool).
    let client = reqwest::Client::new();
    let auth_header = format!("Bearer {}", token);

    if let Some(msg) = args.message {
        // CASE 1: Post a new message
        
        // 1. Optional: Upload image if provided
        let mut media_ids = None;
        if let Some(image_path) = &args.image {
            println!("Uploading image: {}...", image_path);
            let media_id = upload_media(&client, &token, image_path).await?;
            media_ids = Some(vec![media_id]);
            println!("Image uploaded successfully.");
        }

        let body = StatusRequest {
            status: replace_emojis(&msg),
            media_ids,
        };

        let url = "https://mastodon.social/api/v1/statuses";
        let response = client
            .post(url)
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
            eprintln!("Error posting message: {} - {}", status, error_text);
            std::process::exit(1);
        }
    } else {
        // CASE 2: Fetch and print recent messages
        // 1. Get Account ID
        let account_url = "https://mastodon.social/api/v1/accounts/verify_credentials";
        let account_resp = client
            .get(account_url)
            .header(AUTHORIZATION, &auth_header)
            .send()
            .await?;

        if !account_resp.status().is_success() {
            let status = account_resp.status();
            let error_text = account_resp.text().await?;
            eprintln!("Error verifying credentials: {} - {}", status, error_text);
            std::process::exit(1);
        }

        let account_id = account_resp.json::<Account>().await?.id;

        // 2. Get recent statuses
        let statuses_url = format!("https://mastodon.social/api/v1/accounts/{}/statuses?limit=5", account_id);
        let statuses_resp = client
            .get(statuses_url)
            .header(AUTHORIZATION, &auth_header)
            .send()
            .await?;

        if !statuses_resp.status().is_success() {
            let status = statuses_resp.status();
            let error_text = statuses_resp.text().await?;
            eprintln!("Error fetching statuses: {} - {}", status, error_text);
            std::process::exit(1);
        }

        let statuses = statuses_resp.json::<Vec<Status>>().await?;

        if statuses.is_empty() {
            println!("No recent statuses found.");
        } else {
            println!("Recent 5 statuses:");
            for (i, status) in statuses.iter().enumerate() {
                let image_indicator = if !status.media_attachments.is_empty() { " 🖼️" } else { "" };
                println!("{}. {}{}", i + 1, clean_html(&status.content), image_indicator);
            }
        }
    }

    // Returning Ok(()) indicates the program finished successfully.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::replace_emojis;

    #[test]
    fn replaces_apple_shortcode_with_emoji() {
        let result = replace_emojis("I am eating an :apple:");
        assert_eq!(result, "I am eating an 🍎");
    }

    #[test]
    fn keeps_message_unchanged_when_shortcode_is_absent() {
        let result = replace_emojis("No shortcode here");
        assert_eq!(result, "No shortcode here");
    }

    #[test]
    fn replaces_multiple_common_shortcodes() {
        let result = replace_emojis("Launch :rocket: and celebrate :tada:");
        assert_eq!(result, "Launch 🚀 and celebrate 🎉");
    }
}
