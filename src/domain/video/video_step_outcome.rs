use chrono::{DateTime, Utc};

/// What one run of a per-video step left to do — see the `video-retries`
/// capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoStepOutcome {
    /// Done, Unavailable, Failed, or nothing to do: no further run needed.
    Settled,
    /// The step is Retrying (or ran before it was due): run it again `at`.
    RetryAt { at: DateTime<Utc>, reason: String },
}

impl VideoStepOutcome {
    /// Retrying until `next_attempt_at`, or settled once the step gave up
    /// or the video is gone (`None`).
    pub fn retry_at_or_settled(next_attempt_at: Option<DateTime<Utc>>, reason: String) -> Self {
        next_attempt_at.map_or(Self::Settled, |at| Self::RetryAt { at, reason })
    }
}
