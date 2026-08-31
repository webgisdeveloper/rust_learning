use std::path::{Path, PathBuf};

use crate::exif::ExifField;

/// A single privacy finding for a file
#[derive(Debug, Clone)]
pub struct PrivacyFinding {
    pub file: PathBuf,
    pub has_issue: bool,
    /// Human-readable reasons, empty if no issue
    pub reasons: Vec<String>,
    /// Whether EXIF was present (even if not privacy-sensitive)
    pub has_exif: bool,
    /// Total tag count (0 if no exif)
    pub tag_count: usize,
}

const IMAGE_EXTS: &[&str] = &[
    "jpg", "jpeg", "png", "tiff", "tif", "webp", "heic", "heif", "avif", "dng", "cr2", "nef", "arw",
];

pub fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn collect_images(dir: &Path, recursive: bool) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    collect_inner(dir, recursive, &mut out)?;
    out.sort();
    Ok(out)
}

fn collect_inner(dir: &Path, recursive: bool, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let meta = entry.metadata()?;
        if meta.is_dir() {
            if recursive {
                collect_inner(&path, true, out)?;
            }
        } else if meta.is_file() && is_image_file(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn truncate(v: &str, max: usize) -> String {
    if v.len() <= max {
        v.to_string()
    } else {
        format!("{}…[{} chars]", &v[..max], v.len() - max)
    }
}

fn is_mostly_zeros_hex(s: &str) -> bool {
    // Heuristic: 0x followed by >100 chars and >85% zeros
    if s.starts_with("0x") && s.len() > 100 {
        let zeros = s.chars().filter(|c| *c == '0').count();
        (zeros as f64) / (s.len() as f64) > 0.85
    } else {
        false
    }
}

/// Analyze EXIF fields and return privacy reasons.
/// `strict`: if true, any EXIF is considered a privacy concern (device fingerprint).
pub fn analyze(fields: &[ExifField], strict: bool) -> Vec<String> {
    let mut reasons = Vec::new();

    // --- GPS (always privacy-critical) ---
    let gps_tags: Vec<&ExifField> = fields
        .iter()
        .filter(|f| f.tag.to_lowercase().starts_with("gps"))
        .collect();

    // Filter out only GPSVersionID — that alone is not location
    let gps_location_tags: Vec<&ExifField> = gps_tags
        .iter()
        .filter(|f| {
            let t = f.tag.to_lowercase();
            t == "gpslatitude"
                || t == "gpslongitude"
                || t == "gpslatituderef"
                || t == "gpslongituderef"
                || t == "gpsaltitude"
                || t == "gpsaltituderef"
        })
        .copied()
        .collect();

    let other_gps_tags: Vec<&ExifField> = gps_tags
        .iter()
        .filter(|f| {
            let t = f.tag.to_lowercase();
            t != "gpsversionid" && !gps_location_tags.iter().any(|g| g.tag == f.tag)
        })
        .copied()
        .collect();

    if !gps_location_tags.is_empty() {
        let detail = gps_location_tags
            .iter()
            .map(|f| format!("{}: {}", f.tag, truncate(&f.value, 80)))
            .collect::<Vec<_>>()
            .join(", ");
        reasons.push(format!(
            "GPS location — reveals exact shooting location ({}) — MUST strip before publishing",
            detail
        ));
    } else if !other_gps_tags.is_empty() {
        // No lat/lon but other GPS present (e.g., GPSImgDirection, GPSTimeStamp)
        let detail = other_gps_tags
            .iter()
            .map(|f| f.tag.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        reasons.push(format!(
            "GPS metadata present ({}) — may reveal location/time",
            detail
        ));
    } else if gps_tags.iter().any(|f| f.tag == "GPSVersionID") && gps_tags.len() == 1 {
        // Only version tag — not really sensitive, but mention in strict mode
        if strict {
            reasons.push("GPSVersionID tag present (no location, but indicates GPS handling)".to_string());
        }
    }

    // --- Device / Software / Author (strict or always reported as warning) ---
    let has_thumbnail = fields.iter().any(|f| f.ifd == "Thumbnail");

    // Device info
    let device_tags: Vec<&ExifField> = fields
        .iter()
        .filter(|f| {
            matches!(
                f.tag.as_str(),
                "Make" | "Model" | "LensModel" | "LensMake" | "BodySerialNumber" | "LensSerialNumber" | "CameraSerialNumber"
            )
        })
        .collect();
    if !device_tags.is_empty() && strict {
        let detail = device_tags
            .iter()
            .map(|f| format!("{}: {}", f.tag, truncate(&f.value, 60)))
            .collect::<Vec<_>>()
            .join(", ");
        reasons.push(format!("Device info ({}) — reveals camera/lens used", detail));
    }

    // Software / Host
    let sw_tags: Vec<&ExifField> = fields
        .iter()
        .filter(|f| matches!(f.tag.as_str(), "Software" | "HostComputer" | "ProcessingSoftware"))
        .collect();
    if !sw_tags.is_empty() && strict {
        let detail = sw_tags
            .iter()
            .map(|f| format!("{}: {}", f.tag, truncate(&f.value, 60)))
            .collect::<Vec<_>>()
            .join(", ");
        reasons.push(format!("Software info ({}) — reveals editing workflow", detail));
    }

    // Author / copyright
    let author_tags: Vec<&ExifField> = fields
        .iter()
        .filter(|f| {
            matches!(
                f.tag.as_str(),
                "Artist" | "Copyright" | "OwnerName" | "Creator" | "Rights"
            )
        })
        .collect();
    if !author_tags.is_empty() && strict {
        let detail = author_tags
            .iter()
            .map(|f| format!("{}: {}", f.tag, truncate(&f.value, 60)))
            .collect::<Vec<_>>()
            .join(", ");
        reasons.push(format!("Author/copyright ({}) — reveals identity", detail));
    }

    // User comment / description
    let comment_tags: Vec<&ExifField> = fields
        .iter()
        .filter(|f| matches!(f.tag.as_str(), "ImageDescription" | "UserComment" | "XPComment" | "XPAuthor"))
        .collect();
    if !comment_tags.is_empty() && strict {
        let non_empty: Vec<&ExifField> = comment_tags
            .into_iter()
            .filter(|f| {
                let v = f.value.trim();
                if v.is_empty() || v == "\"\"" || v == "\"                               \"" {
                    return false;
                }
                // filter bulk-zero hex
                if is_mostly_zeros_hex(v) {
                    return false;
                }
                true
            })
            .collect();
        if !non_empty.is_empty() {
            let detail = non_empty
                .iter()
                .map(|f| format!("{}: {}", f.tag, truncate(&f.value, 80)))
                .collect::<Vec<_>>()
                .join(", ");
            reasons.push(format!("Embedded description/comment ({}) — may contain personal notes", detail));
        }
    }

    // Thumbnail
    if has_thumbnail && strict {
        reasons.push("Embedded thumbnail present — may contain uncropped image data".to_string());
    }

    // In strict mode, if still no reasons but EXIF exists, flag generic
    if strict && reasons.is_empty() && !fields.is_empty() {
        let sample = fields.iter().take(3).map(|f| f.tag.as_str()).collect::<Vec<_>>().join(", ");
        reasons.push(format!(
            "EXIF present ({} tags, e.g. {}) — consider stripping to avoid fingerprinting",
            fields.len(),
            sample
        ));
    }

    reasons
}

pub fn has_privacy_issue(reasons: &[String]) -> bool {
    !reasons.is_empty()
}

/// Build a PrivacyFinding from a file and its EXIF result
pub fn finding_from_result(
    path: PathBuf,
    result: Result<Vec<ExifField>, crate::exif::ExifError>,
    strict: bool,
) -> PrivacyFinding {
    match result {
        Ok(fields) => {
            let reasons = analyze(&fields, strict);
            PrivacyFinding {
                file: path,
                has_issue: has_privacy_issue(&reasons),
                reasons,
                has_exif: true,
                tag_count: fields.len(),
            }
        }
        Err(crate::exif::ExifError::NoExif(_)) => PrivacyFinding {
            file: path,
            has_issue: false,
            reasons: vec![],
            has_exif: false,
            tag_count: 0,
        },
        Err(e) => PrivacyFinding {
            file: path.clone(),
            has_issue: false,
            reasons: vec![format!("Could not read EXIF: {}", e)],
            has_exif: false,
            tag_count: 0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exif::ExifField;

    fn field(tag: &str, value: &str, ifd: &str) -> ExifField {
        ExifField {
            ifd: ifd.to_string(),
            tag: tag.to_string(),
            code: 0,
            value: value.to_string(),
            raw: value.to_string(),
        }
    }

    #[test]
    fn detects_gps_location() {
        let fields = vec![
            field("GPSLatitude", "43 deg N", "Image"),
            field("GPSLongitude", "11 deg E", "Image"),
            field("Make", "\"Canon\"", "Image"),
        ];
        let reasons = analyze(&fields, false);
        assert!(reasons.iter().any(|r| r.contains("GPS location")));
        assert_eq!(reasons.len(), 1); // non-strict only GPS
    }

    #[test]
    fn strict_flags_device() {
        let fields = vec![field("Make", "\"Canon\"", "Image"), field("Model", "\"EOS\"", "Image")];
        let loose = analyze(&fields, false);
        assert!(loose.is_empty());
        let strict = analyze(&fields, true);
        assert!(strict.iter().any(|r| r.contains("Device info")));
    }

    #[test]
    fn no_exif_no_issue() {
        let fields: Vec<ExifField> = vec![];
        let reasons = analyze(&fields, true);
        assert!(reasons.is_empty());
    }

    #[test]
    fn gps_version_only_not_issue_unless_strict() {
        let fields = vec![field("GPSVersionID", "2.2.0.0", "Image")];
        assert!(analyze(&fields, false).is_empty());
        assert!(!analyze(&fields, true).is_empty());
    }

    #[test]
    fn is_image_file_checks_ext() {
        assert!(is_image_file(Path::new("photo.jpg")));
        assert!(is_image_file(Path::new("photo.JPEG")));
        assert!(is_image_file(Path::new("a/b/c.heic")));
        assert!(!is_image_file(Path::new("doc.txt")));
        assert!(!is_image_file(Path::new("noext")));
    }

    #[test]
    fn finding_from_no_exif() {
        let f = finding_from_result(
            PathBuf::from("x.jpg"),
            Err(crate::exif::ExifError::NoExif(PathBuf::from("x.jpg"))),
            false,
        );
        assert!(!f.has_issue);
        assert!(!f.has_exif);
    }

    #[test]
    fn ignores_zero_hex_usercomment() {
        let big_zero = format!("0x{}", "0".repeat(200));
        let fields = vec![field("UserComment", &big_zero, "Image")];
        let reasons = analyze(&fields, true);
        // Should not produce UserComment reason (filtered), but will produce generic EXIF fallback
        assert!(!reasons.iter().any(|r| r.contains("personal notes")));
    }
}
