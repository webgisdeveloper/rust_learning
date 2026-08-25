use std::collections::BTreeMap;
use std::path::Path;

use colored::Colorize;

use crate::exif::ExifField;

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
