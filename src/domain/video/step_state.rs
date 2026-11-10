use super::retry_attempts::RetryAttempts;
use super::step_status::StepStatus;
use chrono::{DateTime, Utc};

/// One per-video step's status and its consecutive failed attempts — see
/// the `video-retries` capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepState {
    pub status: StepStatus,
    pub attempts: RetryAttempts,
}

impl StepState {
    pub fn pending() -> Self {
        Self {
            status: StepStatus::Pending,
            attempts: RetryAttempts::default(),
        }
    }

    /// Done, its failed attempts cleared.
    pub fn succeed(self) -> Self {
        Self {
            status: StepStatus::Done,
            attempts: RetryAttempts::default(),
        }
    }

    /// Unavailable (YouTube doesn't have it), its failed attempts cleared.
    pub fn unavailable(self) -> Self {
        Self {
            status: StepStatus::Unavailable,
            attempts: RetryAttempts::default(),
        }
    }

    /// One more failed attempt, failed `now`: Retrying, or Failed on the
    /// last allowed one.
    pub fn fail(self, now: DateTime<Utc>) -> Self {
        let attempts = self.attempts.record_failure(now);
        let status = if attempts.gave_up() {
            StepStatus::Failed
        } else {
            StepStatus::Retrying
        };
        Self { status, attempts }
    }

    /// A Retrying or Failed step back to Pending; any other unchanged.
    pub fn retry(self) -> Self {
        match self.status {
            StepStatus::Retrying | StepStatus::Failed => Self::pending(),
            _ => self,
        }
    }

    /// Whether the step should be attempted at `now`: Pending, or Retrying
    /// with its next attempt due.
    pub fn is_due(&self, now: DateTime<Utc>) -> bool {
        match self.status {
            StepStatus::Pending => true,
            StepStatus::Retrying => self.next_attempt_at().is_none_or(|at| at <= now),
            _ => false,
        }
    }

    /// When the next attempt is due, only while Retrying.
    pub fn next_attempt_at(&self) -> Option<DateTime<Utc>> {
        self.attempts
            .next_attempt_at()
            .filter(|_| self.status == StepStatus::Retrying)
    }
}

impl Default for StepState {
    fn default() -> Self {
        Self::pending()
    }
}
