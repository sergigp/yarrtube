use crate::domain::category::{Category, CategoryMember};
use crate::domain::services::{VideoUpdater, VideoUpdaterApi};
use crate::domain::video::{StepStatus, Video, VideoId};
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_video_category_verdict_repository::VideoCategoryVerdictRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;

/// Puts videos' categorizations back to Pending when their categories are
/// pending again, shared by every path that makes them so. A Pending
/// categorization keeps no `failed` verdict: a run asks only pending
/// categories, so each reset video's are deleted too.
#[derive(Clone)]
pub struct CategorizationResetter {
    video_updater: VideoUpdater,
    video_category_verdict_repository: Arc<dyn VideoCategoryVerdictRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    clock: Arc<dyn Clock>,
}

impl CategorizationResetter {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_category_verdict_repository: Arc<dyn VideoCategoryVerdictRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            video_updater: VideoUpdater::new(video_repository),
            video_category_verdict_repository,
            channel_video_repository,
            playlist_video_repository,
            clock,
        }
    }
}

pub trait CategorizationResetterApi: Send + Sync {
    /// The categorization of each of `video_ids` back to Pending, its
    /// `failed` verdicts deleted.
    fn reset(&self, video_ids: &[VideoId]) -> anyhow::Result<()>;
    /// The categorization of every stored video outside `category`'s
    /// members, those an AI category is pending for, back to Pending unless
    /// it already is, its `failed` verdicts deleted.
    fn reset_pending_for(&self, category: &Category) -> anyhow::Result<()>;
}

impl CategorizationResetterApi for CategorizationResetter {
    fn reset(&self, video_ids: &[VideoId]) -> anyhow::Result<()> {
        let now = self.clock.now();
        video_ids
            .iter()
            .try_for_each(|id| self.reset_one(id, |video| Some(video.reset_categorization(now))))
    }

    fn reset_pending_for(&self, category: &Category) -> anyhow::Result<()> {
        let now = self.clock.now();
        self.video_ids_outside(category)?.iter().try_for_each(|id| {
            self.reset_one(id, |video| {
                (video.categorization.status != StepStatus::Pending)
                    .then(|| video.reset_categorization(now))
            })
        })
    }
}

impl CategorizationResetter {
    /// Applies `reset` to the video `id`; when it did, deletes the video's
    /// `failed` verdicts.
    fn reset_one<F>(&self, id: &VideoId, reset: F) -> anyhow::Result<()>
    where
        F: Fn(Video) -> Option<Video>,
    {
        match self.video_updater.update(id, reset)? {
            Some(_) => self
                .video_category_verdict_repository
                .delete_failed_for_video(id),
            None => Ok(()),
        }
    }

    /// The stored videos whose channel or playlist isn't a member of
    /// `category`.
    fn video_ids_outside(&self, category: &Category) -> anyhow::Result<Vec<VideoId>> {
        let channel_videos = self
            .channel_video_repository
            .list()?
            .into_iter()
            .filter(|video| {
                !category.has_member(&CategoryMember::Channel(video.channel_id.clone()))
            })
            .map(|video| video.video_id);
        let playlist_videos = self
            .playlist_video_repository
            .list()?
            .into_iter()
            .filter(|video| {
                !category.has_member(&CategoryMember::Playlist(video.playlist_id.clone()))
            })
            .map(|video| video.video_id);
        Ok(channel_videos.chain(playlist_videos).collect())
    }
}
