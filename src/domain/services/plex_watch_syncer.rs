use crate::domain::services::{PlexEpisodeFinder, VideoWatchStateUpdater};
use crate::domain::task::Task;
use crate::infrastructure::repositories::plex_library_repository::PlexLibraryRepository;
use crate::infrastructure::repositories::sqlite_episode_watch_repository::EpisodeWatchRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::system_clock::Clock;
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;

/// How often a watch sync runs.
pub const SYNC_INTERVAL: Duration = Duration::minutes(5);
/// Episodes written to Plex per sync; more ⇒ the next sync runs now.
pub const BATCH_SIZE: usize = 500;

/// Keeps each downloaded video's watch state and its Plex episode's in
/// sync, both ways.
#[derive(Clone)]
pub struct PlexWatchSyncer {
    plex_library_repository: Arc<dyn PlexLibraryRepository>,
    plex_episode_finder: PlexEpisodeFinder,
    episode_watch_repository: Arc<dyn EpisodeWatchRepository>,
    video_repository: Arc<dyn VideoRepository>,
    video_watch_state_updater: VideoWatchStateUpdater,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
}

impl PlexWatchSyncer {
    pub fn new(
        plex_library_repository: Arc<dyn PlexLibraryRepository>,
        plex_episode_finder: PlexEpisodeFinder,
        episode_watch_repository: Arc<dyn EpisodeWatchRepository>,
        video_repository: Arc<dyn VideoRepository>,
        video_watch_state_updater: VideoWatchStateUpdater,
        task_repository: Arc<dyn TaskRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            plex_library_repository,
            plex_episode_finder,
            episode_watch_repository,
            video_repository,
            video_watch_state_updater,
            task_repository,
            clock,
        }
    }
}

pub trait PlexWatchSyncerApi: Send + Sync {
    /// A pending sync moves forward to `run_at` if due later, else one is
    /// scheduled.
    fn request_sync(&self, run_at: DateTime<Utc>) -> anyhow::Result<()>;
    /// One sync: pull, then push, then request the next one. Plex
    /// unreachable ⇒ logs, requests the next sync, `Ok`.
    fn sync(&self) -> anyhow::Result<()>;
}

impl PlexWatchSyncerApi for PlexWatchSyncer {
    fn request_sync(&self, run_at: DateTime<Utc>) -> anyhow::Result<()> {
        match self
            .task_repository
            .find_pending(&Task::SyncPlexWatchState)?
        {
            Some(pending) => self
                .task_repository
                .update(&pending.bring_forward(run_at, self.clock.now())),
            None => self
                .task_repository
                .schedule(&Task::SyncPlexWatchState, run_at),
        }
    }

    fn sync(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
