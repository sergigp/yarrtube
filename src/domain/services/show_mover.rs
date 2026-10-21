use crate::domain::channel::{Channel, ChannelHandle};
use crate::domain::playlist::{Playlist, PlaylistId, PlaylistPath};
use crate::domain::show::MoveShowError;
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::show_locks::ShowLocks;
use std::path::PathBuf;
use std::sync::Arc;

/// Moves a tracked playlist's or channel's show directory to a new storage
/// path, keeping every video record, episode number and watch state.
#[derive(Clone)]
#[allow(dead_code)]
pub struct ShowMover {
    playlist_repository: Arc<dyn PlaylistRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    video_repository: Arc<dyn VideoRepository>,
    task_repository: Arc<dyn TaskRepository>,
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
    /// progress (a video not Downloaded, Errored or Excluded, or a per-video
    /// task queued or running in its show directory), when `path` is its
    /// current path, is used by, inside or contains any source's path, or
    /// already exists on disk. A failed rename changes nothing; a failure to
    /// store the new path renames the directory back.
    fn move_playlist(&self, id: PlaylistId, path: PlaylistPath) -> Result<Playlist, MoveShowError>;

    /// `move_playlist` for a channel.
    fn move_channel(&self, id: ChannelHandle, path: PlaylistPath)
    -> Result<Channel, MoveShowError>;
}

impl ShowMoverApi for ShowMover {
    fn move_playlist(
        &self,
        _id: PlaylistId,
        _path: PlaylistPath,
    ) -> Result<Playlist, MoveShowError> {
        Err(MoveShowError::NotFound)
    }

    fn move_channel(
        &self,
        _id: ChannelHandle,
        _path: PlaylistPath,
    ) -> Result<Channel, MoveShowError> {
        Err(MoveShowError::NotFound)
    }
}
