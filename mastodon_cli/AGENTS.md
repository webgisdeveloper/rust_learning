# Agents Guide: mastodon_cli

> For human learning guide, see [CONTRIBUTING.md](CONTRIBUTING.md).

## Developer Commands
- Verification: `cargo check`
- Testing: `cargo test`
- Build: `cargo build --release`
- Run: `cargo run -- --help`

## Authentication & API
- **Token**: `--token` → `MASTODON_TOKEN` → error. Scopes: `read:accounts`+`read:statuses` (list), `write:statuses`+`write:media` (post/upload).
- **Instance**: `--instance` → `MASTODON_INSTANCE` → `https://mastodon.social`. Use `api::normalize_instance` / `api::api_url(instance, path)`; never hard-code host outside `main.rs:DEFAULT_INSTANCE`.

## Project Structure (Phase 0)
- `src/main.rs` — `DEFAULT_INSTANCE`, `resolve_instance()`, `#[tokio::main]`, `Client` reuse, POST/GET branch
- `src/cli.rs` — `Args: Parser` (`message?`, `image?`, `token?`, `instance?`, `list` 1..=40 default 5)
- `src/api.rs` — `normalize_instance`, `api_url`, `StatusRequest`/`Account`/`Status`, `upload_media` (multipart, Bearer)
- `src/format.rs` — `replace_emojis` (`OnceLock<Regex>`), `clean_html`, `wrap_text` (`UnicodeWidthStr`), `format_status` (76/72, `saturating_sub`)
- `pub(crate)` throughout; `Cargo.toml` edition 2024

## Key Logic
- **Emoji**: `emojis::get_by_shortcode`, single-pass `:([a-z0-9_]+):`, `EMOJI_RE: OnceLock`, preserve unknown
- **HTML**: `<[^>]*>` strip + `html_escape::decode_html_entities`
- **Wrap**: `UnicodeWidthStr::width`, word-boundary, blank-line preserve, overlong word emit
- **Box**: `format_status` → `┌── Status #N` / `🧵`/`🖼️` row / `├─┤` / padded `│` lines / `└──┘`
- **URL**: `trim`/`trim_end_matches('/')`/`https://` fallback; `resolve_instance` chains `or_else`→`unwrap_or_else`→`normalize`
- **Upload**: `fs::read` → `Part::bytes` → `Form` → `POST /api/v1/media` → `MediaResponse.id` → `StatusRequest.media_ids`

## Conventions
- Reuse `Client::new()`; validate via `clap value_parser`; keep `format.rs` pure/testable

## Verification
- `cargo check` / `cargo test` (9 tests: emoji/html/wrap/box/URL) / `cargo build --release`
