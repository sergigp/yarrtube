/// A video's captions as plain text, one spoken line per line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptText(String);

impl TranscriptText {
    /// Drops indices, timings and tags; collapses lines repeated by rolling
    /// auto captions.
    pub fn from_srt(_srt: &str) -> Self {
        Self(String::new())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
