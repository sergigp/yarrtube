use crate::domain::plex::PlexEpisode;
use crate::domain::video::{VideoId, WatchProgress};

/// The watch state yarrtube and the Plex episode `rating_key` last agreed
/// on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeWatch {
    pub video_id: VideoId,
    pub rating_key: String,
    pub agreed: WatchProgress,
}

impl EpisodeWatch {
    pub fn new(video_id: VideoId, episode: &PlexEpisode, agreed: WatchProgress) -> Self {
        Self {
            video_id,
            rating_key: episode.rating_key.clone(),
            agreed,
        }
    }

    /// What was agreed with `episode`: unplayed when there is no row or it
    /// was agreed with another episode (Plex re-imported the file), so a
    /// re-imported episode's empty state never reads as unwatched in Plex.
    pub fn agreed_with(row: Option<&EpisodeWatch>, episode: &PlexEpisode) -> WatchProgress {
        row.filter(|row| row.rating_key == episode.rating_key)
            .map_or(WatchProgress::Unplayed, |row| row.agreed)
    }
}
