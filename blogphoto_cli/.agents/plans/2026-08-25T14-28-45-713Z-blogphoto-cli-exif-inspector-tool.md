# Plan: blogphoto_cli — List All EXIF Info From a Photo

## 1. Context & Goal

`blogphoto_cli` is currently a `cargo new` skeleton (`Hello, world!`). The goal is a CLI tool that, given a photo path, dumps **all EXIF metadata** in human-readable (and machine-readable) form. This fits the `rust_learning` progression (Day 1 CLI → Day 3 error handling / Day 5 modular architecture) and serves as a foundation for later `blogphoto` features (strip EXIF for privacy, resize/compress, upload to R2).

**Primary use case:**
```bash
blogphoto exif photo.jpg              # pretty table
blogphoto exif photo.jpg --json       # JSON for scripting
blogphoto exif photo.jpg --tag GPS    # filter
blogphoto exif *.jpg                  # batch
```

---

## 2. Research Summary

### Crate Choice

| Crate | Import | Pros | Cons |
|-------|--------|------|------|
| **`kamadak-exif` (exif = 0.3)** | `exif` | Most mature, typed API, supports JPEG/TIFF + HEIF/HEIC/AVIF/PNG/WebP, experimental writer for later stripping | Import name != package name (confusing), writer is unstable |
| `rexif` | `rexif` | Simpler, MIT, JPEG/TIFF only | Early-stage, fewer formats, no writer |
| `rexiv2` | — | Full XMP/IPTC via libexiv2 | Requires C++ `libexiv2` system dep, heavier |

**Recommendation: `kamadak-exif` + `clap` derive.** This is the de-facto standard (verified via docs.rs / exif-rs GitHub). `rexif` is a fallback if MIT-only is required. Add `clap 4` with `derive` already used in `weather_cli`/`mastodon_cli` elsewhere in the repo — consistent with existing patterns.

**Key API sketch (`kamadak-exif`):**
```rust
let file = std::fs::File::open(path)?;
let mut bufreader = std::io::BufReader::new(&file);
let exifreader = exif::Reader::new();
let exif = exifreader.read_from_container(&mut bufreader)?;
for f in exif.fields() {
    println!("{} {}: {}", f.ifd_num, f.tag, f.display_value().with_unit(&exif));
}
```

### Dependencies to Add

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
exif = "0.3"          # kamadak-exif package
colored = "3.0"       # optional, for pretty headers (already used in weather_cli)
serde = { version = "1", features = ["derive"] }
serde_json = "1"      # for --json
```

Consider later: `chrono` for date parsing, `anyhow`/`thiserror` (Day 3 pattern) for errors.

---

## 3. Architecture

Follow Day 5 modular layout (`src/main.rs` + `src/cli.rs` + `src/exif.rs` + `src/display.rs`):

```
blogphoto_cli/
├── Cargo.toml
├── src/
│   ├── main.rs       # entry, dispatch, error handling
│   ├── cli.rs        # clap Args / Subcommands
│   ├── exif.rs       # core: read EXIF → Vec<Field> / structured ExifInfo
│   └── display.rs    # table vs JSON vs filtered output
└── tests/
    └── fixtures/     # sample JPG with known EXIF for integration tests
```

**Why subcommand?** `blogphoto` will grow (`exif`, `strip`, `resize`, `upload`). Start with `exif` subcommand now so CLI is forward-compatible. Alternative is flat `blogphoto_cli <FILE>` — simpler but needs breaking change later. Plan uses subcommand, but ask user (see §7).

---

## 4. CLI Design (clap derive)

```rust
// src/cli.rs
#[derive(Parser)]
#[command(name="blogphoto", version, about="Blog photo tooling")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all EXIF info from a photo
    Exif(ExifArgs),
}

#[derive(Args)]
struct ExifArgs {
    /// Path(s) to image file(s) — supports glob via shell
    #[arg(required=true, value_name="FILE")]
    files: Vec<PathBuf>,

    /// Output as JSON (machine-readable)
    #[arg(long)]
    json: bool,

    /// Output as raw debug (flat tag list)
    #[arg(long)]
    raw: bool,

    /// Filter to tag(s) containing this string (case-insensitive, e.g. "GPS", "Model")
    #[arg(long, short='t')]
    tag: Option<String>,

    /// Include IFD number and tag code alongside description
    #[arg(long)]
    verbose: bool,
}
```

**Help examples:**
```
$ blogphoto exif --help
$ blogphoto exif photo.jpg
$ blogphoto exif photo.jpg --json | jq
$ blogphoto exif *.jpg --tag GPS
$ blogphoto exif photo.jpg --verbose
```

**Exit codes:** 0 = success, 1 = file not found / not an image, 2 = no EXIF found (with friendly message, not crash).

---

## 5. Core Logic (`src/exif.rs`)

```rust
pub struct ExifField {
    pub ifd: String,        // "TIFF", "EXIF", "GPS", "Interop"
    pub tag: String,        // "Model", "ExposureTime"
    pub code: u16,          // raw tag id
    pub value: String,      // display_value with unit
    pub raw: String,        // debug repr for --raw
}

pub fn read_exif(path: &Path) -> Result<Vec<ExifField>, ExifError>
pub enum ExifError { Io(...), NoExif, InvalidFormat, ... } // thiserror
```

Steps:
1. `File::open` → `BufReader`
2. `exif::Reader::new().read_from_container(&mut reader)` — handles JPEG/TIFF/HEIC detection internally.
3. Map `exif.fields()` → `ExifField` (use `f.ifd_num`, `f.tag.description()`, `f.display_value().with_unit(&exif)`).
4. Special handling:
   - GPS: decode rational lat/lon to decimal + show `N/S E/W` and Google Maps link hint.
   - Dates: preserve as-is but optionally parse with `exif::DateTime`.
   - Thumbnail IFD: include but mark as `Thumbnail`.
5. If `exif.fields().next().is_none()` → `NoExif` friendly message: `"No EXIF data found in photo.jpg (image may be stripped or screenshot)"`.

Edge: file not image → `exif::Error::InvalidFormat` → map to user error.

**Batch:** loop over `files`, print header `==> photo.jpg <==` when >1 file (like `grep`).

---

## 6. Display (`src/display.rs`)

* **Default table mode:** Grouped by IFD, aligned columns. Example:

```
File: photo.jpg (2.4 MB, 4032x3024)

[TIFF — Image]
  Make             : Apple
  Model            : iPhone 15 Pro
  Orientation      : 1 (Normal)
  ...

[EXIF — Photo]
  ExposureTime     : 1/120 sec
  FNumber          : f/1.8
  ISO              : 100
  FocalLength      : 6.8 mm
  ...

[GPS]
  GPSLatitude      : 35.6812 N (35°40'52")
  GPSLongitude     : 139.7671 E
  GPSAltitude      : 12.4 m

— 38 tags total (GPS present ⚠️ privacy warning)
```

* Add color via `colored` (header cyan, warning yellow — consistent with `weather_cli`).
* **`--json`:** `serde_json::to_string_pretty(Vec<ExifField>)` or file-keyed map for batch. Good for `jq` / blog pipeline.
* **`--tag GPS`:** case-insensitive substring filter on `tag`+`ifd`+`value`.
* **`--verbose`:** show `IFD 0 [271/0x010F] Make: Apple`.
* **`--raw`:** dump `{:?}` for debugging unknown tags.

Privacy hint: if GPS tags exist, print yellow warning: `⚠️ GPS location found — consider stripping before publishing`.

---

## 7. Open Questions for User (needs decision before implementation)

1. **Subcommand vs flat:** `blogphoto exif <FILE>` (extensible) or `blogphoto <FILE>` (simplest now)? **Recommended: subcommand.**
2. **Batch:** support `*.jpg` / multiple files in one invocation? **Recommended: yes.**
3. **Privacy warning:** show GPS warning by default? **Recommended: yes.**
4. **Output default:** table is default, but should we also support `--json` and `--raw`? **Recommended: both.**
5. **HEIC/WebP support:** needed now or JPEG-only MVP? `kamadak-exif` handles it for free, so include.

---

## 8. Implementation Steps (incremental, testable)

**Phase 1 — MVP (1-2 hours)**
1. `cargo add clap --features derive` + `exif` + `serde` + `serde_json` + `colored` + `thiserror` + `anyhow`
2. Scaffold `src/cli.rs` (clap derive) and `src/exif.rs` (thin wrapper around `exif::Reader`)
3. Implement `main()` dispatch: parse args → `read_exif` → `display` → proper `Result` + `eprintln!` + `std::process::exit`
4. Manual test with 2-3 real photos (iPhone, DSLR, screenshot with no EXIF)

**Phase 2 — Polish**
5. `src/display.rs`: table grouping, color, GPS decimal conversion, privacy warning
6. Flags: `--json`, `--tag`, `--verbose`, `--raw`
7. Batch handling (`files: Vec<PathBuf>`)
8. Integration test: `tests/exif_test.rs` with fixture image (commit a small test JPG) + `assert!(read_exif(...).is_ok())`

**Phase 3 — Nice-to-have / Next PRs (out of scope for initial plan but designed for)**
9. `blogphoto strip photo.jpg --in-place` (uses `kamadak-exif` experimental writer or `rexif`+ manual strip) — privacy feature
10. `blogphoto exif --gps-decimal` / map link generation
11. Directory walk: `blogphoto exif ./photos/ --recursive`
12. Shell completions (`clap_complete`)

---

## 9. Error Handling (Day 3 pattern)

```rust
#[derive(thiserror::Error, Debug)]
enum AppError {
    #[error("File not found: {0}")]
    NotFound(PathBuf),
    #[error("Not a valid image or unsupported format: {0}")]
    InvalidFormat(PathBuf),
    #[error("No EXIF data found in {0}")]
    NoExif(PathBuf),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
```

* Never `unwrap()` on file I/O in user-facing path.
* `NoExif` is *not* an error exit 2 with helpful message, not a panic.

---

## 10. Testing

* **Unit:** `exif.rs` — mock reader, GPS rational → decimal conversion.
* **Integration:** `cargo test` with fixture `tests/fixtures/iphone_sample.jpg` (keep <500KB). Assert tag count, specific tag presence (`Model`, `DateTimeOriginal`).
* **Manual:** `cargo run -- exif <photo>`, `cargo run -- exif <photo> --json | jq`, `cargo run -- exif no_exif.png` (should print friendly message).

---

## 11. Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| `kamadak-exif` import is `exif` not `kamadak_exif` | Document in `Cargo.toml` comment, add `use exif;` |
| HEIC requires `exif` feature? Check docs | Verify `cargo run` on HEIC sample before claiming support |
| Binary size | Not a concern for CLI; `--release` already in curriculum |
| Large directories / many files | Batch is sequential; no async needed |

---

## 12. What This Plan Does NOT Do (explicit non-goals)

* No image resizing/compression (separate subcommand later)
* No R2 upload (separate)
* No EXIF writing/editing (except noting experimental writer for future `strip`)
* No GUI / TUI (CLI only, though `ratatui` pattern from `weather_cli` could be reused later)

---

## 13. Next Action

Await plan approval → then execute Phase 1 in order. Ask user to confirm Q1-Q5 above (or accept defaults) before `cargo add`.
