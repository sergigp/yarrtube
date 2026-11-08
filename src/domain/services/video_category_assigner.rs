use crate::domain::category::{Category, CategoryId, CategoryMember};
use crate::domain::event::DomainEvent;
use crate::domain::video::VideoRecordId;
use crate::domain::video_category::{
    VideoCategories, VideoCategoriesChanged, VideoCategoryVerdict,
};
use crate::infrastructure::repositories::sqlite_category_repository::CategoryRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_video_categories_repository::VideoCategoriesRepository;
use crate::infrastructure::repositories::sqlite_video_category_verdict_repository::VideoCategoryVerdictRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::category_lock::CategoryLock;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use std::collections::HashMap;
use std::sync::{Arc, MutexGuard};

/// Keeps each stored video record's categories at its source's plus its yes
/// verdicts' and announces every change in one batched event.
#[derive(Clone)]
pub struct VideoCategoryAssigner {
    video_repository: Arc<dyn VideoRepository>,
    video_category_verdict_repository: Arc<dyn VideoCategoryVerdictRepository>,
    video_categories_repository: Arc<dyn VideoCategoriesRepository>,
    category_repository: Arc<dyn CategoryRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    category_lock: Arc<CategoryLock>,
}

impl VideoCategoryAssigner {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_category_verdict_repository: Arc<dyn VideoCategoryVerdictRepository>,
        video_categories_repository: Arc<dyn VideoCategoriesRepository>,
        category_repository: Arc<dyn CategoryRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        category_lock: Arc<CategoryLock>,
    ) -> Self {
        Self {
            video_repository,
            video_category_verdict_repository,
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
    /// Serializes video category writes. `assign_video` takes it itself;
    /// callers of `reassign_category` hold it from before they change the
    /// categories it is computed from until it returns.
    fn lock(&self) -> MutexGuard<'_, ()>;
    /// Sets one record of `source` to the source's categories plus its yes
    /// verdicts.
    fn assign_video(&self, video_id: &VideoRecordId, source: &CategoryMember)
    -> anyhow::Result<()>;
    /// After `category_id`'s members changed or it was deleted: recomputes every
    /// record of `previous_members`, of its current members and of records
    /// still holding it, against `categories` (the state to apply): each
    /// gets its source's categories plus its yes verdicts.
    fn reassign_category(
        &self,
        category_id: &CategoryId,
        previous_members: &[CategoryMember],
        categories: &[Category],
    ) -> anyhow::Result<()>;
    /// Announces a rename: one event naming every record holding `category_id`.
    fn announce_renamed(&self, category_id: &CategoryId) -> anyhow::Result<()>;
    /// Under the lock: false, storing nothing, when the video or the
    /// category is gone; else saves `verdict` and sets the record to its
    /// `source`'s categories plus its yes verdicts, announcing a change. A
    /// video removed meanwhile leaves nothing stored either.
    fn apply_verdict(
        &self,
        verdict: &VideoCategoryVerdict,
        source: &CategoryMember,
    ) -> anyhow::Result<bool>;
    /// `apply_verdict`, also storing nothing when `still_current`, asked
    /// under the lock, says the verdict is outdated.
    fn apply_verdict_if(
        &self,
        verdict: &VideoCategoryVerdict,
        source: &CategoryMember,
        still_current: &dyn Fn() -> anyhow::Result<bool>,
    ) -> anyhow::Result<bool>;
    /// Under the lock: sets each of `video_ids` to its source's categories
    /// plus its yes verdicts, announcing the changed ones in one event.
    fn recompute(&self, video_ids: &[VideoRecordId]) -> anyhow::Result<()>;
}

impl VideoCategoryAssignerApi for VideoCategoryAssigner {
    fn lock(&self) -> MutexGuard<'_, ()> {
        self.category_lock.lock()
    }

    fn assign_video(
        &self,
        video_id: &VideoRecordId,
        source: &CategoryMember,
    ) -> anyhow::Result<()> {
        let _guard = self.lock();
        self.recompute_record(video_id, source)
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
            .map(|source| self.desired_for_source(source, categories))
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

    fn apply_verdict(
        &self,
        verdict: &VideoCategoryVerdict,
        source: &CategoryMember,
    ) -> anyhow::Result<bool> {
        self.apply_verdict_if(verdict, source, &|| Ok(true))
    }

    fn apply_verdict_if(
        &self,
        verdict: &VideoCategoryVerdict,
        source: &CategoryMember,
        still_current: &dyn Fn() -> anyhow::Result<bool>,
    ) -> anyhow::Result<bool> {
        let _guard = self.lock();
        let categories = self.category_repository.list()?;
        if !self.pair_exists(verdict, &categories)? || !still_current()? {
            return Ok(false);
        }
        self.video_category_verdict_repository.save(verdict)?;
        self.recompute_with(&verdict.video_id, source, &categories)?;
        self.forget_if_removed(&verdict.video_id)
    }

    fn recompute(&self, video_ids: &[VideoRecordId]) -> anyhow::Result<()> {
        let _guard = self.lock();
        let categories = self.category_repository.list()?;
        let desired = self.desired_for_records(video_ids, &categories)?;
        self.save_and_announce(desired)
    }
}

impl VideoCategoryAssigner {
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

    /// Whether the video of `verdict` is still stored and its category is in
    /// `categories`, the current categories.
    fn pair_exists(
        &self,
        verdict: &VideoCategoryVerdict,
        categories: &[Category],
    ) -> anyhow::Result<bool> {
        Ok(categories
            .iter()
            .any(|category| category.id == verdict.category_id)
            && self.video_repository.find(&verdict.video_id)?.is_some())
    }

    /// Deletes what was just stored for `video_id` when the video was removed
    /// meanwhile: its removal, taking no lock, may have deleted its verdicts
    /// and categories before they were stored. True when it is still there.
    fn forget_if_removed(&self, video_id: &VideoRecordId) -> anyhow::Result<bool> {
        if self.video_repository.find(video_id)?.is_some() {
            return Ok(true);
        }
        self.video_category_verdict_repository
            .delete_for_video(video_id)?;
        self.video_categories_repository.delete(video_id)?;
        Ok(false)
    }

    /// Sets `video_id`, a record of `source`, to the source's categories plus
    /// its yes verdicts; the caller holds the lock.
    fn recompute_record(
        &self,
        video_id: &VideoRecordId,
        source: &CategoryMember,
    ) -> anyhow::Result<()> {
        let categories = self.category_repository.list()?;
        self.recompute_with(video_id, source, &categories)
    }

    /// `recompute` against `categories`, the current categories; the caller
    /// holds the lock.
    fn recompute_with(
        &self,
        video_id: &VideoRecordId,
        source: &CategoryMember,
        categories: &[Category],
    ) -> anyhow::Result<()> {
        let desired = self.with_yes_verdicts(
            vec![video_id.clone()],
            &Self::source_category_ids(source, categories),
        )?;
        self.save_and_announce(desired)
    }

    /// Each of `video_ids` held by a source, in order, with its source's
    /// categories in `categories` plus its yes verdicts; the others are left
    /// out.
    fn desired_for_records(
        &self,
        video_ids: &[VideoRecordId],
        categories: &[Category],
    ) -> anyhow::Result<Vec<VideoCategories>> {
        let records = video_ids
            .iter()
            .map(|video_id| {
                Ok(self.source_of(video_id)?.map(|source| {
                    (
                        video_id.clone(),
                        Self::source_category_ids(&source, categories),
                    )
                }))
            })
            .collect::<anyhow::Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        if records.is_empty() {
            return Ok(Vec::new());
        }
        let held: Vec<VideoRecordId> = records.iter().map(|(id, _)| id.clone()).collect();
        let yes = self.yes_categories(&held)?;
        Ok(records
            .into_iter()
            .map(|(video_id, category_ids)| {
                let mut ids = category_ids;
                ids.extend(yes.get(&video_id).cloned().unwrap_or_default());
                VideoCategories::new(video_id, ids)
            })
            .collect())
    }

    /// The channel holding `video_id`, else the playlist holding it.
    fn source_of(&self, video_id: &VideoRecordId) -> anyhow::Result<Option<CategoryMember>> {
        if let Some(channel_video) = self.channel_video_repository.find_by_video(video_id)? {
            return Ok(Some(CategoryMember::Channel(channel_video.channel_id)));
        }
        Ok(self
            .playlist_video_repository
            .find_by_video(video_id)?
            .map(|playlist_video| CategoryMember::Playlist(playlist_video.playlist_id)))
    }

    /// Every record of `source`, each with the source's categories in
    /// `categories` plus its yes verdicts.
    fn desired_for_source(
        &self,
        source: &CategoryMember,
        categories: &[Category],
    ) -> anyhow::Result<Vec<VideoCategories>> {
        let record_ids = self.record_ids(source)?;
        self.with_yes_verdicts(record_ids, &Self::source_category_ids(source, categories))
    }

    /// Each of `video_ids` in `category_ids` plus the categories of its yes
    /// verdicts.
    fn with_yes_verdicts(
        &self,
        video_ids: Vec<VideoRecordId>,
        category_ids: &[CategoryId],
    ) -> anyhow::Result<Vec<VideoCategories>> {
        if video_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut yes = self.yes_categories(&video_ids)?;
        Ok(video_ids
            .into_iter()
            .map(|video_id| {
                let mut ids = category_ids.to_vec();
                ids.extend(yes.remove(&video_id).unwrap_or_default());
                VideoCategories::new(video_id, ids)
            })
            .collect())
    }

    /// The categories of the yes verdicts of each of `video_ids` having one.
    fn yes_categories(
        &self,
        video_ids: &[VideoRecordId],
    ) -> anyhow::Result<HashMap<VideoRecordId, Vec<CategoryId>>> {
        Ok(self
            .video_category_verdict_repository
            .find_many(video_ids)?
            .into_iter()
            .filter(VideoCategoryVerdict::is_yes)
            .fold(HashMap::new(), |mut yes, verdict| {
                yes.entry(verdict.video_id)
                    .or_default()
                    .push(verdict.category_id);
                yes
            }))
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
        if desired.is_empty() {
            return Ok(Vec::new());
        }
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
