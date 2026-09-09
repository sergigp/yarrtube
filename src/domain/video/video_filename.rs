use std::fmt;
use unicode_general_category::get_general_category;
use unicode_normalization::UnicodeNormalization;

/// A safe base filename (no extension, no video ID) derived from a video's
/// raw title. Pure and infallible: unlike `PlaylistName`, a video's title is
/// external metadata yarrtube doesn't control and can't refuse, so unsafe
/// input is sanitized rather than rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoFilename(String);

const FILESYSTEM_UNSAFE_CHARS: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
const SAFE_SEPARATOR: char = '-';
const MAX_FILENAME_BYTES: usize = 150;

impl VideoFilename {
    /// Derives a sanitized base filename from a raw video title:
    /// Unicode-normalizes lookalike punctuation (e.g. fullwidth quotes) back
    /// to its ASCII form, replaces filesystem-unsafe characters with a safe
    /// separator, strips decorative/emoji symbols by Unicode category,
    /// collapses/trims whitespace, and truncates to a safe byte length.
    pub fn from_title(title: &str) -> Self {
        let mut sanitized = String::with_capacity(title.len());
        for ch in title.nfkc() {
            if FILESYSTEM_UNSAFE_CHARS.contains(&ch) {
                sanitized.push(SAFE_SEPARATOR);
            } else if ch.is_whitespace() || is_keepable(ch) {
                sanitized.push(ch);
            }
        }

        let collapsed = sanitized.split_whitespace().collect::<Vec<_>>().join(" ");
        Self(truncate_at_char_boundary(&collapsed, MAX_FILENAME_BYTES))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VideoFilename {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Keeps letters, marks, numbers and punctuation (Unicode general category
/// groups L/M/N/P); drops everything else, notably the Symbol categories
/// (So/Sk/Sm/Sc) that cover emoji and decorative symbol lookalikes.
fn is_keepable(ch: char) -> bool {
    matches!(
        get_general_category(ch).abbreviation().as_bytes()[0],
        b'L' | b'M' | b'N' | b'P'
    )
}

fn truncate_at_char_boundary(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_collapse_repeated_and_surrounding_whitespace() {
        let filename = VideoFilename::from_title("  My   Video   Title  ");
        assert_eq!(filename.as_str(), "My Video Title");
    }

    #[test]
    fn it_should_replace_filesystem_unsafe_characters_with_a_safe_separator() {
        let filename = VideoFilename::from_title(r#"a/b\c:d*e?f"g<h>i|j"#);
        assert_eq!(filename.as_str(), "a-b-c-d-e-f-g-h-i-j");
        for unsafe_char in FILESYSTEM_UNSAFE_CHARS {
            assert!(!filename.as_str().contains(*unsafe_char));
        }
    }

    #[test]
    fn it_should_strip_emoji_and_decorative_symbols() {
        let filename = VideoFilename::from_title("🎉 Party Time 🎉 ★彡");
        assert_eq!(filename.as_str(), "Party Time 彡");
    }

    #[test]
    fn it_should_preserve_accented_and_non_ascii_letters() {
        let filename = VideoFilename::from_title("Café à Montréal");
        assert_eq!(filename.as_str(), "Café à Montréal");
    }

    #[test]
    fn it_should_truncate_to_the_byte_limit_on_a_char_boundary() {
        let title = "é".repeat(200);
        let filename = VideoFilename::from_title(&title);

        assert!(filename.as_str().len() <= MAX_FILENAME_BYTES);
        assert!(filename.as_str().len() > MAX_FILENAME_BYTES - "é".len());
        // Every remaining character parses back cleanly (no split code point).
        assert_eq!(
            filename.as_str().chars().count(),
            filename.as_str().len() / "é".len()
        );
    }

    #[test]
    fn it_should_sanitize_a_combination_of_rules_together() {
        let filename = VideoFilename::from_title("  My: Café  Trip?  🎉  ");
        assert_eq!(filename.as_str(), "My- Café Trip-");
    }

    #[test]
    fn it_should_sanitize_a_messy_title_with_extra_whitespace_and_colon() {
        let filename = VideoFilename::from_title(
            "9 AI Concepts Explained in 7 minutes: AI Agents, RAGs, Tokenization, RLHF...",
        );
        assert_eq!(
            filename.as_str(),
            "9 AI Concepts Explained in 7 minutes- AI Agents, RAGs, Tokenization, RLHF..."
        );
        assert!(!filename.as_str().contains(':'));
        assert!(filename.as_str().contains("AI"));
        assert!(filename.as_str().contains("RAGs"));
        assert!(filename.as_str().contains("RLHF"));
    }

    #[test]
    fn it_should_sanitize_a_title_with_fullwidth_lookalike_punctuation() {
        let filename = VideoFilename::from_title("\u{FF02}Got any hobbies\u{FF1F}\u{FF02}");

        assert!(
            !filename
                .as_str()
                .chars()
                .any(|c| c == '\u{FF02}' || c == '\u{FF1F}')
        );
        assert!(filename.as_str().contains("Got any hobbies"));
    }

    #[test]
    fn it_should_sanitize_a_title_with_a_math_symbol_slash_lookalike() {
        let filename = VideoFilename::from_title("New Skills! v1.2 brings \u{29F8}wait-what...");

        assert_eq!(filename.as_str(), "New Skills! v1.2 brings wait-what...");
    }
}
