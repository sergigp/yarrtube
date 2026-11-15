use super::late_data_outcome::LateDataOutcome;
use super::late_video_data::{RECHECK_AFTER, YOUNG_FOR};
use super::step_state::StepState;
use super::step_status::StepStatus;
use super::video_chapter::VideoChapter;
use chrono::{DateTime, Utc};

/// The video's YouTube chapters — late YouTube data, see the
/// `late-video-data` capability: the checks' step state, when they last ran
/// and the chapters found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chapters {
    pub state: StepState,
    pub last_checked_at: Option<DateTime<Utc>>,
    /// `None` while never checked, `Some(vec![])` once checked and found to
    /// have none.
    pub found: Option<Vec<VideoChapter>>,
}

impl Chapters {
    /// Never checked, none found.
    pub fn pending() -> Self {
        Self {
            state: StepState::pending(),
            last_checked_at: None,
            found: None,
        }
    }

    /// Whether the chapters are due for another check: Pending and never
    /// checked or checked at least `RECHECK_AFTER` ago, or Retrying with
    /// their next attempt due.
    pub fn is_due(&self, now: DateTime<Utc>) -> bool {
        match self.state.status {
            StepStatus::Pending => self
                .last_checked_at
                .is_none_or(|checked_at| now - checked_at >= RECHECK_AFTER),
            StepStatus::Retrying => self.state.is_due(now),
            _ => false,
        }
    }

    /// Whether the chapters are still to be obtained: Pending or Retrying.
    pub fn is_unsettled(&self) -> bool {
        matches!(
            self.state.status,
            StepStatus::Pending | StepStatus::Retrying
        )
    }

    /// Records a check that ran `now`: `Some` non-empty chapters are
    /// `Complete` (Done), `Some(vec![])` `Incomplete` (Unavailable when the
    /// video is old, else Pending), `None` a failed check (one more failed
    /// attempt), keeping the chapters already found, if any.
    pub fn record_check(
        self,
        found: Option<Vec<VideoChapter>>,
        published_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Self {
        let outcome = match &found {
            Some(chapters) if !chapters.is_empty() => LateDataOutcome::Complete,
            Some(_) => LateDataOutcome::Incomplete,
            None => LateDataOutcome::Failed,
        };
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
            found: found.or(self.found),
        }
    }

    /// Retrying or Failed chapters back to Pending, never checked; any
    /// others unchanged.
    pub fn retry(self) -> Self {
        match self.state.status {
            StepStatus::Retrying | StepStatus::Failed => self.reset_checks(),
            _ => self,
        }
    }

    /// The checks back to Pending, never checked, keeping the chapters
    /// found (a redownload refetches them).
    pub fn reset_checks(self) -> Self {
        Self {
            state: StepState::pending(),
            last_checked_at: None,
            ..self
        }
    }
}
