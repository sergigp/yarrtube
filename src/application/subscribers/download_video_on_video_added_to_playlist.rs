use crate::domain::shared::PlaylistId;
use crate::domain::task::Task;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::shared::system_clock::Clock;
use serde::Deserialize;
use std::path::Path;
use std::sync::Arc;
use tracing::debug;

#[derive(Debug, Deserialize)]
struct VideoAddedToPlaylistPayload {
    playlist_id: String,
    video_id: String,
}

/// Reacts to `VideoAddedToPlaylist` by scheduling a `DownloadVideo` task to
/// run now, rather than downloading inline — downloading is slow and
/// external, so it belongs on the task queue to get retry/backoff/
/// dead-letter for free. Looks up the owning playlist to resolve its
/// `quality` and output directory, which travel with the scheduled task
/// rather than being re-read at execution time (see design.md's "Tasks stay
/// single and container-agnostic" decision).
pub struct DownloadVideoOnVideoAddedToPlaylist {
    playlist_repository: Arc<dyn PlaylistRepository>,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
    videos_path: String,
}

impl DownloadVideoOnVideoAddedToPlaylist {
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

impl EventSubscriber for DownloadVideoOnVideoAddedToPlaylist {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: VideoAddedToPlaylistPayload = serde_json::from_str(payload)?;
        let Ok(playlist_id) = PlaylistId::new(payload.playlist_id.as_str()) else {
            return Ok(());
        };

        let Some(playlist) = self.playlist_repository.find(&playlist_id)? else {
            debug!(playlist_id = %playlist_id, "playlist no longer exists, skipping download scheduling");
            return Ok(());
        };

        let output_dir = Path::new(&self.videos_path).join(playlist.path.as_str());
        self.task_repository.schedule(
            &Task::DownloadVideo {
                video_id: payload.video_id,
                quality: playlist.quality.as_str().to_string(),
                output_dir: output_dir.to_string_lossy().to_string(),
            },
            self.clock.now(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::playlist::{Playlist, PlaylistKind, PlaylistName, PlaylistPath};
    use crate::domain::shared::Quality;
    use crate::domain::task::Task;
    use crate::infrastructure::repositories::sqlite_playlist_repository::FakePlaylistRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::FakeTaskRepository;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn playlist_repository_with(id: &str, quality: Quality) -> Arc<FakePlaylistRepository> {
        let repository = Arc::new(FakePlaylistRepository::default());
        repository
            .insert(&Playlist::create(
                PlaylistId::new(id).unwrap(),
                PlaylistName::new("My Playlist").unwrap(),
                PlaylistPath::new("my-playlist").unwrap(),
                quality,
                PlaylistKind::YoutubeLinked,
                fixed_timestamp(),
            ))
            .unwrap();
        repository
    }

    fn subscriber(
        playlist_repository: Arc<dyn PlaylistRepository>,
        task_repository: Arc<dyn TaskRepository>,
    ) -> DownloadVideoOnVideoAddedToPlaylist {
        DownloadVideoOnVideoAddedToPlaylist::new(
            playlist_repository,
            task_repository,
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
        )
    }

    #[test]
    fn it_should_schedule_a_download_video_task_with_the_playlists_quality_and_output_dir() {
        let playlist_repository = playlist_repository_with("PL1", Quality::Low);
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(playlist_repository, task_repository.clone());

        subscriber
            .handle(r#"{"playlist_id": "PL1", "video_id": "rec1"}"#)
            .unwrap();

        let scheduled = task_repository.scheduled.lock().unwrap();
        assert_eq!(
            *scheduled,
            vec![(
                Task::DownloadVideo {
                    video_id: "rec1".to_string(),
                    quality: "low".to_string(),
                    output_dir: "/videos/my-playlist".to_string(),
                },
                fixed_timestamp()
            )]
        );
    }

    #[test]
    fn it_should_no_op_when_the_payload_playlist_id_is_invalid() {
        let playlist_repository = playlist_repository_with("PL1", Quality::High);
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(playlist_repository, task_repository.clone());

        subscriber
            .handle(r#"{"playlist_id": "", "video_id": "rec1"}"#)
            .unwrap();

        assert!(task_repository.scheduled.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_no_op_when_the_playlist_no_longer_exists() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(playlist_repository, task_repository.clone());

        subscriber
            .handle(r#"{"playlist_id": "PL404", "video_id": "rec1"}"#)
            .unwrap();

        assert!(task_repository.scheduled.lock().unwrap().is_empty());
    }
}
