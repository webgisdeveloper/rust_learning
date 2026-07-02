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
use regex::Regex;

/// Replaces :shortcodes: with actual emoji characters.
fn replace_emojis(text: &str) -> String {
    // Regex to find patterns like :apple: or :smile:
    let re = Regex::new(r":([a-z0-9_]+):").unwrap();
    
    // replace_all takes a closure that receives the matched text (Caps).
    // We extract the word inside the colons and look it up in our mapping.
    re.replace_all(text, |caps: &regex::Captures| {
        let shortcode = &caps[1];
        match shortcode {
            "apple" => "🍎".to_string(),
            "banana" => "🍌".to_string(),
            "cherry" => "🍒".to_string(),
            "strawberry" => "🍓".to_string(),
            "grape" => "🍇".to_string(),
            "pizza" => "🍕".to_string(),
            "hamburger" => "🍔".to_string(),
            "beer" => "🍺".to_string(),
            "coffee" => "☕".to_string(),
            "smile" => "😄".to_string(),
            "joy" => "😂".to_string(),
            "sob" => "😭".to_string(),
            "angry" => "😡".to_string(),
            "heart" => "❤️".to_string(),
            "fire" => "🔥".to_string(),
            "thumbsup" => "👍".to_string(),
            "thumbsdown" => "👎".to_string(),
            "star" => "⭐".to_string(),
            "rocket" => "🚀".to_string(),
            "party_popper" => "🎉".to_string(),
            "sun" => "☀️".to_string(),
            "moon" => "🌙".to_string(),
            "dog" => "🐶".to_string(),
            "cat" => "🐱".to_string(),
            "leaf" => "🍃".to_string(),
            "cloud" => "☁️".to_string(),
            // If no match is found, return the original shortcode as a String.
            _ => caps[0].to_string(),
        }
    }).into_owned()
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
    // We process the message to replace shortcodes (e.g., :apple:) with real emojis.
    let body = StatusRequest {
        status: replace_emojis(&args.message),
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
