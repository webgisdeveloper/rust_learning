# Contributing / Learning Guide — mastodon_cli

> For agent quick reference (commands, scopes, file map), see [AGENTS.md](AGENTS.md).

This project is a **learning project for Rust beginners**. Every source file is heavily commented to explain *why* things are done, not just *what*. This guide is the human complement to those comments.

## Prerequisites & Setup

- Install Rust via [rustup](https://rustup.rs/): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Build: `cargo build --release` → binary at `target/release/mastodon_cli`
- Configure auth (see also `README.md`):
  ```bash
  export MASTODON_TOKEN=your_token_here          # required
  export MASTODON_INSTANCE=https://mastodon.social  # optional, defaults to mastodon.social
  ```
- Run help: `cargo run -- --help`

## Project Tour

| Module | What it does | Key Rust concepts | Where to look |
|--------|--------------|-------------------|---------------|
| `src/main.rs` | Glue: parse args, resolve config, reuse HTTP client, branch POST vs GET, print results | `#[tokio::main]`, `async`/`await`, `Result<Box<dyn Error>>`, `Option` chaining, `?` operator | `DEFAULT_INSTANCE`, `resolve_instance()`, `main()` |
| `src/cli.rs` | Defines CLI only | `#[derive(Parser)]` proc macro, `#[command]`/`#[arg]` attributes, `Option<T>` for optional flags, `value_parser!(u32).range(1..=40)` | `struct Args` |
| `src/api.rs` | Mastodon API types & helpers | `serde` `Serialize`/`Deserialize`, `skip_serializing_if`, `async fn`, `reqwest::multipart`, `Box<dyn Error>` | `normalize_instance`, `api_url`, `upload_media`, `StatusRequest` |
| `src/format.rs` | Pure text transforms & box rendering | `OnceLock<Regex>`, `Regex`, `emojis` crate, `html-escape`, `UnicodeWidthStr`, `saturating_sub` | `replace_emojis`, `clean_html`, `wrap_text`, `format_status` |
| `Cargo.toml` | Dependencies | `edition = "2024"`, `features = ["derive"]` | `dependencies` |

All modules use `pub(crate)` — visible inside the crate but not outside. Good default for a binary.

## How Comments Work

- `//!` at top of file = **module doc comment** (describes the whole file). Appears in `cargo doc`.
- `///` above an item = **doc comment** for that item (struct, function).
- `//` = inline comment explaining a nearby line for learners.
- `main.rs` header has `WORKFLOW OVERVIEW` (5 steps) and `LIBRARIES USED` — read it first.
- Each file repeats its own *Concepts covered* list so you can learn one file at a time.

## Concept Glossary

**`#[derive(Parser)]` (clap):** A procedural macro that generates argument-parsing code at compile time from a struct. `#[command(author, version, about)]` pulls metadata from `Cargo.toml`. `#[arg(short, long)]` creates `-m`/`--message`. `Option<String>` makes a flag optional; `u32` with `value_parser!(u32).range(1..=40)` validates at parse time before your code runs.

**`Option<T>` / `Result<T, E>` / `Box<dyn Error>`:** `Option` is “maybe a value” (`Some`/`None`). `Result` is “success or error” (`Ok`/`Err`). `?` propagates errors early (`ok_or(...)?`). `Box<dyn Error>` erases the concrete error type so one function can return `io::Error`, `reqwest::Error`, etc.

**`async` / `await` + `#[tokio::main]`:** `async fn` returns a `Future`. `await` pauses without blocking the thread. Rust's stdlib defines `Future` but no executor; `tokio` provides the thread pool and reactor. `#[tokio::main]` rewrites `async fn main` into a sync `fn main` that blocks on the future.

**`OnceLock<Regex>`:** Thread-safe lazy init. Compiling a `Regex` is expensive, so `EMOJI_RE`/`HTML_RE` compile once on first use via `get_or_init(|| Regex::new(...).unwrap())` and are reused. Like `lazy_static` but from std (since 1.70).

**`serde` `Serialize`/`Deserialize` + `skip_serializing_if`:** `#[derive(Serialize)]` lets `client.json(&body)` turn a struct into JSON. `skip_serializing_if = "Option::is_none"` omits the key entirely when `None` (Mastodon treats missing vs `null` differently). `Deserialize` does the reverse; unknown JSON fields are ignored by default.

**`reqwest` pooling:** `reqwest::Client::new()` holds a connection pool. Reusing one `Client` for all requests is faster than `reqwest::get` per call. It handles TLS and `AUTHORIZATION: Bearer <token>`.

**`Regex` + `emojis`:** `Regex::new(r":([a-z0-9_]+):")` captures shortcode as group 1. `replace_all` with a closure does single-pass `O(n)` replacement. `emojis::get_by_shortcode(&caps[1])` looks up Unicode; if `None`, we keep `caps[0]` unchanged.

**`html-escape`:** `html_escape::decode_html_entities(&stripped).into_owned()` turns `&amp;`→`&`, `&gt;`→`>` etc. after stripping `<tags>` with `<[^>]*>`.

**`UnicodeWidthStr::width`:** Terminal column width, not byte length (`"🖼️".width() == 2`, `"a".len() == 1`). Needed to align box borders; `saturating_sub` avoids underflow when computing `padding` and `remaining_border`.

**Struct update syntax `..status("...")`:** In tests, `Status { in_reply_to_id: Some(...), ..status("hi") }` fills remaining fields from `status("hi")`. Handy for concise test fixtures.

**`#[cfg(test)]`:** Module only compiled with `cargo test`. Keeps binaries lean while staying close to code under test.

## How to Add a Feature — Checklist

1. **Flag** in `src/cli.rs` — add field to `Args` with `#[arg(...)]`, use `Option` for optional, `value_parser` for validation.
2. **Model/helper** in `src/api.rs` — extend `StatusRequest` or add new struct + `api_url`-based function. Keep URL building via `api_url`.
3. **Pure logic** in `src/format.rs` — if it’s text/display, keep it here (testable without network).
4. **Wire** in `src/main.rs` — resolve config via `resolve_instance` pattern, use shared `Client`, handle `is_success()` + `error_text` + `process::exit(1)` on failure.
5. **Test** — add unit test in `format.rs` or `main.rs` `#[cfg(test)]` (aim for `cargo test` still passing; current: 9 tests). For display, assert `contains("…")` and box borders.
6. **Verify** — `cargo check && cargo test` (and `cargo build --release` if you want the binary). Update `README.md` flags if you added CLI surface.

Example: adding `--visibility` would touch `cli.rs` (`visibility: Option<String>`), `api.rs` (`StatusRequest.visibility`), `main.rs` (pass through), plus a serialization test.

## Code Style

- Keep `pub(crate)` unless you’re extracting a library.
- Prefer `saturating_sub` + `UnicodeWidthStr` for any terminal width math.
- Use `api_url(&instance, "/api/v1/...")` — never hard-code `https://mastodon.social` outside `DEFAULT_INSTANCE`.
- Validate CLI ranges with `clap` rather than manual `if` checks.
- Keep `format.rs` pure — no `reqwest` or `std::env` there.

## Getting Help

- Read `src/main.rs` header, then pick one module to read top-to-bottom.
- Run `cargo test -- --nocapture` to see test output.
- For agent automation details (exact commands, scopes, file map), see `AGENTS.md`.
