use crate::domain::services::VideoUpdater;
use crate::domain::video::{FavoriteVideoError, VideoId};
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;

/// Favorites and unfavorites videos — see the `video-favorites` capability.
/// A favorite belongs to the YouTube video, so every change applies to every
/// stored copy of it.
#[derive(Clone)]
#[allow(dead_code)] // walking skeleton
pub struct VideoFavoriter {
    video_repository: Arc<dyn VideoRepository>,
    video_updater: VideoUpdater,
    clock: Arc<dyn Clock>,
}

impl VideoFavoriter {
    pub fn new(video_repository: Arc<dyn VideoRepository>, clock: Arc<dyn Clock>) -> Self {
        Self {
            video_updater: VideoUpdater::new(video_repository.clone()),
            video_repository,
            clock,
        }
    }
}

pub trait VideoFavoriterApi: Send + Sync {
    /// Every copy of the video's YouTube video favorited now; already-favorite
    /// copies unchanged. A Discarded video can't be favorited.
    fn favorite(&self, video_id: &VideoId) -> Result<(), FavoriteVideoError>;
    /// Every copy of the video's YouTube video unfavorited.
    fn unfavorite(&self, video_id: &VideoId) -> Result<(), FavoriteVideoError>;
}

impl VideoFavoriterApi for VideoFavoriter {
    fn favorite(&self, _video_id: &VideoId) -> Result<(), FavoriteVideoError> {
        Ok(())
    }

    fn unfavorite(&self, _video_id: &VideoId) -> Result<(), FavoriteVideoError> {
        Ok(())
    }
}
