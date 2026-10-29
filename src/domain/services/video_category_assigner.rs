use crate::domain::category::{Category, CategoryId, CategoryMember};
use crate::domain::video::VideoRecordId;
use crate::infrastructure::repositories::sqlite_category_repository::CategoryRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_video_categories_repository::VideoCategoriesRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use std::sync::Arc;

/// Keeps each stored video record's categories in line with its source's
/// (the source regime) and announces every change in one batched event.
#[derive(Clone)]
#[allow(dead_code)]
pub struct VideoCategoryAssigner {
    video_categories_repository: Arc<dyn VideoCategoriesRepository>,
    category_repository: Arc<dyn CategoryRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl VideoCategoryAssigner {
    pub fn new(
        video_categories_repository: Arc<dyn VideoCategoriesRepository>,
        category_repository: Arc<dyn CategoryRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            video_categories_repository,
            category_repository,
            channel_video_repository,
            playlist_video_repository,
            event_publisher,
        }
    }
}

#[allow(dead_code)]
pub trait VideoCategoryAssignerApi: Send + Sync {
    /// Source regime for every record of `source`: its set = the source's
    /// categories. No-op when the source is in no category (AI regime).
    fn assign_source(&self, source: &CategoryMember) -> anyhow::Result<()>;
    /// Same, for one record of `source`.
    fn assign_video(&self, video_id: &VideoRecordId, source: &CategoryMember)
    -> anyhow::Result<()>;
    /// After `category_id`'s members changed or it was deleted: recomputes every
    /// record of `previous_members`, of its current members and of records
    /// still holding it, against `categories` (the state to apply). A source in
    /// no category leaves its records in none.
    fn reassign_category(
        &self,
        category_id: &CategoryId,
        previous_members: &[CategoryMember],
        categories: &[Category],
    ) -> anyhow::Result<()>;
    /// Announces a rename: one event naming every record holding `category_id`.
    fn announce_renamed(&self, category_id: &CategoryId) -> anyhow::Result<()>;
}

impl VideoCategoryAssignerApi for VideoCategoryAssigner {
    fn assign_source(&self, _source: &CategoryMember) -> anyhow::Result<()> {
        Ok(())
    }

    fn assign_video(
        &self,
        _video_id: &VideoRecordId,
        _source: &CategoryMember,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn reassign_category(
        &self,
        _category_id: &CategoryId,
        _previous_members: &[CategoryMember],
        _categories: &[Category],
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn announce_renamed(&self, _category_id: &CategoryId) -> anyhow::Result<()> {
        Ok(())
    }
}
