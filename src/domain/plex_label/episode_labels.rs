use crate::domain::plex::{PlexEpisode, PlexLabels};
use crate::domain::video::VideoRecordId;
use chrono::{DateTime, Utc};

/// A stored video record's Plex label state: whether its episode's labels
/// may be stale, and what yarrtube last wrote to which episode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeLabels {
    pub video_id: VideoRecordId,
    pub dirty: bool,
    /// The episode last written; `None` until first synced.
    pub rating_key: Option<String>,
    pub section_id: Option<String>,
    /// What yarrtube last wrote.
    pub labels: PlexLabels,
    /// Last time a sync looked at it; dirty rows are taken least recently
    /// checked first, so rows waiting on a download or import never starve
    /// the rest.
    pub checked_at: Option<DateTime<Utc>>,
}

impl EpisodeLabels {
    /// Dirty, never synced.
    pub fn pending(_video_id: VideoRecordId) -> Self {
        todo!()
    }

    pub fn mark_dirty(self) -> Self {
        todo!()
    }

    /// Still dirty (not downloaded, not imported or rejected by Plex),
    /// looked at `now`.
    pub fn checked(self, _now: DateTime<Utc>) -> Self {
        todo!()
    }

    /// `labels` written to `episode` at `now`.
    pub fn synced(self, _episode: &PlexEpisode, _labels: PlexLabels, _now: DateTime<Utc>) -> Self {
        todo!()
    }
}
