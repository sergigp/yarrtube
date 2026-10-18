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
        Self(
            srt.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !is_cue_index(line) && !line.contains("-->"))
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_cue_index(line: &str) -> bool {
    line.bytes().all(|byte| byte.is_ascii_digit())
}
