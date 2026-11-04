use super::SummaryText;
use crate::domain::preferences::SummaryModel;
use crate::domain::video::VideoRecordId;
use chrono::{DateTime, Utc};

/// What a video is about, written by an LLM from its transcript — see the
/// `video-summaries` capability. At most one per video.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoSummary {
    pub video_id: VideoRecordId,
    pub text: SummaryText,
    pub model: String,
    pub created_at: DateTime<Utc>,
}

impl VideoSummary {
    pub fn create(
        video_id: VideoRecordId,
        text: SummaryText,
        model: &SummaryModel,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            video_id,
            text,
            model: model.as_str().to_string(),
            created_at: now,
        }
    }
}
