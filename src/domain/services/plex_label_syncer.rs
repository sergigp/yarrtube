use crate::domain::services::VideoOwnerFinder;
use crate::domain::video::VideoRecordId;
use crate::infrastructure::repositories::plex_library_repository::PlexLibraryRepository;
use crate::infrastructure::repositories::sqlite_category_playlist_repository::CategoryPlaylistRepository;
use crate::infrastructure::repositories::sqlite_category_repository::CategoryRepository;
use crate::infrastructure::repositories::sqlite_episode_labels_repository::EpisodeLabelsRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_categories_repository::VideoCategoriesRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::system_clock::Clock;
use chrono::{DateTime, Duration, Utc};
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
        Ok(())
    }
}
