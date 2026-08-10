//! `cli` — Command-line argument definitions
//!
//! This module is only responsible for *describing* the CLI. It does not
//! contain any business logic. `clap` will generate the parser, `--help`
//! text, and validation from the struct below.
//!
//! Key concepts for learners:
//! - `#[derive(Parser)]` is a *procedural macro* from `clap` that auto-generates
//!   argument parsing code at compile time.
//! - `#[command(...)]` and `#[arg(...)]` are attributes that configure that
//!   generated parser (name, version, help text, short/long flags, etc.).

use clap::Parser;

/// Command-line arguments for `mastodon_cli`.
///
/// `clap` reads this struct and creates flags like `--message`, `--image`,
/// `--token`, `--instance` and `--list`/`--limit` automatically.
/// Each field becomes one CLI argument.
#[derive(Parser, Debug)]
// `author`, `version`, `about` are pulled from `Cargo.toml` at compile time.
// `long_about = None` disables a separate long help string.
#[command(author, version, about, long_about = None)]
pub(crate) struct Args {
    /// The message to post.
    ///
    /// `Option<String>` means the flag is optional. If the user omits
    /// `--message`, this will be `None` and the program will enter "list" mode.
    // `short` creates `-m`, `long` creates `--message`.
    #[arg(short, long)]
    pub(crate) message: Option<String>,

    /// Path to an image to upload alongside the post.
    ///
    /// Also `Option<String>` — only used when posting. The actual upload
    /// happens in `api::upload_media`.
    #[arg(short, long)]
    pub(crate) image: Option<String>,

    /// The Mastodon access token.
    ///
    /// Made optional so we can fall back to the `MASTODON_TOKEN` environment
    /// variable in `main.rs`. Priority is: `--token` flag > env var > error.
    #[arg(short, long)]
    pub(crate) token: Option<String>,

    /// Mastodon instance URL (e.g. `https://mastodon.social`).
    ///
    /// Also optional — falls back to `MASTODON_INSTANCE` env var, then to
    /// `https://mastodon.social`. See `resolve_instance()` in `main.rs` and
    /// `api::normalize_instance()` for how the final URL is cleaned.
    #[arg(long)]
    pub(crate) instance: Option<String>,

    /// Number of recent statuses to fetch (only when `--message` is not provided).
    ///
    /// - Primary flag is `--list` (`-l`), alias is `--limit` to match Mastodon API naming.
    /// - `default_value_t = 5` keeps the previous default behavior.
    /// - `value_parser = clap::value_parser!(u32).range(1..=40)` validates input
    ///   at parse time: Mastodon caps `limit` at 40, and `0` would be meaningless.
    ///   If the user passes `--list 100`, clap will print an error and exit before
    ///   our code even runs.
    #[arg(short, long, alias = "limit", default_value_t = 5, value_parser = clap::value_parser!(u32).range(1..=40))]
    pub(crate) list: u32,
}
