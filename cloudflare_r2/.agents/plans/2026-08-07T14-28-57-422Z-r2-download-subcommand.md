# Plan: Add `download` Subcommand to R2 CLI

## Goal
Add `download` to existing `upload` + `list` CLI so learner can pull files from R2 back to local filesystem. Keep shared R2 auth (R2Args) and module split (cli.rs / r2.rs / main.rs).

## Current State
- `src/cli.rs`: `Cli` with `Upload` + `List`, `R2Args` flattened. Tests for upload/list pass.
- `src/r2.rs`: `run_upload`, `run_list`, `derive_endpoint`, `build_client`, `upload`, `list_objects`, pagination fix, verbose handling.
- `src/main.rs`: dispatches to r2.
- `Cargo.toml`: clap[derive,env], tokio[full], aws-sdk-s3, aws-config[behavior-version-latest], aws-smithy-types, anyhow, mime_guess, dotenvy.
- All 7 tests pass, clippy clean.

## Proposed Design

### CLI (src/cli.rs)
```rust
#[derive(Subcommand)]
enum Commands {
    Upload(UploadArgs),
    List(ListArgs),
    Download(DownloadArgs), // NEW
}

#[derive(Args)]
struct DownloadArgs {
    #[command(flatten)] r2: R2Args,
    /// Object key in R2 to download (e.g. "images/photo.jpg")
    #[arg(value_name="KEY")]
    key: String,
    /// Local destination file path (defaults to filename from key)
    #[arg(short, long, value_name="FILE")]
    output: Option<PathBuf>,
    /// Overwrite destination if exists
    #[arg(long)]
    force: bool,
}
```
Invocation:
```bash
cloudflare_r2 download <KEY> --bucket <BUCKET> [--output <FILE>] [--force]
# env fallback same as others: R2_BUCKET, R2_ACCOUNT_ID/R2_ENDPOINT, R2_ACCESS_KEY_ID, R2_SECRET_ACCESS_KEY
# examples:
cloudflare_r2 download images/photo.jpg --bucket my-bucket --output ./local.jpg
cloudflare_r2 download test/README.md --bucket my-bucket          # -> ./README.md
R2_BUCKET=my-bucket cloudflare_r2 download my-key --force --verbose
```

### R2 Logic (src/r2.rs)
- Add `pub async fn run_download(args: DownloadArgs, verbose: bool) -> anyhow::Result<()>`
  - Validate `key` non-empty after trim, reject keys ending with "/" or empty filename.
  - Derive `endpoint_for(&args.r2)`, build client via existing `build_client`.
  - Derive output path: `args.output` else `Path::new(&args.key).file_name()` -> `PathBuf::from(filename)`; bail if filename empty.
  - If destination exists and !force -> bail with hint to use --force.
  - Ensure parent dirs: `tokio::fs::create_dir_all(parent).await` if parent Some.
  - Call `download(&client, &args.r2.bucket, &args.key, &output, verbose).await`

- Add `async fn download(client, bucket, key, output: &Path, verbose: bool)`
  ```rust
  // verbose: eprintln!("Downloading s3://{bucket}/{key} -> {}", output.display());
  let resp = client.get_object().bucket(bucket).key(key).send().await
      .context("get_object failed — check bucket/key, credentials, endpoint and network")?;
  // Use streaming to avoid loading large files into memory:
  let mut body = resp.body.into_async_read();
  let mut file = tokio::fs::File::create(output).await
      .with_context(|| format!("failed to create {}", output.display()))?;
  tokio::io::copy(&mut body, &mut file).await
      .context("failed to write destination file")?;
  if verbose {
      let len = tokio::fs::metadata(output).await.map(|m| m.len()).unwrap_or(0);
      eprintln!("Downloaded s3://{bucket}/{key} -> {} ({len} bytes)", output.display());
  }
  println!("Downloaded s3://{bucket}/{key} to {}", output.display());
  ```
  Alternative fallback for small files: `collect().await` + `write_all`, but streaming is preferred for learner to see async I/O.

- Reuse `endpoint_for`, `build_client`, `derive_endpoint` (already trims). Add helper `derive_output_path(key, output) -> PathBuf` for testability.

- Add unit tests for derive_output_path, cli parsing for download.

### main.rs
- Extend match:
```rust
Commands::Download(args) => r2::run_download(args, cli.verbose).await,
```
- Update about to "Upload, list and download files on Cloudflare R2"

### Cargo.toml
- No new deps needed (tokio already has full -> includes fs, io). Ensure `tokio-util` not required; `aws-sdk-s3` ByteStream already provides `into_async_read`.

### README.md
- Add Download section with examples, env table unchanged (same R2 vars), note --output defaults, --force.

## File Changes
- `src/cli.rs`: add DownloadArgs, new Commands variant, update about, add download tests.
- `src/r2.rs`: add run_download, download, derive_output_path, tests.
- `src/main.rs`: handle Download dispatch, update about string.
- `README.md`: document download.

## Verification
```bash
cargo fmt --check
cargo check
cargo test            # expect 9 -> 11 tests (adds 2 download tests)
cargo clippy -- -D warnings
cargo run -- --help          # shows upload, list, download
cargo run -- download --help
cargo run -- upload --help
cargo run -- list --help
# error paths (no network):
cargo run -- download missing-key --bucket b --access-key ak --secret-key sk --account-id id  # -> get_object failed
cargo run -- download my-key --bucket b --access-key ak --secret-key sk --account-id id --output /tmp/out --verbose
# with real R2 (requires creds):
cargo run -- upload ./README.md --bucket $R2_BUCKET --key test/README.md
cargo run -- download test/README.md --bucket $R2_BUCKET --output /tmp/README.out --force --verbose
diff README.md /tmp/README.out
cargo run -- list --bucket $R2_BUCKET --prefix test/
```

## Risks / Notes
- Large file streaming: test with a few MB file; ensure `into_async_read` + `copy` works on current aws-sdk-s3 1.x.
- Parent dir creation: must not fail if output is just filename (no parent).
- Overwrite safety: without --force, bail if destination exists to avoid silent data loss.
- Key with path traversal: derive_output_path should just use file_name, not full key, to avoid writing to arbitrary dirs when output not specified.
- Pagination not needed for download (single object).

## Execution Order
1. Edit cli.rs
2. Edit r2.rs (run_download, download, helper)
3. Edit main.rs
4. cargo fmt/check/test/clippy
5. help checks
6. Update README.md
7. Commit
