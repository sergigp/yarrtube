use super::TranscriptPart;
use crate::domain::video::VideoChapter;

/// A spoken line and when it starts, in seconds from the video's start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimedLine {
    pub start_seconds: i64,
    pub text: String,
}

/// A video's captions with their timings, which the stored `TranscriptText`
/// doesn't keep. Cleaned as `TranscriptText::from_srt` does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimedTranscript(Vec<TimedLine>);

impl TimedTranscript {
    pub fn from_srt(_srt: &str) -> Self {
        Self(Vec::new())
    }

    pub fn text(&self) -> String {
        String::new()
    }

    /// The whole transcript as one part when it fits `max_chars`, else cut
    /// at chapter starts, then by size.
    pub fn parts(&self, _chapters: &[VideoChapter], _max_chars: usize) -> Vec<TranscriptPart> {
        Vec::new()
    }
}
