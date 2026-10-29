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
    /// How many times it was marked dirty; a sync's save lands only if no
    /// mark came after the sync read it.
    pub generation: i64,
}

impl EpisodeLabels {
    /// Dirty, never synced.
    pub fn pending(video_id: VideoRecordId) -> Self {
        Self {
            video_id,
            dirty: true,
            rating_key: None,
            section_id: None,
            labels: PlexLabels::none(),
            checked_at: None,
            generation: 0,
        }
    }

    pub fn mark_dirty(self) -> Self {
        Self {
            dirty: true,
            generation: self.generation + 1,
            ..self
        }
    }

    /// Dirty, and no longer tied to an episode: Plex stopped listing the one
    /// last written, so neither it nor its labels count any more.
    pub fn forget_episode(self) -> Self {
        Self {
            rating_key: None,
            section_id: None,
            labels: PlexLabels::none(),
            ..self.mark_dirty()
        }
    }

    /// Still dirty (not downloaded, not imported or rejected by Plex),
    /// looked at `now`.
    pub fn checked(self, now: DateTime<Utc>) -> Self {
        Self {
            checked_at: Some(now),
            ..self
        }
    }

    /// `labels` written to `episode` at `now`.
    pub fn synced(self, episode: &PlexEpisode, labels: PlexLabels, now: DateTime<Utc>) -> Self {
        Self {
            dirty: false,
            rating_key: Some(episode.rating_key.clone()),
            section_id: Some(episode.section_id.clone()),
            labels,
            checked_at: Some(now),
            ..self
        }
    }
}
