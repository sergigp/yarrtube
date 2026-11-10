use chrono::{DateTime, Duration, Utc};

/// How many consecutive failed attempts a step gets before it is Failed —
/// see the `video-retries` capability.
pub const MAX_ATTEMPTS: u32 = 11;

/// The wait after the 1st, 2nd, 3rd and every later consecutive failure.
const RETRY_DELAYS: [Duration; 4] = [
    Duration::minutes(10),
    Duration::hours(1),
    Duration::hours(6),
    Duration::hours(24),
];

/// How long a step waits after its `failed`-th consecutive failure.
pub fn retry_delay(failed: u32) -> Duration {
    let index = (failed.max(1) - 1).min(RETRY_DELAYS.len() as u32 - 1);
    RETRY_DELAYS[index as usize]
}

/// A step's consecutive failed attempts and when the last one failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RetryAttempts {
    pub failed: u32,
    pub last_failed_at: Option<DateTime<Utc>>,
}

impl RetryAttempts {
    /// One more failed attempt, failed `now`.
    pub fn record_failure(self, now: DateTime<Utc>) -> Self {
        Self {
            failed: self.failed + 1,
            last_failed_at: Some(now),
        }
    }

    /// Whether the step has used every attempt it is allowed.
    pub fn gave_up(&self) -> bool {
        self.failed >= MAX_ATTEMPTS
    }

    /// When the next attempt is due: `None` when none failed or it gave up.
    pub fn next_attempt_at(&self) -> Option<DateTime<Utc>> {
        self.last_failed_at
            .filter(|_| self.failed > 0 && !self.gave_up())
            .map(|failed_at| failed_at + retry_delay(self.failed))
    }
}
