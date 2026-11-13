use crate::domain::category::{CategoryId, CategoryName};
use crate::domain::plex::{PlexEpisode, PlexLabels, PlexSection};
use crate::domain::plex_label::{CategoryPlaylist, EpisodeLabels};
use crate::domain::services::{PlexEpisodeFinder, PlexEpisodeFinderApi, PlexLibrary};
use crate::domain::task::Task;
use crate::domain::video::{Video, VideoId};
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
/// Dirty videos labelled per sync; a full batch that made progress ⇒ the
/// next sync runs now.
pub const BATCH_SIZE: usize = 500;

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
/// and keeps one smart playlist per category and Plex library on them.
#[derive(Clone)]
pub struct PlexLabelSyncer {
    plex_library_repository: Arc<dyn PlexLibraryRepository>,
    episode_labels_repository: Arc<dyn EpisodeLabelsRepository>,
    category_playlist_repository: Arc<dyn CategoryPlaylistRepository>,
    video_categories_repository: Arc<dyn VideoCategoriesRepository>,
    category_repository: Arc<dyn CategoryRepository>,
    video_repository: Arc<dyn VideoRepository>,
    plex_episode_finder: PlexEpisodeFinder,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
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
        plex_episode_finder: PlexEpisodeFinder,
        task_repository: Arc<dyn TaskRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            plex_library_repository,
            episode_labels_repository,
            category_playlist_repository,
            video_categories_repository,
            category_repository,
            video_repository,
            plex_episode_finder,
            task_repository,
            clock,
        }
    }
}

pub trait PlexLabelSyncerApi: Send + Sync {
    /// Marks the videos dirty (creating their rows) and requests a sync at
    /// now + `delay`.
    fn mark(&self, video_ids: &[VideoId], delay: Duration) -> anyhow::Result<()>;
    /// A pending sync moves forward to `run_at` if due later, else one is
    /// scheduled.
    fn request_sync(&self, run_at: DateTime<Utc>) -> anyhow::Result<()>;
    /// One sync: heals drift, labels a batch of dirty videos, reconciles
    /// playlists, then requests the next sync (now if the batch was full and
    /// at least one of its records was labelled or forgotten, else now +
    /// `SYNC_INTERVAL`). Plex unreachable ⇒ logs, keeps
    /// everything dirty, still requests the next sync, `Ok`.
    fn sync(&self) -> anyhow::Result<()>;
}

impl PlexLabelSyncerApi for PlexLabelSyncer {
    fn mark(&self, video_ids: &[VideoId], delay: Duration) -> anyhow::Result<()> {
        let existing = self.episode_labels_repository.find_many(video_ids)?;
        let mut existing: HashMap<VideoId, EpisodeLabels> = existing
            .into_iter()
            .map(|row| (row.video_id.clone(), row))
            .collect();
        let marked: Vec<EpisodeLabels> = video_ids
            .iter()
            .map(|id| match existing.remove(id) {
                Some(row) => row.mark_dirty(),
                None => EpisodeLabels::pending(id.clone()),
            })
            .collect();
        self.episode_labels_repository.save_many(&marked)?;
        self.request_sync(self.clock.now() + delay)
    }

    fn request_sync(&self, run_at: DateTime<Utc>) -> anyhow::Result<()> {
        match self.task_repository.find_pending(&Task::SyncPlexLabels)? {
            Some(pending) => self
                .task_repository
                .update(&pending.bring_forward(run_at, self.clock.now())),
            None => self.task_repository.schedule(&Task::SyncPlexLabels, run_at),
        }
    }

    fn sync(&self) -> anyhow::Result<()> {
        let library = match self.plex_episode_finder.read_library() {
            Ok(library) => library,
            Err(e) => {
                warn!(error = %e, "failed to reach Plex, retrying the label sync later");
                return self.request_sync(self.clock.now() + SYNC_INTERVAL);
            }
        };
        self.mark_reimported(&library.episodes)?;
        self.track_categorized_videos()?;
        let backlog = self.label_dirty_videos(&library)?;
        self.reconcile_playlists(&library.sections)?;
        self.request_next_sync(backlog)
    }
}

impl PlexLabelSyncer {
    /// Labels a batch of dirty videos, least recently checked first;
    /// returns whether more may be labellable right away: the batch was full
    /// and at least one of its rows was synced or deleted, so a batch of
    /// videos none of which can be labelled yet never chains syncs.
    fn label_dirty_videos(&self, library: &PlexLibrary) -> anyhow::Result<bool> {
        let rows = self.episode_labels_repository.list_dirty(BATCH_SIZE)?;
        let taken = rows.len();
        let desired = self.desired_labels(&rows)?;
        let ids: Vec<VideoId> = rows.iter().map(|row| row.video_id.clone()).collect();
        let videos: HashMap<VideoId, Video> = self
            .video_repository
            .find_many(&ids)?
            .into_iter()
            .map(|video| (video.id.clone(), video))
            .collect();
        let labelled = rows
            .into_iter()
            .filter_map(|row| {
                let video = videos.get(&row.video_id);
                self.label(row, video, &desired, library).transpose()
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let progressed = labelled.len() < taken || labelled.iter().any(|row| !row.dirty);
        self.episode_labels_repository.save_many(&labelled)?;
        Ok(taken == BATCH_SIZE && progressed)
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

    /// One playlist per category whose label is written to a synced episode,
    /// in the episode's section, titled by the category's name and, when it
    /// spans sections, the section's title. A label no category carries any
    /// more (renamed or deleted, its episode not relabelled yet) counts for
    /// none, and one differing from its category only in case counts as it.
    fn desired_playlists(&self, sections: &[PlexSection]) -> anyhow::Result<Vec<DesiredPlaylist>> {
        let names: HashMap<String, String> = self
            .category_repository
            .list()?
            .into_iter()
            .map(|category| {
                (
                    category.name.as_str().to_lowercase(),
                    category.name.as_str().to_string(),
                )
            })
            .collect();
        let labelled: BTreeSet<(String, String)> = self
            .episode_labels_repository
            .list_synced()?
            .into_iter()
            .flat_map(|row| {
                let section_id = row.section_id.unwrap_or_default();
                row.labels
                    .names()
                    .iter()
                    .filter_map(|label| names.get(&label.to_lowercase()))
                    .map(|name| (section_id.clone(), name.clone()))
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

    /// Right away while a backlog of dirty videos remains, else after
    /// `SYNC_INTERVAL`.
    fn request_next_sync(&self, backlog: bool) -> anyhow::Result<()> {
        let now = self.clock.now();
        self.request_sync(if backlog { now } else { now + SYNC_INTERVAL })
    }

    /// Marks dirty every synced row whose episode Plex no longer lists,
    /// forgetting that episode: one Plex re-imported under a new rating key
    /// is labelled again, and one gone for good keeps no playlist alive.
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
            .map(EpisodeLabels::forget_episode)
            .collect();
        self.episode_labels_repository.save_many(&reimported)
    }

    /// Starts tracking, dirty, every categorized video without a row: one
    /// categorized before this sync existed, or whose row was lost.
    fn track_categorized_videos(&self) -> anyhow::Result<()> {
        let categorized = self.categorized_videos()?;
        let tracked: HashSet<VideoId> = self
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

    /// Every video in at least one category, each once.
    fn categorized_videos(&self) -> anyhow::Result<Vec<VideoId>> {
        let ids: HashSet<VideoId> = self
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
    ) -> anyhow::Result<HashMap<VideoId, PlexLabels>> {
        let names: HashMap<CategoryId, CategoryName> = self
            .category_repository
            .list()?
            .into_iter()
            .map(|category| (category.id, category.name))
            .collect();
        let ids: Vec<VideoId> = rows.iter().map(|row| row.video_id.clone()).collect();
        Ok(self
            .video_categories_repository
            .find_many(&ids)?
            .into_iter()
            .map(|categories| (categories.video_id.clone(), labels_of(&categories, &names)))
            .collect())
    }

    /// Writes the video's desired labels to its episode; `None` once the
    /// record is gone and its row deleted.
    fn label(
        &self,
        row: EpisodeLabels,
        video: Option<&Video>,
        desired: &HashMap<VideoId, PlexLabels>,
        library: &PlexLibrary,
    ) -> anyhow::Result<Option<EpisodeLabels>> {
        let Some(video) = video else {
            self.episode_labels_repository.delete(&row.video_id)?;
            return Ok(None);
        };
        let Some(episode) = self.plex_episode_finder.find_episode(video, library)? else {
            return Ok(Some(row.checked(self.clock.now())));
        };
        let desired = desired
            .get(&row.video_id)
            .cloned()
            .unwrap_or_else(PlexLabels::none);
        Ok(Some(self.apply_labels(row, episode, desired)))
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
}

/// The names of the video's categories.
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
