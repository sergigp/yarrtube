use crate::domain::plex::PlexEpisode;
use crate::domain::plex_watch::{EpisodeWatch, MergeOutcome, WatchSide, merge};
use crate::domain::services::{
    PlexEpisodeFinder, PlexEpisodeFinderApi, PlexLibrary, VideoWatchStateUpdater,
    VideoWatchStateUpdaterApi,
};
use crate::domain::task::Task;
use crate::domain::video::{Video, VideoId, WatchProgress};
use crate::infrastructure::repositories::plex_library_repository::PlexLibraryRepository;
use crate::infrastructure::repositories::sqlite_episode_watch_repository::EpisodeWatchRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::system_clock::Clock;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::warn;

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
        let library = self.plex_episode_finder.read_library()?;
        let rows = self.agreed_rows()?;
        let merged = self.merge_videos(&library, &rows)?;
        let merged = if self.pull(&merged)? {
            self.merge_videos(&library, &rows)?
        } else {
            merged
        };
        let synced: Vec<EpisodeWatch> = merged
            .into_iter()
            .filter_map(|(video, episode, outcome)| self.settle(&video, episode, outcome))
            .collect();
        self.episode_watch_repository.save_many(&synced)
    }
}

impl PlexWatchSyncer {
    /// Every stored row, by video id.
    fn agreed_rows(&self) -> anyhow::Result<HashMap<VideoId, EpisodeWatch>> {
        Ok(self
            .episode_watch_repository
            .list()?
            .into_iter()
            .map(|row| (row.video_id.clone(), row))
            .collect())
    }

    /// Each downloaded video with an episode in Plex, its episode and how
    /// their states merge.
    fn merge_videos<'a>(
        &self,
        library: &'a PlexLibrary,
        rows: &HashMap<VideoId, EpisodeWatch>,
    ) -> anyhow::Result<Vec<(Video, &'a PlexEpisode, MergeOutcome)>> {
        let videos = self.video_repository.list_downloaded()?;
        let found = videos
            .into_iter()
            .map(|video| {
                self.plex_episode_finder
                    .find_episode(&video, library)
                    .map(|episode| episode.map(|episode| (video, episode)))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(found
            .into_iter()
            .flatten()
            .map(|(video, episode)| {
                let outcome = merge(
                    local_side(&video),
                    remote_side(episode),
                    EpisodeWatch::agreed_with(rows.get(&video.id), episode),
                );
                (video, episode, outcome)
            })
            .collect())
    }

    /// Copies each pulled state to its video (and every copy of it);
    /// whether anything was pulled.
    fn pull(&self, merged: &[(Video, &PlexEpisode, MergeOutcome)]) -> anyhow::Result<bool> {
        let pulls: Vec<(&Video, &PlexEpisode, WatchProgress)> = merged
            .iter()
            .filter_map(|(video, episode, outcome)| match outcome {
                MergeOutcome::Pull(progress) => Some((video, *episode, *progress)),
                _ => None,
            })
            .collect();
        let now = self.clock.now();
        pulls.iter().try_for_each(|(video, episode, progress)| {
            self.video_watch_state_updater
                .adopt(
                    &video.id,
                    *progress,
                    episode.watch.last_viewed_at.unwrap_or(now),
                )
                .map_err(anyhow::Error::from)
        })?;
        Ok(!pulls.is_empty())
    }

    /// The row to store once `outcome` is applied, `None` when nothing was
    /// agreed (Plex rejected the push).
    fn settle(
        &self,
        video: &Video,
        episode: &PlexEpisode,
        outcome: MergeOutcome,
    ) -> Option<EpisodeWatch> {
        match outcome {
            MergeOutcome::InSync(progress) => {
                Some(EpisodeWatch::new(video.id.clone(), episode, progress))
            }
            MergeOutcome::Push(progress) => self
                .push(episode, progress)
                .inspect_err(|e| warn!(video_id = %video.id, rating_key = %episode.rating_key, error = %e, "failed to write the Plex watch state, retrying on a later sync"))
                .ok()
                .map(|()| EpisodeWatch::new(video.id.clone(), episode, progress)),
            MergeOutcome::Pull(_) => None,
        }
    }

    /// Plex keeps a watched episode watched when only its position is set.
    fn unwatch_if_watched(&self, episode: &PlexEpisode) -> anyhow::Result<()> {
        if episode.watch.view_count > 0 {
            self.plex_library_repository
                .mark_unwatched(&episode.rating_key)?;
        }
        Ok(())
    }

    /// Writes `progress` to the episode.
    fn push(&self, episode: &PlexEpisode, progress: WatchProgress) -> anyhow::Result<()> {
        match progress {
            WatchProgress::Watched => self
                .plex_library_repository
                .mark_watched(&episode.rating_key),
            WatchProgress::InProgress(position) => {
                self.unwatch_if_watched(episode)?;
                self.plex_library_repository
                    .save_position(&episode.rating_key, position)
            }
            WatchProgress::Unplayed => self
                .plex_library_repository
                .mark_unwatched(&episode.rating_key),
        }
    }
}

fn local_side(video: &Video) -> WatchSide {
    WatchSide {
        progress: video.watch.progress(),
        changed_at: video.watch.changed_at(),
    }
}

fn remote_side(episode: &PlexEpisode) -> WatchSide {
    WatchSide {
        progress: episode.watch.progress(),
        changed_at: episode.watch.last_viewed_at,
    }
}
