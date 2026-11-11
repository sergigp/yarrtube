use super::late_data_outcome::LateDataOutcome;
use super::late_video_data::{RECHECK_AFTER, YOUNG_FOR};
use super::step_state::StepState;
use super::step_status::StepStatus;
use crate::domain::shared::Quality;
use chrono::{DateTime, Utc};

/// The quality the video was downloaded at — late YouTube data, see the
/// `late-video-data` capability: the checks' step state, when they last ran
/// and the downloaded file's height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownloadedQuality {
    pub state: StepState,
    pub last_checked_at: Option<DateTime<Utc>>,
    /// The height in pixels of the downloaded file, when reported.
    pub height: Option<u32>,
}

impl DownloadedQuality {
    /// Never checked, no height.
    pub fn pending() -> Self {
        Self {
            state: StepState::pending(),
            last_checked_at: None,
            height: None,
        }
    }

    /// Whether the quality is due for another check: Pending and never
    /// checked or checked at least `RECHECK_AFTER` ago, or Retrying with its
    /// next attempt due.
    pub fn is_due(&self, now: DateTime<Utc>) -> bool {
        match self.state.status {
            StepStatus::Pending => self
                .last_checked_at
                .is_none_or(|checked_at| now - checked_at >= RECHECK_AFTER),
            StepStatus::Retrying => self.state.is_due(now),
            _ => false,
        }
    }

    /// Whether a better quality may still be obtained: Pending or Retrying.
    pub fn is_unsettled(&self) -> bool {
        matches!(
            self.state.status,
            StepStatus::Pending | StepStatus::Retrying
        )
    }

    /// Records the height a download or quality upgrade landed at for
    /// `quality`'s tier, overwriting the one recorded (even with none):
    /// Done when the video is old or the tier's target height is reached; a
    /// `degraded` one (SABR reported) counts as a failed check instead.
    pub fn record_download(
        self,
        quality: Quality,
        height: Option<u32>,
        degraded: bool,
        published_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Self {
        let outcome = match height {
            _ if degraded => LateDataOutcome::Failed,
            Some(height) if height >= quality.target_height() => LateDataOutcome::Complete,
            _ => LateDataOutcome::Incomplete,
        };
        Self {
            height,
            ..self.record_outcome(outcome, published_at, now)
        }
    }

    /// Records a check that found nothing better (`probed_height` the height
    /// the tier would get now) or failed (`None`).
    pub fn record_check(
        self,
        probed_height: Option<u32>,
        published_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Self {
        let outcome = match probed_height {
            Some(_) => LateDataOutcome::Incomplete,
            None => LateDataOutcome::Failed,
        };
        self.record_outcome(outcome, published_at, now)
    }

    /// A Retrying or Failed quality back to Pending, never checked; any
    /// other unchanged.
    pub fn retry(self) -> Self {
        match self.state.status {
            StepStatus::Retrying | StepStatus::Failed => Self {
                state: StepState::pending(),
                last_checked_at: None,
                ..self
            },
            _ => self,
        }
    }

    /// The checks after one that ran `now` with `outcome`: `Complete` Done;
    /// `Incomplete` Done too when the video is old (a quality no better is
    /// kept as it is, not Unavailable), else Pending; `Failed` one more
    /// failed attempt.
    fn record_outcome(
        self,
        outcome: LateDataOutcome,
        published_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Self {
        let is_old = now - published_at >= YOUNG_FOR;
        let state = match outcome {
            LateDataOutcome::Complete => self.state.succeed(),
            LateDataOutcome::Incomplete if is_old => self.state.succeed(),
            LateDataOutcome::Incomplete => StepState::pending(),
            LateDataOutcome::Failed => self.state.fail(now),
        };
        Self {
            state,
            last_checked_at: Some(now),
            ..self
        }
    }
}
