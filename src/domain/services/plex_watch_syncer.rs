use crate::domain::plex::PlexEpisode;
use crate::domain::plex_watch::{EpisodeWatch, MergeOutcome, WatchSide, merge};
use crate::domain::services::{
    PlexEpisodeFinder, PlexEpisodeFinderApi, PlexLibrary, VideoWatchStateUpdater,
    VideoWatchStateUpdaterApi,
};
use crate::domain::task::Task;
use crate::domain::video::{Video, VideoId, WatchProgress, YoutubeVideoId};
use crate::infrastructure::repositories::plex_library_repository::PlexLibraryRepository;
use crate::infrastructure::repositories::sqlite_episode_watch_repository::EpisodeWatchRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::system_clock::Clock;
use chrono::{DateTime, Duration, Utc};
use std::collections::{HashMap, HashSet};
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
        let (library, viewer_id) = match self.read_plex() {
            Ok(plex) => plex,
            Err(e) => {
                warn!(error = %e, "failed to reach Plex, retrying the watch sync later");
                return self.request_sync(self.clock.now() + SYNC_INTERVAL);
            }
        };
        let rows = self.agreed_rows()?;
        self.forget_undownloaded(&rows)?;
        let merged = self.pull_then_merge(&library, &rows, &viewer_id)?;
        let backlog = self.settle_all(merged, &viewer_id)?;
        self.request_next_sync(backlog)
    }
}

impl PlexWatchSyncer {
    /// The library and whose watch state it holds.
    fn read_plex(&self) -> anyhow::Result<(PlexLibrary, String)> {
        let library = self.plex_episode_finder.read_library()?;
        let viewer_id = self.plex_library_repository.viewer_id()?;
        Ok((library, viewer_id))
    }

    /// Every stored row, by video id.
    fn agreed_rows(&self) -> anyhow::Result<HashMap<VideoId, EpisodeWatch>> {
        Ok(self
            .episode_watch_repository
            .list()?
            .into_iter()
            .map(|row| (row.video_id.clone(), row))
            .collect())
    }

    /// Deletes the rows of videos no longer downloaded (or gone).
    fn forget_undownloaded(&self, rows: &HashMap<VideoId, EpisodeWatch>) -> anyhow::Result<()> {
        let downloaded: HashSet<VideoId> = self
            .video_repository
            .list_downloaded()?
            .into_iter()
            .map(|video| video.id)
            .collect();
        let gone: Vec<VideoId> = rows
            .keys()
            .filter(|id| !downloaded.contains(*id))
            .cloned()
            .collect();
        self.episode_watch_repository.delete_many(&gone)
    }

    /// Each downloaded video with an episode in Plex, its episode and how
    /// their states merge.
    fn merge_videos<'a>(
        &self,
        library: &'a PlexLibrary,
        rows: &HashMap<VideoId, EpisodeWatch>,
        viewer_id: &str,
    ) -> anyhow::Result<Vec<(Video, &'a PlexEpisode, MergeOutcome)>> {
        self.video_repository
            .list_downloaded()?
            .into_iter()
            .filter_map(|video| {
                self.plex_episode_finder
                    .find_episode(&video, library)
                    .transpose()
                    .map(|episode| {
                        episode.map(|episode| {
                            let outcome = merge(
                                local_side(&video),
                                remote_side(episode),
                                EpisodeWatch::agreed_with(rows.get(&video.id), episode, viewer_id),
                            );
                            (video, episode, outcome)
                        })
                    })
            })
            .collect()
    }

    /// Merges every video with its episode, pulling what Plex changed
    /// first: once pulled, the pulled copies are in sync and the other
    /// copies of the same YouTube video have something to push.
    fn pull_then_merge<'a>(
        &self,
        library: &'a PlexLibrary,
        rows: &HashMap<VideoId, EpisodeWatch>,
        viewer_id: &str,
    ) -> anyhow::Result<Vec<(Video, &'a PlexEpisode, MergeOutcome)>> {
        let merged = self.merge_videos(library, rows, viewer_id)?;
        if self.pull(&merged)? {
            self.merge_videos(library, rows, viewer_id)
        } else {
            Ok(merged)
        }
    }

    /// Records what is in sync and pushes up to `BATCH_SIZE` states to
    /// Plex; whether pushes were left for the next sync.
    fn settle_all(
        &self,
        merged: Vec<(Video, &PlexEpisode, MergeOutcome)>,
        viewer_id: &str,
    ) -> anyhow::Result<bool> {
        let (pushes, others): (Vec<_>, Vec<_>) = merged
            .into_iter()
            .partition(|(_, _, outcome)| matches!(outcome, MergeOutcome::Push(_)));
        let backlog = pushes.len() > BATCH_SIZE;
        let synced: Vec<EpisodeWatch> = others
            .into_iter()
            .chain(pushes.into_iter().take(BATCH_SIZE))
            .filter_map(|(video, episode, outcome)| {
                self.settle(&video, episode, outcome, viewer_id)
            })
            .collect();
        self.episode_watch_repository.save_many(&synced)?;
        Ok(backlog)
    }

    /// Right away while pushes remain, else after `SYNC_INTERVAL`.
    fn request_next_sync(&self, backlog: bool) -> anyhow::Result<()> {
        let now = self.clock.now();
        self.request_sync(if backlog { now } else { now + SYNC_INTERVAL })
    }

    /// Copies each pulled state to its video (and every copy of it);
    /// whether anything was pulled.
    fn pull(&self, merged: &[(Video, &PlexEpisode, MergeOutcome)]) -> anyhow::Result<bool> {
        let pulls =
            newest_per_youtube_video(merged.iter().filter_map(|(video, episode, outcome)| {
                match outcome {
                    MergeOutcome::Pull(progress) => Some((video, *episode, *progress)),
                    _ => None,
                }
            }));
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
        viewer_id: &str,
    ) -> Option<EpisodeWatch> {
        match outcome {
            MergeOutcome::InSync(progress) => {
                Some(EpisodeWatch::new(video.id.clone(), episode, viewer_id, progress))
            }
            MergeOutcome::Push(progress) => self
                .push(episode, progress)
                .inspect_err(|e| warn!(video_id = %video.id, rating_key = %episode.rating_key, error = %e, "failed to write the Plex watch state, retrying on a later sync"))
                .ok()
                .map(|()| EpisodeWatch::new(video.id.clone(), episode, viewer_id, progress)),
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

/// One pull per YouTube video: the one whose episode Plex played last, so
/// copies changed differently in Plex end up as the latest change.
fn newest_per_youtube_video<'a>(
    pulls: impl Iterator<Item = (&'a Video, &'a PlexEpisode, WatchProgress)>,
) -> Vec<(&'a Video, &'a PlexEpisode, WatchProgress)> {
    pulls
        .fold(
            HashMap::<&YoutubeVideoId, (&Video, &PlexEpisode, WatchProgress)>::new(),
            |mut newest, pull| {
                let played_later = |kept: &(&Video, &PlexEpisode, WatchProgress)| {
                    pull.1.watch.last_viewed_at > kept.1.watch.last_viewed_at
                };
                if newest.get(&pull.0.youtube_id).is_none_or(played_later) {
                    newest.insert(&pull.0.youtube_id, pull);
                }
                newest
            },
        )
        .into_values()
        .collect()
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
