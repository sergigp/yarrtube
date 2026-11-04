use crate::domain::preferences::{ContextLength, SummaryPrompt, SummaryPromptSection};
use crate::domain::video::Video;
use crate::domain::video_summary::SummaryText;
use crate::domain::video_transcript::{CaptionLanguage, TimedTranscript, TranscriptPart};

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
    title: String,
    minutes: i64,
    language: String,
    parts: Vec<TranscriptPart>,
}

impl SummaryInput {
    pub fn new(
        video: &Video,
        language: &CaptionLanguage,
        transcript: &TimedTranscript,
        context_length: &ContextLength,
    ) -> Self {
        let max_chars =
            f64::from(context_length.input_budget_tokens()) * language.chars_per_token();
        Self {
            title: video.title.clone(),
            minutes: video.duration_seconds.unwrap_or(0) / 60,
            language: language.english_name(),
            parts: transcript.parts(&[], max_chars as usize),
        }
    }

    pub fn parts(&self) -> usize {
        self.parts.len()
    }

    /// `system` + `user`; only when `parts() == 1`.
    pub fn whole(&self, prompt: &SummaryPrompt) -> SummaryMessages {
        let transcript = self.parts.first().map_or("", |part| part.text.as_str());
        SummaryMessages {
            system: prompt.render(SummaryPromptSection::System, &self.language_value()),
            user: prompt.render(
                SummaryPromptSection::User,
                &[
                    self.head_values(),
                    vec![
                        ("chapters", String::new()),
                        ("transcript", transcript.to_string()),
                    ],
                ]
                .concat(),
            ),
        }
    }

    /// `map system` + `map user` for the 0-based `index`.
    pub fn part(&self, _prompt: &SummaryPrompt, _index: usize) -> SummaryMessages {
        Self::empty_messages()
    }

    /// `reduce system` + `reduce user`, one `reduce part` per answer.
    pub fn reduce(&self, _prompt: &SummaryPrompt, _answers: &[SummaryText]) -> SummaryMessages {
        Self::empty_messages()
    }

    fn language_value(&self) -> Vec<(&'static str, String)> {
        vec![("lang", self.language.clone())]
    }

    fn head_values(&self) -> Vec<(&'static str, String)> {
        vec![
            ("title", self.title.clone()),
            ("minutes", self.minutes.to_string()),
            ("lang", self.language.clone()),
        ]
    }

    fn empty_messages() -> SummaryMessages {
        SummaryMessages {
            system: String::new(),
            user: String::new(),
        }
    }
}
