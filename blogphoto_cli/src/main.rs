mod cli;
mod display;
mod exif;
mod privacy;

use clap::Parser;
use colored::Colorize;

use cli::{Cli, Commands};
use display::{print_check_human, print_check_json, print_human, print_json_batch, print_json_single, print_no_exif_warning};
use exif::{read_exif, ExifError};
use privacy::{collect_images, finding_from_result};

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
        Commands::Check(args) => {
            if !args.folder.exists() {
                eprintln!("{} Folder not found: {}", "error:".red().bold(), args.folder.display());
                std::process::exit(1);
            }
            if !args.folder.is_dir() {
                eprintln!(
                    "{} Not a directory: {} (check expects a folder, use `exif` for single files)",
                    "error:".red().bold(),
                    args.folder.display()
                );
                std::process::exit(1);
            }

            let images = match collect_images(&args.folder, args.recursive) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{} Failed to read folder {}: {}", "error:".red().bold(), args.folder.display(), e);
                    std::process::exit(1);
                }
            };

            let mut findings = Vec::new();
            for path in images {
                let result = read_exif(&path);
                let finding = finding_from_result(path, result, args.strict);
                findings.push(finding);
            }

            if args.json {
                print_check_json(&findings);
            } else {
                print_check_human(&args.folder, &findings, args.verbose, args.strict);
            }

            // Exit codes: 0 = no issues, 3 = issues found (distinct from 1/2 used by exif)
            let has_issue = findings.iter().any(|f| f.has_issue);
            if has_issue {
                std::process::exit(3);
            }
        }
    }
}
