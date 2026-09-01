use serde::Serialize;
use std::collections::HashMap;

/// Formats a DateTime into a clean, human-readable string (YYYY-MM-DD HH:MM:SS).
pub fn format_date(date: &aws_smithy_types::DateTime) -> String {
    let Ok(s) = date.fmt(aws_smithy_types::date_time::Format::DateTime) else {
        return "-".to_string();
    };
    if s.len() >= 19 {
        s[..19].replace('T', " ")
    } else {
        s.replace('T', " ").replace('Z', "")
    }
}

/// Item representation for long list format table output.
#[derive(Debug)]
pub struct LongListItem {
    pub key: String,
    pub size: u64,
    pub modified: String,
    pub host: String,
    pub description: String,
}

/// Formats long list items into an aligned tabular string.
pub fn format_long_table(items: &[LongListItem]) -> String {
    if items.is_empty() {
        return String::new();
    }

    let w_key = items.iter().map(|i| i.key.len()).max().unwrap_or(0).max(3);
    let w_size = items
        .iter()
        .map(|i| i.size.to_string().len())
        .max()
        .unwrap_or(0)
        .max(4);
    let w_mod = items
        .iter()
        .map(|i| i.modified.len())
        .max()
        .unwrap_or(0)
        .max(13);
    let w_host = items.iter().map(|i| i.host.len()).max().unwrap_or(0).max(4);
    let w_desc = items
        .iter()
        .map(|i| i.description.len())
        .max()
        .unwrap_or(0)
        .max(11);

    let mut out = Vec::with_capacity(items.len() + 1);
    out.push(format!(
        "{:<w_key$}  {:>w_size$}  {:<w_mod$}  {:<w_host$}  {:<w_desc$}",
        "KEY", "SIZE", "LAST_MODIFIED", "HOST", "DESCRIPTION"
    ));

    for item in items {
        out.push(format!(
            "{:<w_key$}  {:>w_size$}  {:<w_mod$}  {:<w_host$}  {:<w_desc$}",
            item.key, item.size, item.modified, item.host, item.description
        ));
    }

    out.join("\n")
}

/// Prints long list items formatted as an aligned table.
pub fn print_long_table(items: &[LongListItem]) {
    let table = format_long_table(items);
    if !table.is_empty() {
        println!("{table}");
    }
}

/// Full metadata for a single R2 object (used by `stat`).
#[derive(Debug, Clone)]
pub struct StatInfo {
    pub key: String,
    pub bucket: String,
    pub size: i64,
    pub last_modified: Option<aws_smithy_types::DateTime>,
    pub etag: Option<String>,
    pub content_type: Option<String>,
    pub content_encoding: Option<String>,
    pub storage_class: Option<String>,
    pub host: String,

    pub description: String,
    pub metadata: HashMap<String, String>,
}

/// Serialized DTO for stat JSON output matching the original format.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatJsonDto<'a> {
    key: &'a str,
    bucket: &'a str,
    size: i64,
    last_modified: String,
    #[serde(rename = "eTag")]
    e_tag: &'a str,
    content_type: &'a str,
    content_encoding: &'a str,
    storage_class: &'a str,
    host: &'a str,
    description: &'a str,
    metadata: &'a HashMap<String, String>,
}

/// Format a `StatInfo` as human-readable aligned text.
pub fn format_stat_human(info: &StatInfo) -> String {
    let last_modified = info
        .last_modified
        .as_ref()
        .map(format_date)
        .unwrap_or_else(|| "-".to_string());
    let etag = info.etag.as_deref().unwrap_or("-");
    let content_type = info.content_type.as_deref().unwrap_or("-");
    let content_encoding = info.content_encoding.as_deref().unwrap_or("-");
    let storage_class = info.storage_class.as_deref().unwrap_or("-");

    let mut out = format!(
        "Key:            {}\nBucket:         {}\nSize:           {} bytes\nLast-Modified:  {}\nETag:           {}\nContent-Type:   {}\nContent-Encoding: {}\nStorage-Class:  {}\nHost:           {}\nDescription:    {}",
        info.key,
        info.bucket,
        info.size,
        last_modified,
        etag,
        content_type,
        content_encoding,
        storage_class,
        info.host,
        info.description
    );

    let extra: Vec<_> = info
        .metadata
        .iter()
        .filter(|(k, _)| *k != "host" && *k != "description")
        .collect();
    if !extra.is_empty() {
        out.push_str("\nMetadata:");
        for (k, v) in extra {
            out.push_str(&format!("\n  {k}: {v}"));
        }
    }
    out
}

/// Format a `StatInfo` as single-line JSON using serde_json.
pub fn format_stat_json(info: &StatInfo) -> anyhow::Result<String> {
    let last_modified = info
        .last_modified
        .as_ref()
        .and_then(|d| d.fmt(aws_smithy_types::date_time::Format::DateTime).ok())
        .unwrap_or_else(|| "-".to_string());

    let dto = StatJsonDto {
        key: &info.key,
        bucket: &info.bucket,
        size: info.size,
        last_modified,
        e_tag: info.etag.as_deref().unwrap_or("-"),
        content_type: info.content_type.as_deref().unwrap_or("-"),
        content_encoding: info.content_encoding.as_deref().unwrap_or("-"),
        storage_class: info.storage_class.as_deref().unwrap_or("-"),
        host: &info.host,
        description: &info.description,
        metadata: &info.metadata,
    };

    serde_json::to_string(&dto).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_date_naturally() {
        let dt = aws_smithy_types::DateTime::from_secs_and_nanos(1786121765, 613_000_000);
        assert_eq!(format_date(&dt), "2026-08-07 16:56:05");
    }

    #[test]
    fn formats_long_table_aligned() {
        let items = vec![
            LongListItem {
                key: "photo.jpg".to_string(),
                size: 89201,
                modified: "2026-08-07 16:56:05".to_string(),
                host: "host1".to_string(),
                description: "Vacation".to_string(),
            },
            LongListItem {
                key: "test/README.md".to_string(),
                size: 4614,
                modified: "2026-08-07 17:00:00".to_string(),
                host: "host2".to_string(),
                description: "-".to_string(),
            },
        ];

        let table = format_long_table(&items);
        let lines: Vec<&str> = table.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("KEY"));
        assert!(lines[0].contains("SIZE"));
        assert!(lines[0].contains("LAST_MODIFIED"));
        assert!(lines[0].contains("HOST"));
        assert!(lines[0].contains("DESCRIPTION"));
    }

    #[test]
    fn formats_stat_human_contains_labels() {
        let info = StatInfo {
            key: "images/photo.jpg".to_string(),
            bucket: "my-bucket".to_string(),
            size: 89201,
            last_modified: Some(aws_smithy_types::DateTime::from_secs_and_nanos(
                1786121765,
                613_000_000,
            )),
            etag: Some("\"abc123\"".to_string()),
            content_type: Some("image/jpeg".to_string()),
            content_encoding: None,
            storage_class: Some("STANDARD".to_string()),
            host: "my-host".to_string(),
            description: "Vacation".to_string(),
            metadata: HashMap::new(),
        };
        let out = format_stat_human(&info);
        assert!(out.contains("Key:"));
        assert!(out.contains("images/photo.jpg"));
        assert!(out.contains("Bucket:"));
        assert!(out.contains("my-bucket"));
        assert!(out.contains("89201 bytes"));
        assert!(out.contains("2026-08-07 16:56:05"));
        assert!(out.contains("Host:"));
        assert!(out.contains("my-host"));
        assert!(out.contains("Description:"));
        assert!(out.contains("Vacation"));
    }

    #[test]
    fn formats_stat_json_roundtrip() {
        let mut meta = HashMap::new();
        meta.insert("host".to_string(), "my-host".to_string());
        meta.insert("description".to_string(), "A \"quoted\" desc".to_string());
        let info = StatInfo {
            key: "images/photo.jpg".to_string(),
            bucket: "my-bucket".to_string(),
            size: 1234,
            last_modified: None,
            etag: None,
            content_type: Some("image/jpeg".to_string()),
            content_encoding: None,
            storage_class: None,
            host: "my-host".to_string(),
            description: "A \"quoted\" desc".to_string(),
            metadata: meta,
        };
        let json = format_stat_json(&info).unwrap();
        assert!(json.contains("\"key\":\"images/photo.jpg\""));
        assert!(json.contains("\"size\":1234"));
        assert!(json.contains("\"host\":\"my-host\""));
        assert!(json.contains("A \\\"quoted\\\" desc"));
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
    }

    #[test]
    fn formats_stat_human_shows_extra_metadata() {
        let mut meta = HashMap::new();
        meta.insert("host".to_string(), "h".to_string());
        meta.insert("description".to_string(), "d".to_string());
        meta.insert("custom".to_string(), "val".to_string());
        let info = StatInfo {
            key: "k".to_string(),
            bucket: "b".to_string(),
            size: 0,
            last_modified: None,
            etag: None,
            content_type: None,
            content_encoding: None,
            storage_class: None,
            host: "h".to_string(),
            description: "d".to_string(),
            metadata: meta,
        };
        let out = format_stat_human(&info);
        assert!(out.contains("Metadata:"));
        assert!(out.contains("custom: val"));
    }

    #[test]
    fn formats_stat_human_missing_fields() {
        let info = StatInfo {
            key: "k".to_string(),
            bucket: "b".to_string(),
            size: 0,
            last_modified: None,
            etag: None,
            content_type: None,
            content_encoding: None,
            storage_class: None,
            host: "-".to_string(),
            description: "-".to_string(),
            metadata: HashMap::new(),
        };
        let out = format_stat_human(&info);
        assert!(out.contains("Last-Modified:  -"));
        assert!(out.contains("ETag:           -"));
        assert!(out.contains("Content-Type:   -"));
        assert!(out.contains("Content-Encoding: -"));
        assert!(out.contains("Storage-Class:  -"));
        let json = format_stat_json(&info).unwrap();
        assert!(json.contains("\"lastModified\":\"-\""));
        assert!(json.contains("\"eTag\":\"-\""));
    }
}
