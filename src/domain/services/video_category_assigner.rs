use crate::domain::category::{Category, CategoryId, CategoryMember};
use crate::domain::event::DomainEvent;
use crate::domain::video::VideoRecordId;
use crate::domain::video_category::{VideoCategories, VideoCategoriesChanged};
use crate::infrastructure::repositories::sqlite_category_repository::CategoryRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_video_categories_repository::VideoCategoriesRepository;
use crate::infrastructure::shared::category_lock::CategoryLock;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use std::collections::HashMap;
use std::sync::{Arc, MutexGuard};

/// Keeps each stored video record's categories in line with its source's
/// (the source regime) and announces every change in one batched event.
#[derive(Clone)]
pub struct VideoCategoryAssigner {
    video_categories_repository: Arc<dyn VideoCategoriesRepository>,
    category_repository: Arc<dyn CategoryRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    category_lock: Arc<CategoryLock>,
}

impl VideoCategoryAssigner {
    pub fn new(
        video_categories_repository: Arc<dyn VideoCategoriesRepository>,
        category_repository: Arc<dyn CategoryRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        category_lock: Arc<CategoryLock>,
    ) -> Self {
        Self {
            video_categories_repository,
            category_repository,
            channel_video_repository,
            playlist_video_repository,
            event_publisher,
            category_lock,
        }
    }
}

pub trait VideoCategoryAssignerApi: Send + Sync {
    /// Serializes video category writes. `assign_source` and `assign_video`
    /// take it themselves; callers of `reassign_category` hold it from
    /// before they change the categories it is computed from until it
    /// returns.
    fn lock(&self) -> MutexGuard<'_, ()>;
    /// Source regime for every record of `source`: its set = the source's
    /// categories. No-op when the source is in no category (AI regime).
    fn assign_source(&self, source: &CategoryMember) -> anyhow::Result<()>;
    /// Same, for one record of `source`.
    fn assign_video(&self, video_id: &VideoRecordId, source: &CategoryMember)
    -> anyhow::Result<()>;
    /// After `category_id`'s members changed or it was deleted: recomputes every
    /// record of `previous_members`, of its current members and of records
    /// still holding it, against `categories` (the state to apply). The
    /// records of a source in no category only lose `category_id`.
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
    fn lock(&self) -> MutexGuard<'_, ()> {
        self.category_lock.lock()
    }

    fn assign_source(&self, source: &CategoryMember) -> anyhow::Result<()> {
        self.assign_records(source, || self.record_ids(source))
    }

    fn assign_video(
        &self,
        video_id: &VideoRecordId,
        source: &CategoryMember,
    ) -> anyhow::Result<()> {
        self.assign_records(source, || Ok(vec![video_id.clone()]))
    }

    fn reassign_category(
        &self,
        category_id: &CategoryId,
        previous_members: &[CategoryMember],
        categories: &[Category],
    ) -> anyhow::Result<()> {
        let sources = Self::distinct(
            previous_members
                .iter()
                .cloned()
                .chain(Self::members_of(category_id, categories))
                .chain(self.stale_sources(category_id)?),
        );
        let desired = sources
            .iter()
            .map(|source| self.desired_for_source(source, category_id, categories))
            .collect::<anyhow::Result<Vec<_>>>()?
            .concat();
        self.save_and_announce(desired)
    }

    fn announce_renamed(&self, category_id: &CategoryId) -> anyhow::Result<()> {
        let holding = self
            .video_categories_repository
            .list_in_category(category_id)?;
        self.announce(&holding)
    }
}

impl VideoCategoryAssigner {
    /// Gives each of `video_ids` (listed only when `source` is in some
    /// category) exactly the source's categories; no-op when it is in none.
    fn assign_records(
        &self,
        source: &CategoryMember,
        video_ids: impl FnOnce() -> anyhow::Result<Vec<VideoRecordId>>,
    ) -> anyhow::Result<()> {
        let _guard = self.lock();
        let category_ids = self.categories_of(source)?;
        if category_ids.is_empty() {
            return Ok(());
        }
        self.save_and_announce(Self::each_in(video_ids()?, &category_ids))
    }

    /// The current members of `category_id` in `categories`.
    fn members_of(category_id: &CategoryId, categories: &[Category]) -> Vec<CategoryMember> {
        categories
            .iter()
            .find(|category| &category.id == category_id)
            .map(|category| category.members.clone())
            .unwrap_or_default()
    }

    /// The sources of the records still holding `category_id`.
    fn stale_sources(&self, category_id: &CategoryId) -> anyhow::Result<Vec<CategoryMember>> {
        let holding = self
            .video_categories_repository
            .list_in_category(category_id)?;
        if holding.is_empty() {
            return Ok(Vec::new());
        }
        let sources = self.sources_by_video()?;
        Ok(holding
            .iter()
            .filter_map(|categories| sources.get(&categories.video_id).cloned())
            .collect())
    }

    /// Every stored record's source.
    fn sources_by_video(&self) -> anyhow::Result<HashMap<VideoRecordId, CategoryMember>> {
        let channel_sources =
            self.channel_video_repository
                .list()?
                .into_iter()
                .map(|channel_video| {
                    (
                        channel_video.video_id,
                        CategoryMember::Channel(channel_video.channel_id),
                    )
                });
        let playlist_sources =
            self.playlist_video_repository
                .list()?
                .into_iter()
                .map(|playlist_video| {
                    (
                        playlist_video.video_id,
                        CategoryMember::Playlist(playlist_video.playlist_id),
                    )
                });
        Ok(channel_sources.chain(playlist_sources).collect())
    }

    /// `sources` each once, first occurrence first.
    fn distinct(sources: impl Iterator<Item = CategoryMember>) -> Vec<CategoryMember> {
        sources.fold(Vec::new(), |mut kept, source| {
            if !kept.contains(&source) {
                kept.push(source);
            }
            kept
        })
    }

    /// The categories of `categories` holding `source`.
    fn source_category_ids(source: &CategoryMember, categories: &[Category]) -> Vec<CategoryId> {
        categories
            .iter()
            .filter(|category| category.has_member(source))
            .map(|category| category.id.clone())
            .collect()
    }

    /// The current categories holding `source`; empty when it is in none
    /// (the AI regime).
    fn categories_of(&self, source: &CategoryMember) -> anyhow::Result<Vec<CategoryId>> {
        let categories = self.category_repository.list()?;
        Ok(Self::source_category_ids(source, &categories))
    }

    /// Each of `video_ids` in exactly `category_ids`.
    fn each_in(video_ids: Vec<VideoRecordId>, category_ids: &[CategoryId]) -> Vec<VideoCategories> {
        video_ids
            .into_iter()
            .map(|video_id| VideoCategories::new(video_id, category_ids.to_vec()))
            .collect()
    }

    /// Every record of `source` after `category_id` changed: each with the
    /// source's categories when it is in any (source regime), else its own
    /// categories without `category_id` (AI regime, the others are kept).
    fn desired_for_source(
        &self,
        source: &CategoryMember,
        category_id: &CategoryId,
        categories: &[Category],
    ) -> anyhow::Result<Vec<VideoCategories>> {
        let category_ids = Self::source_category_ids(source, categories);
        let record_ids = self.record_ids(source)?;
        if !category_ids.is_empty() {
            return Ok(Self::each_in(record_ids, &category_ids));
        }
        Ok(self
            .video_categories_repository
            .find_many(&record_ids)?
            .into_iter()
            .map(|current| current.without(category_id))
            .collect())
    }

    fn record_ids(&self, source: &CategoryMember) -> anyhow::Result<Vec<VideoRecordId>> {
        Ok(match source {
            CategoryMember::Channel(handle) => self
                .channel_video_repository
                .list_for_channel(handle)?
                .into_iter()
                .map(|channel_video| channel_video.video_id)
                .collect(),
            CategoryMember::Playlist(id) => self
                .playlist_video_repository
                .list_for_playlist(id)?
                .into_iter()
                .map(|playlist_video| playlist_video.video_id)
                .collect(),
        })
    }

    /// Saves the records of `desired` whose set differs from the stored one
    /// and announces them in one event, in record id order. Nothing changed →
    /// nothing saved or published.
    fn save_and_announce(&self, desired: Vec<VideoCategories>) -> anyhow::Result<()> {
        let changed = self.changed(desired)?;
        if !changed.is_empty() {
            self.video_categories_repository.save_many(&changed)?;
        }
        self.announce(&changed)
    }

    /// One event naming the records of `affected`; none when it is empty.
    fn announce(&self, affected: &[VideoCategories]) -> anyhow::Result<()> {
        if affected.is_empty() {
            return Ok(());
        }
        self.event_publisher
            .publish(&DomainEvent::VideoCategoriesChanged(
                VideoCategoriesChanged {
                    video_ids: affected
                        .iter()
                        .map(|categories| categories.video_id.as_str().to_string())
                        .collect(),
                },
            ))
    }

    /// The sets of `desired` that differ from the stored ones, in record id
    /// order.
    fn changed(&self, desired: Vec<VideoCategories>) -> anyhow::Result<Vec<VideoCategories>> {
        let ids: Vec<VideoRecordId> = desired.iter().map(|d| d.video_id.clone()).collect();
        let current = self.video_categories_repository.find_many(&ids)?;
        let mut changed: Vec<VideoCategories> = desired
            .into_iter()
            .zip(current)
            .filter(|(wanted, stored)| wanted != stored)
            .map(|(wanted, _)| wanted)
            .collect();
        changed.sort_by(|a, b| a.video_id.as_str().cmp(b.video_id.as_str()));
        Ok(changed)
    }
}
