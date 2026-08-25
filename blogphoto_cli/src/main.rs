mod cli;
mod display;
mod exif;

use clap::Parser;
use colored::Colorize;

use cli::{Cli, Commands};
use display::{print_human, print_json_batch, print_json_single, print_no_exif_warning};
use exif::{read_exif, ExifError};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Exif(args) => {
            let is_batch = args.files.len() > 1;
            let tag_filter = args.tag.as_deref();

            // For --json batch mode, collect all results first then emit single JSON
            if args.json && is_batch {
                let mut results = Vec::new();
                for path in &args.files {
                    let res = read_exif(path);
                    results.push((path.clone(), res));
                }
                print_json_batch(&results, tag_filter);

                // Exit code: 0 if at least one succeeded, 1 if all failed
                let any_ok = results.iter().any(|(_, r)| r.is_ok());
                if !any_ok {
                    std::process::exit(1);
                }
                return;
            }

            let mut any_success = false;
            let mut any_error = false;

            for (idx, path) in args.files.iter().enumerate() {
                if is_batch {
                    if idx > 0 {
                        println!(); // blank line between files
                    }
                    println!("{}", format!("==> {} <==", path.display()).bold());
                }

                match read_exif(path) {
                    Ok(fields) => {
                        any_success = true;
                        if args.json {
                            print_json_single(path, &fields, tag_filter);
                        } else {
                            print_human(path, &fields, tag_filter, args.verbose, args.raw);
                        }
                    }
                    Err(ExifError::NoExif(_)) => {
                        if args.json {
                            // Emit JSON even for no-exif case so callers can parse it
                            print_json_single(path, &[], tag_filter);
                            // Also hint on stderr
                            print_no_exif_warning(path);
                        } else {
                            print_no_exif_warning(path);
                            if !is_batch {
                                // For single file, also show hint about stripping/screenshot
                                eprintln!(
                                    "{}",
                                    "  Tip: screenshots, edited exports, and web downloads often have no EXIF."
                                        .dimmed()
                                );
                            }
                        }
                        // NoExif is exit 2 per plan for single-file; for batch we just mark error
                        if !is_batch {
                            std::process::exit(2);
                        } else {
                            any_error = true;
                        }
                    }
                    Err(e) => {
                        eprintln!("{} {}", "error:".red().bold(), e);
                        any_error = true;
                        if !is_batch {
                            std::process::exit(1);
                        }
                    }
                }
            }

            if is_batch && !any_success {
                std::process::exit(1);
            }
            if is_batch && any_error && any_success {
                // Partial success — exit 0 but caller can inspect json errors
            }
        }
    }
}
