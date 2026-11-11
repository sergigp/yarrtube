use crate::domain::video::VideoStepOutcome;
use chrono::{DateTime, Utc};

/// How a task's handler completed it — see the `task-scheduling`
/// capability: done for good (having changed something, or nothing), or to
/// run again later with a reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskOutcome {
    Done,
    /// Done, and the run changed nothing: its end line reports `unchanged`.
    Unchanged,
    RunAgainAt {
        run_at: DateTime<Utc>,
        reason: String,
    },
}

impl From<VideoStepOutcome> for TaskOutcome {
    fn from(outcome: VideoStepOutcome) -> Self {
        match outcome {
            VideoStepOutcome::Settled => Self::Done,
            VideoStepOutcome::RetryAt { at, reason } => Self::RunAgainAt { run_at: at, reason },
        }
    }
}
