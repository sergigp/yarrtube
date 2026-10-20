use super::playlist_id::PlaylistId;
use super::playlist_kind::PlaylistKind;
use super::playlist_name::PlaylistName;
use super::playlist_path::PlaylistPath;
use crate::domain::shared::{Quality, ReconcileInterval, ValidationError};
use crate::domain::video::PlaylistEpisodeNumber;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Playlist {
    pub id: PlaylistId,
    pub name: PlaylistName,
    pub path: PlaylistPath,
    pub quality: Quality,
    pub kind: PlaylistKind,
    pub exclude_from_home: bool,
    pub hide_unwatched_count: bool,
    pub reconcile_interval: ReconcileInterval,
    pub created_at: DateTime<Utc>,
    /// The highest episode number the playlist has ever assigned.
    pub last_episode: Option<PlaylistEpisodeNumber>,
}

impl Playlist {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        id: PlaylistId,
        name: PlaylistName,
        path: PlaylistPath,
        quality: Quality,
        kind: PlaylistKind,
        exclude_from_home: bool,
        reconcile_interval: ReconcileInterval,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            path,
            quality,
            kind,
            exclude_from_home,
            hide_unwatched_count: false,
            reconcile_interval,
            created_at,
            last_episode: None,
        }
    }

    pub fn with_exclude_from_home(self, exclude_from_home: bool) -> Self {
        Self {
            exclude_from_home,
            ..self
        }
    }

    pub fn with_hide_unwatched_count(self, hide_unwatched_count: bool) -> Self {
        Self {
            hide_unwatched_count,
            ..self
        }
    }

    pub fn with_reconcile_interval(self, reconcile_interval: ReconcileInterval) -> Self {
        Self {
            reconcile_interval,
            ..self
        }
    }

    /// The number the next video stored in the playlist gets: one past the
    /// highest ever assigned, `1` for a playlist that has assigned none.
    pub fn next_episode_number(&self) -> Result<PlaylistEpisodeNumber, ValidationError> {
        match self.last_episode {
            Some(last) => last.next(),
            None => PlaylistEpisodeNumber::new(1),
        }
    }

    pub fn with_last_episode(self, number: PlaylistEpisodeNumber) -> Self {
        Self {
            last_episode: Some(number),
            ..self
        }
    }
}
