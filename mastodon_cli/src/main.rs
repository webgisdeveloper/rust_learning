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
use serde::Serialize;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};

/// Simple CLI tool to post a message to Mastodon
// #[derive(Parser)] allows clap to automatically generate the CLI argument parser 
// from this struct. It maps struct fields to command-line flags.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The message to post
    // #[arg(short, long)] defines both -m and --message as valid flags.
    #[arg(short, long)]
    message: String,

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
}

/// Converts known Mastodon shortcodes into real emoji characters.
/// For now we only replace `:apple:` because that is the requested behavior.
fn replace_emoji_shortcodes(message: &str) -> String {
    // Keep mappings in one place so adding new shortcodes stays beginner-friendly.
    let shortcode_mappings = [(":apple:", "🍎")];

    // Start from the original message and apply each replacement one by one.
    let mut converted = message.to_string();
    for (shortcode, emoji) in shortcode_mappings {
        converted = converted.replace(shortcode, emoji);
    }
    converted
}

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

    // Create the data structure we want to send as JSON.
    let body = StatusRequest {
        // Replace shortcodes before sending the status to Mastodon.
        status: replace_emoji_shortcodes(&args.message),
    };

    // reqwest::Client is designed to be reused across requests for efficiency 
    // (it manages a connection pool).
    let client = reqwest::Client::new();

    // The Mastodon API endpoint for posting a status.
    let url = "https://mastodon.social/api/v1/statuses";

    // Chain methods to build the request. 
    // .json(&body) automatically sets the content-type to application/json 
    // and serializes the StatusRequest struct.
    let response = client
        .post(url)
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .header(CONTENT_TYPE, "application/json")
        .json(&body)
        .send()
        .await?; // .await is used because sending the request is an asynchronous operation.

    // Check if the response status code is in the 200-299 range.
    if response.status().is_success() {
        println!("Successfully posted message to Mastodon!");
    } else {
        let status = response.status();
        // We await the body text for more detailed error reporting from the API.
        let error_text = response.text().await?;
        eprintln!("Error posting message: {} - {}", status, error_text);
        std::process::exit(1);
    }

    // Returning Ok(()) indicates the program finished successfully.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::replace_emoji_shortcodes;

    #[test]
    fn replaces_apple_shortcode_with_emoji() {
        let result = replace_emoji_shortcodes("I am eating an :apple:");
        assert_eq!(result, "I am eating an 🍎");
    }

    #[test]
    fn keeps_message_unchanged_when_shortcode_is_absent() {
        let result = replace_emoji_shortcodes("No shortcode here");
        assert_eq!(result, "No shortcode here");
    }
}
