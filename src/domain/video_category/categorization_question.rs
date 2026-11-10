use crate::domain::category::Category;
use crate::domain::llm_server::{LlmChat, MIN_CHAT_TIMEOUT};
use crate::domain::preferences::{CategorizationSettings, ContextLength};
use crate::domain::video::{Video, VideoChapter};
use crate::domain::video_summary::SummaryText;
use std::time::Duration;

/// Room for one category's thinking and answer: 12,288 for 9, as measured
/// in `evals/categories`.
pub const OUTPUT_TOKENS_PER_CATEGORY: u32 = 1365;
/// The least a call gets, whatever it asks: stops runaway thinking loops
/// without cutting a single category short.
pub const MIN_OUTPUT_TOKENS: u32 = 3072;

/// What the model is asked about a video and some categories, in one call
/// — see the `video-categorization` capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategorizationQuestion {
    title: String,
    source_name: String,
    minutes: i64,
    chapters: Vec<VideoChapter>,
    summary: String,
    categories: Vec<Category>,
}

impl CategorizationQuestion {
    pub fn new(
        video: &Video,
        source_name: &str,
        summary: &SummaryText,
        categories: &[Category],
    ) -> Self {
        Self {
            title: video.title.clone(),
            source_name: source_name.to_string(),
            minutes: video.duration_seconds.unwrap_or(0) / 60,
            chapters: video.chapters.clone().unwrap_or_default(),
            summary: summary.as_str().to_string(),
            categories: categories.to_vec(),
        }
    }

    /// The video's details, the categories and what to answer.
    pub fn user_message(&self) -> String {
        let category = &self.categories[0];
        format!(
            "Title: {}\nChannel: {}\nDuration: {} minutes\nChapters:\n{}\nSummary: {}\n\n\
             Category: {}\nDescription:\n{}\n\n\
             Answer in JSON: \"reason\", one sentence on what the video covers and how it fits, \
             then \"belongs\", true or false.",
            self.title,
            self.source_name,
            self.minutes,
            self.chapter_list(),
            self.summary,
            category.name.as_str(),
            category
                .ai_prompt
                .as_ref()
                .map_or("", |prompt| prompt.as_str()),
        )
    }

    /// The request for this question under `settings` and `context_length`,
    /// deterministic and bound to the answer's schema.
    pub fn chat(
        &self,
        settings: &CategorizationSettings,
        context_length: ContextLength,
        thinking: bool,
    ) -> LlmChat {
        LlmChat {
            model: settings.model.as_str().to_string(),
            system: settings.effective_prompt().as_str().to_string(),
            user: self.user_message(),
            context_length: context_length.value(),
            temperature: settings.temperature.value(),
            thinking,
            max_output_tokens: self.max_output_tokens(context_length),
            format: Some(answer_schema()),
            timeout: self.timeout(context_length),
        }
    }

    /// 1,365 per category, at least 3,072, at most `context_length`.
    pub fn max_output_tokens(&self, context_length: ContextLength) -> u32 {
        MIN_OUTPUT_TOKENS.min(context_length.value())
    }

    /// The output cap at 5 tokens a second, at least 30 minutes.
    pub fn timeout(&self, _context_length: ContextLength) -> Duration {
        MIN_CHAT_TIMEOUT
    }

    /// Each category with its usable answer, if any.
    pub fn answers(&self, content: &str) -> Vec<(&Category, Option<CategorizationAnswer>)> {
        self.categories
            .iter()
            .map(|category| (category, CategorizationAnswer::parse(content)))
            .collect()
    }

    /// One chapter per line, or `(none)`.
    fn chapter_list(&self) -> String {
        match self.chapters.is_empty() {
            true => "(none)".to_string(),
            false => self
                .chapters
                .iter()
                .map(VideoChapter::listed)
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}

/// The JSON schema of a `CategorizationAnswer`, sent as Ollama's `format`.
fn answer_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "reason": {"type": "string"},
            "belongs": {"type": "boolean"},
        },
        "required": ["reason", "belongs"],
    })
}

/// The model's usable answer to a `CategorizationQuestion`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategorizationAnswer {
    pub belongs: bool,
    pub reason: String,
}

impl CategorizationAnswer {
    /// After the last `</think>`, the JSON object with a boolean `belongs`;
    /// `None` otherwise.
    pub fn parse(content: &str) -> Option<Self> {
        let answer = content
            .rsplit_once("</think>")
            .map_or(content, |(_, after)| after);
        let object = answer.get(answer.find('{')?..=answer.rfind('}')?)?;
        let answer: serde_json::Value = serde_json::from_str(object).ok()?;
        Some(Self {
            belongs: answer.get("belongs")?.as_bool()?,
            reason: answer
                .get("reason")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_parse_a_json_answer() {
        assert_eq!(
            CategorizationAnswer::parse(r#"{"reason": "A Sicilian game.", "belongs": true}"#),
            Some(sicilian_answer(true))
        );
    }

    #[test]
    fn it_should_parse_the_answer_after_the_last_leaked_thinking() {
        assert_eq!(
            CategorizationAnswer::parse(
                r#"<think>{"belongs": true}</think><think>Hmm.</think>{"reason": "A Sicilian game.", "belongs": false}"#
            ),
            Some(sicilian_answer(false))
        );
    }

    #[test]
    fn it_should_parse_a_json_answer_surrounded_by_prose() {
        assert_eq!(
            CategorizationAnswer::parse(
                r#"Here it is: {"reason": "A Sicilian game.", "belongs": true} Hope it helps."#
            ),
            Some(sicilian_answer(true))
        );
    }

    #[test]
    fn it_should_not_parse_an_answer_without_a_json_object() {
        assert_eq!(CategorizationAnswer::parse("It belongs."), None);
    }

    #[test]
    fn it_should_not_parse_an_answer_whose_json_object_is_only_in_the_thinking() {
        assert_eq!(
            CategorizationAnswer::parse(r#"<think>{"belongs": true}</think>It belongs."#),
            None
        );
    }

    #[test]
    fn it_should_not_parse_an_answer_with_invalid_json() {
        assert_eq!(
            CategorizationAnswer::parse(r#"{"reason": "A Sicilian game.", "belongs": tru}"#),
            None
        );
    }

    #[test]
    fn it_should_not_parse_an_answer_without_belongs() {
        assert_eq!(
            CategorizationAnswer::parse(r#"{"reason": "A Sicilian game."}"#),
            None
        );
    }

    #[test]
    fn it_should_not_parse_an_answer_whose_belongs_is_not_a_boolean() {
        assert_eq!(
            CategorizationAnswer::parse(r#"{"reason": "A Sicilian game.", "belongs": "true"}"#),
            None
        );
    }

    fn sicilian_answer(belongs: bool) -> CategorizationAnswer {
        CategorizationAnswer {
            belongs,
            reason: "A Sicilian game.".to_string(),
        }
    }
}
