# Plan: Polish 13 — Error-Handling Unification + README + Test Guards

## 1. Goal and Assumptions

**Goal:** Close polish debt left after `stat` landing. Unify not-found error handling across `delete`/`stat`/`download`, normalize user-visible messages, document `stat`, and add regression tests so `cargo test/clippy/fmt` stays green.

**Scope — 13 Only:**
- No new commands/features (copy/sync/etc out of scope)
- Touch only `src/r2.rs`, `src/cli.rs`, `README.md`, `src/main.rs` if needed
- No new dependencies
- Keep `HeadObjectError::is_not_found()` only (discovered 2026-08-10: `is_no_such_key()` is `GetObject`-only); unify via helper, not SDK hack

**Assumptions:**
- R2 `HeadObject` 404 → `is_not_found()` (verified `cargo check` error E0599 when trying `is_no_such_key`)
- `GetObject` 404 → `is_no_such_key()` (`src/r2.rs:164` currently correct)
- User messages should stay `file not found: s3://{bucket}/{key}` for `delete`/`stat` (existing) and `object not found in s3://{bucket}/{key}` for `download` is intentional divergence? Decide to align (see open question) — default: **keep as-is but document**, normalize only internal helper, not strings.
- Verbose still `eprintln!` to stderr, success to stdout.

---

## 2. Key Findings (Durable Paths)

| Area | File:Line | Finding |
|---|---|---|
| **Stat landed** | `src/cli.rs:26-38` | `Commands::Stat` with `alias=head,info` + `StatArgs { key, json }`; 4 parse tests `src/cli.rs:226-` passing |
| **Head helper** | `src/r2.rs:500-553` | `head_stat` matches `is_not_found()` only; returns `StatInfo` with `size:i64`, `metadata: HashMap`; `fetch_object_metadata` now delegates to `head_stat` |
| **Delete Head** | `src/r2.rs:77-93` | `delete` pre-flight `head_object` matches `is_not_found()` → `bail!("file not found: s3://{b}/{k}")`; other → `Context("head_object failed — ...")` |
| **Download Get** | `src/r2.rs:160-173` | `download` matches `is_no_such_key()` → `bail!("object not found in s3://{b}/{k}")`; other → `Context("get_object failed — ...")` |
| **Inconsistency** | `src/r2.rs:86` vs `164` | Different methods + different messages; future `copy` will need both. No shared helper. |
| **Repeated pattern** | `src/r2.rs:22-33, 103-113, 125-135, 143-152` | `endpoint_for` + verbose `eprintln!` + `build_client` repeated 4×; candidate for `fn verbose_endpoint(...)` helper but out of scope for 13 unless trivial. |
| **README gap** | `README.md:24-68` | Documents `upload/list/download/delete` but no `### Stat`; env table still accurate, no `stat` example. |
| **Tests** | `src/r2.rs:690+`, `src/cli.rs:126+` | 23 tests pass; no negative-path unit test for `run_stat` empty key beyond manual `cargo run -- stat "   "`; `cargo test` already covers `escape_json`, `format_stat_*` |
| **Fmt/Clippy** | — | Last run `cargo fmt` + `cargo clippy -- -D warnings` clean (2026-08-10 17:45) |

---

## 3. Proposed Implementation Steps

### Step 1 — Helper: Unified Not-Found Check `src/r2.rs`
Add small private helper **without changing SDK traits**:
```rust
fn is_not_found_head(err: &aws_sdk_s3::operation::head_object::HeadObjectError) -> bool {
    err.is_not_found()
}
fn is_not_found_get(err: &aws_sdk_s3::operation::get_object::GetObjectError) -> bool {
    err.is_no_such_key()
}
```
Or single generic using `as`? Keep two trivial fns for clarity. Use in `delete`/`head_stat` vs `download`. No behavior change, just docs + future-proofing for `copy`.

Alternative minimal: just add comment `// HeadObject 404 is is_not_found(); GetObject 404 is is_no_such_key(); see src/r2.rs:512 fix` and leave code as-is. **Recommend comment-only** to avoid dead-code warnings.

**Action:** Add `// Note: HeadObjectError only has is_not_found(); GetObjectError uses is_no_such_key() — intentional.` above each match.

### Step 2 — Normalize Messages (Optional, Low-Risk)
Decide: Keep `download` message distinct? Currently:
- `delete`/`stat`: `file not found: s3://{b}/{k}` (`src/r2.rs:87`, `512`)
- `download`: `object not found in s3://{b}/{k}` (`src/r2.rs:165`)

**Proposal for 13:** **Do not change strings** — they are intentional (file vs object). Just ensure `stat` matches `delete` (already does). Add comment noting divergence. If user wants unify later, single-line change.

### Step 3 — README `README.md` Update
Add after `### Delete` section (before `### Global Options`):
```md
### Stat

# Human-readable
cloudflare_r2 stat test/README.md --bucket $R2_BUCKET --verbose
# JSON for scripting
cloudflare_r2 stat images/photo.jpg --bucket $R2_BUCKET --json | jq .size
# Aliases
cloudflare_r2 head images/photo.jpg --bucket $R2_BUCKET
cloudflare_r2 info images/photo.jpg --bucket $R2_BUCKET
```
Mirror `### Delete` error handling bullet: add `- Missing object (stat) → Error: file not found: s3://<bucket>/<key> exit 1`.

Update Features bullet list top to include `stat` subcommand.

### Step 4 — Test Guards `src/r2.rs` + `src/cli.rs`
- `cli::tests::rejects_stat_missing_key` already covered by clap required arg; add explicit test for `stat` without KEY → expects clap error contains `required` (mirrors `rejects_missing_subcommand`).
- `r2::tests::head_stat_empty_key_bails`? Can't unit-test async without mock client; instead add **format-only** guard: `formats_stat_human_missing_fields` where `last_modified=None`, `etag=None` → expects `-` placeholders (already partially in `formats_stat_human_contains_labels` but add case for missing).
- Ensure `cargo test` still 23→25 passing after docs.

No async mock needed; keep unit-level.

### Step 5 — Optional Polish (if <15 min)
- Extract `fn log_endpoint_verbose(verbose: bool, endpoint: &str, bucket: &str, extra: &str)` to DRY 4 call sites. **Defer unless requested** — not in 13 minimal slice to avoid churn.
- Ensure `Cargo.toml` `about` still correct (already updated in cli.rs to include stat).

**Files touched:** `src/r2.rs` (comments ±2 lines), `README.md` (~15 lines), `src/r2.rs`/`src/cli.rs` tests (+2 tests). No `Cargo.toml`.

---

## 4. Verification Plan

**Static (must all pass):**
```bash
cargo check
cargo clippy -- -D warnings
cargo fmt --check
cargo test
```

**Expected:** 25 tests (23 existing +2 new); zero warnings.

**Manual:**
```bash
cargo run -- --help | grep -q stat && echo ok
cargo run -- stat --help | grep -q "\-\-json"
cargo run -- head images/photo.jpg --bucket $R2_BUCKET --help | grep -q "stat"
cargo run -- stat "   " --bucket test --access-key ak --secret-key sk --account-id id 2>&1 | grep -q "object key must not be empty"
cargo run -- stat no/such --bucket test --access-key ak --secret-key sk --account-id id 2>&1 | grep -q "head_object failed"
# README check
grep -q "### Stat" README.md && echo readme_ok
cargo run -- list --help | grep -q "long"
```

**Error matrix (no change in exit codes):**
- Empty trimmed key → `bail!("object key must not be empty")` exit 1 (already)
- Missing object stat → `file not found: s3://...` exit 1 (already, via head_stat)
- Missing object download → `object not found in s3://...` exit 1 (unchanged)
- Clap missing KEY → exit 2

---

## 5. Risks, Open Questions, Rejected Alternatives

**Risks:**
| Risk | Mitigation |
|---|---|
| Touching error strings breaks scripts parsing stderr | **Do not change strings** in 13; only comments |
| Adding verbose helper refactors 4 call sites → merge conflict with future copy work | Defer DRY helper; comment-only for 13 |
| README drift from CLI `--help` | Copy exact flag `--json` and alias list from `src/cli.rs:32-38`; single source |

**Open Questions (non-blocking):**
- Unify `download` message to `file not found`? Keep distinct for now; decide later if users report confusion.
- Need `stat --verbose` already logs `Endpoint/Bucket/Key` (`src/r2.rs:149`) — keep as is? Yes.

**Rejected Alternatives:**
| Alt | Why |
|---|---|
| Add `serde_json` for README JSON example validation | Out of scope; manual `escape_json` already tested; adding dep for docs not needed |
| Generic `is_not_found(err: &dyn Any)` helper | Rust generics over SDK error enums messy; two tiny fns or comments clearer |
| Refactor `endpoint_for+build_client` into `fn client_for(...)` now | Good idea but larger churn; defer to `copy`/`bulk delete` work where reuse proven |
| Change `stat` to also accept `key.ends_with('/')` rejection like `download` | Not needed; `stat` on prefix-like key with trailing slash is valid S3 key (unlikely but allowed); keep only empty check |
