use crate::domain::shared::{PlaylistId, VideoId};
use crate::domain::video::{ListVideosError, Video};
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use std::sync::Arc;

/// Reads videos, single or listed by playlist.
#[derive(Clone)]
pub struct VideoSearcher {
    playlist_repository: Arc<dyn PlaylistRepository>,
    video_repository: Arc<dyn VideoRepository>,
}

impl VideoSearcher {
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        video_repository: Arc<dyn VideoRepository>,
    ) -> Self {
        Self {
            playlist_repository,
            video_repository,
        }
    }

    pub fn find(
        &self,
        playlist_id: &PlaylistId,
        video_id: &VideoId,
    ) -> anyhow::Result<Option<Video>> {
        self.video_repository.find(playlist_id, video_id)
    }

    /// Lists every video recorded for a playlist, confirming the playlist
    /// exists first.
    pub fn list(&self, playlist_id: &PlaylistId) -> Result<Vec<Video>, ListVideosError> {
        self.playlist_repository
            .find(playlist_id)
            .map_err(ListVideosError::Repository)?
            .ok_or_else(|| ListVideosError::PlaylistNotFound(playlist_id.clone()))?;

        self.video_repository
            .list_for_playlist(playlist_id)
            .map_err(ListVideosError::Repository)
    }
}
