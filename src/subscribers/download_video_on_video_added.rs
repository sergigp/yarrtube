use crate::domain::shared::{PlaylistId, VideoId};
use crate::domain::task::Task;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::shared::system_clock::Clock;
use serde::Deserialize;
use std::sync::Arc;
use tracing::debug;

#[derive(Debug, Deserialize)]
struct VideoAddedPayload {
    playlist_id: String,
    video_id: String,
}

/// Reacts to `VideoAdded` by scheduling a `DownloadVideo` task to run now,
/// rather than downloading inline — downloading is slow and external, so it
/// belongs on the task queue to get retry/backoff/dead-letter for free.
/// Looks up the owning playlist to resolve its `quality`, which travels with
/// the scheduled task rather than being re-read at execution time (see
/// design.md's "Quality resolved in the subscriber" decision).
pub struct DownloadVideoOnVideoAdded {
    playlist_repository: Arc<dyn PlaylistRepository>,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
}

impl DownloadVideoOnVideoAdded {
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        task_repository: Arc<dyn TaskRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            playlist_repository,
            task_repository,
            clock,
        }
    }
}

impl EventSubscriber for DownloadVideoOnVideoAdded {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: VideoAddedPayload = serde_json::from_str(payload)?;
        let (Ok(playlist_id), Ok(_video_id)) = (
            PlaylistId::new(payload.playlist_id.as_str()),
            VideoId::new(payload.video_id.as_str()),
        ) else {
            return Ok(());
        };

        let Some(playlist) = self.playlist_repository.find(&playlist_id)? else {
            debug!(playlist_id = %playlist_id, "playlist no longer exists, skipping download scheduling");
            return Ok(());
        };

        self.task_repository.schedule(
            &Task::DownloadVideo {
                playlist_id: payload.playlist_id,
                video_id: payload.video_id,
                quality: playlist.quality.as_str().to_string(),
            },
            self.clock.now(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::playlist::{Playlist, PlaylistName, Quality};
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
                quality,
                fixed_timestamp(),
            ))
            .unwrap();
        repository
    }

    fn subscriber(
        playlist_repository: Arc<dyn PlaylistRepository>,
        task_repository: Arc<dyn TaskRepository>,
    ) -> DownloadVideoOnVideoAdded {
        DownloadVideoOnVideoAdded::new(
            playlist_repository,
            task_repository,
            Arc::new(FixedClock(fixed_timestamp())),
        )
    }

    #[test]
    fn it_should_schedule_a_download_video_task_with_the_playlists_quality() {
        let playlist_repository = playlist_repository_with("PL1", Quality::Low);
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(playlist_repository, task_repository.clone());

        subscriber
            .handle(r#"{"playlist_id": "PL1", "video_id": "vid1"}"#)
            .unwrap();

        let scheduled = task_repository.scheduled.lock().unwrap();
        assert_eq!(
            *scheduled,
            vec![(
                Task::DownloadVideo {
                    playlist_id: "PL1".to_string(),
                    video_id: "vid1".to_string(),
                    quality: "low".to_string(),
                },
                fixed_timestamp()
            )]
        );
    }

    #[test]
    fn it_should_no_op_when_the_payload_ids_are_invalid() {
        let playlist_repository = playlist_repository_with("PL1", Quality::High);
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(playlist_repository, task_repository.clone());

        subscriber
            .handle(r#"{"playlist_id": "", "video_id": "vid1"}"#)
            .unwrap();

        assert!(task_repository.scheduled.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_no_op_when_the_playlist_no_longer_exists() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        let task_repository = Arc::new(FakeTaskRepository::default());
        let subscriber = subscriber(playlist_repository, task_repository.clone());

        subscriber
            .handle(r#"{"playlist_id": "PL404", "video_id": "vid1"}"#)
            .unwrap();

        assert!(task_repository.scheduled.lock().unwrap().is_empty());
    }
}
