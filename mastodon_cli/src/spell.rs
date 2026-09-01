//! `spell` — US English spell check engine, tokenization, and dictionary management
//!
//! This module handles pre-post spell checking for Mastodon messages.
//! It includes:
//! - Smart tokenization (ignoring URLs, handles, hashtags, emojis, code/numbers, acronyms)
//! - US English (`en_US`) dictionary lookups (built-in wordlist + system dict + custom user dict)
//! - Edit-distance suggestion generation (Levenshtein distance)
//! - Personal dictionary persistence (`~/.config/mastodon_cli/dict.txt`)

use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// A word token extracted from a message alongside its character position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Token {
    /// The cleaned word token suitable for spell checking.
    pub word: String,
    /// The original raw word substring before cleaning.
    pub raw: String,
    /// Byte offset start index in the input string.
    pub start: usize,
    /// Byte offset end index in the input string.
    pub end: usize,
}

/// A misspelling warning containing the raw word and top suggestions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpellWarning {
    /// Misspelled word as typed in the message.
    pub word: String,
    /// Top suggestions for replacement (up to 3-5).
    pub suggestions: Vec<String>,
}

/// Built-in core US English dictionary wordlist.
/// Built-in list guarantees out-of-the-box spell checking works offline
/// without requiring system dictionary packages.
static BUILTIN_WORDS: &[&str] = &[
    // Common tech & domain words
    "mastodon", "rust", "cli", "api", "url", "http", "https", "json", "html", "tokio",
    "cargo", "github", "git", "linux", "unix", "async", "await", "blog", "post", "status",
    "image", "media", "server", "instance", "token", "auth", "login", "user", "username",
    "email", "account", "profile", "feed", "timeline", "toot", "boost", "favourite",
    "favorite", "reply", "thread", "message", "text", "file", "path", "code", "data",
    // Pronouns & Basic words
    "i", "me", "my", "myself", "we", "our", "ours", "ourselves", "you", "your", "yours",
    "yourself", "yourselves", "he", "him", "his", "himself", "she", "her", "hers",
    "herself", "it", "its", "itself", "they", "them", "their", "theirs", "themselves",
    "what", "which", "who", "whom", "this", "that", "these", "those", "am", "is", "are",
    "was", "were", "be", "been", "being", "have", "has", "had", "having", "do", "does",
    "did", "doing", "a", "an", "the", "and", "but", "if", "or", "because", "as", "until",
    "while", "of", "at", "by", "for", "with", "about", "against", "between", "into",
    "through", "during", "before", "after", "above", "below", "to", "from", "up", "down",
    "in", "out", "on", "off", "over", "under", "again", "further", "then", "once", "here",
    "there", "when", "where", "why", "how", "all", "any", "both", "each", "few", "more",
    "most", "other", "some", "such", "no", "nor", "not", "only", "own", "same", "so",
    "than", "too", "very", "s", "t", "can", "will", "just", "don", "should", "now",
    // Common Verbs & Nouns
    "say", "says", "said", "saying", "make", "makes", "made", "making", "go", "goes",
    "went", "going", "gone", "take", "takes", "took", "taking", "taken", "come", "comes",
    "came", "coming", "see", "sees", "saw", "seeing", "seen", "know", "knows", "knew",
    "knowing", "known", "get", "gets", "got", "getting", "gotten", "give", "gives",
    "gave", "giving", "given", "find", "finds", "found", "finding", "think", "thinks",
    "thought", "thinking", "tell", "tells", "told", "telling", "become", "becomes",
    "became", "becoming", "show", "shows", "showed", "showing", "shown", "leave",
    "leaves", "left", "leaving", "feel", "feels", "felt", "feeling", "put", "puts",
    "putting", "bring", "brings", "brought", "bringing", "begin", "begins", "began",
    "beginning", "begun", "keep", "keeps", "kept", "keeping", "hold", "holds", "held",
    "holding", "write", "writes", "wrote", "writing", "written", "stand", "stands",
    "stood", "standing", "hear", "hears", "heard", "hearing", "let", "lets", "letting",
    "mean", "means", "meant", "meaning", "set", "sets", "setting", "meet", "meets",
    "met", "meeting", "run", "runs", "ran", "running", "pay", "pays", "paid", "paying",
    "sit", "sits", "sat", "sitting", "speak", "speaks", "spoke", "speaking", "spoken",
    "lie", "lies", "lay", "lying", "lain", "lead", "leads", "led", "leading", "read",
    "reads", "reading", "grow", "grows", "grew", "growing", "grown", "lose", "loses",
    "lost", "losing", "fall", "falls", "fell", "falling", "fallen", "send", "sends",
    "sent", "sending", "build", "builds", "built", "building", "understand",
    "understands", "understood", "understanding", "draw", "draws", "drew", "drawing",
    "drawn", "break", "breaks", "broke", "breaking", "broken", "spend", "spends",
    "spent", "spending", "cut", "cuts", "cutting", "rise", "rises", "rose", "rising",
    "risen", "drive", "drives", "drove", "driving", "driven", "buy", "buys", "bought",
    "buying", "wear", "wears", "wore", "wearing", "worn", "choose", "chooses", "chose",
    "choosing", "chosen", "hello", "world", "quick", "brown", "fox", "jumps", "lazy",
    "dog", "test", "testing", "checked", "checking", "checker", "good", "great", "awesome",
    "nice", "cool", "new", "old", "first", "last", "long", "short", "high", "low",
    "big", "small", "large", "little", "right", "wrong", "true", "false", "early", "late",
    "important", "public", "private", "simple", "complex", "easy", "hard", "best", "better",
    "worse", "worst", "today", "tomorrow", "yesterday", "now", "later", "soon", "always",
    "never", "sometimes", "often", "usually", "maybe", "please", "thanks", "thank",
    "welcome", "hi", "hey", "yes", "yeah", "no", "nope", "ok", "okay", "fine", "sure",
];

/// Lazy static initialized default spell checker.
static DEFAULT_CHECKER: OnceLock<SpellChecker> = OnceLock::new();

/// Spell checker holding valid dictionary words and custom user additions.
#[derive(Debug, Clone)]
pub(crate) struct SpellChecker {
    words: HashSet<String>,
}

impl SpellChecker {
    /// Create a new spell checker initialized with built-in US English words,
    /// system dictionaries (if present), and any custom user dictionary files.
    pub(crate) fn load(custom_dict: Option<&Path>) -> Self {
        let mut words: HashSet<String> = HashSet::new();

        // 1. Add built-in US English words
        for &w in BUILTIN_WORDS {
            words.insert(w.to_lowercase());
        }

        // 2. Try loading system dictionary files if available (/usr/share/hunspell/en_US.dic or /usr/share/dict/words)
        let system_dicts = [
            Path::new("/usr/share/hunspell/en_US.dic"),
            Path::new("/usr/share/dict/words"),
            Path::new("/usr/share/dict/american-english"),
        ];
        for dict_path in &system_dicts {
            if dict_path.exists() {
                if let Ok(content) = fs::read_to_string(dict_path) {
                    for line in content.lines() {
                        let word = line.trim().split('/').next().unwrap_or("").trim();
                        if !word.is_empty() && word.chars().all(|c| c.is_alphabetic() || c == '\'') {
                            words.insert(word.to_lowercase());
                        }
                    }
                }
            }
        }

        // 3. Load default user personal dictionary (~/.config/mastodon_cli/dict.txt)
        if let Some(user_dict_path) = default_personal_dict_path() {
            if user_dict_path.exists() {
                if let Ok(content) = fs::read_to_string(&user_dict_path) {
                    for line in content.lines() {
                        let word = line.trim();
                        if !word.is_empty() {
                            words.insert(word.to_lowercase());
                        }
                    }
                }
            }
        }

        // 4. Load explicit custom dictionary if provided
        if let Some(path) = custom_dict {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(path) {
                    for line in content.lines() {
                        let word = line.trim();
                        if !word.is_empty() {
                            words.insert(word.to_lowercase());
                        }
                    }
                }
            }
        }

        SpellChecker { words }
    }

    /// Check if a word is recognized in the dictionary.
    pub(crate) fn is_valid(&self, word: &str) -> bool {
        let lc = word.to_lowercase();
        if self.words.contains(&lc) {
            return true;
        }

        // Handle common English contractions with apostrophes (e.g., "don't", "it's", "they're")
        if word.contains('\'') {
            let base = word.replace('\'', "");
            if self.words.contains(&base.to_lowercase()) {
                return true;
            }
            // Strip trailing 's (e.g. "user's" -> "user")
            if let Some(stripped) = word.strip_suffix("'s").or_else(|| word.strip_suffix("’s")) {
                if self.words.contains(&stripped.to_lowercase()) {
                    return true;
                }
            }
        }

        false
    }

    /// Generate replacement suggestions for a misspelled word sorted by edit distance and similarity.
    pub(crate) fn suggest(&self, query: &str, limit: usize) -> Vec<String> {
        let query_lc = query.to_lowercase();
        let is_title_case = query
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false);

        let mut candidates: Vec<(usize, usize, usize, String)> = Vec::new();

        for dict_word in &self.words {
            // Quick length filter: skip words with length difference > 3
            let len_diff = (dict_word.len() as isize - query_lc.len() as isize).abs() as usize;
            if len_diff > 3 {
                continue;
            }

            let dist = levenshtein(&query_lc, dict_word);
            let is_builtin = BUILTIN_WORDS.contains(&dict_word.as_str());
            let freq_rank = if is_builtin { 0 } else { 1 };

            // Accept candidates with Levenshtein distance <= 2, or distance <= 3 for long words (> 6 chars)
            let max_allowed_dist = if query_lc.len() > 6 { 3 } else { 2 };

            if dist <= max_allowed_dist {
                candidates.push((dist, freq_rank, len_diff, dict_word.clone()));
            }
        }

        // Sort candidates:
        // 1. Edit distance (lower is better)
        // 2. Frequency rank (built-in common words preferred)
        // 3. Length difference (closer in length is better)
        // 4. Alphabetical
        candidates.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| a.1.cmp(&b.1))
                .then_with(|| a.2.cmp(&b.2))
                .then_with(|| a.3.cmp(&b.3))
        });

        candidates.dedup_by(|a, b| a.3 == b.3);

        candidates
            .into_iter()
            .take(limit)
            .map(|(_, _, _, word)| {
                if is_title_case {
                    capitalize(&word)
                } else {
                    word
                }
            })
            .collect()
    }
}



/// Returns the default personal dictionary path (`~/.config/mastodon_cli/dict.txt`).
pub(crate) fn default_personal_dict_path() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        Some(PathBuf::from(home).join(".config").join("mastodon_cli").join("dict.txt"))
    } else {
        None
    }
}

/// Appends a word to the user's personal dictionary.
pub(crate) fn add_word_to_personal_dict(word: &str, custom_path: Option<&Path>) -> io::Result<()> {
    let target_path = match custom_path {
        Some(p) => p.to_path_buf(),
        None => match default_personal_dict_path() {
            Some(p) => p,
            None => return Err(io::Error::new(io::ErrorKind::NotFound, "Home directory not found")),
        },
    };

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&target_path)?;

    writeln!(file, "{}", word.trim().to_lowercase())?;
    Ok(())
}

/// Capitalizes the first letter of a string.
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Pure Levenshtein distance calculation.
fn levenshtein(a: &str, b: &str) -> usize {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();

    if a_bytes.is_empty() {
        return b_bytes.len();
    }
    if b_bytes.is_empty() {
        return a_bytes.len();
    }

    let mut prev: Vec<usize> = (0..=b_bytes.len()).collect();
    let mut cur = vec![0; b_bytes.len() + 1];

    for (i, &ca) in a_bytes.iter().enumerate() {
        cur[0] = i + 1;
        for (j, &cb) in b_bytes.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            cur[j + 1] = (prev[j + 1] + 1)
                .min(cur[j] + 1)
                .min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b_bytes.len()]
}

/// Extracts tokens suitable for US English spell checking from a message.
///
/// Skips non-prose tokens:
/// - URLs (starts with http:// or https://)
/// - Handles (starts with @)
/// - Hashtags (starts with #)
/// - Emoji shortcodes (e.g. :rocket:)
/// - Digits/numbers (e.g. 123, v1.0)
/// - ALL_CAPS acronyms of length >= 2 (e.g. API, HTTP, CLI, JSON)
/// - Identifiers with code symbols (containing _, /, \, <, >)
pub(crate) fn tokenize(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut offset = 0;

    for raw_word in text.split_whitespace() {
        let start = text[offset..].find(raw_word).map(|i| offset + i).unwrap_or(offset);
        let end = start + raw_word.len();
        offset = end;

        // Skip URLs
        if raw_word.starts_with("http://") || raw_word.starts_with("https://") || raw_word.starts_with("www.") {
            continue;
        }

        // Skip Handles (@user or @user@domain)
        if raw_word.starts_with('@') {
            continue;
        }

        // Skip Hashtags (#topic)
        if raw_word.starts_with('#') {
            continue;
        }

        // Skip Emoji Shortcodes (:shortcode:)
        if raw_word.starts_with(':') && raw_word.ends_with(':') && raw_word.len() > 2 {
            continue;
        }

        // Skip code symbols or path-like strings containing slashes, backticks, underscores
        if raw_word.contains('/') || raw_word.contains('\\') || raw_word.contains('`') || raw_word.contains('_') {
            continue;
        }

        // Clean surrounding punctuation (keep apostrophes for contractions)
        let cleaned = raw_word
            .trim_matches(|c: char| !c.is_alphabetic() && c != '\'');

        if cleaned.is_empty() {
            continue;
        }

        // Skip digits/numbers
        if cleaned.chars().any(|c| c.is_numeric()) {
            continue;
        }

        // Skip ALL_CAPS acronyms of length >= 2 (e.g. "API", "CLI", "HTTP", "POST")
        if cleaned.len() >= 2 && cleaned.chars().all(|c| c.is_uppercase() || !c.is_alphabetic()) {
            continue;
        }

        // Must contain at least one alphabetic character
        if !cleaned.chars().any(|c| c.is_alphabetic()) {
            continue;
        }

        tokens.push(Token {
            word: cleaned.to_string(),
            raw: raw_word.to_string(),
            start,
            end,
        });
    }

    tokens
}

/// Evaluates a message string for US English misspellings.
///
/// Returns a list of `SpellWarning`s for any unrecognised words.
pub(crate) fn check_message(text: &str, custom_dict: Option<&Path>) -> Vec<SpellWarning> {
    let checker = match custom_dict {
        Some(p) => SpellChecker::load(Some(p)),
        None => DEFAULT_CHECKER
            .get_or_init(|| SpellChecker::load(None))
            .clone(),
    };

    let mut warnings = Vec::new();
    let tokens = tokenize(text);

    for token in tokens {
        if !checker.is_valid(&token.word) {
            // Avoid duplicate warnings for the same misspelled word repeated
            if !warnings.iter().any(|w: &SpellWarning| w.word.eq_ignore_ascii_case(&token.word)) {
                let suggestions = checker.suggest(&token.word, 3);
                warnings.push(SpellWarning {
                    word: token.word.clone(),
                    suggestions,
                });
            }
        }
    }

    warnings
}

/// Prompt user interactively in terminal mode for spell check corrections.
/// Returns the corrected message text.
pub(crate) fn check_and_correct_interactively(
    message: &str,
    custom_dict: Option<&Path>,
) -> io::Result<String> {
    let warnings = check_message(message, custom_dict);
    if warnings.is_empty() {
        return Ok(message.to_string());
    }

    let mut corrected = message.to_string();

    for warning in &warnings {
        eprintln!(
            "\nwarning: misspelled word \"{}\" detected",
            warning.word
        );
        eprintln!("Suggestions for \"{}\":", warning.word);
        eprintln!("  0) Keep as-is \"{}\"", warning.word);

        for (i, sug) in warning.suggestions.iter().enumerate() {
            eprintln!("  {}) Replace with \"{}\"", i + 1, sug);
        }
        eprintln!("  i) Ignore & Add \"{}\" to personal dictionary", warning.word);
        eprintln!("  a) Abort - exit without posting");

        let choice = loop {
            eprint!(
                "Enter choice [0-{}, i, a]: ",
                warning.suggestions.len()
            );
            io::stderr().flush()?;

            let mut input = String::new();
            let n = io::stdin().read_line(&mut input)?;
            if n == 0 {
                // EOF / Ctrl-D
                eprintln!("\nAborted. Message not posted.");
                std::process::exit(0);
            }

            let trimmed = input.trim();
            if trimmed.eq_ignore_ascii_case("a")
                || trimmed.eq_ignore_ascii_case("abort")
                || trimmed.eq_ignore_ascii_case("q")
                || trimmed.eq_ignore_ascii_case("quit")
            {
                eprintln!("Aborted. Message not posted.");
                std::process::exit(0);
            }

            if trimmed.eq_ignore_ascii_case("i") || trimmed.eq_ignore_ascii_case("ignore") {
                if let Err(e) = add_word_to_personal_dict(&warning.word, custom_dict) {
                    eprintln!("Warning: failed to write to personal dictionary: {}", e);
                } else {
                    eprintln!(" -> Added \"{}\" to personal dictionary", warning.word);
                }
                break Choice::Ignore;
            }

            match trimmed.parse::<usize>() {
                Ok(num) if num <= warning.suggestions.len() => {
                    if num == 0 {
                        break Choice::Keep;
                    } else {
                        break Choice::Replace(warning.suggestions[num - 1].clone());
                    }
                }
                _ => {
                    eprintln!(
                        "Invalid choice. Please enter 0..{}, 'i' to ignore/add, or 'a' to abort.",
                        warning.suggestions.len()
                    );
                    continue;
                }
            }
        };

        match choice {
            Choice::Keep | Choice::Ignore => {
                eprintln!(" -> keeping \"{}\" as-is", warning.word);
            }
            Choice::Replace(replacement) => {
                // Replace instances of misspelled word matching word boundary
                corrected = replace_word(&corrected, &warning.word, &replacement);
                eprintln!(
                    " -> replaced \"{}\" with \"{}\"",
                    warning.word, replacement
                );
            }
        }
    }

    eprintln!("\nFinal corrected message: {}", corrected);
    Ok(corrected)
}

enum Choice {
    Keep,
    Ignore,
    Replace(String),
}

/// Replaces whole instances of `target` word with `replacement` in `text`.
fn replace_word(text: &str, target: &str, replacement: &str) -> String {
    let mut result = String::new();
    let mut remaining = text;

    while let Some(idx) = remaining.find(target) {
        let before = &remaining[..idx];
        let after = &remaining[idx + target.len()..];

        // Check word boundaries before and after target
        let char_before = before.chars().next_back();
        let char_after = after.chars().next();

        let is_start_boundary = char_before.map(|c| !c.is_alphanumeric()).unwrap_or(true);
        let is_end_boundary = char_after.map(|c| !c.is_alphanumeric()).unwrap_or(true);

        if is_start_boundary && is_end_boundary {
            result.push_str(before);
            result.push_str(replacement);
            remaining = after;
        } else {
            result.push_str(&remaining[..idx + target.len()]);
            remaining = after;
        }
    }

    result.push_str(remaining);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenization_ignores_urls_handles_hashtags_emojis() {
        let msg = "Helo world! Visit https://example.com or email @alice@mastodon.social with #rustlang :rocket: and API v1.0";
        let tokens = tokenize(msg);
        let words: Vec<String> = tokens.into_iter().map(|t| t.word).collect();

        assert!(words.contains(&"Helo".to_string()));
        assert!(words.contains(&"world".to_string()));
        assert!(!words.contains(&"https://example.com".to_string()));
        assert!(!words.contains(&"@alice@mastodon.social".to_string()));
        assert!(!words.contains(&"#rustlang".to_string()));
        assert!(!words.contains(&":rocket:".to_string()));
        assert!(!words.contains(&"API".to_string()));
        assert!(!words.contains(&"v1.0".to_string()));
    }

    #[test]
    fn test_checker_validates_known_words() {
        let checker = SpellChecker::load(None);
        assert!(checker.is_valid("hello"));
        assert!(checker.is_valid("world"));
        assert!(checker.is_valid("rust"));
        assert!(checker.is_valid("mastodon"));
        assert!(!checker.is_valid("helo"));
        assert!(!checker.is_valid("roket"));
    }

    #[test]
    fn test_suggests_corrections() {
        let checker = SpellChecker::load(None);
        let suggestions = checker.suggest("Helo", 3);
        assert!(suggestions.contains(&"Hello".to_string()) || suggestions.contains(&"help".to_string()) || suggestions.contains(&"Help".to_string()));
    }

    #[test]
    fn test_check_message_detects_misspellings() {
        let warnings = check_message("Helo world this is a testt", None);
        assert_eq!(warnings.len(), 2);
        assert_eq!(warnings[0].word, "Helo");
        assert_eq!(warnings[1].word, "testt");
    }

    #[test]
    fn test_word_replacement_respects_boundaries() {
        let original = "Helo world! Helo again.";
        let replaced = replace_word(original, "Helo", "Hello");
        assert_eq!(replaced, "Hello world! Hello again.");
    }

    #[test]
    fn test_custom_dictionary_loading() {
        let temp_dir = std::env::temp_dir();
        let dict_path = temp_dir.join("test_custom_dict.txt");
        fs::write(&dict_path, "customword\nanothertest\n").unwrap();

        let checker = SpellChecker::load(Some(&dict_path));
        assert!(checker.is_valid("customword"));
        assert!(checker.is_valid("anothertest"));
        let _ = fs::remove_file(dict_path);
    }

    #[test]
    fn test_contractions_and_possessives() {
        let checker = SpellChecker::load(None);
        assert!(checker.is_valid("don't"));
        assert!(checker.is_valid("user's"));
        assert!(checker.is_valid("it's"));
    }
}

