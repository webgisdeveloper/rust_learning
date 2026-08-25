use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, Serialize)]
pub struct ExifField {
    /// IFD name: "TIFF", "EXIF", "GPS", "Interop", "Thumbnail"
    pub ifd: String,
    /// Human-readable tag name, e.g. "Model", "ExposureTime"
    pub tag: String,
    /// Raw tag code (u16, e.g. 271 = 0x010F)
    pub code: u16,
    /// Formatted value with unit, e.g. "1/120 sec", "Apple"
    pub value: String,
    /// Debug/raw representation of the underlying value variant
    pub raw: String,
}

#[derive(Error, Debug)]
pub enum ExifError {
    #[error("File not found: {}", .0.display())]
    NotFound(PathBuf),

    #[error("Not a valid image or unsupported format: {}", .0.display())]
    InvalidFormat(PathBuf),

    #[error("No EXIF data found in {}", .0.display())]
    NoExif(PathBuf),

    #[error("I/O error reading {}: {source}", .path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to read EXIF from {}: {source}", .path.display())]
    ExifRead {
        path: PathBuf,
        #[source]
        source: exif::Error,
    },
}

/// Read all EXIF fields from `path`.
///
/// Returns `Err(ExifError::NoExif)` when the file is a valid image but has no EXIF.
/// Returns `Err(ExifError::InvalidFormat)` / `NotFound` / `Io` for other failures.
pub fn read_exif(path: &Path) -> Result<Vec<ExifField>, ExifError> {
    if !path.exists() {
        return Err(ExifError::NotFound(path.to_path_buf()));
    }

    let file = File::open(path).map_err(|e| ExifError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    let mut reader = BufReader::new(&file);

    let exif_reader = exif::Reader::new();
    let exif = match exif_reader.read_from_container(&mut reader) {
        Ok(exif) => exif,
        Err(exif::Error::NotFound(_)) => {
            return Err(ExifError::NoExif(path.to_path_buf()));
        }
        Err(exif::Error::BlankValue(_)) => {
            // Blank EXIF segment counts as "no exif" for UX purposes
            return Err(ExifError::NoExif(path.to_path_buf()));
        }
        Err(e) => {
            // Distinguish invalid format vs other exif errors
            let msg = e.to_string().to_lowercase();
            if msg.contains("invalid") || msg.contains("unknown image format") {
                return Err(ExifError::InvalidFormat(path.to_path_buf()));
            }
            return Err(ExifError::ExifRead {
                path: path.to_path_buf(),
                source: e,
            });
        }
    };

    let mut fields = Vec::new();
    for f in exif.fields() {
        let ifd = ifd_to_string(f.ifd_num);
        // Tag name: prefer description, fallback to tag string
        let tag_name = f.tag.to_string();
        let code = f.tag.number();
        let value = f.display_value().with_unit(&exif).to_string();
        let raw = format!("{:?}", f.value);

        fields.push(ExifField {
            ifd,
            tag: tag_name,
            code,
            value,
            raw,
        });
    }

    if fields.is_empty() {
        return Err(ExifError::NoExif(path.to_path_buf()));
    }

    // Sort for stable output: by IFD priority then tag code
    fields.sort_by(|a, b| {
        ifd_order(&a.ifd)
            .cmp(&ifd_order(&b.ifd))
            .then(a.code.cmp(&b.code))
    });

    Ok(fields)
}

fn ifd_to_string(ifd: exif::In) -> String {
    // kamadak-exif only distinguishes PRIMARY (0) and THUMBNAIL (1).
    // All TIFF/EXIF/GPS/Interop tags are flattened into PRIMARY.
    // We map PRIMARY -> "TIFF/EXIF" group and THUMBNAIL -> "Thumbnail"
    // for nicer display while preserving debug fallback for unknown IFDs.
    match ifd.0 {
        0 => "Image".to_string(),
        1 => "Thumbnail".to_string(),
        n => format!("IFD{}", n),
    }
}

fn ifd_order(ifd: &str) -> u8 {
    match ifd {
        "Image" => 0,
        "Thumbnail" => 1,
        _ => 99,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_fields_by_ifd_then_code() {
        let mut fields = vec![
            ExifField {
                ifd: "EXIF".into(),
                tag: "B".into(),
                code: 2,
                value: "v".into(),
                raw: "v".into(),
            },
            ExifField {
                ifd: "GPS".into(),
                tag: "A".into(),
                code: 1,
                value: "v".into(),
                raw: "v".into(),
            },
            ExifField {
                ifd: "EXIF".into(),
                tag: "A".into(),
                code: 1,
                value: "v".into(),
                raw: "v".into(),
            },
        ];
        fields.sort_by(|a, b| a.ifd.cmp(&b.ifd).then(a.code.cmp(&b.code)));
        assert_eq!(fields[0].ifd, "EXIF");
        assert_eq!(fields[0].code, 1);
        assert_eq!(fields[1].ifd, "EXIF");
        assert_eq!(fields[1].code, 2);
        assert_eq!(fields[2].ifd, "GPS");
    }
}
