use chrono::{DateTime, Utc};

/// What one run of a per-video step left to do — see the `video-retries`
/// capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoStepOutcome {
    /// Done, Unavailable, Failed, or nothing to do: no further run needed.
    Settled,
    /// The step is Retrying (or ran before it was due): run it again `at`.
    #[allow(dead_code)] // skeleton: used once its video-retries behaviour lands
    RetryAt { at: DateTime<Utc>, reason: String },
}
