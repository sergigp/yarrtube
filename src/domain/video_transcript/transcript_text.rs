/// A video's captions as plain text, one spoken line per line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptText(String);

impl TranscriptText {
    /// A transcript of exactly `text`, e.g. as stored.
    pub fn new(text: &str) -> Self {
        Self(text.to_string())
    }

    /// Block by block: drops each cue's index line and timing line and
    /// tag-like `<…>` markup only, keeping digit-only lines, a literal `<`
    /// and `-->` in text; collapses lines repeated by rolling auto captions.
    pub fn from_srt(srt: &str) -> Self {
        let lines: Vec<&str> = srt.lines().map(str::trim).collect();
        let mut text: Vec<String> = lines
            .split(|line| line.is_empty())
            .flat_map(cue_text)
            .map(|line| strip_tags(line))
            .filter(|line| !line.is_empty())
            .collect();
        text.dedup();
        Self(text.join("\n"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A cue's text lines: without its leading index line, then its timing
/// line.
fn cue_text<'a>(cue: &'a [&'a str]) -> &'a [&'a str] {
    let without_index = without_first_if(cue, is_cue_index);
    without_first_if(without_index, |line| line.contains("-->"))
}

fn without_first_if<'a>(lines: &'a [&'a str], drop: impl Fn(&str) -> bool) -> &'a [&'a str] {
    match lines.split_first() {
        Some((first, rest)) if drop(first) => rest,
        _ => lines,
    }
}

fn is_cue_index(line: &str) -> bool {
    !line.is_empty() && line.bytes().all(|byte| byte.is_ascii_digit())
}

/// `line` without its tag-like markup (`<i>`, `</i>`, `<font ...>`, `<c>`,
/// `<00:01:02.345>`), trimmed. A `<` not opening such a tag is text.
fn strip_tags(line: &str) -> String {
    let mut text = String::new();
    let mut rest = line;
    while let Some(start) = rest.find('<') {
        text.push_str(&rest[..start]);
        let from_tag = &rest[start..];
        rest = match tag_len(from_tag) {
            Some(len) => &from_tag[len..],
            None => {
                text.push('<');
                &from_tag[1..]
            }
        };
    }
    text.push_str(rest);
    text.trim().to_string()
}

/// The length of the tag `text` opens with: a `<` directly followed by a
/// letter, digit or `/`, up to the next `>`, with no other `<` in between.
fn tag_len(text: &str) -> Option<usize> {
    let inner = &text[1..];
    let opens_tag = inner
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric() || c == '/');
    let end = inner.find('>')?;
    (opens_tag && !inner[..end].contains('<')).then_some(end + 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_keep_only_the_spoken_text_of_every_cue() {
        assert_eq!(
            TranscriptText::from_srt(
                "1\n00:00:00,000 --> 00:00:02,000\nHello there\n\n2\n00:00:02,000 --> 00:00:04,000\nGeneral Kenobi\nYou are a bold one\n"
            ),
            TranscriptText::new("Hello there\nGeneral Kenobi\nYou are a bold one")
        );
    }

    #[test]
    fn it_should_strip_formatting_tags() {
        assert_eq!(
            TranscriptText::from_srt(
                "1\n00:00:00,000 --> 00:00:02,000\n<i>Hello</i> <font color=\"#fff\">there</font>\n<c></c>\n"
            ),
            TranscriptText::new("Hello there")
        );
    }

    #[test]
    fn it_should_collapse_lines_repeated_by_rolling_captions() {
        assert_eq!(
            TranscriptText::from_srt(
                "1\n00:00:00,000 --> 00:00:02,000\nHello there\n\n2\n00:00:02,000 --> 00:00:04,000\nHello there\nGeneral Kenobi\n\n3\n00:00:04,000 --> 00:00:06,000\nGeneral Kenobi\nYou are a bold one\n"
            ),
            TranscriptText::new("Hello there\nGeneral Kenobi\nYou are a bold one")
        );
    }

    #[test]
    fn it_should_keep_a_text_line_made_of_digits() {
        assert_eq!(
            TranscriptText::from_srt(
                "1\n00:00:00,000 --> 00:00:02,000\nThe year was\n2024\n\n2\n00:00:02,000 --> 00:00:04,000\n42\n"
            ),
            TranscriptText::new("The year was\n2024\n42")
        );
    }

    #[test]
    fn it_should_keep_a_literal_less_than_sign() {
        assert_eq!(
            TranscriptText::from_srt(
                "1\n00:00:00,000 --> 00:00:02,000\nif x < 5 and y > 3\n<i>then</i> x<y\n"
            ),
            TranscriptText::new("if x < 5 and y > 3\nthen x<y")
        );
    }

    #[test]
    fn it_should_keep_text_containing_an_arrow() {
        assert_eq!(
            TranscriptText::from_srt(
                "1\n00:00:00,000 --> 00:00:02,000\nuse x --> y\n\n2\n00:00:02,000 --> 00:00:04,000\n-->\n"
            ),
            TranscriptText::new("use x --> y\n-->")
        );
    }

    #[test]
    fn it_should_keep_the_text_of_a_cue_without_an_index() {
        assert_eq!(
            TranscriptText::from_srt("00:00:00,000 --> 00:00:02,000\nHello there\n"),
            TranscriptText::new("Hello there")
        );
    }

    #[test]
    fn it_should_keep_text_continuing_after_a_blank_line() {
        assert_eq!(
            TranscriptText::from_srt(
                "1\n00:00:00,000 --> 00:00:02,000\nHello there\n\nGeneral Kenobi\n"
            ),
            TranscriptText::new("Hello there\nGeneral Kenobi")
        );
    }

    #[test]
    fn it_should_be_empty_for_an_empty_srt() {
        assert_eq!(TranscriptText::from_srt(""), TranscriptText::new(""));
    }
}
