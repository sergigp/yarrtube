use crate::domain::category::{CategoryId, CategoryName};
use crate::domain::plex::{PlexEpisode, PlexLabels, PlexSection, plex_folder_path};
use crate::domain::plex_label::{CategoryPlaylist, EpisodeLabels};
use crate::domain::services::{VideoOwnerFinder, VideoOwnerFinderApi};
use crate::domain::task::Task;
use crate::domain::video::{Video, VideoRecordId};
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
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use tracing::warn;

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

/// The Plex library sections holding yarrtube's videos, as one sync reads
/// them.
struct PlexLibrary {
    sections: Vec<PlexSection>,
    /// Every episode of `sections`, by its server-side file.
    episodes: HashMap<String, PlexEpisode>,
}

/// A smart playlist a category should have in a section.
struct DesiredPlaylist {
    section_id: String,
    label: String,
    title: String,
}

impl DesiredPlaylist {
    /// Whether `playlist` is this one: for this section and label, under
    /// this title. One with another title is replaced.
    fn is(&self, playlist: &CategoryPlaylist) -> bool {
        playlist.section_id == self.section_id
            && playlist.label.to_lowercase() == self.label.to_lowercase()
            && playlist.title == self.title
    }
}

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

    fn request_sync(&self, run_at: DateTime<Utc>) -> anyhow::Result<()> {
        self.task_repository.schedule(&Task::SyncPlexLabels, run_at)
    }

    fn sync(&self) -> anyhow::Result<()> {
        let library = match self.read_library() {
            Ok(library) => library,
            Err(e) => {
                warn!(error = %e, "failed to reach Plex, retrying the label sync later");
                return self.request_sync(self.clock.now() + SYNC_INTERVAL);
            }
        };
        self.mark_reimported(&library.episodes)?;
        self.track_categorized_records()?;
        let labelled = self.label_dirty_records(&library.episodes)?;
        self.reconcile_playlists(&library.sections)?;
        self.request_next_sync(labelled == BATCH_SIZE)
    }
}

impl PlexLabelSyncer {
    /// Labels a batch of dirty records, least recently checked first;
    /// returns how many were taken.
    fn label_dirty_records(
        &self,
        episodes: &HashMap<String, PlexEpisode>,
    ) -> anyhow::Result<usize> {
        let rows = self.episode_labels_repository.list_dirty(BATCH_SIZE)?;
        let taken = rows.len();
        let desired = self.desired_labels(&rows)?;
        let labelled = rows
            .into_iter()
            .filter_map(|row| self.label(row, &desired, episodes).transpose())
            .collect::<anyhow::Result<Vec<_>>>()?;
        self.episode_labels_repository.save_many(&labelled)?;
        Ok(taken)
    }

    /// Keeps one smart playlist per label and section holding labelled
    /// episodes.
    fn reconcile_playlists(&self, sections: &[PlexSection]) -> anyhow::Result<()> {
        let desired = self.desired_playlists(sections)?;
        let (live, gone) = self.stored_playlists()?;
        gone.iter()
            .try_for_each(|playlist| self.category_playlist_repository.delete(playlist))?;
        live.iter()
            .filter(|playlist| !desired.iter().any(|desired| desired.is(playlist)))
            .try_for_each(|playlist| self.delete_playlist(playlist))?;
        desired
            .into_iter()
            .filter(|desired| !live.iter().any(|playlist| desired.is(playlist)))
            .try_for_each(|desired| self.create_playlist(desired))
    }

    /// The playlists yarrtube created: those still in Plex, then those
    /// deleted in Plex since.
    fn stored_playlists(&self) -> anyhow::Result<(Vec<CategoryPlaylist>, Vec<CategoryPlaylist>)> {
        let in_plex: HashSet<String> = self
            .plex_library_repository
            .list_playlists()?
            .into_iter()
            .map(|playlist| playlist.rating_key)
            .collect();
        Ok(self
            .category_playlist_repository
            .list()?
            .into_iter()
            .partition(|playlist| in_plex.contains(&playlist.rating_key)))
    }

    /// One playlist per label written to a synced episode and the episode's
    /// section, titled by the label and, when the label spans sections, the
    /// section's title.
    fn desired_playlists(&self, sections: &[PlexSection]) -> anyhow::Result<Vec<DesiredPlaylist>> {
        let labelled: BTreeSet<(String, String)> = self
            .episode_labels_repository
            .list_synced()?
            .into_iter()
            .flat_map(|row| {
                let section_id = row.section_id.unwrap_or_default();
                row.labels
                    .names()
                    .iter()
                    .map(|label| (section_id.clone(), label.clone()))
                    .collect::<Vec<_>>()
            })
            .collect();
        Ok(labelled
            .iter()
            .map(|(section_id, label)| DesiredPlaylist {
                section_id: section_id.clone(),
                label: label.clone(),
                title: playlist_title(label, section_id, &labelled, sections),
            })
            .collect())
    }

    fn delete_playlist(&self, playlist: &CategoryPlaylist) -> anyhow::Result<()> {
        self.plex_library_repository
            .delete_playlist(&playlist.rating_key)?;
        self.category_playlist_repository.delete(playlist)
    }

    fn create_playlist(&self, desired: DesiredPlaylist) -> anyhow::Result<()> {
        let server_id = self.plex_library_repository.server_id()?;
        let playlist = self.plex_library_repository.create_smart_playlist(
            &server_id,
            &desired.section_id,
            &desired.label,
            &desired.title,
        )?;
        self.category_playlist_repository.save(&CategoryPlaylist {
            section_id: desired.section_id,
            label: desired.label,
            rating_key: playlist.rating_key,
            title: playlist.title,
        })
    }

    /// Right away while a backlog of dirty records remains, else after
    /// `SYNC_INTERVAL`.
    fn request_next_sync(&self, backlog: bool) -> anyhow::Result<()> {
        let now = self.clock.now();
        self.request_sync(if backlog { now } else { now + SYNC_INTERVAL })
    }

    /// The sections holding yarrtube's videos and their episodes.
    fn read_library(&self) -> anyhow::Result<PlexLibrary> {
        let sections = self.served_sections()?;
        let episodes = self.episodes_by_file(&sections)?;
        Ok(PlexLibrary { sections, episodes })
    }

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

    /// Marks dirty every synced row whose episode Plex no longer lists, so
    /// an episode Plex re-imported under a new rating key is labelled again.
    fn mark_reimported(&self, episodes: &HashMap<String, PlexEpisode>) -> anyhow::Result<()> {
        let rating_keys: HashSet<&str> = episodes
            .values()
            .map(|episode| episode.rating_key.as_str())
            .collect();
        let reimported: Vec<EpisodeLabels> = self
            .episode_labels_repository
            .list_synced()?
            .into_iter()
            .filter(|row| {
                row.rating_key
                    .as_deref()
                    .is_some_and(|key| !rating_keys.contains(key))
            })
            .map(EpisodeLabels::mark_dirty)
            .collect();
        self.episode_labels_repository.save_many(&reimported)
    }

    /// Starts tracking, dirty, every categorized record without a row: one
    /// categorized before this sync existed, or whose row was lost.
    fn track_categorized_records(&self) -> anyhow::Result<()> {
        let categorized = self.categorized_records()?;
        let tracked: HashSet<VideoRecordId> = self
            .episode_labels_repository
            .find_many(&categorized)?
            .into_iter()
            .map(|row| row.video_id)
            .collect();
        let untracked: Vec<EpisodeLabels> = categorized
            .into_iter()
            .filter(|id| !tracked.contains(id))
            .map(EpisodeLabels::pending)
            .collect();
        self.episode_labels_repository.save_many(&untracked)
    }

    /// Every record in at least one category, each once.
    fn categorized_records(&self) -> anyhow::Result<Vec<VideoRecordId>> {
        let ids: HashSet<VideoRecordId> = self
            .category_repository
            .list()?
            .iter()
            .map(|category| {
                self.video_categories_repository
                    .list_in_category(&category.id)
            })
            .collect::<anyhow::Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .map(|categories| categories.video_id)
            .collect();
        Ok(ids.into_iter().collect())
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

    /// Writes the record's desired labels to its episode; `None` once the
    /// record is gone and its row deleted.
    fn label(
        &self,
        row: EpisodeLabels,
        desired: &HashMap<VideoRecordId, PlexLabels>,
        episodes: &HashMap<String, PlexEpisode>,
    ) -> anyhow::Result<Option<EpisodeLabels>> {
        let Some(video) = self.video_repository.find(&row.video_id)? else {
            self.episode_labels_repository.delete(&row.video_id)?;
            return Ok(None);
        };
        let Some(episode) = self.find_episode(&video, episodes)? else {
            return Ok(Some(row.checked(self.clock.now())));
        };
        let desired = desired
            .get(&row.video_id)
            .cloned()
            .unwrap_or_else(PlexLabels::none);
        Ok(Some(self.apply_labels(row, &episode, desired)))
    }

    /// The row once `desired` is written to `episode`; still dirty when
    /// Plex rejects it, so a later sync retries.
    fn apply_labels(
        &self,
        row: EpisodeLabels,
        episode: &PlexEpisode,
        desired: PlexLabels,
    ) -> EpisodeLabels {
        let now = self.clock.now();
        match self.write_labels(episode, &desired) {
            Ok(()) => row.synced(episode, desired, now),
            Err(e) => {
                warn!(video_id = %row.video_id, rating_key = %episode.rating_key, error = %e, "failed to write Plex labels, retrying on a later sync");
                row.checked(now)
            }
        }
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
        video: &Video,
        episodes: &HashMap<String, PlexEpisode>,
    ) -> anyhow::Result<Option<PlexEpisode>> {
        let Some(owner) = self.video_owner_finder.find(video)? else {
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

/// `label`, followed by the section's title when `label` has playlists in
/// more than one section.
fn playlist_title(
    label: &str,
    section_id: &str,
    labelled: &BTreeSet<(String, String)>,
    sections: &[PlexSection],
) -> String {
    let spans_sections = labelled.iter().filter(|(_, other)| other == label).count() > 1;
    match sections.iter().find(|section| section.id == section_id) {
        Some(section) if spans_sections => format!("{label} · {}", section.title),
        _ => label.to_string(),
    }
}
