use super::download_failure::DownloadFailure;
use super::downloaded_file::DownloadedFile;
use super::step_state::StepState;
use super::step_status::StepStatus;
use super::video_status::VideoStatus;
use chrono::{DateTime, Utc};

/// The video's download — see the `video-download` and `video-retries`
/// capabilities: a step like the others, plus In Progress (`started`) on top
/// of a Pending or Retrying state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Download {
    pub state: StepState,
    /// In Progress: running on top of a Pending or Retrying state.
    pub started: bool,
    /// Discarded by the user (see the `video-discard` capability): never
    /// attempted again until its source restores it.
    pub discarded: bool,
    /// Why the last attempt failed; kept while started and when Unavailable,
    /// cleared on success and retry.
    pub failure: Option<String>,
    /// Some only once Done.
    pub file: Option<DownloadedFile>,
}

impl Download {
    /// Never attempted, no file.
    pub fn pending() -> Self {
        Self {
            state: StepState::pending(),
            started: false,
            discarded: false,
            failure: None,
            file: None,
        }
    }

    /// The download's lifecycle status: In Progress while started, else its
    /// step's status (Done is Downloaded).
    pub fn status(&self) -> VideoStatus {
        if self.discarded {
            return VideoStatus::Discarded;
        }
        if self.started {
            return VideoStatus::InProgress;
        }
        match self.state.status {
            StepStatus::Pending => VideoStatus::Pending,
            StepStatus::Retrying => VideoStatus::Retrying,
            StepStatus::Failed => VideoStatus::Failed,
            StepStatus::Done => VideoStatus::Downloaded,
            StepStatus::Unavailable => VideoStatus::Unavailable,
        }
    }

    /// Whether the user discarded it.
    pub fn is_discarded(&self) -> bool {
        self.discarded
    }

    /// Whether it can be discarded: Pending, Retrying, Failed or
    /// Downloaded, not started nor already discarded.
    pub fn is_discardable(&self) -> bool {
        matches!(
            self.status(),
            VideoStatus::Pending
                | VideoStatus::Retrying
                | VideoStatus::Failed
                | VideoStatus::Downloaded
        )
    }

    pub fn is_downloaded(&self) -> bool {
        self.status() == VideoStatus::Downloaded
    }

    /// Whether a download should run at `now`: started, Pending, or Retrying
    /// with its next attempt due; never while discarded.
    pub fn is_due(&self, now: DateTime<Utc>) -> bool {
        !self.discarded && (self.started || self.state.is_due(now))
    }

    /// Unavailable (not discarded, not started) with a recheckable reason or
    /// none recorded.
    pub fn is_recheckable_unavailable(&self) -> bool {
        false
    }

    /// When a Retrying download's next attempt is due; none while started.
    pub fn next_attempt_at(&self) -> Option<DateTime<Utc>> {
        self.state.next_attempt_at().filter(|_| !self.started)
    }

    /// The downloaded file's path, once Done.
    pub fn filename(&self) -> Option<&str> {
        self.file.as_ref().map(|file| file.filename.as_str())
    }

    /// The downloaded file's duration, once Done and known.
    pub fn duration_seconds(&self) -> Option<i64> {
        self.file.as_ref().and_then(|file| file.duration_seconds)
    }

    /// Whether the downloaded file is in the layout that predates the
    /// TV-show layout.
    pub fn is_in_legacy_layout(&self) -> bool {
        self.file
            .as_ref()
            .is_some_and(DownloadedFile::is_in_legacy_layout)
    }

    /// In Progress, its state (and attempts) kept.
    pub fn start(self) -> Self {
        Self {
            started: true,
            ..self
        }
    }

    /// Done with `file`, its failed attempts and failure cleared.
    pub fn succeed(self, file: DownloadedFile) -> Self {
        Self {
            state: self.state.succeed(),
            started: false,
            discarded: self.discarded,
            failure: None,
            file: Some(file),
        }
    }

    /// One more failed attempt and why: Retrying, or Failed on the last.
    pub fn fail(self, reason: impl Into<String>, now: DateTime<Utc>) -> Self {
        Self {
            state: self.state.fail(now),
            started: false,
            failure: Some(reason.into()),
            ..self
        }
    }

    /// Unavailable, its failed attempts cleared and `failure`'s reason kept.
    pub fn unavailable(self, failure: &DownloadFailure) -> Self {
        Self {
            state: self.state.unavailable(),
            started: false,
            failure: Some(failure.reason().to_string()),
            ..self
        }
    }

    /// A Retrying or Failed download back to Pending, its failure cleared;
    /// a started one, or any other, unchanged.
    pub fn retry(self) -> Self {
        if self.started || !matches!(self.state.status, StepStatus::Retrying | StepStatus::Failed) {
            return self;
        }
        Self {
            state: self.state.retry(),
            failure: None,
            ..self
        }
    }

    /// Discarded: Pending with no attempts, no failure and no
    /// file.
    pub fn discard(self) -> Self {
        Self {
            discarded: true,
            ..Self::pending()
        }
    }

    /// Pending with no attempts, no failure and no file when discarded;
    /// unchanged otherwise.
    pub fn restore(self) -> Self {
        if !self.discarded {
            return self;
        }
        Self::pending()
    }

    /// The file recorded as `file` (a relocation or quality upgrade), state
    /// unchanged.
    pub fn with_file(self, file: Option<DownloadedFile>) -> Self {
        Self { file, ..self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::video::retry_attempts::{RetryAttempts, retry_delay};
    use chrono::Duration;

    #[test]
    fn it_should_stay_due_while_started() {
        let now = fixed_timestamp();
        let download = retrying_download(now).start();

        assert!(download.is_due(now));
    }

    #[test]
    fn it_should_not_be_due_if_discarded() {
        let now = fixed_timestamp();

        let due = [Download::pending(), Download::pending().discard()]
            .map(|download| download.is_due(now));

        assert_eq!(due, [true, false]);
    }

    #[test]
    fn it_should_keep_its_attempts_when_started_from_retrying() {
        let now = fixed_timestamp();
        let retrying = retrying_download(now);

        let started = retrying.clone().start();

        assert_eq!(
            started,
            Download {
                started: true,
                ..retrying
            }
        );
    }

    #[test]
    fn it_should_leave_a_started_download_unchanged_on_retry() {
        let now = fixed_timestamp();
        let started = retrying_download(now).start();

        assert_eq!(started.clone().retry(), started);
    }

    #[test]
    fn it_should_have_no_next_attempt_while_started() {
        let now = fixed_timestamp();
        let retrying = retrying_download(now);
        assert_eq!(retrying.next_attempt_at(), Some(now + retry_delay(1)));

        assert_eq!(retrying.start().next_attempt_at(), None);
    }

    fn retrying_download(now: DateTime<Utc>) -> Download {
        let download = Download::pending().fail("boom", now);
        assert_eq!(
            download.state.attempts,
            RetryAttempts {
                failed: 1,
                last_failed_at: Some(now)
            }
        );
        download
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(0, 0).unwrap() + Duration::days(365)
    }
}
