use crate::domain::category::Category;
use crate::domain::llm_server::LlmChat;
use crate::domain::preferences::CategorizationSettings;
use crate::domain::video::{Video, VideoChapter};
use crate::domain::video_summary::SummaryText;

/// Stops runaway thinking loops.
pub const MAX_OUTPUT_TOKENS: u32 = 3072;

/// What the model is asked about one (video, category) pair — see the
/// `video-categorization` capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategorizationQuestion {
    title: String,
    source_name: String,
    minutes: i64,
    chapters: Vec<VideoChapter>,
    summary: String,
    category: Category,
}

impl CategorizationQuestion {
    pub fn new(
        video: &Video,
        source_name: &str,
        summary: &SummaryText,
        category: &Category,
    ) -> Self {
        Self {
            title: video.title.clone(),
            source_name: source_name.to_string(),
            minutes: video.duration_seconds.unwrap_or(0) / 60,
            chapters: video.chapters.clone().unwrap_or_default(),
            summary: summary.as_str().to_string(),
            category: category.clone(),
        }
    }

    /// The video's details, the category and what to answer.
    pub fn user_message(&self) -> String {
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
            self.category.name.as_str(),
            self.category
                .ai_prompt
                .as_ref()
                .map_or("", |prompt| prompt.as_str()),
        )
    }

    /// The request for this question under `settings`, deterministic and
    /// bound to the answer's schema.
    pub fn chat(&self, settings: &CategorizationSettings, thinking: bool) -> LlmChat {
        LlmChat {
            model: settings.model.as_str().to_string(),
            system: settings.effective_prompt().as_str().to_string(),
            user: self.user_message(),
            context_length: settings.context_length.value(),
            temperature: settings.temperature.value(),
            thinking,
            max_output_tokens: MAX_OUTPUT_TOKENS.min(settings.context_length.value()),
            format: Some(answer_schema()),
        }
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
