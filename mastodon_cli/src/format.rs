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
}
