use reqwest::header::AUTHORIZATION;
use serde::{Deserialize, Serialize};

/// Removes a trailing slash so paths can be appended consistently.
pub(crate) fn normalize_instance(instance: &str) -> String {
    let trimmed = instance.trim();
    let without_slash = trimmed.trim_end_matches('/');
    if without_slash.is_empty() {
        "https://mastodon.social".to_string()
    } else if without_slash.starts_with("http://") || without_slash.starts_with("https://") {
        without_slash.to_string()
    } else {
        format!("https://{without_slash}")
    }
}

pub(crate) fn api_url(instance: &str, path: &str) -> String {
    format!("{}{}", normalize_instance(instance), path)
}

#[derive(Serialize)]
pub(crate) struct StatusRequest {
    pub(crate) status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) media_ids: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct MediaResponse {
    pub(crate) id: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Account {
    pub(crate) id: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Status {
    pub(crate) content: String,
    pub(crate) media_attachments: Vec<MediaAttachment>,
    pub(crate) in_reply_to_id: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct MediaAttachment {}

pub(crate) async fn upload_media(
    client: &reqwest::Client,
    token: &str,
    instance: &str,
    file_path: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let url = api_url(instance, "/api/v1/media");
    let file_bytes = std::fs::read(file_path)
        .map_err(|e| format!("Failed to read image file {file_path}: {e}"))?;

    let part = reqwest::multipart::Part::bytes(file_bytes).file_name("image.jpg");
    let form = reqwest::multipart::Form::new().part("file", part);
    let response = client
        .post(url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .multipart(form)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        return Err(format!("Media upload failed: {status} - {error_text}").into());
    }

    Ok(response.json::<MediaResponse>().await?.id)
}
