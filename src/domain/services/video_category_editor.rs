use crate::domain::category::{Category, CategoryId, CategoryMember};
use crate::domain::services::{
    VideoCategoryAssigner, VideoCategoryAssignerApi, VideoOwnerFinder, VideoOwnerFinderApi,
};
use crate::domain::task::Task;
use crate::domain::video::VideoRecordId;
use crate::domain::video_category::{EditVideoCategoriesError, VideoCategoryVerdict};
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
        video_id: &VideoRecordId,
        category_id: &CategoryId,
        belongs: bool,
    ) -> Result<(), EditVideoCategoriesError> {
        let source = self.find_source(video_id)?;
        Self::ensure_not_from_source(&self.find_category(category_id)?, &source)?;
        let verdict = VideoCategoryVerdict::by_user(
            video_id.clone(),
            category_id.clone(),
            belongs,
            self.clock.now(),
        );
        self.video_category_assigner
            .apply_verdict(&verdict, &source)
            .map_err(EditVideoCategoriesError::Repository)?;
        Ok(())
    }

    fn rerun_video(&self, video_id: &VideoRecordId) -> Result<(), EditVideoCategoriesError> {
        self.find_source(video_id)?;
        self.clear_ai_verdicts(std::slice::from_ref(video_id))?;
        self.schedule(&Task::SyncVideo {
            video_id: video_id.as_str().to_string(),
        })
    }

    fn rerun_source(&self, source: &CategoryMember) -> Result<(), EditVideoCategoriesError> {
        let video_ids = self.record_ids(source)?;
        self.clear_ai_verdicts(&video_ids)?;
        self.schedule(&Self::reconcile_task(source))
    }

    fn rerun_category(&self, category_id: &CategoryId) -> Result<(), EditVideoCategoriesError> {
        self.find_category(category_id)?;
        let affected = self
            .video_category_verdict_repository
            .delete_ai_for_category(category_id)
            .map_err(EditVideoCategoriesError::Repository)?;
        self.recompute(&affected)?;
        affected.iter().try_for_each(|video_id| {
            self.schedule(&Task::SyncVideo {
                video_id: video_id.as_str().to_string(),
            })
        })
    }
}

impl VideoCategoryEditor {
    /// The channel or playlist holding the stored `video_id`.
    fn find_source(
        &self,
        video_id: &VideoRecordId,
    ) -> Result<CategoryMember, EditVideoCategoriesError> {
        let video = self
            .video_repository
            .find(video_id)
            .map_err(EditVideoCategoriesError::Repository)?
            .ok_or(EditVideoCategoriesError::VideoNotFound)?;
        self.video_owner_finder
            .find(&video)
            .map_err(EditVideoCategoriesError::Repository)?
            .map(|owner| owner.source)
            .ok_or(EditVideoCategoriesError::VideoNotFound)
    }

    /// Deletes the AI verdicts of `video_ids` and recomputes the categories
    /// of those that had one.
    fn clear_ai_verdicts(
        &self,
        video_ids: &[VideoRecordId],
    ) -> Result<(), EditVideoCategoriesError> {
        let affected = self
            .video_category_verdict_repository
            .delete_ai_for_videos(video_ids)
            .map_err(EditVideoCategoriesError::Repository)?;
        self.recompute(&affected)
    }

    fn recompute(&self, video_ids: &[VideoRecordId]) -> Result<(), EditVideoCategoriesError> {
        self.video_category_assigner
            .recompute(video_ids)
            .map_err(EditVideoCategoriesError::Repository)
    }

    /// The stored records of `source`; `SourceNotFound` when it isn't tracked.
    fn record_ids(
        &self,
        source: &CategoryMember,
    ) -> Result<Vec<VideoRecordId>, EditVideoCategoriesError> {
        self.find_tracked(source)?;
        let video_ids = match source {
            CategoryMember::Channel(handle) => self
                .channel_video_repository
                .list_for_channel(handle)
                .map(|videos| videos.into_iter().map(|video| video.video_id).collect()),
            CategoryMember::Playlist(id) => self
                .playlist_video_repository
                .list_for_playlist(id)
                .map(|videos| videos.into_iter().map(|video| video.video_id).collect()),
        };
        video_ids.map_err(EditVideoCategoriesError::Repository)
    }

    fn find_tracked(&self, source: &CategoryMember) -> Result<(), EditVideoCategoriesError> {
        let tracked = match source {
            CategoryMember::Channel(handle) => self
                .channel_repository
                .find(handle)
                .map(|channel| channel.is_some()),
            CategoryMember::Playlist(id) => self
                .playlist_repository
                .find(id)
                .map(|playlist| playlist.is_some()),
        };
        match tracked.map_err(EditVideoCategoriesError::Repository)? {
            true => Ok(()),
            false => Err(EditVideoCategoriesError::SourceNotFound),
        }
    }

    /// The pass that syncs every video of `source`.
    fn reconcile_task(source: &CategoryMember) -> Task {
        match source {
            CategoryMember::Channel(handle) => Task::ReconcileChannel {
                channel_id: handle.as_str().to_string(),
            },
            CategoryMember::Playlist(id) => Task::ReconcilePlaylist {
                playlist_id: id.as_str().to_string(),
            },
        }
    }

    /// Schedules `task` to run now.
    fn schedule(&self, task: &Task) -> Result<(), EditVideoCategoriesError> {
        self.task_repository
            .schedule(task, self.clock.now())
            .map_err(EditVideoCategoriesError::Repository)
    }

    /// A category holding the video's `source` is changed through the
    /// source's membership, never by hand.
    fn ensure_not_from_source(
        category: &Category,
        source: &CategoryMember,
    ) -> Result<(), EditVideoCategoriesError> {
        match category.has_member(source) {
            true => Err(EditVideoCategoriesError::FromSource(source.clone())),
            false => Ok(()),
        }
    }

    fn find_category(
        &self,
        category_id: &CategoryId,
    ) -> Result<Category, EditVideoCategoriesError> {
        self.category_repository
            .find(category_id)
            .map_err(EditVideoCategoriesError::Repository)?
            .ok_or(EditVideoCategoriesError::CategoryNotFound)
    }
}
