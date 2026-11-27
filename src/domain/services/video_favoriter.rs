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
        let Some(video) = self.find_video(video_id)? else {
            return Ok(());
        };
        Self::ensure_not_discarded(&video)?;
        let at = self.clock.now();
        self.update_copies(&video, |video| video.favorite(at))
    }

    fn unfavorite(&self, video_id: &VideoId) -> Result<(), FavoriteVideoError> {
        let Some(video) = self.find_video(video_id)? else {
            return Ok(());
        };
        let now = self.clock.now();
        self.update_copies(&video, |video| video.unfavorite(now))
    }
}

impl VideoFavoriter {
    fn find_video(&self, video_id: &VideoId) -> Result<Option<Video>, FavoriteVideoError> {
        self.video_repository
            .find(video_id)
            .map_err(FavoriteVideoError::Repository)
    }

    fn ensure_not_discarded(video: &Video) -> Result<(), FavoriteVideoError> {
        if video.download.is_discarded() {
            Err(FavoriteVideoError::Discarded(video.id.clone()))
        } else {
            Ok(())
        }
    }

    /// Applies `transition` to every stored copy of `video`'s YouTube video,
    /// `video` included: a favorite belongs to the YouTube video.
    fn update_copies(
        &self,
        video: &Video,
        transition: impl Fn(Video) -> Option<Video>,
    ) -> Result<(), FavoriteVideoError> {
        self.video_repository
            .find_by_youtube_id(&video.youtube_id)
            .and_then(|copies| {
                copies
                    .iter()
                    .try_for_each(|copy| self.video_updater.update(&copy.id, &transition).map(drop))
            })
            .map_err(FavoriteVideoError::Repository)
    }
}
