use crate::domain::services::{VideoUpdater, VideoUpdaterApi};
use crate::domain::video::{FavoriteVideoError, Video, VideoId};
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;

/// Favorites and unfavorites videos — see the `video-favorites` capability.
/// A favorite belongs to the YouTube video, so every change applies to every
/// stored copy of it.
#[derive(Clone)]
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
    fn favorite(&self, video_id: &VideoId) -> Result<(), FavoriteVideoError> {
        let at = self.clock.now();
        self.update_copies(video_id, |video| video.favorite(at))
    }

    fn unfavorite(&self, video_id: &VideoId) -> Result<(), FavoriteVideoError> {
        let now = self.clock.now();
        self.update_copies(video_id, |video| video.unfavorite(now))
    }
}

impl VideoFavoriter {
    /// Applies `transition` to every stored copy of video `video_id`'s
    /// YouTube video.
    fn update_copies(
        &self,
        video_id: &VideoId,
        transition: impl Fn(Video) -> Option<Video>,
    ) -> Result<(), FavoriteVideoError> {
        self.find_copies(video_id)?
            .iter()
            .try_for_each(|copy| self.video_updater.update(&copy.id, &transition).map(drop))
            .map_err(FavoriteVideoError::Repository)
    }

    /// Every stored copy of video `video_id`'s YouTube video: a favorite
    /// belongs to the YouTube video.
    fn find_copies(&self, video_id: &VideoId) -> Result<Vec<Video>, FavoriteVideoError> {
        let Some(video) = self
            .video_repository
            .find(video_id)
            .map_err(FavoriteVideoError::Repository)?
        else {
            return Ok(Vec::new());
        };
        self.video_repository
            .find_by_youtube_id(&video.youtube_id)
            .map_err(FavoriteVideoError::Repository)
    }
}
