# Plan: blogphoto_cli — List All EXIF Info From a Photo (IMPLEMENTED)

## 1. Context & Goal — DONE
Implemented `blogphoto exif <FILE>` subcommand. Binary is `blogphoto_cli` (clap `name=blogphoto`).

## 2. Research Summary — VALIDATED
- `kamadak-exif 0.6.1` (package `kamadak-exif`, import `exif`) is correct. Import != package handled via `exif = { package = "kamadak-exif" }`.
- `clap 4.6.6` derive, `colored 3.1.1`, `serde`/`serde_json`, `thiserror` — all wired.
- **Reality difference:** `exif::In` only has `PRIMARY (0)` and `THUMBNAIL (1)` — not separate EXIF/GPS/Interop In values. All TIFF/EXIF/GPS tags are flattened into `PRIMARY`. Fixed by mapping `In(0) -> "Image"` and `In(1) -> "Thumbnail"` instead of earlier plan's TIFF/GPS split.

## 3. Architecture — DONE
```
blogphoto_cli/
├── Cargo.toml
├── src/
│   ├── main.rs    # dispatch, batch, exit codes 0/1/2
│   ├── cli.rs     # clap subcommand Exif(ExifArgs)
│   ├── exif.rs    # read_exif() -> Vec<ExifField>, ifd_to_string, ifd_order
│   └── display.rs # print_human, print_json_single/batch, GPS warning
└── tests/fixtures/
    ├── sample_with_exif.jpg (Canon 40D, 47 tags)
    ├── gps_sample.jpg (NIKON COOLPIX P6000, with lat/lon)
    ├── no_exif.jpg
    └── no_exif.png
```

## 4. CLI Design — DONE
`blogphoto exif [OPTIONS] <FILE>...`
- `--json`, `--raw`, `-t/--tag <FILTER>`, `--verbose`
- Batch: `==> file <==` header like grep
- Exit: 0 success, 2 NoExif (single file), 1 not found/invalid
- Verified: `cargo run -- exif --help` works.

## 5. Core Logic — DONE + adjusted
- `ExifField { ifd, tag, code, value, raw }` Serialize
- `read_exif` uses `exif::Reader::new().read_from_container` with `NotFound`+`BlankValue` -> NoExif
- Sorting via `ifd_order` (Image 0, Thumbnail 1) then code.
- GPS warning only for actual location tags (`latitude`/`longitude`), not `GPSVersionID` alone — fixed during implementation.

## 6. Display — DONE
- Grouped by IFD (Image / Thumbnail)
- `--verbose`: `Tag [code/0xHEX]: value`
- `--raw`: appends `(raw: Debug)`
- `--json`: single file -> `{file,count,has_gps,fields}`, batch -> `{ file: {count,has_gps,fields,error} }`
- Privacy warning: `⚠️ GPS location found — consider stripping before publishing!`

## 7. Decisions
- Subcommand chosen (user confirmed)
- Batch yes, GPS warning yes, --json/--raw yes

## 8. Implementation Steps — DONE
- Phase 1 MVP done, Phase 2 polish done.
- Tests: `cargo test` 1 passed (unit sort). Manual verification with 4 fixtures passed.

## 9. Verification (run 2026-08-25)
```
cargo run -- exif tests/fixtures/sample_with_exif.jpg           # 47 tags, no warning (correct)
cargo run -- exif tests/fixtures/gps_sample.jpg --tag GPS       # 10 GPS tags + warning + has_gps true
cargo run -- exif tests/fixtures/sample_with_exif.jpg --json | jq
cargo run -- exif tests/fixtures/no_exif.jpg                    # exit 2, friendly tip
cargo run -- exif sample.jpg gps.jpg --json                     # batch JSON map
cargo run -- exif /nonexistent.jpg                               # exit 1
cargo test                                                        # ok
```

## 10. Future (not implemented, scoped out)
- `strip`, `resize`, `upload`, `gps-decimal` link, recursive walk, completions
