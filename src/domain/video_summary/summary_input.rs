use crate::domain::preferences::{ContextLength, SummaryPrompt, SummaryPromptSection};
use crate::domain::video::{Video, VideoChapter};
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
    chapters: Vec<VideoChapter>,
    parts: Vec<TranscriptPart>,
}

impl SummaryInput {
    pub fn new(
        video: &Video,
        language: &CaptionLanguage,
        transcript: &TimedTranscript,
        context_length: &ContextLength,
    ) -> Self {
        let chapters = video.chapters.clone().unwrap_or_default();
        let max_chars =
            f64::from(context_length.input_budget_tokens()) * language.chars_per_token();
        let parts = transcript.parts(&chapters, max_chars as usize);
        Self {
            title: video.title.clone(),
            minutes: video.duration_seconds.unwrap_or(0) / 60,
            language: language.english_name(),
            chapters,
            parts,
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
                &self.head_values_with(vec![
                    ("chapters", self.chapter_block(prompt)),
                    ("transcript", transcript.to_string()),
                ]),
            ),
        }
    }

    /// `map system` + `map user` for the 0-based `index`.
    pub fn part(&self, prompt: &SummaryPrompt, index: usize) -> SummaryMessages {
        let part = &self.parts[index];
        SummaryMessages {
            system: prompt.render(SummaryPromptSection::MapSystem, &self.language_value()),
            user: prompt.render(
                SummaryPromptSection::MapUser,
                &self.head_values_with(vec![
                    ("part", (index + 1).to_string()),
                    ("parts", self.parts.len().to_string()),
                    ("span", Self::span(part)),
                    ("chapters", self.part_chapter_block(prompt, part)),
                    ("transcript", part.text.clone()),
                ]),
            ),
        }
    }

    /// `reduce system` + `reduce user`, one `reduce part` per answer.
    pub fn reduce(&self, prompt: &SummaryPrompt, answers: &[SummaryText]) -> SummaryMessages {
        SummaryMessages {
            system: prompt.render(SummaryPromptSection::ReduceSystem, &self.language_value()),
            user: prompt.render(
                SummaryPromptSection::ReduceUser,
                &self.head_values_with(vec![
                    ("chapters", self.chapter_block(prompt)),
                    ("part_summaries", self.part_summaries(prompt, answers)),
                ]),
            ),
        }
    }

    fn part_summaries(&self, prompt: &SummaryPrompt, answers: &[SummaryText]) -> String {
        answers
            .iter()
            .zip(&self.parts)
            .enumerate()
            .map(|(index, (answer, part))| {
                prompt.render(
                    SummaryPromptSection::ReducePart,
                    &[
                        ("part", (index + 1).to_string()),
                        ("span", Self::span(part)),
                        ("summary", answer.as_str().to_string()),
                    ],
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn span(part: &TranscriptPart) -> String {
        format!("{}–{}", clock(part.start_seconds), clock(part.end_seconds))
    }

    fn language_value(&self) -> Vec<(&'static str, String)> {
        vec![("lang", self.language.clone())]
    }

    fn head_values_with(&self, extras: Vec<(&'static str, String)>) -> Vec<(&'static str, String)> {
        let mut values = vec![
            ("title", self.title.clone()),
            ("minutes", self.minutes.to_string()),
        ];
        values.extend(self.language_value());
        values.extend(extras);
        values
    }

    /// The whole video's chapters as the `chapters` section.
    fn chapter_block(&self, prompt: &SummaryPrompt) -> String {
        block(prompt, SummaryPromptSection::Chapters, &self.chapters)
    }

    /// The chapters overlapping `part` as the `map chapters` section.
    fn part_chapter_block(&self, prompt: &SummaryPrompt, part: &TranscriptPart) -> String {
        let overlapping: Vec<VideoChapter> = self
            .chapters
            .iter()
            .filter(|chapter| {
                chapter.end_seconds > part.start_seconds && chapter.start_seconds < part.end_seconds
            })
            .cloned()
            .collect();
        block(prompt, SummaryPromptSection::MapChapters, &overlapping)
    }
}

/// `chapters` as `section`, on lines of its own; empty without chapters.
fn block(
    prompt: &SummaryPrompt,
    section: SummaryPromptSection,
    chapters: &[VideoChapter],
) -> String {
    match chapters.is_empty() {
        true => String::new(),
        false => {
            let list = chapter_list(chapters);
            format!("\n{}\n", prompt.render(section, &[("chapter_list", list)]))
        }
    }
}

fn chapter_list(chapters: &[VideoChapter]) -> String {
    chapters
        .iter()
        .map(|chapter| format!("{} {}", clock(chapter.start_seconds), chapter.title))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `seconds` as a viewer reads it: `2:05`, or `1:02:05` from an hour on.
fn clock(seconds: i64) -> String {
    match seconds >= 3600 {
        true => format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds % 3600 / 60,
            seconds % 60
        ),
        false => format!("{}:{:02}", seconds / 60, seconds % 60),
    }
}
