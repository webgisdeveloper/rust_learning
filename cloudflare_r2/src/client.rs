use crate::cli::R2Args;
use anyhow::Context;

/// Helper to determine the R2 endpoint URL.
pub fn endpoint_for(args: &R2Args) -> anyhow::Result<String> {
    derive_endpoint(args.endpoint.as_deref(), args.account_id.as_deref())
        .context("must provide --endpoint / R2_ENDPOINT or --account-id / R2_ACCOUNT_ID")
}

/// Logic to derive an endpoint URL. Returns None if neither is provided.
pub fn derive_endpoint(endpoint: Option<&str>, account_id: Option<&str>) -> Option<String> {
    // Try to use the explicit endpoint first.
    if let Some(endpoint) = endpoint.map(str::trim)
        && !endpoint.is_empty()
    {
        return Some(endpoint.to_string());
    }

    // Otherwise, construct the endpoint from the account ID.
    let account_id = account_id?.trim();
    (!account_id.is_empty()).then(|| format!("https://{account_id}.r2.cloudflarestorage.com"))
}

/// Configures and builds the AWS S3 Client for R2.
pub async fn build_client(
    endpoint_url: &str,
    access_key: &str,
    secret_key: &str,
) -> aws_sdk_s3::Client {
    let credentials = aws_sdk_s3::config::Credentials::new(
        access_key.to_owned(),
        secret_key.to_owned(),
        None,
        None,
        "r2",
    );

    let base_config = aws_config::load_from_env().await;
    let s3_config = aws_sdk_s3::config::Builder::from(&base_config)
        .endpoint_url(endpoint_url)
        .credentials_provider(credentials)
        .region(aws_config::Region::new("auto"))
        .build();

    aws_sdk_s3::Client::from_conf(s3_config)
}

/// Helper to obtain the hostname of the current machine.
pub fn get_hostname() -> String {
    hostname::get()
        .map(|h| h.to_string_lossy().into_owned())
        .or_else(|_| std::env::var("HOSTNAME"))
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_endpoint_from_account_id() {
        assert_eq!(
            derive_endpoint(None, Some("abc123")),
            Some("https://abc123.r2.cloudflarestorage.com".to_string())
        );
    }

    #[test]
    fn explicit_endpoint_takes_precedence() {
        assert_eq!(
            derive_endpoint(Some("https://custom.example.com"), Some("abc123")),
            Some("https://custom.example.com".to_string())
        );
    }

    #[test]
    fn blank_endpoint_and_account_id_are_rejected() {
        assert_eq!(derive_endpoint(Some("  "), Some("\t")), None);
    }

    #[test]
    fn returns_hostname() {
        let host = get_hostname();
        assert!(!host.is_empty());
    }
}
