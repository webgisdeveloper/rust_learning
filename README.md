# Rust Learning & CLI Tools

This repository contains educational materials, sample projects, and standalone command-line applications built while learning Rust—ranging from language fundamentals and Actix Web backend services to full-featured CLI tools, fractal renderers, and runtime integrations.

---

## 10-Day Rust & Actix Web Tutorial

This 10-day tutorial guides users from Python to high-performance web development with Rust and Actix Web:

- **[Day 1](Day1.md): Mental Model & Syntax** – AOT compilation, static typing, immutability, and pattern matching.
- **[Day 2](Day2.md): Ownership & Borrowing** – Memory safety via ownership laws, moving, and borrowing rules.
- **[Day 3](Day3.md): Data Modeling & Errors** – Structs, Enums (ADTs), and replacing exceptions with `Option` and `Result`.
- **[Day 4](Day4.md): Traits & Generics** – Interfaces and polymorphism, specifically the `FromRequest` trait for extractors.
- **[Day 5](Day5.md): Cargo & Architecture** – Project management with Cargo and modular code organization (`mod`/`pub`).
- **[Day 6](Day6.md): Async & Actix Basics** – Async/await, the Tokio runtime, and launching a multi-threaded HTTP server.
- **[Day 7](Day7.md): Routing & Extractors** – Type-safe request handling using `web::Path`, `web::Query`, `web::Json`, and Serde.
- **[Day 8](Day8.md): Database & State** – Compile-time SQL validation with SQLx and shared state via `web::Data`.
- **[Day 9](Day9.md): Middleware & Auth** – Request pipelines, `Logger` middleware, and header-based authentication guards.
- **[Day 10](Day10.md): Testing & Production** – In-memory integration tests, `--release` optimizations, and multi-stage Docker builds.

---

## Command-Line Tools & Applications

This repository features several standalone command-line tools demonstrating various Rust libraries (e.g., `clap`, `tokio`, `reqwest`, `aws-sdk-s3`, `rmcp`, `rayon`, `image`, `jlrs`):

| Tool | Directory | Documentation | Description |
| :--- | :--- | :--- | :--- |
| **`blogphoto_cli`** | [`blogphoto_cli/`](blogphoto_cli/) | [README](blogphoto_cli/README.md) | Inspect EXIF metadata and audit image folders for privacy-sensitive data (GPS, camera model) |
| **`checkIP_cli`** | [`checkIP_cli/`](checkIP_cli/) | [README](checkIP_cli/README.md) | Network diagnostic utility printing hostname, LAN IP, and public IP address |
| **`cloudflare_r2`** | [`cloudflare_r2/`](cloudflare_r2/) | [README](cloudflare_r2/README.md) | S3-compatible CLI client for Cloudflare R2 object storage (upload, download, list, stat, presign, delete) |
| **`mastodon_cli`** | [`mastodon_cli/`](mastodon_cli/) | [README](mastodon_cli/README.md) | Mastodon CLI for posting statuses with attachments, browsing timelines, emoji shortcodes, and spell checks |
| **`myjira`** | [`myjira/`](myjira/) | [README](myjira/README.md) | Jira Cloud CLI and Model Context Protocol (MCP) server for querying issues and managing comments |
| **`weather_cli`** | [`weather_cli/`](weather_cli/) | [README](weather_cli/README.md) | Real-time weather reporting with geocoding, multi-city matching, and ASCII world maps |
| **`triple_dragon`** | [`triple_dragon/`](triple_dragon/) | [README](triple_dragon/README.md) | High-performance multi-threaded fractal generator rendering high-resolution PNGs of the Triple Dragon dynamical system |
| **`rust_julia_3d`** | [`rust_julia_3d/`](rust_julia_3d/) | [README](rust_julia_3d/README.md) | Scientific 3D surface visualizer embedding the Julia runtime via `jlrs` with `GLMakie` |

---

### Tool Details & Usage Examples

You can run any tool from the workspace root with `cargo run -p <package_name> -- <arguments>`.

#### 1. [blogphoto_cli](blogphoto_cli/)
Blog photo auditing tool built with `clap` and `kamadak-exif`.
- **`exif`**: Dump and filter EXIF tags from one or more photos (table view, raw debug, or JSON).
- **`check`**: Audit folders for privacy concerns prior to web publication (flags GPS coordinates and hardware serials).
```bash
cargo run -p blogphoto_cli -- exif ./photo.jpg --tag GPS
cargo run -p blogphoto_cli -- check ./photos_folder -r --strict
```

#### 2. [checkIP_cli](checkIP_cli/)
Quick network interface query tool.
- Resolves system hostname.
- Detects outbound local IP via UDP socket probe (non-transmitting).
- Queries public-facing IP via the `ipify` API using `reqwest`.
```bash
cargo run -p checkIP_cli
```

#### 3. [cloudflare_r2](cloudflare_r2/)
Fast CLI for Cloudflare R2 object storage using `aws-sdk-s3` and `ByteStream::from_path`.
- **`upload`**: Stream files with auto-detected MIME types, custom metadata descriptions (`-d`), and host tagging.
- **`list`**: Enumerate bucket objects with prefix filtering and `--long` metadata display.
- **`download`**: Stream objects directly to local files.
- **`stat`**: Head object query for metadata, file size, and timestamps without downloading.
- **`presign`**: Generate SigV4 presigned URLs for GET/PUT.
- **`delete`**: Delete objects from a bucket.
```bash
cargo run -p cloudflare_r2 -- list --bucket my-bucket
cargo run -p cloudflare_r2 -- upload ./file.png --bucket my-bucket -d "Sample image"
cargo run -p cloudflare_r2 -- presign download my-key --bucket my-bucket --expires 3600
```

#### 4. [mastodon_cli](mastodon_cli/)
Mastodon client supporting posts, media uploads, and timeline viewing.
- Post status updates (`-m` / `--message`) with optional image attachments (`-i` / `--image`).
- Fetch recent timeline statuses (`-l` / `--list`).
- Automatic shortcode replacement (e.g., `:rocket:` → 🚀) via the `emojis` crate.
- HTML tag stripping and entity decoding for clean CLI output.
- US English spell check with custom dictionary support (`--custom-dict`).
```bash
cargo run -p mastodon_cli -- -m "Hello from Rust! :wave:"
cargo run -p mastodon_cli -- --list 5
```

#### 5. [myjira](myjira/)
Jira Cloud integration serving as both a terminal client and a Model Context Protocol (MCP) server.
- List assigned tickets or search using JQL expressions (`--jql`).
- Inspect issue details and discussion comments (`--key`).
- Post comments directly to Jira issues (`--comment`).
- **MCP Server Mode (`--mcp`)**: Exposes `list_jira_issues`, `get_jira_issue`, and `add_jira_comment` tools over stdio for AI agent environments using `rmcp`.
```bash
cargo run -p myjira -- --jql "assignee = currentUser() ORDER BY updated DESC"
cargo run -p myjira -- --key PROJ-123 --comment "Investigating root cause."
cargo run -p myjira -- --mcp
```

#### 6. [weather_cli](weather_cli/)
Terminal weather utility powered by OpenWeatherMap and `ip-api.com`.
- Lookup current weather by city, state, country, ZIP code, or latitude/longitude.
- Automatic location detection via IP address (`--auto`).
- Disambiguation tools (`--list`, `--all`) for identical city names.
- Optional 80×40 ASCII world map (`--map`) highlighting geographic position with `★`.
```bash
cargo run -p weather_cli -- "Tokyo" --country JP
cargo run -p weather_cli -- auto --map
```

#### 7. [triple_dragon](triple_dragon/)
High-performance fractal rendering engine.
- Implements the chaotic rational map recurrence relation:
  $$z_{n+1} = \frac{z_n^3}{z_n^3 + 1} + c$$
- Parallel multi-threaded pixel generation using `rayon`.
- Exports native high-resolution PNG images in both Classic Monochrome and Pastel Coral Sky palettes.
```bash
cargo run -p triple_dragon --release
```

#### 8. [rust_julia_3d](rust_julia_3d/)
Interoperability demonstration embedding the Julia runtime inside Rust via `jlrs`.
- Embeds Julia runtime into Rust and dynamically manages the `GLMakie` package via Julia's `Pkg`.
- Generates 3D surface plot data ($f(x, y) = \sin(\sqrt{x^2 + y^2})$) and displays an interactive 3D window.
```bash
cargo run -p rust_julia_3d
```
