//! List Jira issues assigned to the current user (or a supplied account ID).
//!
//! Required environment: JIRA_BASE_URL, JIRA_USER_EMAIL, JIRA_API_TOKEN.
//! Usage: myjira [me|ACCOUNT_ID] [--json] [--max N] [--jql JQL] [--key ISSUE_KEY] [--comment TEXT]

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

const FIELDS: &[&str] = &["summary", "status", "priority"];
const SUMMARY_WIDTH: usize = 20;

#[derive(Debug, Deserialize, Serialize)]
struct Issue {
    key: String,
    fields: Fields,
}

#[derive(Debug, Deserialize, Serialize)]
struct Fields {
    summary: Option<String>,
    status: Option<Named>,
    priority: Option<Named>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Named {
    name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct User {
    #[serde(rename = "displayName")]
    display_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Comment {
    author: Option<User>,
    body: Option<serde_json::Value>,
    created: Option<String>,
}

#[derive(Deserialize)]
struct CommentPage {
    #[serde(default)]
    comments: Vec<Comment>,
    #[serde(rename = "startAt")]
    start_at: Option<usize>,
    total: Option<usize>,
}

#[derive(Deserialize)]
struct SearchPage {
    #[serde(default)]
    issues: Vec<Issue>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
    #[serde(rename = "isLast")]
    is_last: Option<bool>,
}

#[derive(Serialize)]
struct SearchRequest<'a> {
    jql: &'a str,
    #[serde(rename = "maxResults")]
    max_results: usize,
    fields: &'a [&'a str],
    #[serde(rename = "nextPageToken", skip_serializing_if = "Option::is_none")]
    next_page_token: Option<&'a str>,
}

#[derive(Debug, PartialEq, Eq)]
struct Config {
    assignee: String,
    json: bool,
    jql: Option<String>,
    key: Option<String>,
    comment: Option<String>,
    max: usize,
    help: bool,
}

fn usage() {
    eprintln!(
        "Usage: myjira [me|ACCOUNT_ID] [--json] [--max N] [--jql JQL] [--key ISSUE_KEY] [--comment TEXT]"
    );
    eprintln!("  Defaults to me. Jira Cloud requires an account ID for another user.");
}

fn parse_args(args: &[String]) -> Result<Config, String> {
    let mut config = Config {
        assignee: "me".into(),
        json: false,
        jql: None,
        key: None,
        comment: None,
        max: 100,
        help: false,
    };
    let mut assignee_seen = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => config.json = true,
            "-h" | "--help" => config.help = true,
            "--max" => {
                i += 1;
                config.max = args
                    .get(i)
                    .ok_or("--max requires a number")?
                    .parse()
                    .map_err(|_| "--max requires a positive number")?;
                if config.max == 0 {
                    return Err("--max requires a positive number".into());
                }
                config.max = config.max.min(100);
            }
            "--jql" => {
                i += 1;
                config.jql = Some(
                    args.get(i)
                        .ok_or("--jql requires a JQL expression")?
                        .to_string(),
                );
            }
            "--key" => {
                i += 1;
                let key = args.get(i).ok_or("--key requires an issue key")?;
                if !key.chars().all(|character| {
                    character.is_ascii_alphanumeric() || character == '-' || character == '_'
                }) {
                    return Err(
                        "--key must contain only letters, numbers, hyphens, or underscores".into(),
                    );
                }
                config.key = Some(key.to_string());
            }
            "--comment" => {
                i += 1;
                let comment = args.get(i).ok_or("--comment requires text")?;
                if comment.trim().is_empty() {
                    return Err("--comment requires non-empty text".into());
                }
                config.comment = Some(comment.to_string());
            }
            flag if flag.starts_with('-') => return Err(format!("Unknown flag: {flag}")),
            value if assignee_seen => return Err(format!("Unexpected argument: {value}")),
            value => {
                config.assignee = value.to_string();
                assignee_seen = true;
            }
        }
        i += 1;
    }
    if config.comment.is_some() && config.key.is_none() {
        return Err("--comment requires --key ISSUE_KEY".into());
    }
    Ok(config)
}

async fn get_issue(
    client: &Client,
    base_url: &str,
    email: &str,
    token: &str,
    key: &str,
) -> Result<Issue, String> {
    let url = format!(
        "{}/rest/api/3/issue/{key}?fields=summary,status,priority",
        base_url.trim_end_matches('/')
    );
    let response = client
        .get(url)
        .basic_auth(email, Some(token))
        .send()
        .await
        .map_err(|error| format!("Could not reach Jira: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "Jira issue lookup failed (HTTP {status}): {}",
            response.text().await.unwrap_or_default()
        ));
    }
    response
        .json()
        .await
        .map_err(|error| format!("Jira returned invalid issue JSON: {error}"))
}

async fn get_comments(
    client: &Client,
    base_url: &str,
    email: &str,
    token: &str,
    key: &str,
) -> Result<Vec<Comment>, String> {
    let mut comments = Vec::new();
    let mut start_at = 0;
    loop {
        let url = format!(
            "{}/rest/api/3/issue/{key}/comment?startAt={start_at}&maxResults=100",
            base_url.trim_end_matches('/'),
        );
        let response = client
            .get(url)
            .basic_auth(email, Some(token))
            .send()
            .await
            .map_err(|error| format!("Could not reach Jira: {error}"))?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!(
                "Jira comment lookup failed (HTTP {status}): {}",
                response.text().await.unwrap_or_default()
            ));
        }
        let page: CommentPage = response
            .json()
            .await
            .map_err(|error| format!("Jira returned invalid comment JSON: {error}"))?;
        let page_start = page.start_at.unwrap_or(start_at);
        let page_count = page.comments.len();
        comments.extend(page.comments);
        if page_count == 0 || page_start + page_count >= page.total.unwrap_or(usize::MAX) {
            return Ok(comments);
        }
        start_at = page_start + page_count;
    }
}

async fn add_comment(
    client: &Client,
    base_url: &str,
    email: &str,
    token: &str,
    key: &str,
    comment: &str,
) -> Result<(), String> {
    let url = format!(
        "{}/rest/api/3/issue/{key}/comment",
        base_url.trim_end_matches('/')
    );
    let body = serde_json::json!({
        "body": {
            "type": "doc",
            "version": 1,
            "content": comment.lines().map(|line| serde_json::json!({
                "type": "paragraph",
                "content": [{"type": "text", "text": line}],
            })).collect::<Vec<_>>(),
        }
    });
    let response = client
        .post(url)
        .basic_auth(email, Some(token))
        .json(&body)
        .send()
        .await
        .map_err(|error| format!("Could not reach Jira: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "Jira comment creation failed (HTTP {status}): {}",
            response.text().await.unwrap_or_default()
        ));
    }
    Ok(())
}

fn required_env(name: &str) -> Result<String, String> {
    env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("Missing required environment variable: {name}"))
}

fn jql_for(config: &Config) -> Result<String, String> {
    if let Some(jql) = &config.jql {
        return Ok(jql.clone());
    }
    let assignee =
        if config.assignee.eq_ignore_ascii_case("me") || config.assignee == "currentUser()" {
            "currentUser()".to_string()
        } else if config.assignee.contains('@') {
            return Err(
                "Jira Cloud does not accept email addresses in JQL; pass an account ID or use me"
                    .into(),
            );
        } else {
            format!("\"{}\"", config.assignee.replace('"', "\\\""))
        };
    Ok(format!("assignee = {assignee} ORDER BY updated DESC"))
}

async fn search(
    client: &Client,
    base_url: &str,
    email: &str,
    token: &str,
    jql: &str,
    max: usize,
) -> Result<Vec<Issue>, String> {
    let url = format!("{}/rest/api/3/search/jql", base_url.trim_end_matches('/'));
    let mut issues = Vec::new();
    let mut page_token: Option<String> = None;
    loop {
        let request = SearchRequest {
            jql,
            max_results: max,
            fields: FIELDS,
            next_page_token: page_token.as_deref(),
        };
        let response = client
            .post(&url)
            .basic_auth(email, Some(token))
            .json(&request)
            .send()
            .await
            .map_err(|error| format!("Could not reach Jira: {error}"))?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!(
                "Jira search failed (HTTP {status}): {}",
                response.text().await.unwrap_or_default()
            ));
        }
        let page: SearchPage = response
            .json()
            .await
            .map_err(|error| format!("Jira returned invalid search JSON: {error}"))?;
        let next = page.next_page_token;
        issues.extend(page.issues);
        if page.is_last == Some(true) || next.is_none() || next == page_token {
            return Ok(issues);
        }
        page_token = next;
    }
}

fn print_table(issues: &[Issue], base_url: &str) {
    if issues.is_empty() {
        println!("No issues found.");
        return;
    }
    println!(
        "{:<10} {:<12}{:<8} {:<summary_width$} URL",
        "KEY",
        "STATUS",
        "PRIORITY",
        "SUMMARY",
        summary_width = SUMMARY_WIDTH,
    );
    for issue in issues {
        println!(
            "{:<10} {}          {:<8} {:<summary_width$} {}/browse/{}",
            issue.key,
            status_emoji(&issue.fields.status),
            field_name(&issue.fields.priority),
            truncate_summary(issue.fields.summary.as_deref().unwrap_or("")),
            base_url.trim_end_matches('/'),
            issue.key,
            summary_width = SUMMARY_WIDTH,
        );
    }
}

fn print_issue_details(issue: &Issue, comments: &[Comment], base_url: &str) {
    println!("Key:      {}", issue.key);
    println!(
        "Summary:  {}",
        issue.fields.summary.as_deref().unwrap_or("-")
    );
    println!(
        "Status:   {} {}",
        status_emoji(&issue.fields.status),
        field_name(&issue.fields.status)
    );
    println!("Priority: {}", field_name(&issue.fields.priority));
    println!(
        "URL:      {}/browse/{}",
        base_url.trim_end_matches('/'),
        issue.key
    );
    if comments.is_empty() {
        println!("Comments: none");
        return;
    }
    println!("Comments:");
    for comment in comments {
        let author = comment
            .author
            .as_ref()
            .and_then(|author| author.display_name.as_deref())
            .unwrap_or("Unknown author");
        let created = comment.created.as_deref().unwrap_or("Unknown date");
        println!("  {author} — {created}");
        let body = comment
            .body
            .as_ref()
            .map(adf_text)
            .filter(|body| !body.is_empty())
            .unwrap_or_else(|| "-".into());
        for line in body.lines() {
            println!("    {line}");
        }
    }
}

fn adf_text(value: &serde_json::Value) -> String {
    fn append_text(value: &serde_json::Value, output: &mut String) {
        let Some(object) = value.as_object() else {
            return;
        };
        if let Some(text) = object.get("text").and_then(serde_json::Value::as_str) {
            output.push_str(text);
        }
        if object.get("type").and_then(serde_json::Value::as_str) == Some("hardBreak") {
            output.push('\n');
        }
        if let Some(content) = object.get("content").and_then(serde_json::Value::as_array) {
            for child in content {
                append_text(child, output);
            }
        }
        if matches!(
            object.get("type").and_then(serde_json::Value::as_str),
            Some("paragraph" | "heading" | "listItem")
        ) && !output.ends_with('\n')
        {
            output.push('\n');
        }
    }

    let mut output = String::new();
    append_text(value, &mut output);
    output.trim().to_string()
}

fn status_emoji(status: &Option<Named>) -> &'static str {
    match field_name(status).to_ascii_lowercase().as_str() {
        "done" => "✅",
        "in progress" => "🔄",
        "to do" => "📝",
        "review" => "👀",
        "on hold" => "⏸️",
        _ => "❔",
    }
}

fn status_rank(status: &Option<Named>) -> u8 {
    match field_name(status).to_ascii_lowercase().as_str() {
        "in progress" => 0,
        "review" => 1,
        "to do" => 2,
        "on hold" => 3,
        "done" => 4,
        _ => 5,
    }
}

fn truncate_summary(summary: &str) -> String {
    let mut characters = summary.chars();
    let shortened: String = characters.by_ref().take(SUMMARY_WIDTH).collect();
    if characters.next().is_some() {
        let visible: String = shortened
            .chars()
            .take(SUMMARY_WIDTH.saturating_sub(1))
            .collect();
        format!("{visible}…")
    } else {
        shortened
    }
}

fn field_name(field: &Option<Named>) -> &str {
    field
        .as_ref()
        .and_then(|value| value.name.as_deref())
        .unwrap_or("-")
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    let args: Vec<String> = env::args().skip(1).collect();
    let result: Result<(), String> = async {
        let config = parse_args(&args)?;
        if config.help {
            usage();
            return Ok(());
        }
        let base_url = required_env("JIRA_BASE_URL")?;
        let email = required_env("JIRA_USER_EMAIL")?;
        let token = required_env("JIRA_API_TOKEN")?;
        if let Some(key) = &config.key {
            let client = Client::new();
            if let Some(comment) = &config.comment {
                add_comment(&client, &base_url, &email, &token, key, comment).await?;
            }
            let issue = get_issue(&client, &base_url, &email, &token, key).await?;
            let comments = get_comments(&client, &base_url, &email, &token, key).await?;
            if config.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "issue": issue,
                        "comments": comments,
                    }))
                    .map_err(|e| e.to_string())?
                );
            } else {
                print_issue_details(&issue, &comments, &base_url);
            }
            return Ok(());
        }
        let mut issues = search(
            &Client::new(),
            &base_url,
            &email,
            &token,
            &jql_for(&config)?,
            config.max,
        )
        .await?;
        issues.sort_by_key(|issue| status_rank(&issue.fields.status));
        if config.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&issues).map_err(|e| e.to_string())?
            );
        } else {
            print_table(&issues, &base_url);
        }
        Ok(())
    }
    .await;
    if let Err(error) = result {
        eprintln!("Error: {error}");
        usage();
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_options() {
        let args = vec![
            "abc123".into(),
            "--json".into(),
            "--max".into(),
            "12".into(),
        ];
        assert_eq!(
            parse_args(&args).unwrap(),
            Config {
                assignee: "abc123".into(),
                json: true,
                jql: None,
                key: None,
                comment: None,
                max: 12,
                help: false
            }
        );
    }

    #[test]
    fn parses_issue_key() {
        let config = parse_args(&["--key".into(), "PROJ-123".into()]).unwrap();
        assert_eq!(config.key.as_deref(), Some("PROJ-123"));
    }

    #[test]
    fn parses_comment_for_an_issue() {
        let config = parse_args(&[
            "--key".into(),
            "PROJ-123".into(),
            "--comment".into(),
            "Looks good".into(),
        ])
        .unwrap();
        assert_eq!(config.comment.as_deref(), Some("Looks good"));
        assert!(parse_args(&["--comment".into(), "Looks good".into()]).is_err());
    }

    #[test]
    fn renders_adf_comment_text() {
        let comment = serde_json::json!({
            "type": "doc",
            "content": [
                {"type": "paragraph", "content": [{"type": "text", "text": "First line"}]},
                {"type": "paragraph", "content": [{"type": "text", "text": "Second line"}]}
            ]
        });
        assert_eq!(adf_text(&comment), "First line\nSecond line");
    }

    #[test]
    fn builds_safe_default_jql() {
        assert_eq!(
            jql_for(&parse_args(&[]).unwrap()).unwrap(),
            "assignee = currentUser() ORDER BY updated DESC"
        );
        assert!(jql_for(&parse_args(&["person@example.com".into()]).unwrap()).is_err());
    }

    #[test]
    fn deserializes_search_page() {
        let page: SearchPage = serde_json::from_str(r#"{"issues":[{"key":"PROJ-1","fields":{"summary":"Fix","status":{"name":"Open"}}}],"isLast":true}"#).unwrap();
        assert_eq!(page.issues[0].key, "PROJ-1");
    }

    #[test]
    fn truncates_summaries_to_the_table_width() {
        assert_eq!(truncate_summary("short"), "short");
        assert_eq!(
            truncate_summary(&"a".repeat(SUMMARY_WIDTH)),
            "a".repeat(SUMMARY_WIDTH)
        );
        assert_eq!(
            truncate_summary(&"a".repeat(SUMMARY_WIDTH + 1)),
            format!("{}…", "a".repeat(SUMMARY_WIDTH - 1))
        );
    }

    #[test]
    fn uses_compact_status_markers() {
        for (name, marker) in [
            ("Done", "✅"),
            ("In Progress", "🔄"),
            ("To Do", "📝"),
            ("Review", "👀"),
            ("On Hold", "⏸️"),
        ] {
            assert_eq!(
                status_emoji(&Some(Named {
                    name: Some(name.into())
                })),
                marker
            );
        }
    }

    #[test]
    fn ranks_statuses_for_display() {
        let status = |name: &str| {
            Some(Named {
                name: Some(name.into()),
            })
        };
        assert!(status_rank(&status("In Progress")) < status_rank(&status("Review")));
        assert!(status_rank(&status("Review")) < status_rank(&status("To Do")));
        assert!(status_rank(&status("To Do")) < status_rank(&status("On Hold")));
        assert!(status_rank(&status("On Hold")) < status_rank(&status("Done")));
    }
}
