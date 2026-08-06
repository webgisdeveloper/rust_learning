# Plan: mastodon_cli Hardening & Branch Reconciliation

## Goal
Harden `mastodon_cli` (currently on `main` at `0ecbc3f`) for a clean next release by fixing performance/correctness gaps and explicitly reconciling the divergence with `origin/copilot/replace-shortcode-with-real-emoji`, so the workspace has a single, well-tested implementation. The user invoked `/plan` with no explicit scope — this plan infers the goal from repo evidence and pauses for scope confirmation before execution.

## Success Criteria
- `cargo check` and `cargo test` pass on the hardened `main`.
- Emoji replacement retains comprehensive Unicode coverage (AGENTS.md contract: `emojis::get_by_shortcode` + single-pass regex) without per-call `Regex::new().unwrap()` recompilation.
- `clean_html` has unit-test coverage (tag stripping + entity decoding via `html-escape`).
- API endpoint is not hardcoded in 4+ places; `--instance`/`MASTODON_INSTANCE` or equivalent makes `https://mastodon.social` overridable without breaking default behavior.
- Token resolution priority (`--token` > `MASTODON_TOKEN`) remains intact per `AGENTS.md`.
- Divergence decision with the `copilot/replace-shortcode-with-real-emoji` branch is documented (keep vs. drop constant-map approach) and no useful commit is lost.
- No untracked auto-save artifacts (`src/#main.rs#`, `src/.#main.rs`) remain in the working tree.

## Context And Current Facts
- **Workspace root:** `/home/cicuser/Learning/rust_learning/mastodon_cli` — single-crate binary, edition 2024, Rust toolchain required. Verification gates per [AGENTS.md](/home/cicuser/Learning/rust_learning/mastodon_cli/AGENTS.md): `cargo check` / `cargo test` / `cargo build --release`. No `.github/`, no `Makefile`, no CI config observed.
- **Current implementation** ([src/main.rs](/home/cicuser/Learning/rust_learning/mastodon_cli/src/main.rs:1)): 
  - `Args { message: Option<String>, image: Option<String>, token: Option<String> }` — posting when `--message` present, otherwise fetch-and-print 5 recent statuses (verify_credentials → accounts/:id/statuses). 
  - `replace_emojis` at line 32: `Regex::new(r":([a-z0-9_]+):").unwrap()` per call + `emojis::get_by_shortcode`. `clean_html` at line 47: `Regex::new(r"<[^>]*>").unwrap()` per call + `html_escape::decode_html_entities`. Matches AGENTS.md "single-pass regex" and "comprehensive Unicode support".
  - `upload_media` hardcodes `https://mastodon.social/api/v1/media`, filename `image.jpg`. Posting/fetching also hardcode `https://mastodon.social`. 
  - `Status { content, media_attachments: Vec<MediaAttachment> }` where `MediaAttachment` is empty struct — placeholder; fetch prints ` 🖼️` indicator.
  - Tests: 3 unit tests for `replace_emojis` only (apple, absent, rocket+tada) — all passing as of `cargo test` 2026-08-06 run.
- **Copilot branch** (`origin/copilot/replace-shortcode-with-real-emoji`, 5 commits ahead of initial plan): simplifies to `Args { message: String (required), token }`, no image/fetch, constant `SHORTCODE_MAPPINGS: &[(&str,&str)]` (25 entries) with iterative `.replace()` in `replace_emoji_shortcodes`. Diff vs `main`: `-1126 +74` lines after excluding deleted `cloudflare_r2`/`triple_dragon` directories. Represents an earlier, narrower feature slice; `main` has since added fetching, image upload, HTML cleaning, and `emojis` crate.
- **Git state:** `main` clean except untracked `src/#main.rs#`, `src/.#main.rs` (Emacs auto-save) and `../decoded_reddit.txt` outside workspace. `cargo check` → `Finished dev profile` (0.13s). `cargo test` → 3 passed.
- **Dependencies** ([Cargo.toml](/home/cicuser/Learning/rust_learning/mastodon_cli/Cargo.toml)): `clap 4/derive`, `reqwest 0.12/json+multipart`, `tokio 1/full`, `serde 1/derive`, `regex 1`, `html-escape 0.2`, `emojis 0.9`.
- **Scopes** per AGENTS.md: `read:accounts` + `read:statuses` for fetch, `write:statuses` for post — not enforced in code, just docs.

## Constraints And Non-goals
- **Constraints:** Must remain a beginner-friendly learning example (AGENTS.md, README overview comments). Keep `clap`/`reqwest`/`tokio`/`serde` stack. Token priority unchanged. Default endpoint stays `https://mastodon.social` for backward compat.
- **Non-goals (unless user expands scope):**
  - No new external services (Cloudflare R2 / triple_dragon are separate deleted crates on copilot branch — out of scope).
  - No publishing to crates.io, no CI pipeline setup, no secrets rotation.
  - No async runtime migration (stay on `tokio` `full`).
  - No broad refactor to library+binary split — single `src/main.rs` is intentional for learning.

## Key Decisions
| Decision | Recommendation | Alternatives Rejected | Why |
|---|---|---|---|
| Emoji strategy | Keep `emojis` crate + single-pass regex per AGENTS.md; cache `Regex` via `OnceLock` | Constant map (copilot branch, 25 entries, iterative `.replace()`) | Crate gives comprehensive coverage, single pass is O(n) vs O(n*m) for m mappings; constant map is incomplete and slower at scale. |
| Regex caching | `std::sync::OnceLock<Regex>` (stdlib, no new dep) | `lazy_static` / `once_cell` crate | Avoids new dep for learning example; `OnceLock` stable since 1.70. |
| Endpoint configurability | Add `--instance`/`--base-url` flag + `MASTODON_INSTANCE` env, default `https://mastodon.social`; construct URLs via helper `fn api_url(base, path)` | Hardcoded strings (current) | Needed for testing against other instances; minimal surface, preserves default. |
| Error handling | Replace `unwrap()` on `Regex::new` with `OnceLock` init; keep `Box<dyn Error>` + `process::exit(1)` for CLI (idiomatic for binary) rather than introducing `anyhow`/`thiserror` | Introduce `anyhow` | Keeps dep count low; `anyhow` adds little for this size. Could reconsider if user wants richer errors. |
| MediaAttachment placeholder | Populate minimal fields (`id`, `type`, `url`, `preview_url`) or `#[serde(deny_unknown_fields = false)]` with real fields | Keep empty struct | Empty struct is confusing for learners and fragile if API adds fields (currently works because unknown fields ignored, but opaque). |
| Branch reconciliation | Document rejection of copilot branch's simplified model; cherry-pick nothing (its tests are subset of main's). Archive branch after plan approval | Merge copilot branch | Merge would regress features (drops fetch, image, html cleaning, emojis crate). |

## Recommended Approach
Single-phase, low-risk hardening on `main` without new crates. Keep the learning-example comments and workflow overview header. Changes are additive and backward-compatible:

1. Cache both regexes with `OnceLock` and add targeted unit tests (`clean_html`, emoji edge cases).
2. Introduce configurable base URL (flag + env, centralize URL construction) while retaining default.
3. Tighten `MediaAttachment`/`Account`/`Status` deserialization and improve `upload_media` filename/MIME handling.
4. Clean working tree (remove Emacs auto-save files, add `.gitignore` entries if missing).
5. Explicitly close the copilot branch divergence with a note in README/AGENTS or commit message.

No new abstraction layers; each work unit maps to one file (`src/main.rs` + `Cargo.toml` if flag added + `README.md`).

## Work Plan
**Unit 1 — Regex performance & correctness (src/main.rs)**
- Replace per-call `Regex::new(...).unwrap()` in `replace_emojis` and `clean_html` with `static OnceLock<Regex>` (or `LazyLock` if edition allows). Ensure thread-safe init.
- Add tests: `clean_html` strips tags, decodes `&amp;`/`&gt;`/`&quot;`, handles nested tags; `replace_emojis` unknown shortcode unchanged, adjacent shortcodes, empty string.
- Files: `src/main.rs`.

**Unit 2 — Configurable instance URL (src/main.rs, Cargo.toml if needed)**
- Add `#[arg(long, env = "MASTODON_INSTANCE", default_value = "https://mastodon.social")] instance: String` to `Args` (or `--base-url`). Validate trailing slash handling.
- Introduce `fn api_url(base: &str, path: &str) -> String` helper; replace 4 hardcoded `https://mastodon.social` strings (media, statuses, verify_credentials, statuses fetch).
- Update README usage to document new flag/env.
- Files: `src/main.rs`, `README.md`.

**Unit 3 — Media & model hardening (src/main.rs)**
- `upload_media`: derive filename from `file_path` (`Path::file_name`) instead of hardcoded `image.jpg`; optionally infer MIME via extension (keep simple match to avoid new dep).
- Flesh out `MediaAttachment { id, r#type, url, preview_url }` with `#[serde(default)]` or keep empty but add comment explaining `deny_unknown` behavior; add `Account { id, username? }` as needed.
- Files: `src/main.rs`.

**Unit 4 — Working-tree hygiene (repo root)**
- Delete `src/#main.rs#`, `src/.#main.rs` (or gitignore them); ensure `.gitignore` covers `#*#`, `.#*`, `target/`, `decoded_reddit.txt` if it should not be tracked.
- Files: `.gitignore` (if missing rule), deletions.

**Unit 5 — Branch reconciliation & docs (git, README.md/AGENTS.md)**
- Write decision record: why `emojis` crate + regex approach retained over constant map (coverage, performance, AGENTS.md contract). 
- Delete or archive `origin/copilot/replace-shortcode-with-real-emoji` after approval (or keep with note).
- Files: `README.md` or `AGENTS.md` comment, git branch ops.

Dependencies: Unit 1 independent; Unit 2 after 1 (touches same file but no logical dep — can parallelize); Unit 3 after 2 (uses `api_url`); Units 4–5 last, no code dep.

## Validation Plan
| Unit | Command / Check | Expected Evidence |
|---|---|---|
| 1 | `cargo test` | New tests pass (≥6 total); existing 3 still pass. Manual: `cargo test -- --nocapture` shows no `unwrap` panic path. |
| 1 | `cargo check` | No warnings about `OnceLock` usage. |
| 2 | `cargo run -- --help` | Shows `--instance` flag; `MASTODON_INSTANCE=https://example.social cargo run` hits that host (dry-run with invalid token returns auth error from that host, not mastodon.social). |
| 2 | `cargo run -- --message "hi" --token dummy --instance https://example.invalid` | URL construction verified via error message containing `example.invalid`. |
| 3 | `cargo test` + `cargo check` | Deserialization of sample `Status` JSON with media attachments succeeds. |
| All | `cargo build --release` | Release binary builds. |
| Hygiene | `git status` | No untracked `#main.rs#` / `.#main.rs`. |
| E2E (manual, requires token) | `cargo run -- --message "test :rocket:"` and `cargo run` (fetch) | Post shows 🚀; fetch prints 5 cleaned statuses without HTML tags. Scope errors surface clearly if token lacks `read:accounts`. |

Highest-risk validation: Unit 2 URL centralization — a malformed `api_url` helper (double slash / missing slash) would break all 3 endpoints. Mitigate with unit test for `api_url` covering `https://a/`, `https://a`, `https://a/` + `api/v1/...`.

## Risks / Rollback
- **Risk:** `OnceLock` regex init changes failure mode (was `unwrap` panic per call; now panic at first use if pattern invalid — still panic, but later). Mitigation: patterns are literals, validated by existing tests; no behavior change.
- **Risk:** Configurable base URL breaks existing users relying on hardcoded default. Mitigation: default preserves `https://mastodon.social`; flag is optional with env fallback.
- **Risk:** Dropping copilot branch loses the 25-entry constant map if comprehensive `emojis` crate is unavailable offline. Mitigation: `emojis` crate is already a dependency and works offline (bundled data); no loss.
- **Rollback:** Each unit is one commit on `main`; revert single commit via `git revert <sha>` without affecting others (units are file-localized). No DB/migration to roll back.

## Open Questions
1. **What is the actual goal of `/plan`?** This plan is inferred from repo health. If you had a specific feature in mind (e.g., "add `--dry-run`", "support multiple images", "add timeline filtering"), state it and this plan will be rescoped.
2. **Instance configurability scope:** Should the flag be `--instance` (host only, e.g., `mastodon.social`) or `--base-url` (full URL)? Recommendation is `--instance` with env `MASTODON_INSTANCE` for brevity, but either works. Confirm naming.
3. **Error-handling ambition:** Keep simple `Box<dyn Error>` + `exit(1)` (current) or adopt `anyhow`/`thiserror` for richer context? Recommendation is keep simple per learning-example constraint.
4. **Copilot branch disposition:** Delete remote branch after reconciliation, or keep archived with a note? Default is delete after approval.
5. **Test scope:** Should `clean_html` tests cover malformed HTML (unclosed tags) or just well-formed Mastodon HTML? Recommendation is cover Mastodon-realistic cases only.

---
*Plan saved as inferred scope — no code edited. Awaiting approval before execution. No file was created beyond this plan document.*
