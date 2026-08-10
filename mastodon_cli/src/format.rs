use regex::Regex;
use std::sync::OnceLock;
use unicode_width::UnicodeWidthStr;

use crate::api::Status;

static EMOJI_RE: OnceLock<Regex> = OnceLock::new();

/// Replaces :shortcodes: with actual emoji characters using a single-pass regex.
pub(crate) fn replace_emojis(text: &str) -> String {
    let re = EMOJI_RE.get_or_init(|| Regex::new(r":([a-z0-9_]+):").unwrap());
    re.replace_all(text, |caps: &regex::Captures| {
        match emojis::get_by_shortcode(&caps[1]) {
            Some(emoji) => emoji.as_str().to_string(),
            None => caps[0].to_string(),
        }
    }).into_owned()
}

static HTML_RE: OnceLock<Regex> = OnceLock::new();

/// Strips HTML tags and decodes HTML entities.
pub(crate) fn clean_html(text: &str) -> String {
    let re = HTML_RE.get_or_init(|| Regex::new(r"<[^>]*>").unwrap());
    let stripped = re.replace_all(text, "");
    html_escape::decode_html_entities(&stripped).into_owned()
}

pub(crate) fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for raw_line in text.lines() {
        let trimmed = raw_line.trim_end();
        if trimmed.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current_line = String::new();
        let mut current_width = 0;
        for word in trimmed.split_whitespace() {
            let word_len = UnicodeWidthStr::width(word);
            if current_line.is_empty() {
                if word_len > max_width {
                    lines.push(word.to_string());
                } else {
                    current_line.push_str(word);
                    current_width = word_len;
                }
            } else if current_width + 1 + word_len <= max_width {
                current_line.push(' ');
                current_line.push_str(word);
                current_width += 1 + word_len;
            } else {
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
        lines.push(String::new());
    }
    lines
}

pub(crate) fn format_status(index: usize, status: &Status) -> String {
    let box_width: usize = 76;
    let inner_width: usize = box_width - 4;
    let mut output = String::new();
    let header_title = format!(" Status #{} ", index + 1);
    let title_len = UnicodeWidthStr::width(header_title.as_str());
    let remaining_border = box_width.saturating_sub(title_len + 4);
    output.push_str(&format!("┌──{}{}\n", header_title, format!("{}┐", "─".repeat(remaining_border))));

    let has_reply = status.in_reply_to_id.is_some();
    let has_image = !status.media_attachments.is_empty();
    if has_reply || has_image {
        let mut indicators = Vec::new();
        if has_reply { indicators.push("🧵 Reply"); }
        if has_image { indicators.push("🖼️ Attachment"); }
        let indicator_str = indicators.join("  ");
        let padding = inner_width.saturating_sub(UnicodeWidthStr::width(indicator_str.as_str()));
        output.push_str(&format!("│ {}{} │\n", indicator_str, " ".repeat(padding)));
        output.push_str(&format!("├{}┤\n", "─".repeat(box_width - 2)));
    }

    let content = replace_emojis(&clean_html(&status.content));
    for line in wrap_text(&content, inner_width) {
        let padding = inner_width.saturating_sub(UnicodeWidthStr::width(line.as_str()));
        output.push_str(&format!("│ {}{} │\n", line, " ".repeat(padding)));
    }
    output.push_str(&format!("└{}┘", "─".repeat(box_width - 2)));
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(content: &str) -> Status {
        Status { content: content.to_string(), media_attachments: vec![], in_reply_to_id: None }
    }

    #[test]
    fn replaces_shortcodes() { assert_eq!(replace_emojis("Launch :rocket:!"), "Launch 🚀!"); }

    #[test]
    fn preserves_unknown_shortcodes() { assert_eq!(replace_emojis("No :unknown_shortcode:"), "No :unknown_shortcode:"); }

    #[test]
    fn cleans_html() { assert_eq!(clean_html("<strong>Bold</strong> &amp; text"), "Bold & text"); }

    #[test]
    fn wraps_text_at_word_boundaries() {
        assert_eq!(wrap_text("The quick brown fox jumps over the lazy dog", 20), vec!["The quick brown fox", "jumps over the lazy", "dog"]);
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
        let reply = Status { in_reply_to_id: Some("123".to_string()), ..status("Replying!") };
        assert!(format_status(0, &reply).contains("🧵 Reply"));
    }

    #[test]
    fn formats_status_with_image() {
        let image = Status { media_attachments: vec![crate::api::MediaAttachment {}], ..status("Image!") };
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
