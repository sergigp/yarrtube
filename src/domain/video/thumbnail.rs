use super::step_state::StepState;
use chrono::{DateTime, Utc};

/// The episode's thumbnail — see the `video-thumbnails` capability: the
/// fetch's step state and the file once recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thumbnail {
    pub state: StepState,
    pub filename: Option<String>,
}

impl Thumbnail {
    /// Never fetched, no file.
    pub fn pending() -> Self {
        Self {
            state: StepState::pending(),
            filename: None,
        }
    }

    /// The file's name as an NFO references it, next to the episode.
    pub fn basename(&self) -> Option<String> {
        self.filename
            .as_deref()
            .and_then(|f| std::path::Path::new(f).file_name())
            .and_then(|f| f.to_str())
            .map(str::to_string)
    }

    /// A thumbnail fetched to `filename`: Done.
    pub fn succeed(self, filename: impl Into<String>) -> Self {
        Self {
            state: self.state.succeed(),
            filename: Some(filename.into()),
        }
    }

    /// The thumbnail a download wrote, else the one fetched ahead of it:
    /// Done if either, else unchanged.
    pub fn record_download(self, filename: Option<String>) -> Self {
        match filename.or(self.filename) {
            Some(filename) => Self {
                state: self.state.succeed(),
                filename: Some(filename),
            },
            None => Self {
                state: self.state,
                filename: None,
            },
        }
    }

    /// A fetch that obtained no thumbnail: one more failed attempt.
    pub fn fail(self, now: DateTime<Utc>) -> Self {
        Self {
            state: self.state.fail(now),
            ..self
        }
    }

    /// A Retrying or Failed fetch back to Pending; any other unchanged.
    pub fn retry(self) -> Self {
        Self {
            state: self.state.retry(),
            ..self
        }
    }

    /// The file recorded at `filename` (a relocation), state unchanged.
    pub fn with_filename(self, filename: Option<String>) -> Self {
        Self { filename, ..self }
    }
}
