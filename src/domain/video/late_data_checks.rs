use super::LateDataOutcome;
use chrono::{DateTime, Duration, Utc};

/// How long after its publish time a video counts as young: its late data
/// may still show up.
pub const YOUNG_FOR: Duration = Duration::days(7);
/// How long unsettled data waits after a check before it is due again.
pub const RECHECK_AFTER: Duration = Duration::hours(24);
/// How many failed checks settle an old video's data.
pub const MAX_FAILED_ATTEMPTS: u32 = 3;

/// The checks of one piece of late YouTube data (subtitles, chapters,
/// downloaded quality) — see the `late-video-data` capability: when it was
/// last checked and how many checks failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LateDataChecks {
    pub last_checked_at: Option<DateTime<Utc>>,
    pub failed_attempts: u32,
}

impl LateDataChecks {
    /// Whether unsettled data is due for another check: never checked, or
    /// last checked at least `RECHECK_AFTER` ago.
    pub fn is_due(&self, now: DateTime<Utc>) -> bool {
        self.last_checked_at
            .is_none_or(|checked_at| now - checked_at >= RECHECK_AFTER)
    }

    /// Whether the data has failed as many checks as it is allowed.
    pub fn failures_exhausted(&self) -> bool {
        self.failed_attempts >= MAX_FAILED_ATTEMPTS
    }

    /// Records a check that ran `now`, counting a failed attempt for a
    /// `Failed` outcome.
    pub fn record(self, outcome: LateDataOutcome, now: DateTime<Utc>) -> Self {
        Self {
            last_checked_at: Some(now),
            failed_attempts: match outcome {
                LateDataOutcome::Failed => self.failed_attempts + 1,
                _ => self.failed_attempts,
            },
        }
    }

    /// Whether the data is final after a check with `outcome` (already
    /// `record`ed): found, or missing/failed on an old video (failures only
    /// once `MAX_FAILED_ATTEMPTS` are reached).
    pub fn settles(
        &self,
        outcome: LateDataOutcome,
        published_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> bool {
        let is_old = now - published_at >= YOUNG_FOR;
        match outcome {
            LateDataOutcome::Complete => true,
            LateDataOutcome::Incomplete => is_old,
            LateDataOutcome::Failed => is_old && self.failures_exhausted(),
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
    fn it_should_settle_missing_data_once_the_video_is_a_week_old() {
        let checks = LateDataChecks::default().record(LateDataOutcome::Incomplete, now());

        assert_eq!(
            (
                checks.settles(
                    LateDataOutcome::Incomplete,
                    now() - Duration::days(7),
                    now()
                ),
                checks.settles(
                    LateDataOutcome::Incomplete,
                    now() - Duration::days(7) + Duration::seconds(1),
                    now()
                ),
            ),
            (true, false)
        );
    }

    #[test]
    fn it_should_settle_failed_checks_of_an_old_video_on_the_third_failure() {
        let published_at = now() - Duration::days(30);
        let third = LateDataChecks {
            last_checked_at: None,
            failed_attempts: 2,
        }
        .record(LateDataOutcome::Failed, now());
        let second = LateDataChecks {
            last_checked_at: None,
            failed_attempts: 1,
        }
        .record(LateDataOutcome::Failed, now());

        assert_eq!(
            (
                third.settles(LateDataOutcome::Failed, published_at, now()),
                second.settles(LateDataOutcome::Failed, published_at, now()),
            ),
            (true, false)
        );
    }

    #[test]
    fn it_should_settle_found_data_even_on_a_young_video() {
        let checks = LateDataChecks::default().record(LateDataOutcome::Complete, now());

        assert!(checks.settles(LateDataOutcome::Complete, now(), now()));
    }

    #[test]
    fn it_should_count_a_failed_attempt_only_for_a_failed_check() {
        let checks = LateDataChecks {
            last_checked_at: None,
            failed_attempts: 1,
        };

        assert_eq!(
            [
                checks.record(LateDataOutcome::Complete, now()),
                checks.record(LateDataOutcome::Incomplete, now()),
                checks.record(LateDataOutcome::Failed, now()),
            ],
            [
                LateDataChecks {
                    last_checked_at: Some(now()),
                    failed_attempts: 1,
                },
                LateDataChecks {
                    last_checked_at: Some(now()),
                    failed_attempts: 1,
                },
                LateDataChecks {
                    last_checked_at: Some(now()),
                    failed_attempts: 2,
                },
            ]
        );
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 3, 15, 12, 0, 0).unwrap()
    }

    fn checked_at(at: DateTime<Utc>) -> LateDataChecks {
        LateDataChecks {
            last_checked_at: Some(at),
            failed_attempts: 0,
        }
    }
}
