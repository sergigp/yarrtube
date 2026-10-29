use crate::domain::category::{CategoryId, CategoryName};
use crate::domain::plex::{PlexEpisode, PlexLabels, PlexSection, plex_folder_path};
use crate::domain::plex_label::EpisodeLabels;
use crate::domain::services::{VideoOwnerFinder, VideoOwnerFinderApi};
use crate::domain::video::VideoRecordId;
use crate::domain::video_category::VideoCategories;
use crate::infrastructure::repositories::plex_library_repository::PlexLibraryRepository;
use crate::infrastructure::repositories::sqlite_category_playlist_repository::CategoryPlaylistRepository;
use crate::infrastructure::repositories::sqlite_category_repository::CategoryRepository;
use crate::infrastructure::repositories::sqlite_episode_labels_repository::EpisodeLabelsRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_categories_repository::VideoCategoriesRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::system_clock::Clock;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// How often a label sync runs to heal drift.
pub const SYNC_INTERVAL: Duration = Duration::minutes(15);
/// How long after a download its label sync runs, so Plex's asynchronous
/// scan has imported the episode.
pub const AFTER_DOWNLOAD_DELAY: Duration = Duration::seconds(60);
/// A reconcile pass publishes one `VideoCategoriesChanged` per repaired
/// video; the first one schedules the sync this far out and `bring_forward`
/// keeps it there, so the pass's events collapse into one sync.
pub const CATEGORY_CHANGE_DELAY: Duration = Duration::seconds(10);
/// Dirty records labelled per sync; more dirty ⇒ the next sync runs now.
pub const BATCH_SIZE: usize = 500;

/// Mirrors each stored video's categories into its Plex episode's labels,
/// and keeps one smart playlist per category and Plex library on them. Plex
/// sees yarrtube's `videos_path` at `plex_videos_path`.
#[derive(Clone)]
pub struct PlexLabelSyncer {
    plex_library_repository: Arc<dyn PlexLibraryRepository>,
    episode_labels_repository: Arc<dyn EpisodeLabelsRepository>,
    category_playlist_repository: Arc<dyn CategoryPlaylistRepository>,
    video_categories_repository: Arc<dyn VideoCategoriesRepository>,
    category_repository: Arc<dyn CategoryRepository>,
    video_repository: Arc<dyn VideoRepository>,
    video_owner_finder: Arc<VideoOwnerFinder>,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
    videos_path: String,
    plex_videos_path: String,
}

impl PlexLabelSyncer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        plex_library_repository: Arc<dyn PlexLibraryRepository>,
        episode_labels_repository: Arc<dyn EpisodeLabelsRepository>,
        category_playlist_repository: Arc<dyn CategoryPlaylistRepository>,
        video_categories_repository: Arc<dyn VideoCategoriesRepository>,
        category_repository: Arc<dyn CategoryRepository>,
        video_repository: Arc<dyn VideoRepository>,
        video_owner_finder: Arc<VideoOwnerFinder>,
        task_repository: Arc<dyn TaskRepository>,
        clock: Arc<dyn Clock>,
        videos_path: impl Into<String>,
        plex_videos_path: impl Into<String>,
    ) -> Self {
        Self {
            plex_library_repository,
            episode_labels_repository,
            category_playlist_repository,
            video_categories_repository,
            category_repository,
            video_repository,
            video_owner_finder,
            task_repository,
            clock,
            videos_path: videos_path.into(),
            plex_videos_path: plex_videos_path.into(),
        }
    }
}

pub trait PlexLabelSyncerApi: Send + Sync {
    /// Marks the records dirty (creating their rows) and requests a sync at
    /// now + `delay`.
    fn mark(&self, video_ids: &[VideoRecordId], delay: Duration) -> anyhow::Result<()>;
    /// A pending sync moves forward to `run_at` if due later, else one is
    /// scheduled.
    fn request_sync(&self, run_at: DateTime<Utc>) -> anyhow::Result<()>;
    /// One sync: heals drift, labels a batch of dirty records, reconciles
    /// playlists, then requests the next sync (now if dirty records remain,
    /// else now + `SYNC_INTERVAL`). Plex unreachable ⇒ logs, keeps
    /// everything dirty, still requests the next sync, `Ok`.
    fn sync(&self) -> anyhow::Result<()>;
}

impl PlexLabelSyncerApi for PlexLabelSyncer {
    fn mark(&self, _video_ids: &[VideoRecordId], _delay: Duration) -> anyhow::Result<()> {
        Ok(())
    }

    fn request_sync(&self, _run_at: DateTime<Utc>) -> anyhow::Result<()> {
        Ok(())
    }

    fn sync(&self) -> anyhow::Result<()> {
        let sections = self.served_sections()?;
        let episodes = self.episodes_by_file(&sections)?;
        let rows = self.episode_labels_repository.list_dirty(BATCH_SIZE)?;
        let desired = self.desired_labels(&rows)?;
        let labelled = rows
            .into_iter()
            .map(|row| self.label(row, &desired, &episodes))
            .collect::<anyhow::Result<Vec<_>>>()?;
        self.episode_labels_repository.save_many(&labelled)
    }
}

impl PlexLabelSyncer {
    /// The library sections holding yarrtube's videos.
    fn served_sections(&self) -> anyhow::Result<Vec<PlexSection>> {
        Ok(self
            .plex_library_repository
            .list_sections()?
            .into_iter()
            .filter(|section| section.serves(&self.plex_videos_path))
            .collect())
    }

    /// Every episode of `sections`, by its server-side file.
    fn episodes_by_file(
        &self,
        sections: &[PlexSection],
    ) -> anyhow::Result<HashMap<String, PlexEpisode>> {
        sections
            .iter()
            .map(|section| self.plex_library_repository.list_episodes(&section.id))
            .collect::<anyhow::Result<Vec<_>>>()
            .map(|episodes| {
                episodes
                    .into_iter()
                    .flatten()
                    .map(|episode| (episode.file.clone(), episode))
                    .collect()
            })
    }

    /// The labels each row's record should carry: its categories' names.
    fn desired_labels(
        &self,
        rows: &[EpisodeLabels],
    ) -> anyhow::Result<HashMap<VideoRecordId, PlexLabels>> {
        let names: HashMap<CategoryId, CategoryName> = self
            .category_repository
            .list()?
            .into_iter()
            .map(|category| (category.id, category.name))
            .collect();
        let ids: Vec<VideoRecordId> = rows.iter().map(|row| row.video_id.clone()).collect();
        Ok(self
            .video_categories_repository
            .find_many(&ids)?
            .into_iter()
            .map(|categories| (categories.video_id.clone(), labels_of(&categories, &names)))
            .collect())
    }

    /// Writes the record's desired labels to its episode.
    fn label(
        &self,
        row: EpisodeLabels,
        desired: &HashMap<VideoRecordId, PlexLabels>,
        episodes: &HashMap<String, PlexEpisode>,
    ) -> anyhow::Result<EpisodeLabels> {
        let Some(episode) = self.find_episode(&row.video_id, episodes)? else {
            return Ok(row);
        };
        let desired = desired
            .get(&row.video_id)
            .cloned()
            .unwrap_or_else(PlexLabels::none);
        self.write_labels(&episode, &desired)?;
        Ok(row.synced(&episode, desired, self.clock.now()))
    }

    /// Makes the episode's labels exactly `desired`, unless they already are.
    fn write_labels(&self, episode: &PlexEpisode, desired: &PlexLabels) -> anyhow::Result<()> {
        let current = self
            .plex_library_repository
            .find_labels(&episode.rating_key)?;
        if &current == desired {
            return Ok(());
        }
        self.plex_library_repository
            .save_labels(episode, desired, &current.missing_from(desired))
    }

    /// The Plex episode of the record's file.
    fn find_episode(
        &self,
        video_id: &VideoRecordId,
        episodes: &HashMap<String, PlexEpisode>,
    ) -> anyhow::Result<Option<PlexEpisode>> {
        let Some(video) = self.video_repository.find(video_id)? else {
            return Ok(None);
        };
        let Some(owner) = self.video_owner_finder.find(&video)? else {
            return Ok(None);
        };
        Ok(video
            .filename
            .as_deref()
            .map(|filename| owner.target.output_dir.join(filename))
            .and_then(|file| {
                plex_folder_path(Path::new(&self.videos_path), &self.plex_videos_path, &file)
            })
            .and_then(|file| episodes.get(&file).cloned()))
    }
}

/// The names of the record's categories.
fn labels_of(
    categories: &VideoCategories,
    names: &HashMap<CategoryId, CategoryName>,
) -> PlexLabels {
    PlexLabels::new(
        categories
            .category_ids
            .iter()
            .filter_map(|id| names.get(id))
            .map(|name| name.as_str().to_string())
            .collect(),
    )
}
