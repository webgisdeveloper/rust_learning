use std::collections::BTreeMap;
use std::path::Path;

use colored::Colorize;

use crate::exif::ExifField;
use crate::privacy::PrivacyFinding;

/// Filter fields by optional case-insensitive substring on tag/ifd/value/code.
pub fn filter_fields<'a>(fields: &'a [ExifField], filter: Option<&str>) -> Vec<&'a ExifField> {
    match filter {
        None => fields.iter().collect(),
        Some(q) => {
            let q = q.to_lowercase();
            fields
                .iter()
                .filter(|f| {
                    f.tag.to_lowercase().contains(&q)
                        || f.ifd.to_lowercase().contains(&q)
                        || f.value.to_lowercase().contains(&q)
                        || format!("{:04x}", f.code).contains(&q)
                        || f.code.to_string().contains(&q)
                })
                .collect()
        }
    }
}

fn is_gps_location_field(f: &ExifField) -> bool {
    // Only warn on actual location, not just GPSVersionID
    let t = f.tag.to_lowercase();
    t.contains("latitude") || t.contains("longitude") || t == "gpsaltitude" || t == "gps altitude"
}

fn has_gps(fields: &[&ExifField]) -> bool {
    fields.iter().any(|f| is_gps_location_field(f))
}

/// Print human-readable grouped table for one file.
pub fn print_human(
    path: &Path,
    fields: &[ExifField],
    tag_filter: Option<&str>,
    verbose: bool,
    raw: bool,
) {
    let filtered = filter_fields(fields, tag_filter);

    // File header
    let file_label = format!("File: {}", path.display());
    println!("{}", file_label.bold().cyan());

    // Try to show file size
    if let Ok(meta) = std::fs::metadata(path) {
        let size = meta.len();
        let size_str = if size < 1024 {
            format!("{} B", size)
        } else if size < 1024 * 1024 {
            format!("{:.1} KB", size as f64 / 1024.0)
        } else {
            format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
        };
        println!("Size: {}", size_str.dimmed());
    }

    if let Some(q) = tag_filter {
        println!("Filter: tag contains {:?} ({} matched)", q, filtered.len());
    }
    println!();

    if filtered.is_empty() {
        println!("{}", "  (no tags matched filter)".dimmed());
        return;
    }

    // Group by IFD preserving sorted order (fields already sorted)
    let mut grouped: BTreeMap<String, Vec<&&ExifField>> = BTreeMap::new();
    for f in &filtered {
        grouped.entry(f.ifd.clone()).or_default().push(f);
    }

    for (ifd, group) in grouped {
        println!("{}", format!("[{}]", ifd).bold().yellow());
        for f in group {
            if raw {
                // Raw mode: show debug value
                if verbose {
                    println!(
                        "  {:<22} [{} / 0x{:04X}]: {}  (raw: {})",
                        f.tag, f.code, f.code, f.value, f.raw
                    );
                } else {
                    println!("  {:<22} : {}  (raw: {})", f.tag, f.value, f.raw);
                }
            } else if verbose {
                println!(
                    "  {:<22} [{} / 0x{:04X}]: {}",
                    f.tag, f.code, f.code, f.value
                );
            } else {
                // Align colon: pad tag to 22 chars
                println!("  {:<22} : {}", f.tag, f.value);
            }
        }
        println!();
    }

    let total = filtered.len();
    let gps = has_gps(&filtered);
    if gps {
        println!(
            "{}",
            "⚠️  GPS location found — consider stripping before publishing!".yellow().bold()
        );
    }
    println!(
        "{}",
        format!(
            "— {} tag{} total{}",
            total,
            if total == 1 { "" } else { "s" },
            if gps { " (GPS present)" } else { "" }
        )
        .dimmed()
    );
}

/// Print one file's fields as pretty JSON to stdout.
pub fn print_json_single(path: &Path, fields: &[ExifField], tag_filter: Option<&str>) {
    let filtered = filter_fields(fields, tag_filter);
    // Collect owned filtered fields for serialization
    let owned: Vec<&ExifField> = filtered;
    let wrapper = serde_json::json!({
        "file": path.display().to_string(),
        "count": owned.len(),
        "has_gps": has_gps(&owned),
        "fields": owned,
    });
    println!("{}", serde_json::to_string_pretty(&wrapper).unwrap());
}

/// Print batch as JSON: map of file -> fields.
pub fn print_json_batch(
    results: &[(std::path::PathBuf, Result<Vec<ExifField>, crate::exif::ExifError>)],
    tag_filter: Option<&str>,
) {
    let mut map: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    for (path, res) in results {
        let key = path.display().to_string();
        match res {
            Ok(fields) => {
                let filtered = filter_fields(fields, tag_filter);
                let owned: Vec<&ExifField> = filtered.clone();
                map.insert(
                    key,
                    serde_json::json!({
                        "count": owned.len(),
                        "has_gps": has_gps(&owned),
                        "fields": owned,
                        "error": serde_json::Value::Null,
                    }),
                );
            }
            Err(e) => {
                map.insert(
                    key,
                    serde_json::json!({
                        "count": 0,
                        "has_gps": false,
                        "fields": [],
                        "error": e.to_string(),
                    }),
                );
            }
        }
    }
    println!("{}", serde_json::to_string_pretty(&map).unwrap());
}

/// Called when a file has no EXIF — prints friendly message to stderr.
pub fn print_no_exif_warning(path: &Path) {
    eprintln!(
        "{} No EXIF data found in {} (image may be stripped or screenshot)",
        "info:".cyan(),
        path.display().to_string().yellow()
    );
}

pub fn print_check_human(folder: &Path, findings: &[PrivacyFinding], verbose: bool, strict: bool) {
    let total = findings.len();
    let issues = findings.iter().filter(|f| f.has_issue).count();
    let safe = total - issues;

    println!(
        "{} {}",
        "Scanning:".bold().cyan(),
        folder.display().to_string().yellow()
    );
    println!(
        "{} {} image{} found{}",
        "—".dimmed(),
        total.to_string().bold(),
        if total == 1 { "" } else { "s" },
        if strict {
            " (strict mode: any EXIF flagged)".dimmed().to_string()
        } else {
            "".to_string()
        }
    );
    println!();

    if total == 0 {
        println!("{}", "  No images found in folder (supported: jpg, jpeg, png, tiff, webp, heic, heif, avif)".dimmed());
        return;
    }

    if issues == 0 {
        println!("{}", "✅ No privacy issues found — all photos safe to publish".green().bold());
        if verbose {
            // list safe files
            for f in findings {
                let name = f.file.file_name().and_then(|n| n.to_str()).unwrap_or("?");
                if !f.has_exif {
                    println!("  {} {} — {}", "SAFE".green(), name.bold(), "No EXIF".dimmed());
                } else {
                    println!(
                        "  {} {} — {} tags, no GPS",
                        "SAFE".green(),
                        name.bold(),
                        f.tag_count
                    );
                }
            }
        } else {
            println!("{}", "  (use --verbose to list safe files)".dimmed());
        }
        println!();
        println!(
            "{}",
            format!("Summary: {}/{} safe, 0 with issues", safe, total).dimmed()
        );
        return;
    }

    // Issues found
    println!(
        "{}",
        format!(
            "⚠️  Privacy issues found in {} / {} image{}",
            issues,
            total,
            if total == 1 { "" } else { "s" }
        )
        .yellow()
        .bold()
    );
    println!();

    for f in findings.iter().filter(|f| f.has_issue) {
        // Show relative path if nested
        let display = if f.file.strip_prefix(folder).is_ok() {
            f.file.strip_prefix(folder).unwrap().display().to_string()
        } else {
            f.file.display().to_string()
        };
        println!("  {} {}", "→".red().bold(), display.bold().red());
        for reason in &f.reasons {
            println!("     {} {}", "•".yellow(), reason);
        }
        println!();
    }

    if verbose || safe > 0 {
        let safe_findings: Vec<&PrivacyFinding> = findings.iter().filter(|f| !f.has_issue).collect();
        if !safe_findings.is_empty() {
            if verbose {
                println!("{}", "Safe files:".green().bold());
                for f in safe_findings {
                    let name = f.file.display().to_string();
                    let display = if f.file.strip_prefix(folder).is_ok() {
                        f.file.strip_prefix(folder).unwrap().display().to_string()
                    } else {
                        name.clone()
                    };
                    if !f.has_exif {
                        println!("  {} {} — {}", "✔".green(), display, "No EXIF".dimmed());
                    } else {
                        println!("  {} {} — {} tags, no sensitive data", "✔".green(), display, f.tag_count);
                    }
                }
                println!();
            } else {
                println!(
                    "{}",
                    format!("  {} safe file{} not shown (use --verbose to list)", safe, if safe==1{""} else {"s"}).dimmed()
                );
            }
        }
    }

    println!(
        "{}",
        format!(
            "Summary: {} with issues, {} safe — {} total",
            issues, safe, total
        )
        .dimmed()
    );
    if issues > 0 {
        println!(
            "{}",
            "Tip: strip EXIF before publishing: blogphoto exif <FILE> to inspect, then use a strip tool (e.g. exiftool -all= file.jpg)"
                .dimmed()
        );
    }
}

pub fn print_check_json(findings: &[PrivacyFinding]) {
    let total = findings.len();
    let issues = findings.iter().filter(|f| f.has_issue).count();
    let json = serde_json::json!({
        "total": total,
        "issues": issues,
        "safe": total - issues,
        "findings": findings.iter().map(|f| {
            serde_json::json!({
                "file": f.file.display().to_string(),
                "has_issue": f.has_issue,
                "has_exif": f.has_exif,
                "tag_count": f.tag_count,
                "reasons": f.reasons,
            })
        }).collect::<Vec<_>>()
    });
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}
