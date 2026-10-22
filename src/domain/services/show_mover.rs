use crate::domain::channel::{Channel, ChannelHandle};
use crate::domain::event::{DomainEvent, ScheduledEvent};
use crate::domain::playlist::{Playlist, PlaylistId, PlaylistPath};
use crate::domain::show::{MoveShowError, ShowKey, ShowMoved, has_work_in_progress};
use crate::domain::task::ScheduledTask;
use crate::domain::video::VideoRecordId;
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::domain_events::event_repository::EventRepository;
use crate::infrastructure::shared::show_locks::{ShowLockGuard, ShowLocks};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{error, info, warn};

/// Moves a tracked playlist's or channel's show directory to a new storage
/// path, keeping every video record, episode number and watch state.
#[derive(Clone)]
pub struct ShowMover {
    playlist_repository: Arc<dyn PlaylistRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    video_repository: Arc<dyn VideoRepository>,
    task_repository: Arc<dyn TaskRepository>,
    event_repository: Arc<dyn EventRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    show_locks: Arc<ShowLocks>,
    videos_root: PathBuf,
}

impl ShowMover {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        video_repository: Arc<dyn VideoRepository>,
        task_repository: Arc<dyn TaskRepository>,
        event_repository: Arc<dyn EventRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        show_locks: Arc<ShowLocks>,
        videos_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            playlist_repository,
            channel_repository,
            playlist_video_repository,
            channel_video_repository,
            video_repository,
            task_repository,
            event_repository,
            video_file_repository,
            event_publisher,
            show_locks,
            videos_root: videos_root.into(),
        }
    }
}

pub trait ShowMoverApi: Send + Sync {
    /// Renames the playlist's show directory to `path` under the videos root
    /// and stores the playlist with that path, returning it as stored.
    /// Refused while the playlist is being reconciled or still has work in
    /// progress (a video not Downloaded, Errored or Excluded, a per-video
    /// task queued or running in its show directory, or a pending
    /// `VideoDownloaded` event for it), when `path` is its current path, is
    /// used by, inside or contains any source's path or the path of a
    /// deleted source whose folder is not removed yet, or already exists on
    /// disk. A failed rename changes nothing; a failure to
    /// store the new path renames the directory back.
    fn move_playlist(&self, id: PlaylistId, path: PlaylistPath) -> Result<Playlist, MoveShowError>;

    /// `move_playlist` for a channel.
    fn move_channel(&self, id: ChannelHandle, path: PlaylistPath)
    -> Result<Channel, MoveShowError>;
}

impl ShowMoverApi for ShowMover {
    fn move_playlist(&self, id: PlaylistId, path: PlaylistPath) -> Result<Playlist, MoveShowError> {
        let _guard = self.lock(ShowKey::Playlist(id.clone()))?;
        let playlist = self.find_playlist(&id)?;
        let current = playlist.path.clone();
        let moved = playlist.with_path(path.clone());
        self.move_show(
            &current,
            &path,
            || self.playlist_video_ids(&id),
            || self.update_playlist(&moved),
        )?;
        info!(playlist_id = %id, path = %moved.path, "moved playlist");
        Ok(moved)
    }

    fn move_channel(
        &self,
        id: ChannelHandle,
        path: PlaylistPath,
    ) -> Result<Channel, MoveShowError> {
        let _guard = self.lock(ShowKey::Channel(id.clone()))?;
        let channel = self.find_channel(&id)?;
        let current = channel.path.clone();
        let moved = channel.with_path(path.clone());
        self.move_show(
            &current,
            &path,
            || self.channel_video_ids(&id),
            || self.update_channel(&moved),
        )?;
        info!(channel_id = %id, path = %moved.path, "moved channel");
        Ok(moved)
    }
}

impl ShowMover {
    /// A reconcile can hold the show for as long as its YouTube calls take,
    /// so a move doesn't wait for it: it's refused.
    fn lock(&self, key: ShowKey) -> Result<ShowLockGuard<'_>, MoveShowError> {
        self.show_locks
            .try_lock(key)
            .ok_or(MoveShowError::Reconciling)
    }

    fn find_playlist(&self, id: &PlaylistId) -> Result<Playlist, MoveShowError> {
        self.playlist_repository
            .find(id)
            .map_err(MoveShowError::Repository)?
            .ok_or(MoveShowError::NotFound)
    }

    fn find_channel(&self, id: &ChannelHandle) -> Result<Channel, MoveShowError> {
        self.channel_repository
            .find(id)
            .map_err(MoveShowError::Repository)?
            .ok_or(MoveShowError::NotFound)
    }

    /// The moves' shared checks and steps: `path` must be free and the
    /// show's videos (`video_ids`) settled before its directory moves from
    /// `current` to `path` and `record` stores the new path.
    fn move_show(
        &self,
        current: &PlaylistPath,
        path: &PlaylistPath,
        video_ids: impl FnOnce() -> Result<Vec<VideoRecordId>, MoveShowError>,
        record: impl FnOnce() -> Result<(), MoveShowError>,
    ) -> Result<(), MoveShowError> {
        self.ensure_destination_free(current, path)?;
        let (old_dir, new_dir) = (self.show_dir(current), self.show_dir(path));
        self.ensure_settled(&video_ids()?, &old_dir)?;
        self.relocate(&old_dir, &new_dir, record)?;
        self.publish_show_moved(&old_dir, &new_dir);
        Ok(())
    }

    fn ensure_destination_free(
        &self,
        current: &PlaylistPath,
        path: &PlaylistPath,
    ) -> Result<(), MoveShowError> {
        if current == path {
            return Err(MoveShowError::SamePath);
        }
        if self.used_paths()?.iter().any(|used| used.overlaps(path)) {
            return Err(MoveShowError::DestinationInUse(path.clone()));
        }
        if self.video_file_repository.dir_exists(&self.show_dir(path)) {
            return Err(MoveShowError::DestinationExists(path.clone()));
        }
        Ok(())
    }

    fn playlist_video_ids(&self, id: &PlaylistId) -> Result<Vec<VideoRecordId>, MoveShowError> {
        Ok(self
            .playlist_video_repository
            .list_for_playlist(id)
            .map_err(MoveShowError::Repository)?
            .into_iter()
            .map(|playlist_video| playlist_video.video_id)
            .collect())
    }

    fn channel_video_ids(&self, id: &ChannelHandle) -> Result<Vec<VideoRecordId>, MoveShowError> {
        Ok(self
            .channel_video_repository
            .list_for_channel(id)
            .map_err(MoveShowError::Repository)?
            .into_iter()
            .map(|channel_video| channel_video.video_id)
            .collect())
    }

    /// Nothing writes into the show directory on its own any more (see
    /// `has_work_in_progress`).
    fn ensure_settled(&self, ids: &[VideoRecordId], show_dir: &Path) -> Result<(), MoveShowError> {
        let videos = self
            .video_repository
            .find_many(ids)
            .map_err(MoveShowError::Repository)?;
        let tasks = self
            .task_repository
            .list_non_completed()
            .map_err(MoveShowError::Repository)?;
        let events = self
            .event_repository
            .list_eligible()
            .map_err(MoveShowError::Repository)?;
        if has_work_in_progress(&videos, &tasks, &events, show_dir) {
            return Err(MoveShowError::WorkInProgress);
        }
        Ok(())
    }

    /// Every path a move must stay clear of: each source's stored path and
    /// each path still queued for recursive deletion.
    fn used_paths(&self) -> Result<Vec<PlaylistPath>, MoveShowError> {
        Ok(self
            .source_paths()?
            .into_iter()
            .chain(self.pending_deletion_paths()?)
            .collect())
    }

    /// The path of every deleted source whose folder is not removed yet: its
    /// `delete_*_files` task is queued, or the deletion event that schedules
    /// it is not handled yet. Unparsable paths are skipped.
    fn pending_deletion_paths(&self) -> Result<Vec<PlaylistPath>, MoveShowError> {
        let tasks = self
            .task_repository
            .list_non_completed()
            .map_err(MoveShowError::Repository)?;
        let events = self
            .event_repository
            .list_eligible()
            .map_err(MoveShowError::Repository)?;
        Ok(tasks
            .iter()
            .filter_map(ScheduledTask::deleted_path)
            .chain(events.iter().filter_map(ScheduledEvent::deleted_path))
            .filter_map(|path| PlaylistPath::new(path).ok())
            .collect())
    }

    /// The stored path of every playlist and channel.
    fn source_paths(&self) -> Result<Vec<PlaylistPath>, MoveShowError> {
        let playlists = self
            .playlist_repository
            .list()
            .map_err(MoveShowError::Repository)?;
        let channels = self
            .channel_repository
            .list()
            .map_err(MoveShowError::Repository)?;
        Ok(playlists
            .into_iter()
            .map(|playlist| playlist.path)
            .chain(channels.into_iter().map(|channel| channel.path))
            .collect())
    }

    fn show_dir(&self, path: &PlaylistPath) -> PathBuf {
        self.videos_root.join(path.as_str())
    }

    /// Renames the show directory, then `record`s the new path; a failure to
    /// record it renames the directory back.
    fn relocate(
        &self,
        from: &Path,
        to: &Path,
        record: impl FnOnce() -> Result<(), MoveShowError>,
    ) -> Result<(), MoveShowError> {
        let renamed = self.rename(from, to)?;
        record().inspect_err(|_| {
            if renamed {
                self.rename_back(to, from);
            }
        })
    }

    /// Whether there was a directory to rename. A source that never stored
    /// a file has no show directory: there is only the new path to record.
    fn rename(&self, from: &Path, to: &Path) -> Result<bool, MoveShowError> {
        if !self.video_file_repository.dir_exists(from) {
            return Ok(false);
        }
        self.video_file_repository
            .rename_dir(from, to)
            .map(|()| true)
            .map_err(MoveShowError::Filesystem)
    }

    fn rename_back(&self, from: &Path, to: &Path) {
        if let Err(e) = self.video_file_repository.rename_dir(from, to) {
            error!(from = ?from, to = ?to, error = %e, "failed to rename show directory back");
        }
    }

    fn update_playlist(&self, playlist: &Playlist) -> Result<(), MoveShowError> {
        self.playlist_repository
            .update(playlist)
            .map_err(MoveShowError::Repository)
    }

    fn update_channel(&self, channel: &Channel) -> Result<(), MoveShowError> {
        self.channel_repository
            .update(channel)
            .map_err(MoveShowError::Repository)
    }

    /// Best effort: the move is done once the new path is stored, so a lost
    /// event only costs the Plex scan it would trigger.
    fn publish_show_moved(&self, from: &Path, to: &Path) {
        let event = DomainEvent::ShowMoved(ShowMoved {
            from: from.to_string_lossy().into_owned(),
            to: to.to_string_lossy().into_owned(),
        });
        if let Err(e) = self.event_publisher.publish(&event) {
            warn!(from = ?from, to = ?to, error = %e, "failed to publish show moved");
        }
    }
}
