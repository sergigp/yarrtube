use crate::domain::plex::PlexEpisode;
use crate::domain::video::{VideoId, WatchProgress};

/// The watch state yarrtube and the Plex episode `rating_key` last agreed
/// on, for the viewer `viewer_id` (the Plex server and token).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeWatch {
    pub video_id: VideoId,
    pub rating_key: String,
    pub viewer_id: String,
    pub agreed: WatchProgress,
}

impl EpisodeWatch {
    pub fn new(
        video_id: VideoId,
        episode: &PlexEpisode,
        viewer_id: &str,
        agreed: WatchProgress,
    ) -> Self {
        Self {
            video_id,
            rating_key: episode.rating_key.clone(),
            viewer_id: viewer_id.to_string(),
            agreed,
        }
    }

    /// What was agreed with `episode` for `viewer_id`: unplayed when there is
    /// no row, or it was agreed with another episode (Plex re-imported the
    /// file) or another viewer (the token or server changed), so an empty
    /// state that isn't the one agreed on never reads as unwatched in Plex.
    pub fn agreed_with(
        row: Option<&EpisodeWatch>,
        episode: &PlexEpisode,
        viewer_id: &str,
    ) -> WatchProgress {
        row.filter(|row| row.rating_key == episode.rating_key && row.viewer_id == viewer_id)
            .map_or(WatchProgress::Unplayed, |row| row.agreed)
    }
}
