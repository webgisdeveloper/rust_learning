use clap::Parser;

/// Command-line arguments for mastodon_cli.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Args {
    /// The message to post
    #[arg(short, long)]
    pub(crate) message: Option<String>,

    /// Path to an image to upload
    #[arg(short, long)]
    pub(crate) image: Option<String>,

    /// The Mastodon access token
    #[arg(short, long)]
    pub(crate) token: Option<String>,

    /// Mastodon instance URL
    #[arg(long)]
    pub(crate) instance: Option<String>,

    /// Number of recent statuses to fetch (only when --message is not provided)
    #[arg(short, long, alias = "limit", default_value_t = 5, value_parser = clap::value_parser!(u32).range(1..=40))]
    pub(crate) list: u32,
}
