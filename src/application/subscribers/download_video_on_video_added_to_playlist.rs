use crate::domain::playlist::PlaylistId;
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
    use crate::domain::task::{ScheduledTask, TaskStatus};
    use crate::infrastructure::repositories::sqlite_playlist_repository::SqlitePlaylistRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::SqliteTaskRepository;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use rusqlite::Connection;
    use std::sync::Mutex;

    #[test]
    fn it_should_schedule_the_download() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        playlist_repository
            .insert(&Playlist {
                quality: Quality::Low,
                ..playlist("PL1")
            })
            .unwrap();
        let subscriber = DownloadVideoOnVideoAddedToPlaylist::new(
            playlist_repository,
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
        );

        let result = handle(&subscriber, r#"{"playlist_id": "PL1", "video_id": "rec1"}"#);

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &Task::DownloadVideo {
                    video_id: "rec1".to_string(),
                    quality: "low".to_string(),
                    output_dir: "/videos/my-playlist".to_string(),
                },
            )]
        );
    }

    #[test]
    fn it_should_skip_if_invalid_playlist_id_provided() {
        let result = handle(
            &any_subscriber(),
            r#"{"playlist_id": "", "video_id": "rec1"}"#,
        );

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn it_should_skip_if_playlist_is_gone() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let subscriber = DownloadVideoOnVideoAddedToPlaylist::new(
            Arc::new(SqlitePlaylistRepository::new(db.connection())),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
        );

        let result = handle(
            &subscriber,
            r#"{"playlist_id": "PL404", "video_id": "rec1"}"#,
        );

        assert_eq!(result, Ok(()));
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    /// A subscriber for tests whose payload is rejected before any lookup.
    /// Its repositories sit on an unmigrated in-memory database, so a
    /// payload that wrongly got through would fail loudly instead of passing.
    fn any_subscriber() -> DownloadVideoOnVideoAddedToPlaylist {
        DownloadVideoOnVideoAddedToPlaylist::new(
            Arc::new(SqlitePlaylistRepository::new(unused_connection())),
            Arc::new(SqliteTaskRepository::new(
                Arc::new(Mutex::new(unused_connection())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
        )
    }

    fn unused_connection() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    fn playlist(id: &str) -> Playlist {
        Playlist::create(
            PlaylistId::new(id).unwrap(),
            PlaylistName::new("My Playlist").unwrap(),
            PlaylistPath::new("my-playlist").unwrap(),
            Quality::High,
            PlaylistKind::YoutubeLinked,
            fixed_timestamp(),
        )
    }

    fn pending_task(id: i64, task: &Task) -> ScheduledTask {
        ScheduledTask {
            id,
            task_type: task.task_type().to_string(),
            payload: task.payload().to_string(),
            status: TaskStatus::Pending,
            retries: 0,
            run_at: fixed_timestamp(),
            created_at: fixed_timestamp(),
            updated_at: fixed_timestamp(),
            last_error: None,
        }
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn handle(
        subscriber: &DownloadVideoOnVideoAddedToPlaylist,
        payload: &str,
    ) -> Result<(), String> {
        subscriber.handle(payload).map_err(|e| e.to_string())
    }
}
