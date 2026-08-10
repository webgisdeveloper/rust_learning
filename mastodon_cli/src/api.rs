//! `api` — Mastodon HTTP API models and helpers
//!
//! This module isolates all knowledge about the Mastodon REST API:
//! URL construction, request/response structs, and the media upload helper.
//! Keeping it separate from `main.rs` makes the binary easier to extend
//! (new endpoints) and to test (URL helpers are pure functions).
//!
//! Libraries used here:
//! - `serde` / `serde_json`: serialize Rust structs → JSON and deserialize JSON → Rust.
//! - `reqwest`: async HTTP client (wraps `tokio`, connection pooling, TLS).
//! - `std::fs`: synchronous file read for the image upload (simple for a CLI).

use reqwest::header::AUTHORIZATION;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// URL helpers — pure functions, easy to unit-test
// ---------------------------------------------------------------------------

/// Normalizes a user-provided instance string into a canonical base URL.
///
/// Learner notes:
/// - `&str` is a borrowed string slice; we return an owned `String` because
///   we may need to allocate (e.g. prepending `https://`).
/// - `trim()` removes whitespace; `trim_end_matches('/')` removes a trailing
///   slash so `format!("{}{}", base, "/api/v1/...")` never produces `//`.
/// - We accept both `mastodon.social` and `https://mastodon.social/` for
///   ergonomics. If no scheme is given, we default to `https://`.
pub(crate) fn normalize_instance(instance: &str) -> String {
    let trimmed = instance.trim();
    let without_slash = trimmed.trim_end_matches('/');
    if without_slash.is_empty() {
        // Fallback if someone passes "" or "/"
        "https://mastodon.social".to_string()
    } else if without_slash.starts_with("http://") || without_slash.starts_with("https://") {
        without_slash.to_string()
    } else {
        // Bare hostname like "mastodon.social" → add scheme
        format!("https://{without_slash}")
    }
}

/// Joins a normalized instance base with an API path.
///
/// Example: `api_url("https://mastodon.social/", "/api/v1/statuses")`
///          → `"https://mastodon.social/api/v1/statuses"`
/// Using a helper avoids hard-coding `https://mastodon.social` in every call
/// site and guarantees consistent slash handling.
pub(crate) fn api_url(instance: &str, path: &str) -> String {
    format!("{}{}", normalize_instance(instance), path)
}

// ---------------------------------------------------------------------------
// Request / Response models — serde does the JSON mapping
// ---------------------------------------------------------------------------

/// Request body for `POST /api/v1/statuses` (creating a new status).
///
/// `#[derive(Serialize)]` lets `reqwest::Client::json(&body)` turn this
/// struct into JSON automatically. Field names map 1:1 to JSON keys.
#[derive(Serialize)]
pub(crate) struct StatusRequest {
    /// The text content of the toot. Already emoji-expanded by `format::replace_emojis`.
    pub(crate) status: String,
    /// Optional list of previously-uploaded media IDs to attach.
    /// `skip_serializing_if = "Option::is_none"` means the JSON key is omitted
    /// entirely when `None` — the API treats missing vs `null` differently.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) media_ids: Option<Vec<String>>,
}

/// Minimal response from `POST /api/v1/media` (media upload).
///
/// Mastodon returns many fields; we only deserialize the one we need (`id`).
/// `serde` will ignore unknown fields by default — this makes the struct
/// forward-compatible if the API adds new keys.
#[derive(Deserialize, Debug)]
pub(crate) struct MediaResponse {
    pub(crate) id: String,
}

/// Minimal response from `GET /api/v1/accounts/verify_credentials`.
///
/// We only need the account `id` to then fetch `GET /accounts/{id}/statuses`.
#[derive(Deserialize, Debug)]
pub(crate) struct Account {
    pub(crate) id: String,
}

/// A single status (toot) returned by `GET /api/v1/accounts/{id}/statuses`.
///
/// `content` is HTML from the API (e.g. `<p>Hello <a>world</a></p>`);
/// we clean it with `format::clean_html` before display.
#[derive(Deserialize, Debug)]
pub(crate) struct Status {
    /// HTML content — needs `clean_html` before printing.
    pub(crate) content: String,
    /// Media attachments — we only check `is_empty()` to show the 🖼️ indicator.
    /// The struct is empty because we don't need attachment details yet.
    pub(crate) media_attachments: Vec<MediaAttachment>,
    /// If `Some`, this status is a reply (used to show the 🧵 indicator).
    pub(crate) in_reply_to_id: Option<String>,
}

/// Placeholder for a media attachment.
///
/// Currently a unit struct (`{}`) because we only care about *count*.
/// Future: add `url`, `description`, `type` fields to support downloads
/// or alt-text rendering.
#[derive(Deserialize, Debug)]
pub(crate) struct MediaAttachment {}

// ---------------------------------------------------------------------------
// Media upload helper — async I/O with reqwest
// ---------------------------------------------------------------------------

/// Uploads a local file to `POST /api/v1/media` and returns the media ID.
///
/// Learner notes:
/// - `async fn` + `.await` — this function does non-blocking I/O. It must be
///   called from a `tokio` runtime (see `#[tokio::main]` in `main.rs`).
/// - `Box<dyn std::error::Error>` is a type-erased error that can hold any
///   error type (`std::io::Error`, `reqwest::Error`, etc.). Convenient for
///   quick CLI prototypes; larger apps often use `anyhow` or custom enums.
/// - `reqwest::multipart` builds a `multipart/form-data` body, which is how
///   browsers and the Mastodon API expect file uploads.
pub(crate) async fn upload_media(
    client: &reqwest::Client,
    token: &str,
    instance: &str,
    file_path: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // Build the full URL from the instance base + path
    let url = api_url(instance, "/api/v1/media");

    // `std::fs::read` is synchronous — fine for a CLI that uploads one file.
    // For many/large files you might use `tokio::fs::read` instead.
    let file_bytes = std::fs::read(file_path)
        .map_err(|e| format!("Failed to read image file {file_path}: {e}"))?;

    // Create a multipart part from raw bytes. `file_name` is required by the
    // API; we keep it simple ("image.jpg") — Mastodon sniffs the MIME type.
    // Future improvement: guess MIME from extension via `mime_guess`.
    let part = reqwest::multipart::Part::bytes(file_bytes).file_name("image.jpg");
    let form = reqwest::multipart::Form::new().part("file", part);

    // Send the request. `AUTHORIZATION: Bearer <token>` is the Mastodon auth
    // scheme. `client` is reused (connection pooling) — see `main.rs`.
    let response = client
        .post(url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .multipart(form)
        .send()
        .await?;

    // Check HTTP status before trying to parse JSON — on error the body may
    // not be JSON at all.
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        return Err(format!("Media upload failed: {status} - {error_text}").into());
    }

    // Parse the JSON response into `MediaResponse` and return its `id`.
    Ok(response.json::<MediaResponse>().await?.id)
}
