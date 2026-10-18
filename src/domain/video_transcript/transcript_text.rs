/// A video's captions as plain text, one spoken line per line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptText(String);

impl TranscriptText {
    /// A transcript of exactly `text`, e.g. as stored.
    pub fn new(text: &str) -> Self {
        Self(text.to_string())
    }

    /// Drops indices, timings and tags; collapses lines repeated by rolling
    /// auto captions.
    pub fn from_srt(srt: &str) -> Self {
        let mut lines: Vec<String> = srt
            .lines()
            .map(str::trim)
            .filter(|line| !is_cue_index(line) && !line.contains("-->"))
            .map(strip_tags)
            .filter(|line| !line.is_empty())
            .collect();
        lines.dedup();
        Self(lines.join("\n"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_cue_index(line: &str) -> bool {
    !line.is_empty() && line.bytes().all(|byte| byte.is_ascii_digit())
}

/// `line` without its `<...>` markup (`<i>`, `<font ...>`, `<c>`), trimmed.
fn strip_tags(line: &str) -> String {
    line.split('<')
        .enumerate()
        .map(|(i, part)| match i {
            0 => part,
            _ => part.split_once('>').map_or("", |(_, text)| text),
        })
        .collect::<String>()
        .trim()
        .to_string()
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
    fn it_should_be_empty_for_an_empty_srt() {
        assert_eq!(TranscriptText::from_srt(""), TranscriptText::new(""));
    }
}
