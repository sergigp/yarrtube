use crate::domain::category::Category;
use crate::domain::category::CategoryId;
use crate::domain::channel::ChannelHandle;
use crate::domain::playlist::PlaylistId;
use crate::domain::services::{CategoryVideoCollector, CategoryVideoCollectorApi};
use crate::domain::video::{
    PlaybackPosition, UpdateWatchStateError, Video, VideoDuration, VideoId, VideoRecordId,
};
use crate::infrastructure::repositories::sqlite_category_repository::CategoryRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::system_clock::Clock;
use chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::sync::Arc;

/// Updates whether videos have been watched and where their playback
/// stopped. Watch state belongs to the YouTube video, so every change
/// applies to every stored copy of it.
#[derive(Clone)]
pub struct VideoWatchStateUpdater {
    video_repository: Arc<dyn VideoRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    playlist_repository: Arc<dyn PlaylistRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    clock: Arc<dyn Clock>,
    category_repository: Arc<dyn CategoryRepository>,
    category_video_collector: CategoryVideoCollector,
}

impl VideoWatchStateUpdater {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        playlist_repository: Arc<dyn PlaylistRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        clock: Arc<dyn Clock>,
        category_repository: Arc<dyn CategoryRepository>,
        category_video_collector: CategoryVideoCollector,
    ) -> Self {
        Self {
            video_repository,
            channel_repository,
            channel_video_repository,
            playlist_repository,
            playlist_video_repository,
            clock,
            category_repository,
            category_video_collector,
        }
    }
}

pub trait VideoWatchStateUpdaterApi: Send + Sync {
    /// Applies `Video::update_watch_state` to every stored copy of
    /// `youtube_id`. Returns whether the video is watched once the report is
    /// applied.
    fn update(
        &self,
        youtube_id: &VideoId,
        position: PlaybackPosition,
        reported_duration: Option<VideoDuration>,
        was_watched: bool,
    ) -> Result<bool, UpdateWatchStateError>;
    /// Marks every unwatched copy of `youtube_id` watched, if any copy is
    /// `Downloaded`. An already watched copy is left unchanged.
    fn mark_video_watched(&self, youtube_id: &VideoId) -> Result<(), UpdateWatchStateError>;
    /// Marks every `Downloaded` video of the channel watched, with every
    /// stored copy of each; pending/in-flight videos are left unchanged.
    fn mark_channel_watched(&self, channel_id: &ChannelHandle)
    -> Result<(), UpdateWatchStateError>;
    /// Marks every downloaded video of the category watched, every copy of
    /// each; pending/in-flight videos are left unchanged.
    fn mark_category_watched(&self, id: &CategoryId) -> Result<(), UpdateWatchStateError>;
    /// Marks every `Downloaded` video of the playlist watched, with every
    /// stored copy of each; pending/in-flight videos are left unchanged.
    fn mark_playlist_watched(&self, playlist_id: &PlaylistId) -> Result<(), UpdateWatchStateError>;
}

impl VideoWatchStateUpdaterApi for VideoWatchStateUpdater {
    fn update(
        &self,
        youtube_id: &VideoId,
        position: PlaybackPosition,
        reported_duration: Option<VideoDuration>,
        was_watched: bool,
    ) -> Result<bool, UpdateWatchStateError> {
        let copies = self.find_copies(youtube_id)?;
        let now = self.clock.now();
        copies
            .into_iter()
            .map(|video| video.update_watch_state(position, reported_duration, was_watched, now))
            .try_fold(false, |watched, video| {
                self.video_repository
                    .update(&video)
                    .map(|()| watched || video.is_watched())
            })
            .map_err(UpdateWatchStateError::Repository)
    }

    fn mark_video_watched(&self, youtube_id: &VideoId) -> Result<(), UpdateWatchStateError> {
        let copies = self.find_downloaded_copies(youtube_id)?;
        let now = self.clock.now();
        copies
            .into_iter()
            .filter(|video| !video.is_watched())
            .map(|video| video.mark_watched(now))
            .try_for_each(|video| self.video_repository.update(&video))
            .map_err(UpdateWatchStateError::Repository)
    }

    fn mark_channel_watched(
        &self,
        channel_id: &ChannelHandle,
    ) -> Result<(), UpdateWatchStateError> {
        self.ensure_channel_exists(channel_id)?;
        let now = self.clock.now();
        Self::unwatched_downloaded_youtube_ids(self.channel_videos(channel_id)?)
            .iter()
            .try_for_each(|youtube_id| self.mark_copies_watched(youtube_id, now))
    }

    fn mark_category_watched(&self, id: &CategoryId) -> Result<(), UpdateWatchStateError> {
        let category = self.find_category(id)?;
        let now = self.clock.now();
        self.unwatched_category_youtube_ids(&category)?
            .iter()
            .try_for_each(|youtube_id| self.mark_copies_watched(youtube_id, now))
    }

    fn mark_playlist_watched(&self, playlist_id: &PlaylistId) -> Result<(), UpdateWatchStateError> {
        self.ensure_playlist_exists(playlist_id)?;
        let now = self.clock.now();
        Self::unwatched_downloaded_youtube_ids(self.playlist_videos(playlist_id)?)
            .iter()
            .try_for_each(|youtube_id| self.mark_copies_watched(youtube_id, now))
    }
}

impl VideoWatchStateUpdater {
    fn ensure_channel_exists(
        &self,
        channel_id: &ChannelHandle,
    ) -> Result<(), UpdateWatchStateError> {
        self.channel_repository
            .find(channel_id)
            .map_err(UpdateWatchStateError::Repository)?
            .map(|_| ())
            .ok_or_else(|| UpdateWatchStateError::ChannelNotFound(channel_id.clone()))
    }

    fn find_category(&self, id: &CategoryId) -> Result<Category, UpdateWatchStateError> {
        self.category_repository
            .find(id)
            .map_err(UpdateWatchStateError::Repository)?
            .ok_or_else(|| UpdateWatchStateError::CategoryNotFound(id.clone()))
    }

    /// The YouTube IDs of the category's downloaded videos not yet watched,
    /// each once.
    fn unwatched_category_youtube_ids(
        &self,
        category: &Category,
    ) -> Result<HashSet<VideoId>, UpdateWatchStateError> {
        Ok(self
            .category_video_collector
            .downloaded_videos(category)
            .map_err(UpdateWatchStateError::Repository)?
            .into_iter()
            .filter(|sourced| !sourced.video.is_watched())
            .map(|sourced| sourced.video.youtube_id)
            .collect())
    }

    fn ensure_playlist_exists(
        &self,
        playlist_id: &PlaylistId,
    ) -> Result<(), UpdateWatchStateError> {
        self.playlist_repository
            .find(playlist_id)
            .map_err(UpdateWatchStateError::Repository)?
            .map(|_| ())
            .ok_or_else(|| UpdateWatchStateError::PlaylistNotFound(playlist_id.clone()))
    }

    /// The YouTube IDs of the `Downloaded` videos not yet watched, each once.
    fn unwatched_downloaded_youtube_ids(videos: Vec<Video>) -> HashSet<VideoId> {
        videos
            .into_iter()
            .filter(|video| video.is_downloaded_and_unwatched())
            .map(|video| video.youtube_id)
            .collect()
    }

    fn channel_videos(
        &self,
        channel_id: &ChannelHandle,
    ) -> Result<Vec<Video>, UpdateWatchStateError> {
        self.channel_video_repository
            .list_for_channel(channel_id)
            .and_then(|channel_videos| {
                let ids: Vec<VideoRecordId> = channel_videos
                    .into_iter()
                    .map(|channel_video| channel_video.video_id)
                    .collect();
                self.video_repository.find_many(&ids)
            })
            .map_err(UpdateWatchStateError::Repository)
    }

    fn playlist_videos(
        &self,
        playlist_id: &PlaylistId,
    ) -> Result<Vec<Video>, UpdateWatchStateError> {
        self.playlist_video_repository
            .list_for_playlist(playlist_id)
            .and_then(|playlist_videos| {
                let ids: Vec<VideoRecordId> = playlist_videos
                    .into_iter()
                    .map(|playlist_video| playlist_video.video_id)
                    .collect();
                self.video_repository.find_many(&ids)
            })
            .map_err(UpdateWatchStateError::Repository)
    }

    fn mark_copies_watched(
        &self,
        youtube_id: &VideoId,
        now: DateTime<Utc>,
    ) -> Result<(), UpdateWatchStateError> {
        self.video_repository
            .find_by_youtube_id(youtube_id)
            .map_err(UpdateWatchStateError::Repository)?
            .into_iter()
            .map(|video| video.mark_watched(now))
            .try_for_each(|video| self.video_repository.update(&video))
            .map_err(UpdateWatchStateError::Repository)
    }

    /// Every stored copy of `youtube_id`, as long as at least one of them is
    /// `Downloaded`.
    fn find_downloaded_copies(
        &self,
        youtube_id: &VideoId,
    ) -> Result<Vec<Video>, UpdateWatchStateError> {
        Some(self.find_copies(youtube_id)?)
            .filter(|copies| copies.iter().any(Video::is_downloaded))
            .ok_or_else(|| UpdateWatchStateError::VideoNotDownloaded(youtube_id.clone()))
    }

    fn find_copies(&self, youtube_id: &VideoId) -> Result<Vec<Video>, UpdateWatchStateError> {
        Some(
            self.video_repository
                .find_by_youtube_id(youtube_id)
                .map_err(UpdateWatchStateError::Repository)?,
        )
        .filter(|copies| !copies.is_empty())
        .ok_or_else(|| UpdateWatchStateError::VideoNotFound(youtube_id.clone()))
    }
}
