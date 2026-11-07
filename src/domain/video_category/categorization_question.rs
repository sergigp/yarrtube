use crate::domain::category::Category;
use crate::domain::llm_server::LlmChat;
use crate::domain::preferences::CategorizationSettings;
use crate::domain::video::{Video, VideoChapter};
use crate::domain::video_summary::SummaryText;

/// Context of one categorization call: prompts, details and answer fit.
pub const CONTEXT_LENGTH: u32 = 4096;
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
        String::new()
    }

    /// The request for this question under `settings`.
    pub fn chat(&self, _settings: &CategorizationSettings, _thinking: bool) -> LlmChat {
        LlmChat {
            model: String::new(),
            system: String::new(),
            user: String::new(),
            context_length: 0,
            temperature: 0.0,
            thinking: false,
            max_output_tokens: 0,
            format: None,
        }
    }
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
        let answer: serde_json::Value = serde_json::from_str(content).ok()?;
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
