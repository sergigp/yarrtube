use super::late_data_outcome::LateDataOutcome;
use super::late_video_data::{RECHECK_AFTER, YOUNG_FOR};
use super::step_state::StepState;
use super::step_status::StepStatus;
use chrono::{DateTime, Utc};

/// The episode's subtitles — late YouTube data, see the `late-video-data`
/// capability: the fetch's step state, when it was last checked and the
/// subtitle file once fetched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subtitles {
    pub state: StepState,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub filename: Option<String>,
}

impl Subtitles {
    /// Never checked, no file.
    pub fn pending() -> Self {
        Self {
            state: StepState::pending(),
            last_checked_at: None,
            filename: None,
        }
    }

    /// The language code the fetched subtitle file is named with (`en` for
    /// `… .en.srt`); `None` unless the subtitles are Done.
    pub fn language(&self) -> Option<String> {
        self.filename
            .as_deref()
            .filter(|_| self.state.status == StepStatus::Done)
            .and_then(|filename| filename.strip_suffix(".srt"))
            .and_then(|stem| stem.rsplit_once('.'))
            .map(|(_, language)| language.to_string())
    }

    /// Whether the subtitles are due for another check: Pending and never
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

    /// Whether the subtitles are still to be obtained: Pending or Retrying.
    pub fn is_unsettled(&self) -> bool {
        matches!(
            self.state.status,
            StepStatus::Pending | StepStatus::Retrying
        )
    }

    /// The fetched subtitle file: Done, checked `now`.
    pub fn fetched(self, filename: impl Into<String>, now: DateTime<Utc>) -> Self {
        Self {
            state: self.state.succeed(),
            last_checked_at: Some(now),
            filename: Some(filename.into()),
        }
    }

    /// Records a check that ran `now`: `Complete` makes the subtitles Done;
    /// `Incomplete` Unavailable when the video is old, else Pending; `Failed`
    /// one more failed attempt (Retrying, or Failed on the last one).
    pub fn record_check(
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
            ..self
        }
    }

    /// Retrying or Failed subtitles back to Pending, never checked; any
    /// others unchanged.
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    #[test]
    fn it_should_be_due_if_never_checked() {
        assert!(Subtitles::pending().is_due(now()));
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
            Subtitles::pending()
                .record_check(LateDataOutcome::Incomplete, published_at, now())
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
        let subtitles = Subtitles::pending().record_check(LateDataOutcome::Complete, now(), now());

        assert_eq!(subtitles.state.status, StepStatus::Done);
    }

    #[test]
    fn it_should_report_the_language_from_its_filename() {
        let filename = "Season 1/S01E003 - My Video.en.srt";

        let languages = [
            Subtitles::pending().fetched(filename, now()),
            Subtitles::pending(),
            Subtitles {
                filename: Some(filename.to_string()),
                ..Subtitles::unavailable_at(None)
            },
        ]
        .map(|subtitles| subtitles.language());

        assert_eq!(languages, [Some("en".to_string()), None, None]);
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 3, 15, 12, 0, 0).unwrap()
    }

    fn checked_at(at: DateTime<Utc>) -> Subtitles {
        Subtitles {
            last_checked_at: Some(at),
            ..Subtitles::pending()
        }
    }
}
