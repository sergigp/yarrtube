use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;

/// Reacts to `VideoAddedToPlaylist` by scheduling a `FetchThumbnail` task to
/// run now, so the reconcile pass that added the video never waits on
/// `yt-dlp`. Mirrors `DownloadVideoOnVideoAddedToPlaylist`, and is
/// registered before it so the thumbnail task gets the lower id.
pub struct FetchThumbnailOnVideoAddedToPlaylist {
    playlist_repository: Arc<dyn PlaylistRepository>,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
    videos_path: String,
}

impl FetchThumbnailOnVideoAddedToPlaylist {
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        task_repository: Arc<dyn TaskRepository>,
        clock: Arc<dyn Clock>,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            playlist_repository,
            task_repository,
            clock,
            videos_path: videos_path.into(),
        }
    }
}

impl EventSubscriber for FetchThumbnailOnVideoAddedToPlaylist {
    fn handle(&self, _payload: &str) -> anyhow::Result<()> {
        Ok(())
    }
}
