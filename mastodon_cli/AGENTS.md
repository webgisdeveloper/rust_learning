# Agents Guide: mastodon_cli

## Developer Commands
- Verification: `cargo check`
- Testing: `cargo test`
- Build: `cargo build --release`

## Authentication & API
- **Token**: Priority is `--token` flag $\rightarrow$ `MASTODON_TOKEN` environment variable.
- **API Endpoint**: `https://mastodon.social`
- **Scope Gotcha**: Fetching recent statuses requires `read:accounts` and `read:statuses` scopes. Posting requires `write:statuses`.

## Key Logic
- **Emoji Replacement**: Uses the `emojis` crate (`emojis::get_by_shortcode`) for comprehensive Unicode support. The `replace_emojis` function uses a single-pass regex for performance.
- **HTML Cleaning**: `clean_html` strips HTML tags via regex and decodes entities using the `html-escape` crate.
