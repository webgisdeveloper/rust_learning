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
    /// Check a folder for privacy-sensitive EXIF (GPS etc.)
    Check(CheckArgs),
}

#[derive(Args, Debug)]
pub struct CheckArgs {
    /// Folder to scan for images
    #[arg(value_name = "FOLDER")]
    pub folder: PathBuf,

    /// Recurse into subdirectories
    #[arg(long, short = 'r')]
    pub recursive: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Strict mode: flag any EXIF (device/software/thumbnail) as privacy issue, not just GPS
    #[arg(long)]
    pub strict: bool,

    /// Show all files, not just those with issues
    #[arg(long, short = 'v')]
    pub verbose: bool,
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
