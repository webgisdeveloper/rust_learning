//! List Jira issues assigned to the current user (or a supplied account ID).
//!
//! Required environment: JIRA_BASE_URL, JIRA_USER_EMAIL, JIRA_API_TOKEN.
//! Usage: myjira [me|ACCOUNT_ID] [--json] [--max N] [--jql JQL]

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
    max: usize,
    help: bool,
}

fn usage() {
    eprintln!("Usage: myjira [me|ACCOUNT_ID] [--json] [--max N] [--jql JQL]");
    eprintln!("  Defaults to me. Jira Cloud requires an account ID for another user.");
}

fn parse_args(args: &[String]) -> Result<Config, String> {
    let mut config = Config {
        assignee: "me".into(),
        json: false,
        jql: None,
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
            flag if flag.starts_with('-') => return Err(format!("Unknown flag: {flag}")),
            value if assignee_seen => return Err(format!("Unexpected argument: {value}")),
            value => {
                config.assignee = value.to_string();
                assignee_seen = true;
            }
        }
        i += 1;
    }
    Ok(config)
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
        "{:<10} {:<12} {:<8} {:<summary_width$} URL",
        "KEY",
        "STATUS",
        "PRIORITY",
        "SUMMARY",
        summary_width = SUMMARY_WIDTH,
    );
    for issue in issues {
        println!(
            "{:<10} {:<12} {:<8} {:<summary_width$} {}/browse/{}",
            issue.key,
            field_name(&issue.fields.status),
            field_name(&issue.fields.priority),
            truncate_summary(issue.fields.summary.as_deref().unwrap_or("")),
            base_url.trim_end_matches('/'),
            issue.key,
            summary_width = SUMMARY_WIDTH,
        );
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
        let issues = search(
            &Client::new(),
            &base_url,
            &email,
            &token,
            &jql_for(&config)?,
            config.max,
        )
        .await?;
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
                max: 12,
                help: false
            }
        );
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
}
