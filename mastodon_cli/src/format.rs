//! `format` — Text cleaning, emoji expansion, and pretty status rendering
//!
//! This module handles everything that turns raw Mastodon HTML into nice
//! terminal output. It is intentionally separate from `api.rs` (data) and
//! `main.rs` (control flow) so it can be unit-tested without network I/O.
//!
//! Concepts covered:
//! - `OnceLock<Regex>`: thread-safe, lazy initialization of a compiled regex.
//!   Compiling a regex is expensive, so we compile once on first use and reuse it.
//! - `regex::Regex` + `emojis` crate for shortcode → Unicode replacement.
//! - `html-escape` for decoding `&amp;`, `&gt;`, etc.
//! - `unicode-width` for correct terminal column width (emojis are 2 columns wide).

use regex::Regex;
use std::sync::OnceLock;
use unicode_width::UnicodeWidthStr;

use crate::api::Status;

// ---------------------------------------------------------------------------
// Emoji replacement — single-pass regex with `emojis` crate
// ---------------------------------------------------------------------------

/// Lazily-initialized regex that matches `:shortcode:` patterns.
///
/// `OnceLock` is like `lazy_static` but from the standard library (stable
/// since Rust 1.70). The regex is compiled exactly once, even if multiple
/// threads call `replace_emojis` concurrently.
static EMOJI_RE: OnceLock<Regex> = OnceLock::new();

/// Replaces `:shortcodes:` with real Unicode emoji where possible.
///
/// Example: `"Hello :rocket: :apple:"` → `"Hello 🚀 🍎"`
///
/// Learner notes:
/// - `Regex::new(r":([a-z0-9_]+):")` captures the inner shortcode as group 1.
/// - `replace_all` takes a closure that receives each match (`Caps`) and returns
///   the replacement string. This is a single pass over the input — O(n).
/// - `emojis::get_by_shortcode` does a lookup in a large Unicode table. If no
///   emoji is found (e.g. `:not_a_real_emoji:`), we keep the original text so
///   we never corrupt user input.
/// - `.into_owned()` converts the `Cow<str>` returned by `replace_all` into an
///   owned `String`.
pub(crate) fn replace_emojis(text: &str) -> String {
    // `get_or_init` compiles the regex on first call; subsequent calls reuse it.
    let re = EMOJI_RE.get_or_init(|| Regex::new(r":([a-z0-9_]+):").unwrap());
    re.replace_all(text, |caps: &regex::Captures| {
        match emojis::get_by_shortcode(&caps[1]) {
            Some(emoji) => emoji.as_str().to_string(),
            None => caps[0].to_string(), // keep unknown shortcodes unchanged
        }
    })
    .into_owned()
}

/// A warning that a shortcode was not found, with close suggestions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EmojiWarning {
    /// The unknown shortcode without colons, e.g. `"fox"`.
    pub shortcode: String,
    /// Up to 3 suggestions as `(shortcode, emoji)` pairs, e.g. `[("fox_face", "🦊")]`.
    pub suggestions: Vec<(String, String)>,
}

/// Compute Levenshtein edit distance (pure, no alloc beyond two rows).
fn levenshtein(a: &str, b: &str) -> usize {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for (i, &ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            cur[j + 1] = (prev[j + 1] + 1) // deletion
                .min(cur[j] + 1) // insertion
                .min(prev[j] + cost); // substitution
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// Suggest up to `limit` close shortcodes for an unknown `query`.
///
/// Heuristic: meaningful substring match OR Levenshtein distance ≤ 2.
/// Substring is considered meaningful only if `candidate.contains(query)` with
/// `query.len()>=2` or `query.contains(candidate)` with `candidate.len()>=3`
/// to avoid trivial 1-char matches like `:o:` for `:unknown_shortcode:`.
/// Results are sorted with substring matches first, then by distance.
/// Uses all gemoji shortcodes (including aliases) via `emoji.shortcodes()`.
pub(crate) fn suggest_emojis(query: &str, limit: usize) -> Vec<(String, String)> {
    let query_lc = query.to_ascii_lowercase();
    let mut candidates: Vec<(u8, usize, String, String)> = Vec::new();
    for emoji in emojis::iter() {
        for sc in emoji.shortcodes() {
            let sc_lc = sc.to_ascii_lowercase();
            let dist = levenshtein(&query_lc, &sc_lc);
            let is_substring = (sc_lc.contains(&query_lc) && query_lc.len() >= 2)
                || (query_lc.contains(&sc_lc) && sc_lc.len() >= 3);
            if dist <= 2 || is_substring {
                // 0 = substring (preferred), 1 = edit-distance only
                let rank: u8 = if is_substring { 0 } else { 1 };
                candidates.push((rank, dist, sc.to_string(), emoji.as_str().to_string()));
            }
        }
    }
    // Sort: substring first, then distance, then alphabetically for stability
    candidates.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)).then_with(|| a.2.cmp(&b.2)));
    candidates.dedup_by(|a, b| a.2 == b.2);
    candidates
        .into_iter()
        .take(limit)
        .map(|(_, _, sc, em)| (sc, em))
        .collect()
}

/// Like `replace_emojis` but also returns warnings for unknown shortcodes.
///
/// Keeps unknown `:shortcode:` unchanged (no data loss) and attaches
/// `EmojiWarning`s with up to 3 suggestions each. Pure — caller decides how
/// to display warnings (e.g. `eprintln!` in `main.rs` posting path).
///
/// Example: `"Hi :fox: :rocket:"` → `("Hi :fox: 🚀", [EmojiWarning{shortcode:"fox", suggestions:[("fox_face","🦊")] }])`
pub(crate) fn replace_emojis_with_warnings(text: &str) -> (String, Vec<EmojiWarning>) {
    let re = EMOJI_RE.get_or_init(|| Regex::new(r":([a-z0-9_]+):").unwrap());
    let mut warnings: Vec<EmojiWarning> = Vec::new();
    // Use `replace_all` with a closure that captures &mut warnings
    let replaced = re
        .replace_all(text, |caps: &regex::Captures| {
            let shortcode = &caps[1];
            match emojis::get_by_shortcode(shortcode) {
                Some(emoji) => emoji.as_str().to_string(),
                None => {
                    // Collect warning deduped by shortcode (avoid duplicate warnings for same typo repeated)
                    if !warnings.iter().any(|w| w.shortcode == shortcode) {
                        let suggestions = suggest_emojis(shortcode, 3);
                        warnings.push(EmojiWarning {
                            shortcode: shortcode.to_string(),
                            suggestions,
                        });
                    }
                    caps[0].to_string()
                }
            }
        })
        .into_owned();
    (replaced, warnings)
}

// ---------------------------------------------------------------------------
// HTML cleaning — strip tags + decode entities
// ---------------------------------------------------------------------------

/// Lazily-initialized regex that matches any HTML tag like `<p>`, `</a>`, `<br/>`.
static HTML_RE: OnceLock<Regex> = OnceLock::new();

/// Strips HTML tags and decodes HTML entities.
///
/// Mastodon returns status `content` as HTML (e.g. `"<p>Hello &amp; <a>world</a></p>"`).
/// For a terminal we want plain text: `"Hello & world"`.
///
/// Steps:
/// 1. Remove all `<...>` tags via regex (simple but sufficient for display).
///    A full HTML parser would be more robust but heavier.
/// 2. Decode entities like `&gt;` → `>`, `&amp;` → `&` via `html_escape`.
pub(crate) fn clean_html(text: &str) -> String {
    let re = HTML_RE.get_or_init(|| Regex::new(r"<[^>]*>").unwrap());
    let stripped = re.replace_all(text, "");
    html_escape::decode_html_entities(&stripped).into_owned()
}

// ---------------------------------------------------------------------------
// Word wrapping — unicode-aware, word-boundary preserving
// ---------------------------------------------------------------------------

/// Wraps text to a maximum display width, preserving word boundaries.
///
/// Learner notes:
/// - `UnicodeWidthStr::width` gives the *terminal column width*, not byte length.
///   `"hello".len() == 5` but `"🖼️".width() == 2`. Using `.len()` would misalign boxes.
/// - We split on `split_whitespace()` (handles multiple spaces/newlines) and
///   rebuild lines greedily: keep adding words while `current_width + 1 + word_len <= max_width`.
/// - `text.lines()` preserves original line breaks from the HTML → plain text.
/// - Edge case: a single word longer than `max_width` is emitted on its own line
///   (no hyphenation) to avoid infinite loops.
pub(crate) fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for raw_line in text.lines() {
        let trimmed = raw_line.trim_end();
        if trimmed.is_empty() {
            // Preserve intentional blank lines (paragraph breaks).
            lines.push(String::new());
            continue;
        }
        let mut current_line = String::new();
        let mut current_width = 0;
        for word in trimmed.split_whitespace() {
            let word_len = UnicodeWidthStr::width(word);
            if current_line.is_empty() {
                if word_len > max_width {
                    // Word alone exceeds width — emit as-is (no split).
                    lines.push(word.to_string());
                } else {
                    current_line.push_str(word);
                    current_width = word_len;
                }
            } else if current_width + 1 + word_len <= max_width {
                // "+1" for the space between words.
                current_line.push(' ');
                current_line.push_str(word);
                current_width += 1 + word_len;
            } else {
                // Word doesn't fit — push current line and start a new one.
                lines.push(current_line);
                current_line = word.to_string();
                current_width = word_len;
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }
    }
    if lines.is_empty() {
        // Ensure we always return at least one line so the box is not empty.
        lines.push(String::new());
    }
    lines
}

// ---------------------------------------------------------------------------
// Box rendering — the pretty terminal UI
// ---------------------------------------------------------------------------

/// Formats a status for display inside a clean Unicode text box.
///
/// The box is fixed at 76 terminal columns (a common readable width):
/// ```text
/// ┌── Status #1 ─────────────────────────────────────────────────────┐
/// │ 🧵 Reply  🖼️ Attachment                                          │
/// ├──────────────────────────────────────────────────────────────────┤
/// │ Hello world! This is the status content wrapped to fit inside   │
/// │ the box.                                                         │
/// └──────────────────────────────────────────────────────────────────┘
/// ```
///
/// Steps:
/// 1. Build a header row `┌── Status #N ───┐` with the remaining width filled by `─`.
/// 2. If the status is a reply or has attachments, show an indicator row + separator.
/// 3. Clean HTML, expand emojis, wrap to `inner_width` (box minus borders).
/// 4. Emit each wrapped line padded to `inner_width` so the right border aligns.
/// 5. Close with `└────┘`.
pub(crate) fn format_status(index: usize, status: &Status) -> String {
    // `box_width` includes borders; `inner_width` is the usable text area.
    // `76 - 4 = 72`: 2 for "│ " on the left + 2 for " │" on the right.
    let box_width: usize = 76;
    let inner_width: usize = box_width - 4;
    let mut output = String::new();

    // --- 1. Top border with title ---
    let header_title = format!(" Status #{} ", index + 1);
    let title_len = UnicodeWidthStr::width(header_title.as_str());
    // Remaining dashes: box_width - "┌──" (3?) actually "┌──" + title + "┐" accounting.
    // We use saturating_sub to avoid underflow if title is absurdly long.
    let remaining_border = box_width.saturating_sub(title_len + 4);
    output.push_str(&format!(
        "┌──{}{}\n",
        header_title,
        format!("{}┐", "─".repeat(remaining_border))
    ));

    // --- 2. Metadata indicators (reply / image) ---
    let has_reply = status.in_reply_to_id.is_some();
    let has_image = !status.media_attachments.is_empty();
    if has_reply || has_image {
        let mut indicators = Vec::new();
        if has_reply {
            indicators.push("🧵 Reply");
        }
        if has_image {
            indicators.push("🖼️ Attachment");
        }
        let indicator_str = indicators.join("  ");
        // Pad with spaces so the right border aligns (unicode-aware).
        let padding = inner_width.saturating_sub(UnicodeWidthStr::width(indicator_str.as_str()));
        output.push_str(&format!("│ {}{} │\n", indicator_str, " ".repeat(padding)));
        output.push_str(&format!("├{}┤\n", "─".repeat(box_width - 2)));
    }

    // --- 3 & 4. Content: clean HTML → expand emojis → wrap → pad ---
    let content = replace_emojis(&clean_html(&status.content));
    for line in wrap_text(&content, inner_width) {
        let padding = inner_width.saturating_sub(UnicodeWidthStr::width(line.as_str()));
        output.push_str(&format!("│ {}{} │\n", line, " ".repeat(padding)));
    }

    // --- 5. Bottom border ---
    output.push_str(&format!("└{}┘", "─".repeat(box_width - 2)));
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a minimal `Status` for testing the formatter without
    // needing a full API response. `..status("...")` uses Rust's struct update
    // syntax to fill the remaining fields from `status()`.
    fn status(content: &str) -> Status {
        Status {
            content: content.to_string(),
            media_attachments: vec![],
            in_reply_to_id: None,
        }
    }

    #[test]
    fn replaces_shortcodes() {
        assert_eq!(replace_emojis("Launch :rocket:!"), "Launch 🚀!");
    }

    #[test]
    fn preserves_unknown_shortcodes() {
        // Unknown shortcodes should be left as-is, not stripped.
        assert_eq!(
            replace_emojis("No :unknown_shortcode:"),
            "No :unknown_shortcode:"
        );
    }

    #[test]
    fn cleans_html() {
        assert_eq!(clean_html("<strong>Bold</strong> &amp; text"), "Bold & text");
    }

    #[test]
    fn wraps_text_at_word_boundaries() {
        assert_eq!(
            wrap_text("The quick brown fox jumps over the lazy dog", 20),
            vec!["The quick brown fox", "jumps over the lazy", "dog"]
        );
    }

    #[test]
    fn formats_status() {
        let formatted = format_status(0, &status("Hello world!"));
        assert!(formatted.starts_with("┌── Status #1 "));
        assert!(formatted.contains("Hello world!"));
        assert!(formatted.contains("└"));
    }

    #[test]
    fn formats_reply_status() {
        let reply = Status {
            in_reply_to_id: Some("123".to_string()),
            ..status("Replying!")
        };
        assert!(format_status(0, &reply).contains("🧵 Reply"));
    }

    #[test]
    fn formats_status_with_image() {
        let image = Status {
            media_attachments: vec![crate::api::MediaAttachment {}],
            ..status("Image!")
        };
        assert!(format_status(0, &image).contains("🖼️ Attachment"));
    }

    #[test]
    fn formats_reply_with_image() {
        let both = Status {
            media_attachments: vec![crate::api::MediaAttachment {}],
            in_reply_to_id: Some("456".to_string()),
            ..status("Reply with image!")
        };
        assert!(format_status(0, &both).contains("🧵 Reply  🖼️ Attachment"));
    }

    #[test]
    fn suggests_fox_face_for_fox() {
        let sug = suggest_emojis("fox", 3);
        assert!(sug.iter().any(|(sc, _)| sc == "fox_face"), "expected fox_face in {sug:?}");
        assert_eq!(sug[0].0, "fox_face"); // substring ranked first
    }

    #[test]
    fn suggests_rocket_for_typo() {
        let sug = suggest_emojis("roket", 3);
        assert!(sug.iter().any(|(sc, _)| sc == "rocket"));
    }

    #[test]
    fn warnings_for_unknown_shortcode() {
        let (text, warnings) = replace_emojis_with_warnings("Hi :fox: :rocket:");
        assert_eq!(text, "Hi :fox: 🚀"); // unknown kept, known expanded
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].shortcode, "fox");
        assert!(warnings[0].suggestions.iter().any(|(sc, _)| sc == "fox_face"));
    }

    #[test]
    fn no_warnings_for_known() {
        let (text, warnings) = replace_emojis_with_warnings("Launch :rocket:");
        assert_eq!(text, "Launch 🚀");
        assert!(warnings.is_empty());
    }

    #[test]
    fn warns_no_suggestions_for_gibberish() {
        let (_, warnings) = replace_emojis_with_warnings("Hi :unknown_shortcode:");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].suggestions.is_empty());
    }
}
