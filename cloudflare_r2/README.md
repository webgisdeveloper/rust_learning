# Cloudflare R2 CLI

A Rust command line tool to upload, list and download files on Cloudflare R2 (S3-compatible API). Uses `aws-sdk-s3` with `aws-config` (Cloudflare's recommended Rust path) and streams via `ByteStream::from_path`.

## Features

- `upload` subcommand: Upload a local file to R2 (supports optional description metadata).
- `list` subcommand: Enumerate objects in a bucket (supports pagination, prefixes, and detailed output).
- `download` subcommand: Stream an object to a local file without loading it entirely into memory.
- Credentials via env vars or flags (flags override env).
- Endpoint auto-derived from `R2_ACCOUNT_ID` → `https://{account_id}.r2.cloudflarestorage.com` or explicit `R2_ENDPOINT`.
- Content-Type auto-detected via `mime_guess` for uploads, overridable with `--content-type`.
- Optional file description stored in metadata via `-d` / `--description` flag (`x-amz-meta-description`).
- Automatically attaches computer hostname in object metadata under key `host` (`x-amz-meta-host`).
- Streaming upload (no full file buffering), `--verbose` logging, proper exit codes.
- `.env` support via `dotenvy`.

## Prerequisites

Create an R2 bucket and API token: Cloudflare dashboard → R2 → Manage R2 API Tokens. You'll need:

- `R2_ACCOUNT_ID` — Cloudflare account ID
- `R2_ACCESS_KEY_ID` / `R2_SECRET_ACCESS_KEY` — token credentials
- `R2_BUCKET` — bucket name

See https://developers.cloudflare.com/r2/api/tokens/ and https://developers.cloudflare.com/r2/examples/aws/aws-sdk-rust/

## Install

```bash
cargo build --release
# binary at target/release/cloudflare_r2
cargo install --path .
```

## Usage

### Upload

```bash
# Upload using explicit flags
cloudflare_r2 upload ./photo.jpg --bucket my-bucket --key images/photo.jpg \
  --account-id $R2_ACCOUNT_ID --access-key $R2_ACCESS_KEY_ID --secret-key $R2_SECRET_ACCESS_KEY

# Upload with optional description metadata (-d / --description)
cloudflare_r2 upload ./photo.jpg --bucket my-bucket -d "Vacation photo"

# Upload using env vars (recommended)
export R2_ACCOUNT_ID=abc123
export R2_ACCESS_KEY_ID=...
export R2_SECRET_ACCESS_KEY=...
export R2_BUCKET=sdk-example

cloudflare_r2 upload ./README.md --verbose
# key defaults to basename: README.md
cloudflare_r2 upload ./README.md --key test/README.md --description "Project README file" --verbose
```

### List

```bash
# List all objects in bucket
cloudflare_r2 list --bucket $R2_BUCKET

# List with prefix (e.g. images folder)
cloudflare_r2 list --bucket $R2_BUCKET --prefix images/

# List with detailed output (size, last modified, host metadata)
cloudflare_r2 list --bucket $R2_BUCKET --long

# Combine prefix and detailed output
cloudflare_r2 list --bucket $R2_BUCKET --prefix images/ --long --verbose
```

### Download

```bash
# Download to the filename from the object key: ./README.md
cloudflare_r2 download test/README.md --bucket $R2_BUCKET

# Download to a specific path, creating parent directories when needed
cloudflare_r2 download images/photo.jpg --bucket $R2_BUCKET --output ./downloads/photo.jpg

# Overwrite an existing local file
cloudflare_r2 download images/photo.jpg --bucket $R2_BUCKET --output ./photo.jpg --force --verbose
```

Without `--output`, the destination uses the filename portion of the object key. Existing files are protected by default; use `--force` to overwrite them.

### Global Options

- `-v, --verbose`: Enable verbose logging of endpoints, buckets, and counts.

### Env Var Table

| Flag | Env | Required | Description |
|------|-----|----------|-------------|
| `--bucket` | `R2_BUCKET` | yes | R2 bucket name |
| `--access-key` | `R2_ACCESS_KEY_ID` | yes | R2 API token Access Key ID |
| `--secret-key` | `R2_SECRET_ACCESS_KEY` | yes | R2 API token Secret Access Key |
| `--account-id` | `R2_ACCOUNT_ID` | yes *or* `--endpoint` | Cloudflare Account ID |
| `--endpoint` | `R2_ENDPOINT` | yes *or* `--account-id` | Full R2 endpoint URL |

`--endpoint` takes precedence over `--account-id`.

## Verification

```bash
cargo check
cargo clippy -- -D warnings
cargo fmt --check
cargo test

# Real R2 test: upload, list, then download
cargo run -- upload ./README.md --bucket $R2_BUCKET --key test/README.md --verbose
cargo run -- list --bucket $R2_BUCKET --prefix test/ --long --verbose
cargo run -- download test/README.md --bucket $R2_BUCKET --output /tmp/README.out --force --verbose
diff README.md /tmp/README.out
```

## Error Handling

- Missing file (upload) → `Error: file not found: ...` exit 1
- Directory (upload) → `Error: not a file: ... (directories not supported)` exit 1
- Missing object (download) → `Error: object not found in s3://<bucket>/<key>` exit 1
- No subcommand → help exit 2
- Missing required args → clap error exit 2
- Bad creds / wrong endpoint / network → `... failed — check bucket, credentials, endpoint and network` with SDK cause, exit 1
- Success → `Uploaded ...` or object list, exit 0
