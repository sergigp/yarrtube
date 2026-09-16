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
struct VideoRemovedFromPlaylistPayload {
    playlist_id: String,
    filename: Option<String>,
    thumbnail_filename: Option<String>,
    was_downloaded: bool,
}

/// Reacts to `VideoRemovedFromPlaylist` by scheduling a `DeleteVideoFile`
/// task to run now, only when `was_downloaded` — a video that never
/// finished downloading has no file to clean up. Resolves the playlist's
/// output directory before scheduling (see design.md's "Tasks stay single
/// and container-agnostic" decision).
pub struct DeleteVideoFileOnVideoRemovedFromPlaylist {
    playlist_repository: Arc<dyn PlaylistRepository>,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
    videos_path: String,
}

impl DeleteVideoFileOnVideoRemovedFromPlaylist {
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

impl EventSubscriber for DeleteVideoFileOnVideoRemovedFromPlaylist {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: VideoRemovedFromPlaylistPayload = serde_json::from_str(payload)?;
        let Ok(playlist_id) = PlaylistId::new(payload.playlist_id.as_str()) else {
            return Ok(());
        };

        if !payload.was_downloaded {
            return Ok(());
        }

        let Some(playlist) = self.playlist_repository.find(&playlist_id)? else {
            debug!(playlist_id = %playlist_id, "playlist no longer exists, skipping file deletion scheduling");
            return Ok(());
        };

        let output_dir = Path::new(&self.videos_path).join(playlist.path.as_str());
        self.task_repository.schedule(
            &Task::DeleteVideoFile {
                filename: payload.filename,
                thumbnail_filename: payload.thumbnail_filename,
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
    use crate::infrastructure::repositories::sqlite_playlist_repository::FakePlaylistRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::FakeTaskRepository;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn playlist_repository_with(id: &str) -> Arc<FakePlaylistRepository> {
        let repository = Arc::new(FakePlaylistRepository::default());
        repository
            .insert(&Playlist::create(
                PlaylistId::new(id).unwrap(),
                PlaylistName::new("My Playlist").unwrap(),
                PlaylistPath::new("my-playlist").unwrap(),
                Quality::High,
                PlaylistKind::YoutubeLinked,
                fixed_timestamp(),
            ))
            .unwrap();
        repository
    }

    fn subscriber(
        playlist_repository: Arc<dyn PlaylistRepository>,
        task_repository: Arc<dyn TaskRepository>,
    ) -> DeleteVideoFileOnVideoRemovedFromPlaylist {
        DeleteVideoFileOnVideoRemovedFromPlaylist::new(
            playlist_repository,
            task_repository,
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
        )
    }

    #[test]
    fn it_should_schedule_a_delete_video_file_task_when_the_video_was_downloaded() {
        let playlist_repository = playlist_repository_with("PL1");
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(playlist_repository, task_repository.clone());

        subscriber
            .handle(
                r#"{"playlist_id": "PL1", "video_id": "rec1", "title": "My Video", "filename": "My Video.mp4", "thumbnail_filename": "My Video.jpg", "was_downloaded": true}"#,
            )
            .unwrap();

        let scheduled = task_repository.scheduled.lock().unwrap();
        assert_eq!(
            *scheduled,
            vec![(
                Task::DeleteVideoFile {
                    filename: Some("My Video.mp4".to_string()),
                    thumbnail_filename: Some("My Video.jpg".to_string()),
                    output_dir: "/videos/my-playlist".to_string(),
                },
                fixed_timestamp()
            )]
        );
    }

    #[test]
    fn it_should_not_schedule_a_task_when_the_video_was_not_downloaded() {
        let playlist_repository = playlist_repository_with("PL1");
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(playlist_repository, task_repository.clone());

        subscriber
            .handle(
                r#"{"playlist_id": "PL1", "video_id": "rec1", "title": "My Video", "filename": null, "thumbnail_filename": null, "was_downloaded": false}"#,
            )
            .unwrap();

        assert!(task_repository.scheduled.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_no_op_when_the_payload_playlist_id_is_invalid() {
        let playlist_repository = playlist_repository_with("PL1");
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(playlist_repository, task_repository.clone());

        subscriber
            .handle(
                r#"{"playlist_id": "", "video_id": "rec1", "title": "My Video", "filename": null, "thumbnail_filename": null, "was_downloaded": true}"#,
            )
            .unwrap();

        assert!(task_repository.scheduled.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_no_op_when_the_playlist_no_longer_exists() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(playlist_repository, task_repository.clone());

        subscriber
            .handle(
                r#"{"playlist_id": "PL404", "video_id": "rec1", "title": "My Video", "filename": "My Video.mp4", "thumbnail_filename": null, "was_downloaded": true}"#,
            )
            .unwrap();

        assert!(task_repository.scheduled.lock().unwrap().is_empty());
    }
}
