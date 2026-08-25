use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "blogphoto",
    version,
    about = "Blog photo tooling — inspect EXIF and more",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List all EXIF info from a photo
    Exif(ExifArgs),
}

#[derive(Args, Debug)]
pub struct ExifArgs {
    /// Path(s) to image file(s) — supports glob via shell expansion (e.g. *.jpg)
    #[arg(required = true, value_name = "FILE")]
    pub files: Vec<PathBuf>,

    /// Output as JSON (machine-readable, file-keyed map in batch mode)
    #[arg(long)]
    pub json: bool,

    /// Output raw debug representation (flat tag list, good for unknown tags)
    #[arg(long)]
    pub raw: bool,

    /// Filter to tag(s) containing this string (case-insensitive, e.g. "GPS", "Model")
    #[arg(long, short = 't', value_name = "FILTER")]
    pub tag: Option<String>,

    /// Include IFD number and tag code alongside description
    #[arg(long)]
    pub verbose: bool,
}
