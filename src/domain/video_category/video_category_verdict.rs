use super::CategorizationAnswer;
use crate::domain::category::CategoryId;
use crate::domain::shared::ValidationError;
use crate::domain::video::VideoRecordId;
use chrono::{DateTime, Utc};

/// Whether a video belongs to a category, as decided — see the
/// `video-categorization` capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Yes,
    No,
    /// The model couldn't answer under the current settings.
    Unavailable,
}

impl Verdict {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Yes => "YES",
            Self::No => "NO",
            Self::Unavailable => "UNAVAILABLE",
        }
    }

    pub fn parse(value: &str) -> Result<Self, ValidationError> {
        match value {
            "YES" => Ok(Self::Yes),
            "NO" => Ok(Self::No),
            "UNAVAILABLE" => Ok(Self::Unavailable),
            other => Err(ValidationError(format!("unknown verdict '{other}'"))),
        }
    }
}

/// Who decided a verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerdictAuthor {
    Ai,
    User,
}

impl VerdictAuthor {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ai => "AI",
            Self::User => "USER",
        }
    }

    pub fn parse(value: &str) -> Result<Self, ValidationError> {
        match value {
            "AI" => Ok(Self::Ai),
            "USER" => Ok(Self::User),
            other => Err(ValidationError(format!("unknown verdict author '{other}'"))),
        }
    }
}

/// One (video, category) pair's verdict. A pair holds at most one and, while
/// it does, is never evaluated again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoCategoryVerdict {
    pub video_id: VideoRecordId,
    pub category_id: CategoryId,
    pub verdict: Verdict,
    pub decided_by: VerdictAuthor,
    /// The model that decided it; AI verdicts only.
    pub model: Option<String>,
    /// The model's one-sentence reason; AI verdicts only.
    pub reason: Option<String>,
    pub decided_at: DateTime<Utc>,
}

impl VideoCategoryVerdict {
    /// The AI's `answer`: `yes` when it belongs, else `no`.
    pub fn answered(
        video_id: VideoRecordId,
        category_id: CategoryId,
        answer: CategorizationAnswer,
        model: &str,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            video_id,
            category_id,
            verdict: match answer.belongs {
                true => Verdict::Yes,
                false => Verdict::No,
            },
            decided_by: VerdictAuthor::Ai,
            model: Some(model.to_string()),
            reason: Some(answer.reason),
            decided_at: now,
        }
    }

    /// The AI couldn't answer for this pair under the current settings.
    pub fn unavailable(
        video_id: VideoRecordId,
        category_id: CategoryId,
        model: &str,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            video_id,
            category_id,
            verdict: Verdict::Unavailable,
            decided_by: VerdictAuthor::Ai,
            model: Some(model.to_string()),
            reason: None,
            decided_at: now,
        }
    }

    pub fn is_yes(&self) -> bool {
        self.verdict == Verdict::Yes
    }
}
