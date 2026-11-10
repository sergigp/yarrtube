use super::LateDataOutcome;
use super::retry_attempts::RetryAttempts;
use super::step_state::StepState;
use super::step_status::StepStatus;
use chrono::{DateTime, Duration, Utc};

/// How long after its publish time a video counts as young: its late data
/// may still show up.
pub const YOUNG_FOR: Duration = Duration::days(7);
/// How long Pending data waits after a conclusive check before it is due
/// again.
pub const RECHECK_AFTER: Duration = Duration::hours(24);

/// One piece of late YouTube data (subtitles, chapters, downloaded quality)
/// — see the `late-video-data` capability: its step state and when it was
/// last checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LateDataChecks {
    pub state: StepState,
    pub last_checked_at: Option<DateTime<Utc>>,
}

impl LateDataChecks {
    /// The checks as stored: a check counts as a failed attempt's time only
    /// while some attempt failed.
    pub fn restored(
        status: StepStatus,
        failed: u32,
        last_checked_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            state: StepState {
                status,
                attempts: RetryAttempts {
                    failed,
                    last_failed_at: last_checked_at.filter(|_| failed > 0),
                },
            },
            last_checked_at,
        }
    }

    /// Records a check that ran `now`: `Complete` makes the data Done;
    /// `Incomplete` Unavailable when the video is old, else Pending; `Failed`
    /// one more failed attempt (Retrying, or Failed on the last one).
    pub fn record(
        self,
        outcome: LateDataOutcome,
        published_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Self {
        let state = match outcome {
            LateDataOutcome::Complete => self.state.succeed(),
            LateDataOutcome::Incomplete if now - published_at >= YOUNG_FOR => {
                self.state.unavailable()
            }
            LateDataOutcome::Incomplete => StepState::pending(),
            LateDataOutcome::Failed => self.state.fail(now),
        };
        Self {
            state,
            last_checked_at: Some(now),
        }
    }

    /// Whether the data is due for another check: Pending and never checked
    /// or checked at least `RECHECK_AFTER` ago, or Retrying with its next
    /// attempt due.
    pub fn is_due(&self, now: DateTime<Utc>) -> bool {
        match self.state.status {
            StepStatus::Pending => self
                .last_checked_at
                .is_none_or(|checked_at| now - checked_at >= RECHECK_AFTER),
            StepStatus::Retrying => self.state.is_due(now),
            _ => false,
        }
    }

    /// Whether the data is still to be obtained: Pending or Retrying.
    pub fn is_unsettled(&self) -> bool {
        matches!(
            self.state.status,
            StepStatus::Pending | StepStatus::Retrying
        )
    }

    /// Test fixture: data last checked at `last_checked_at` with `failed`
    /// consecutive failed checks — Pending without failures, else Retrying
    /// (Failed from `MAX_ATTEMPTS`), the last failed at the last check.
    #[cfg(test)]
    pub fn checked(last_checked_at: Option<DateTime<Utc>>, failed: u32) -> Self {
        use super::retry_attempts::{MAX_ATTEMPTS, RetryAttempts};
        let status = match failed {
            0 => StepStatus::Pending,
            f if f >= MAX_ATTEMPTS => StepStatus::Failed,
            _ => StepStatus::Retrying,
        };
        Self {
            state: StepState {
                status,
                attempts: RetryAttempts {
                    failed,
                    last_failed_at: last_checked_at.filter(|_| failed > 0),
                },
            },
            last_checked_at,
        }
    }

    /// Test fixture: data found (Done), never checked again.
    #[cfg(test)]
    pub fn done() -> Self {
        Self::done_at(None)
    }

    /// Test fixture: data found (Done) by the check at `last_checked_at`.
    #[cfg(test)]
    pub fn done_at(last_checked_at: Option<DateTime<Utc>>) -> Self {
        Self {
            state: StepState::pending().succeed(),
            last_checked_at,
        }
    }

    /// Test fixture: data found missing on an old video (Unavailable) by the
    /// check at `last_checked_at`.
    #[cfg(test)]
    pub fn unavailable_at(last_checked_at: Option<DateTime<Utc>>) -> Self {
        Self {
            state: StepState::pending().unavailable(),
            last_checked_at,
        }
    }

    /// Any Retrying or Failed data back to Pending, never checked.
    pub fn retry(self) -> Self {
        match self.state.status {
            StepStatus::Retrying | StepStatus::Failed => Self::default(),
            _ => self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn it_should_be_due_if_never_checked() {
        assert!(LateDataChecks::default().is_due(now()));
    }

    #[test]
    fn it_should_be_due_once_a_day_has_passed_since_the_last_check() {
        assert_eq!(
            (
                checked_at(now() - Duration::hours(24)).is_due(now()),
                checked_at(now() - Duration::hours(24) + Duration::seconds(1)).is_due(now()),
            ),
            (true, false)
        );
    }

    #[test]
    fn it_should_make_missing_data_unavailable_once_the_video_is_a_week_old() {
        let record = |published_at| {
            LateDataChecks::default()
                .record(LateDataOutcome::Incomplete, published_at, now())
                .state
                .status
        };

        assert_eq!(
            (
                record(now() - Duration::days(7)),
                record(now() - Duration::days(7) + Duration::seconds(1)),
            ),
            (StepStatus::Unavailable, StepStatus::Pending)
        );
    }

    #[test]
    fn it_should_make_found_data_done_even_on_a_young_video() {
        let checks = LateDataChecks::default().record(LateDataOutcome::Complete, now(), now());

        assert_eq!(checks.state.status, StepStatus::Done);
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 3, 15, 12, 0, 0).unwrap()
    }

    fn checked_at(at: DateTime<Utc>) -> LateDataChecks {
        LateDataChecks {
            state: StepState::pending(),
            last_checked_at: Some(at),
        }
    }
}
