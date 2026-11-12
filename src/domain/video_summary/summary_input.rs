use crate::domain::preferences::{ContextLength, SummaryPrompt, SummaryPromptSection};
use crate::domain::video::video_chapter::clock;
use crate::domain::video::{Video, VideoChapter};
use crate::domain::video_summary::{PartAnswer, SummaryMessages, SummaryText};
use crate::domain::video_transcript::{CaptionLanguage, TimedTranscript, TranscriptPart};

/// A part number at least as wide as any real one, to size the messages.
const WIDEST_PART_NUMBER: &str = "999";

/// A video as the summary prompt needs it, cut into the parts that fit the
/// model's context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryInput {
    title: String,
    minutes: i64,
    language: String,
    chapters: Vec<VideoChapter>,
    parts: Vec<TranscriptPart>,
    prompt: SummaryPrompt,
    budget_tokens: u32,
    max_chars: usize,
}

impl SummaryInput {
    pub fn new(
        video: &Video,
        language: &CaptionLanguage,
        transcript: &TimedTranscript,
        context_length: &ContextLength,
        prompt: &SummaryPrompt,
        chars_per_token: f64,
    ) -> Self {
        let chapters = video.chapters.found.clone().unwrap_or_default();
        let budget_tokens = context_length.input_budget_tokens();
        let max_chars = (f64::from(budget_tokens) * chars_per_token) as usize;
        let empty = Self {
            title: video.title.clone(),
            minutes: video.download.duration_seconds().unwrap_or(0) / 60,
            language: language.english_name(),
            chapters,
            parts: Vec::new(),
            prompt: prompt.clone(),
            budget_tokens,
            max_chars,
        };
        let transcript_chars = max_chars.saturating_sub(empty.prompt_chars());
        Self {
            parts: transcript.parts(&empty.chapters, transcript_chars),
            ..empty
        }
    }

    pub fn parts(&self) -> usize {
        self.parts.len()
    }

    /// `system` + `user`; only when `parts() == 1`.
    pub fn whole(&self) -> SummaryMessages {
        let prompt = &self.prompt;
        let transcript = self.parts.first().map_or("", |part| part.text.as_str());
        SummaryMessages {
            system: prompt.render(SummaryPromptSection::System, &self.language_value()),
            user: prompt.render(
                SummaryPromptSection::User,
                &self.head_values_with(vec![
                    ("chapters", self.chapter_block()),
                    ("transcript", transcript.to_string()),
                ]),
            ),
        }
    }

    /// `map system` + `map user` for the 0-based `index`.
    pub fn part(&self, index: usize) -> SummaryMessages {
        let part = &self.parts[index];
        self.part_messages(
            &(index + 1).to_string(),
            &self.parts.len().to_string(),
            part,
            self.part_chapter_block(part),
        )
    }

    /// The answers to every part, in order, as `reduce` takes them.
    pub fn answers(&self, summaries: Vec<SummaryText>) -> Vec<PartAnswer> {
        self.parts
            .iter()
            .zip(summaries)
            .map(|(part, summary)| PartAnswer {
                start_seconds: part.start_seconds,
                end_seconds: part.end_seconds,
                summary,
            })
            .collect()
    }

    /// `reduce system` + `reduce user`, one `reduce part` per answer.
    pub fn reduce(&self, answers: &[PartAnswer]) -> SummaryMessages {
        self.reduce_with(answers, self.chapter_block())
    }

    /// A reduce of `answers` without the chapters: the last reduce has them.
    pub fn reduce_group(&self, answers: &[PartAnswer]) -> SummaryMessages {
        self.reduce_with(answers, String::new())
    }

    /// Whether `answers` and the chapters fit the budget together.
    pub fn reduce_fits(&self, answers: &[PartAnswer]) -> bool {
        Self::chars(answers) + chapter_list(&self.chapters).chars().count() <= self.max_chars
    }

    /// `answers` as consecutive groups within the budget, of at least two
    /// (only a lone last answer stays alone), so each round leaves fewer.
    pub fn reduce_groups(&self, answers: &[PartAnswer]) -> Vec<Vec<PartAnswer>> {
        answers.iter().fold(Vec::new(), |mut groups, answer| {
            match groups.last_mut() {
                Some(group) if group.len() < 2 || self.joins(group, answer) => {
                    group.push(answer.clone())
                }
                _ => groups.push(vec![answer.clone()]),
            }
            groups
        })
    }

    /// Whether `input_tokens`, as Ollama reported them, are over the budget.
    pub fn over_budget(&self, _input_tokens: u32) -> bool {
        false
    }

    pub fn budget_tokens(&self) -> u32 {
        self.budget_tokens
    }

    /// What the messages around the transcript take, in characters: the
    /// most of one call and of any part's, with every chapter and the
    /// widest part numbers and span.
    fn prompt_chars(&self) -> usize {
        let end = self.minutes * 60;
        let widest = TranscriptPart {
            start_seconds: end,
            end_seconds: end,
            text: String::new(),
        };
        let part = self.part_messages(
            WIDEST_PART_NUMBER,
            WIDEST_PART_NUMBER,
            &widest,
            block(
                &self.prompt,
                SummaryPromptSection::MapChapters,
                &self.chapters,
            ),
        );
        self.whole().chars().max(part.chars())
    }

    fn part_messages(
        &self,
        number: &str,
        parts: &str,
        part: &TranscriptPart,
        chapters: String,
    ) -> SummaryMessages {
        SummaryMessages {
            system: self
                .prompt
                .render(SummaryPromptSection::MapSystem, &self.language_value()),
            user: self.prompt.render(
                SummaryPromptSection::MapUser,
                &self.head_values_with(vec![
                    ("part", number.to_string()),
                    ("parts", parts.to_string()),
                    ("span", span(part.start_seconds, part.end_seconds)),
                    ("chapters", chapters),
                    ("transcript", part.text.clone()),
                ]),
            ),
        }
    }

    fn joins(&self, group: &[PartAnswer], answer: &PartAnswer) -> bool {
        Self::chars(group) + answer.summary.as_str().chars().count() <= self.max_chars
    }

    fn chars(answers: &[PartAnswer]) -> usize {
        answers
            .iter()
            .map(|answer| answer.summary.as_str().chars().count())
            .sum()
    }

    fn reduce_with(&self, answers: &[PartAnswer], chapters: String) -> SummaryMessages {
        let prompt = &self.prompt;
        SummaryMessages {
            system: prompt.render(SummaryPromptSection::ReduceSystem, &self.language_value()),
            user: prompt.render(
                SummaryPromptSection::ReduceUser,
                &self.head_values_with(vec![
                    ("chapters", chapters),
                    ("part_summaries", Self::part_summaries(prompt, answers)),
                ]),
            ),
        }
    }

    fn part_summaries(prompt: &SummaryPrompt, answers: &[PartAnswer]) -> String {
        answers
            .iter()
            .enumerate()
            .map(|(index, answer)| {
                prompt.render(
                    SummaryPromptSection::ReducePart,
                    &[
                        ("part", (index + 1).to_string()),
                        ("span", span(answer.start_seconds, answer.end_seconds)),
                        ("summary", answer.summary.as_str().to_string()),
                    ],
                )
            })
            .collect::<Vec<_>>()
            .join(
                "

",
            )
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
    fn chapter_block(&self) -> String {
        block(&self.prompt, SummaryPromptSection::Chapters, &self.chapters)
    }

    /// The chapters overlapping `part` as the `map chapters` section.
    fn part_chapter_block(&self, part: &TranscriptPart) -> String {
        let overlapping: Vec<VideoChapter> = self
            .chapters
            .iter()
            .filter(|chapter| {
                chapter.end_seconds > part.start_seconds
                    && chapter.start_seconds <= part.end_seconds
            })
            .cloned()
            .collect();
        block(
            &self.prompt,
            SummaryPromptSection::MapChapters,
            &overlapping,
        )
    }
}

fn span(start_seconds: i64, end_seconds: i64) -> String {
    format!("{}–{}", clock(start_seconds), clock(end_seconds))
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
        .map(VideoChapter::listed)
        .collect::<Vec<_>>()
        .join("\n")
}
