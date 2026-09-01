use anyhow::{Context, bail};
use std::path::{Path, PathBuf};

/// Returns true if the string contains glob meta characters.
pub fn is_glob_pattern(s: &str) -> bool {
    s.contains('*') || s.contains('?') || s.contains('[')
}

/// Derive the S3 ListObjectsV2 prefix for a glob pattern: substring up to first wildcard (`*`, `?`, `[`).
pub fn derive_list_prefix(pattern: &str) -> String {
    let mut first = pattern.len();
    for (i, c) in pattern.char_indices() {
        if c == '*' || c == '?' || c == '[' {
            first = i;
            break;
        }
    }
    pattern[..first].to_string()
}

/// Check whether a key matches a glob pattern using `glob::Pattern`.
#[allow(dead_code)]
pub fn glob_pattern_matches(pattern: &str, key: &str) -> anyhow::Result<bool> {
    let pat =
        glob::Pattern::new(pattern).with_context(|| format!("invalid glob pattern: {pattern}"))?;
    Ok(pat.matches(key))
}

/// Expand CLI file arguments that may contain wildcards (e.g. "*.gif", "mm*.jpg").
/// - Plain paths without glob meta are kept as-is.
/// - Patterns with meta are expanded via `glob`. If nothing matches, an error is returned.
/// - Deduplicates while preserving order.
pub fn expand_file_patterns(patterns: &[String]) -> anyhow::Result<Vec<PathBuf>> {
    let mut expanded: Vec<PathBuf> = Vec::new();
    for pat in patterns {
        if is_glob_pattern(pat) {
            let entries =
                glob::glob(pat).with_context(|| format!("invalid glob pattern: {pat}"))?;
            let mut matched: Vec<PathBuf> = Vec::new();
            for entry in entries {
                matched.push(
                    entry.with_context(|| {
                        format!("failed to read glob result for pattern: {pat}")
                    })?,
                );
            }
            if matched.is_empty() {
                bail!("no files matched pattern: {pat}");
            }
            expanded.extend(matched);
        } else {
            expanded.push(PathBuf::from(pat));
        }
    }
    // Deduplicate preserving order
    let mut seen = std::collections::HashSet::new();
    let mut deduped = Vec::new();
    for p in expanded {
        if seen.insert(p.clone()) {
            deduped.push(p);
        }
    }
    Ok(deduped)
}

/// Expand a single remote wildcard pattern by listing objects with the derived prefix
/// and filtering locally via `glob::Pattern`. Assumes `pattern` contains glob meta.
pub async fn expand_single_remote_pattern(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    pattern: &str,
) -> anyhow::Result<Vec<String>> {
    let prefix = derive_list_prefix(pattern);
    let pat =
        glob::Pattern::new(pattern).with_context(|| format!("invalid glob pattern: {pattern}"))?;
    let mut matched = Vec::new();
    let mut token: Option<String> = None;
    loop {
        let mut req = client.list_objects_v2().bucket(bucket);
        if !prefix.is_empty() {
            req = req.prefix(prefix.clone());
        }
        if let Some(t) = token.take() {
            req = req.continuation_token(t);
        }
        let resp = req
            .send()
            .await
            .context("list_objects failed — check bucket, credentials, endpoint and network")?;
        for obj in resp.contents() {
            if let Some(k) = obj.key()
                && pat.matches(k)
            {
                matched.push(k.to_string());
            }
        }
        if !resp.is_truncated().unwrap_or(false) {
            break;
        }
        token = resp.next_continuation_token().map(|s| s.to_owned());
        if token.is_none() {
            bail!("R2 returned a truncated object list without a continuation token");
        }
    }
    Ok(matched)
}

/// Expand a list of remote key patterns (literal or wildcard) into concrete keys.
/// Wildcards are resolved via `expand_single_remote_pattern`; literals are kept as-is.
pub async fn expand_remote_keys(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    patterns: &[String],
) -> anyhow::Result<Vec<String>> {
    let mut resolved: Vec<String> = Vec::new();
    for pat in patterns {
        let trimmed = pat.trim();
        if trimmed.is_empty() {
            bail!("object key must not be empty");
        }
        if is_glob_pattern(trimmed) {
            let matched = expand_single_remote_pattern(client, bucket, trimmed).await?;
            if matched.is_empty() {
                bail!("no objects matched pattern: {trimmed}");
            }
            resolved.extend(matched);
        } else {
            if trimmed.ends_with('/') {
                bail!("object key must identify a file, not a directory: {trimmed}");
            }
            resolved.push(trimmed.to_string());
        }
    }
    // Deduplicate preserving order
    let mut seen = std::collections::HashSet::new();
    let mut deduped = Vec::new();
    for k in resolved {
        if seen.insert(k.clone()) {
            deduped.push(k);
        }
    }
    Ok(deduped)
}

/// Hint about shell expansion if a local file exists with the key name (e.g. `delete *.jpg` unquoted).
pub fn shell_expansion_hint(key: &str) -> String {
    if Path::new(key).exists() {
        format!(
            " (hint: local file '{key}' exists — if you meant a wildcard like '*.jpg', quote it as \"*.jpg\" to prevent shell expansion)"
        )
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_glob_pattern() {
        assert!(is_glob_pattern("*.gif"));
        assert!(is_glob_pattern("mm*.jpg"));
        assert!(is_glob_pattern("file?.txt"));
        assert!(is_glob_pattern("a[bc].txt"));
        assert!(!is_glob_pattern("photo.jpg"));
        assert!(!is_glob_pattern("a/b/photo.jpg"));
    }

    #[test]
    fn derives_list_prefix_for_glob() {
        assert_eq!(derive_list_prefix("*.gif"), "");
        assert_eq!(derive_list_prefix("mm*.jpg"), "mm");
        assert_eq!(derive_list_prefix("images/*.jpg"), "images/");
        assert_eq!(derive_list_prefix("a/b/c*.txt"), "a/b/c");
        assert_eq!(derive_list_prefix("plain.txt"), "plain.txt");
        assert_eq!(derive_list_prefix("a[bc].txt"), "a");
        assert_eq!(derive_list_prefix("file?.txt"), "file");
    }

    #[test]
    fn matches_glob_pattern_for_keys() {
        assert!(glob_pattern_matches("*.gif", "a.gif").unwrap());
        assert!(glob_pattern_matches("*.gif", "dir/a.gif").unwrap());
        assert!(glob_pattern_matches("mm*.jpg", "mm1.jpg").unwrap());
        assert!(!glob_pattern_matches("mm*.jpg", "xx1.jpg").unwrap());
        assert!(glob_pattern_matches("images/*.jpg", "images/a.jpg").unwrap());
        assert!(glob_pattern_matches("*.jpg", "a.jpg").unwrap());
        assert!(glob_pattern_matches("a[bc].txt", "ab.txt").unwrap());
        assert!(!glob_pattern_matches("a[bc].txt", "ad.txt").unwrap());
        assert!(glob_pattern_matches("file?.txt", "file1.txt").unwrap());
    }

    #[test]
    fn expands_wildcard_patterns() {
        let dir = std::env::temp_dir().join(format!(
            "cloudflare_r2_test_wildcard_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["a.gif", "b.gif", "mm1.jpg", "mm2.jpg", "note.txt"] {
            std::fs::write(dir.join(name), b"x").unwrap();
        }

        let pat = dir.join("*.gif").to_string_lossy().to_string();
        let res = expand_file_patterns(&[pat]).unwrap();
        let mut names: Vec<String> = res
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        names.sort();
        assert_eq!(names, vec!["a.gif".to_string(), "b.gif".to_string()]);

        let pat2 = dir.join("mm*.jpg").to_string_lossy().to_string();
        let res2 = expand_file_patterns(&[pat2]).unwrap();
        let mut names2: Vec<String> = res2
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        names2.sort();
        assert_eq!(names2, vec!["mm1.jpg".to_string(), "mm2.jpg".to_string()]);

        let plain = dir.join("note.txt").to_string_lossy().to_string();
        let res3 = expand_file_patterns(&[plain.clone()]).unwrap();
        assert_eq!(res3.len(), 1);
        assert_eq!(res3[0].file_name().unwrap().to_string_lossy(), "note.txt");

        let no_match = dir.join("nope*.gif").to_string_lossy().to_string();
        assert!(expand_file_patterns(&[no_match]).is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn expands_multiple_patterns_and_dedups() {
        let dir =
            std::env::temp_dir().join(format!("cloudflare_r2_test_dedup_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.gif"), b"x").unwrap();
        std::fs::write(dir.join("b.gif"), b"x").unwrap();
        let pat_star = dir.join("*.gif").to_string_lossy().to_string();
        let pat_explicit = dir.join("a.gif").to_string_lossy().to_string();
        let res = expand_file_patterns(&[pat_star, pat_explicit]).unwrap();
        assert_eq!(res.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
