# Jira Cloud CLI & MCP Server (`myjira`)

A lightweight, asynchronous Rust command-line tool and [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) server for Atlassian Jira Cloud.

`myjira` allows developers and AI assistants to quickly list assigned tickets, query issues with custom JQL, inspect issue details and discussion threads (with Atlassian Document Format rendering), and post comments—either interactively via terminal or programmatically via MCP over standard I/O (`stdio`).

---

## Features

- **Jira Cloud REST API v3 Integration**: Communicates directly with Jira Cloud REST endpoints with Basic Auth (email + API token).
- **Interactive Terminal & Table View**: Clean CLI output displaying issue key, status emoji, priority, truncated summary, and direct browser URLs.
- **Single Issue Inspection**: Inspect full issue details and comment threads formatted cleanly in plain text from Atlassian Document Format (ADF).
- **Comment Creation**: Post new comments directly to Jira issues from the command line or via MCP.
- **Custom JQL Queries**: Run flexible Jira Query Language (JQL) expressions with automatic pagination support.
- **JSON Output Mode**: Export structured JSON payloads for piping into `jq` or integrating into scripts.
- **Model Context Protocol (MCP) Server**: Run as an MCP server (`--mcp`) exposing tools to AI clients (Claude Desktop, Antigravity, Cursor, etc.) via `rmcp`.
- **Environment & `.env` Support**: Automatically loads credentials from `.env` using `dotenvy`.

---

## Prerequisites & Configuration

### 1. Generate an Atlassian API Token
1. Log in to [Atlassian Account Security](https://id.atlassian.com/manage-profile/security/api-tokens).
2. Click **Create API token**, label it (e.g. `myjira`), and copy the generated token.

### 2. Set Environment Variables
Create a `.env` file in the `myjira/` directory or export the following variables in your shell:

```bash
# Jira Cloud base URL
JIRA_BASE_URL="https://your-domain.atlassian.net"

# Your Atlassian account email
JIRA_USER_EMAIL="user@example.com"

# The generated Atlassian API token
JIRA_API_TOKEN="your_jira_api_token"
```

> [!NOTE]
> Jira Cloud uses **Account IDs** instead of email addresses or usernames in JQL. When targeting another user, pass their Jira Account ID (e.g., `5b10ac8d82e05b22cc7d4ef5`).

---

## Installation & Building

Compile the binary with Cargo:

```bash
# Build release binary
cargo build --release

# The binary will be available at:
./target/release/myjira
```

You can also run commands directly using `cargo run -- ...`.

---

## CLI Usage

### Synopsis
```text
Usage: myjira [me|ACCOUNT_ID] [--json] [--max N] [--jql JQL] [--key ISSUE_KEY] [--comment TEXT]
       myjira --mcp
```

### Examples

#### 1. List Issues Assigned to You
Defaults to the authenticated user (`me`):
```bash
cargo run --
```

#### 2. List Issues Assigned to Another User
```bash
cargo run -- 5b10ac8d82e05b22cc7d4ef5
```

#### 3. Custom JQL Search
Specify custom Jira Query Language expressions with `--jql`:
```bash
cargo run -- --jql "project = PROJ AND status = 'In Progress' ORDER BY priority DESC"
```

#### 4. Limit Result Count
Limit the number of results returned (1–100, default: 100):
```bash
cargo run -- --max 10
```

#### 5. Inspect an Issue and Comments
Fetch the issue summary, status, priority, web URL, and formatted comment history:
```bash
cargo run -- --key PROJ-123
```

#### 6. Add a Comment to an Issue
Add a new comment and immediately print the updated issue details:
```bash
cargo run -- --key PROJ-123 --comment "Investigating root cause on staging environment."
```

#### 7. Output JSON for Scripting
Append `--json` to any query to get raw JSON responses:
```bash
cargo run -- --key PROJ-123 --json | jq .
```

---

## Model Context Protocol (MCP) Server Mode

`myjira` implements the Model Context Protocol using the `rmcp` crate and stdio transport.

### Running MCP Mode
```bash
cargo run -- --mcp
```

### Available MCP Tools

| Tool | Parameters | Description |
| :--- | :--- | :--- |
| `list_jira_issues` | `assignee` *(optional string)*<br>`jql` *(optional string)*<br>`max` *(optional integer, 1–100)* | List Jira issues assigned to a user or matching a JQL query. Returns key, summary, status, and priority sorted by status. |
| `get_jira_issue` | `key` *(required string)* | Retrieve full details and all comments for a specific issue key (e.g. `PROJ-123`). |
| `add_jira_comment` | `key` *(required string)*<br>`comment` *(required string)* | Add a plain-text comment to an issue and return the updated issue data. |

### Client Configuration Example

#### Claude Desktop (`claude_desktop_config.json`)
```json
{
  "mcpServers": {
    "jira": {
      "command": "/absolute/path/to/myjira/target/release/myjira",
      "args": ["--mcp"],
      "env": {
        "JIRA_BASE_URL": "https://your-domain.atlassian.net",
        "JIRA_USER_EMAIL": "user@example.com",
        "JIRA_API_TOKEN": "your_jira_api_token"
      }
    }
  }
}
```

---

## Dependencies

- [`reqwest`](https://crates.io/crates/reqwest) – HTTP client with JSON support and Basic Authentication.
- [`tokio`](https://crates.io/crates/tokio) – Asynchronous runtime.
- [`serde`](https://crates.io/crates/serde) & [`serde_json`](https://crates.io/crates/serde_json) – Data serialization and ADF manipulation.
- [`dotenvy`](https://crates.io/crates/dotenvy) – `.env` file management.
- [`rmcp`](https://crates.io/crates/rmcp) – Model Context Protocol server implementation over standard I/O.
- [`schemars`](https://crates.io/crates/schemars) – JSON Schema generation for MCP tool parameter validation.

---

## Running Tests

Run the built-in unit tests (covering argument parsing, ADF document parsing, JQL generation, and status ranking):

```bash
cargo test
```
