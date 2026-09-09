# blogphoto CLI

Blog photo tooling — inspect EXIF metadata and check folders for privacy-sensitive data (GPS, device info) before publishing.

Built with Rust + `clap` + [`kamadak-exif`](https://github.com/kamadak/exif-rs). Part of the `rust_learning` series (CLI → error handling → modular architecture).

## Features

- **`exif` — dump all EXIF tags** from one or more photos, grouped and colorized
  - Human-readable table, JSON output, raw debug dump
  - Case-insensitive tag filter (`--tag GPS`)
  - Verbose mode with IFD tag codes
  - Batch mode with `==> file <==` headers and file-keyed JSON
  - ⚠️ GPS warning when location tags are present
- **`check` — privacy scan** of a folder of images
  - Flags GPS location by default; `--strict` also flags device / software / author / comments / thumbnails
  - Human summary + `--json` for CI, `--recursive`, `--verbose`
  - Distinct exit codes so scripts can gate publishing

## Installation

Prerequisites: Rust 1.70+ (tested on 1.96), Cargo.

```bash
git clone <repo-url>
cd blogphoto_cli
cargo build --release
# binary: ./target/release/blogphoto_cli
# (clap name is `blogphoto`, so help shows `blogphoto`; rename/symlink if you like)
```

Optional: alias it to the shorter name:

```bash
ln -s "$PWD/target/release/blogphoto_cli" ~/.cargo/bin/blogphoto
blogphoto --help
```

Run without installing:

```bash
cargo run -q -- exif photo.jpg
```

## Quick start

```bash
# Inspect a photo
blogphoto exif photo.jpg

# Machine-readable output for scripting
blogphoto exif photo.jpg --json | jq

# Only GPS-related tags
blogphoto exif photo.jpg --tag GPS

# Batch: shell glob expands to multiple files
blogphoto exif *.jpg
blogphoto exif img1.jpg img2.png --json > exif.json

# Privacy check a folder before publishing
blogphoto check ./photos
blogphoto check ./photos --recursive --strict --verbose
blogphoto check ./photos --json | jq '.issues'
```

## Usage

```
blogphoto <COMMAND>

Commands:
  exif   List all EXIF info from a photo
  check  Check a folder for privacy-sensitive EXIF (GPS etc.)
```

### `blogphoto exif <FILE>...`

```
Usage: blogphoto exif [OPTIONS] <FILE>...

Arguments:
  <FILE>...  Path(s) to image file(s) — supports glob via shell expansion (e.g. *.jpg)

Options:
      --json            Output as JSON (machine-readable, file-keyed map in batch mode)
      --raw             Output raw debug representation (flat tag list, good for unknown tags)
  -t, --tag <FILTER>    Filter to tag(s) containing this string (case-insensitive, e.g. "GPS", "Model")
      --verbose         Include IFD number and tag code alongside description
  -h, --help            Print help
```

Examples:

```bash
blogphoto exif tests/fixtures/sample_with_exif.jpg
blogphoto exif tests/fixtures/sample_with_exif.jpg --tag Model
blogphoto exif tests/fixtures/sample_with_exif.jpg --verbose
blogphoto exif tests/fixtures/sample_with_exif.jpg --raw
blogphoto exif tests/fixtures/sample_with_exif.jpg --json
blogphoto exif a.jpg b.jpg --json   # {"a.jpg": {...}, "b.jpg": {...}}
```

Example output (truncated):

```
File: tests/fixtures/sample_with_exif.jpg
Size: 7.8 KB

[Image]
  Make                   : "Canon"
  Model                  : "Canon EOS 40D"
  ExposureTime           : 1/160 s
  FNumber                : f/7.1
  FocalLength            : 135 mm
  DateTimeOriginal       : 2008-05-30 15:56:01
  ...

[Thumbnail]
  Compression            : JPEG
  ...

— 47 tags total
```

If GPS location tags exist, a warning is appended:

```
⚠️  GPS location found — consider stripping before publishing!
— 38 tags total (GPS present)
```

Files with no EXIF print a friendly note to stderr (not a crash):

```
info: No EXIF data found in no_exif.jpg (image may be stripped or screenshot)
```

Single-file JSON shape:

```json
{
  "file": "photo.jpg",
  "count": 47,
  "has_gps": false,
  "fields": [
    { "ifd": "Image", "tag": "Model", "code": 272, "value": "\"Canon EOS 40D\"", "raw": "Ascii([\"Canon EOS 40D\"])" }
  ]
}
```

Batch JSON shape (`--json` with >1 file): `{ "<file>": { "count": N, "has_gps": bool, "fields": [...], "error": null | "msg" } }`.

### `blogphoto check <FOLDER>`

```
Usage: blogphoto check [OPTIONS] <FOLDER>

Arguments:
  <FOLDER>  Folder to scan for images

Options:
  -r, --recursive  Recurse into subdirectories
      --json       Output as JSON
      --strict     Strict mode: flag any EXIF (device/software/thumbnail) as privacy issue, not just GPS
  -v, --verbose    Show all files, not just those with issues
  -h, --help       Print help
```

Supported extensions: `jpg jpeg png tiff tif webp heic heif avif dng cr2 nef arw` (case-insensitive).

What gets flagged:

| Mode | Flagged |
|------|---------|
| Default | GPS lat/lon/altitude refs ("MUST strip"); other GPS tags (direction, timestamp) as "may reveal location/time". `GPSVersionID` alone is **not** an issue. |
| `--strict` | All of the above, plus: device (`Make`, `Model`, `LensModel`, serials…), software (`Software`, `HostComputer`…), author (`Artist`, `Copyright`, `OwnerName`…), non-empty descriptions/comments (`ImageDescription`, `UserComment`…; all-zero padding is ignored), embedded thumbnails, and a generic "EXIF present — consider stripping" fallback if anything else remains. |

Example:

```bash
$ blogphoto check tests/fixtures
Scanning: tests/fixtures
— 4 images found

⚠️  Privacy issues found in 1 / 4 images

  → gps_sample.jpg
     • GPS location — reveals exact shooting location (GPSLatitudeRef: N, GPSLatitude: 43 deg 28 min 5.67599999 sec N, ...) — MUST strip before publishing

  3 safe files not shown (use --verbose to list)
Summary: 1 with issues, 3 safe — 4 total
Tip: strip EXIF before publishing: blogphoto exif <FILE> to inspect, then use a strip tool (e.g. exiftool -all= file.jpg)
```

JSON shape:

```json
{
  "total": 4,
  "issues": 1,
  "safe": 3,
  "findings": [
    { "file": "tests/fixtures/gps_sample.jpg", "has_issue": true, "has_exif": true, "tag_count": 20, "reasons": ["GPS location — ..."] }
  ]
}
```

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | Success (for `check`: no privacy issues; for batch `exif`: at least one file succeeded) |
| `1` | File/folder not found, not an image / unsupported format, batch `exif` where all files failed |
| `2` | `exif` single file: valid image but no EXIF data |
| `3` | `check`: privacy issues found (so CI can fail the publish step) |

Useful patterns:

```bash
blogphoto check ./photos || [ $? -eq 0 ] && echo "safe to publish"
blogphoto exif photo.jpg --json > meta.json # $? == 2 means "no EXIF", not a bug
```

## Stripping EXIF

This tool inspects and flags — it does not strip (yet). To strip before publishing:

```bash
exiftool -all= photo.jpg
# or: magick photo.jpg -strip photo_clean.jpg
```

Verify with `blogphoto check` afterwards.

## Development

```bash
cargo test        # unit tests (sorting, GPS/strict privacy logic, ext detection, zero-hex filtering)
cargo run -q -- exif --help
cargo run -q -- check --help
```

Test fixtures live in `tests/fixtures/`: `sample_with_exif.jpg`, `gps_sample.jpg`, `no_exif.jpg`, `no_exif.png`.

Project layout:

```
src/
  main.rs      # dispatch + exit-code handling
  cli.rs       # clap Parser / Subcommands / Args
  exif.rs      # read_exif() → Vec<ExifField>, ExifError (thiserror)
  display.rs   # human table / JSON / check rendering (colored)
  privacy.rs   # folder scan, analyze(), PrivacyFinding
```

Key dependencies: `clap 4 (derive)`, `kamadak-exif`, `colored`, `serde` / `serde_json`, `thiserror`.

Notes / limitations:

- `kamadak-exif` exposes only `PRIMARY` vs `THUMBNAIL` IFDs, so all main tags display under the `[Image]` group and thumbnails under `[Thumbnail]`.
- HEIC/HEIF/AVIF/WebP/PNG support depends on what `kamadak-exif` can parse; unreadable files surface as `InvalidFormat` rather than `NoExif`.
- `--tag` matches case-insensitively against tag, IFD, value, and code (hex and decimal).
