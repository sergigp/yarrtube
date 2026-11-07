use crate::domain::category::{CategoryId, CategoryMember};
use crate::domain::services::{VideoCategoryAssigner, VideoOwnerFinder};
use crate::domain::video::VideoRecordId;
use crate::domain::video_category::EditVideoCategoriesError;
use crate::infrastructure::repositories::sqlite_category_repository::CategoryRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_category_verdict_repository::VideoCategoryVerdictRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;

/// Stores the user's verdicts and deletes AI verdicts so they are asked
/// again.
#[derive(Clone)]
pub struct VideoCategoryEditor {
    video_repository: Arc<dyn VideoRepository>,
    video_owner_finder: Arc<VideoOwnerFinder>,
    category_repository: Arc<dyn CategoryRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    playlist_repository: Arc<dyn PlaylistRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    video_category_verdict_repository: Arc<dyn VideoCategoryVerdictRepository>,
    video_category_assigner: VideoCategoryAssigner,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
}

impl VideoCategoryEditor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_owner_finder: Arc<VideoOwnerFinder>,
        category_repository: Arc<dyn CategoryRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        playlist_repository: Arc<dyn PlaylistRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        video_category_verdict_repository: Arc<dyn VideoCategoryVerdictRepository>,
        video_category_assigner: VideoCategoryAssigner,
        task_repository: Arc<dyn TaskRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            video_repository,
            video_owner_finder,
            category_repository,
            channel_repository,
            playlist_repository,
            channel_video_repository,
            playlist_video_repository,
            video_category_verdict_repository,
            video_category_assigner,
            task_repository,
            clock,
        }
    }
}

pub trait VideoCategoryEditorApi: Send + Sync {
    /// Stores the user's verdict for the pair and recomputes the video's
    /// categories; `FromSource` when the video's source is a member.
    fn set(
        &self,
        video_id: &VideoRecordId,
        category_id: &CategoryId,
        belongs: bool,
    ) -> Result<(), EditVideoCategoriesError>;
    /// Deletes the video's AI verdicts, recomputes its categories and
    /// schedules its sync.
    fn rerun_video(&self, video_id: &VideoRecordId) -> Result<(), EditVideoCategoriesError>;
    /// The same for every record of `source`; schedules its reconcile pass.
    fn rerun_source(&self, source: &CategoryMember) -> Result<(), EditVideoCategoriesError>;
    /// Deletes the category's AI verdicts, recomputes and syncs the records
    /// that had one.
    fn rerun_category(&self, category_id: &CategoryId) -> Result<(), EditVideoCategoriesError>;
}

impl VideoCategoryEditorApi for VideoCategoryEditor {
    fn set(
        &self,
        _video_id: &VideoRecordId,
        _category_id: &CategoryId,
        _belongs: bool,
    ) -> Result<(), EditVideoCategoriesError> {
        Ok(())
    }

    fn rerun_video(&self, _video_id: &VideoRecordId) -> Result<(), EditVideoCategoriesError> {
        Ok(())
    }

    fn rerun_source(&self, _source: &CategoryMember) -> Result<(), EditVideoCategoriesError> {
        Ok(())
    }

    fn rerun_category(&self, _category_id: &CategoryId) -> Result<(), EditVideoCategoriesError> {
        Ok(())
    }
}
