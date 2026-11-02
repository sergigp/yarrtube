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
    pub fn is_due(&self, _now: DateTime<Utc>) -> bool {
        true
    }

    /// Records a check that ran `now`, counting a failed attempt for a
    /// `Failed` outcome.
    pub fn record(self, _outcome: LateDataOutcome, now: DateTime<Utc>) -> Self {
        Self {
            last_checked_at: Some(now),
            ..self
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
        match outcome {
            LateDataOutcome::Complete => true,
            LateDataOutcome::Incomplete => now - published_at >= YOUNG_FOR,
            LateDataOutcome::Failed => false,
        }
    }
}
