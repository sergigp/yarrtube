use crate::domain::preferences::{ContextLength, SummaryPrompt};
use crate::domain::video::Video;
use crate::domain::video_summary::SummaryText;
use crate::domain::video_transcript::{CaptionLanguage, TimedTranscript};

/// The two messages of one call to the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryMessages {
    pub system: String,
    pub user: String,
}

/// A video as the summary prompt needs it, cut into the parts that fit the
/// model's context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryInput {
    parts: usize,
}

impl SummaryInput {
    pub fn new(
        _video: &Video,
        _language: &CaptionLanguage,
        _transcript: &TimedTranscript,
        _context_length: &ContextLength,
    ) -> Self {
        Self { parts: 1 }
    }

    pub fn parts(&self) -> usize {
        self.parts
    }

    /// `system` + `user`; only when `parts() == 1`.
    pub fn whole(&self, _prompt: &SummaryPrompt) -> SummaryMessages {
        Self::empty_messages()
    }

    /// `map system` + `map user` for the 0-based `index`.
    pub fn part(&self, _prompt: &SummaryPrompt, _index: usize) -> SummaryMessages {
        Self::empty_messages()
    }

    /// `reduce system` + `reduce user`, one `reduce part` per answer.
    pub fn reduce(&self, _prompt: &SummaryPrompt, _answers: &[SummaryText]) -> SummaryMessages {
        Self::empty_messages()
    }

    fn empty_messages() -> SummaryMessages {
        SummaryMessages {
            system: String::new(),
            user: String::new(),
        }
    }
}
