pub mod dto;

use super::blocking::run_blocking;
use super::error::ApiError;
use crate::domain::services::TaskViewSearcher;
use axum::Json;
use axum::extract::State;
use dto::TaskResponse;

pub async fn list_tasks(
    State(task_view_searcher): State<TaskViewSearcher>,
) -> Result<Json<Vec<TaskResponse>>, ApiError> {
    let views = run_blocking(move || task_view_searcher.search_all_pending())
        .await?
        .map_err(ApiError::internal)?;

    Ok(Json(views.into_iter().map(TaskResponse::from).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::{Channel, ChannelHandle, VideoLimit};
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::playlist::{Playlist, PlaylistKind, PlaylistName, PlaylistPath};
    use crate::domain::playlist_video::PlaylistVideo;
    use crate::domain::shared::{PlaylistId, Quality, VideoId, VideoRecordId};
    use crate::domain::task::Task;
    use crate::domain::video::{Video, VideoStatus};
    use crate::infrastructure::repositories::sqlite_channel_repository::{
        ChannelRepository, FakeChannelRepository,
    };
    use crate::infrastructure::repositories::sqlite_channel_video_repository::{
        ChannelVideoRepository, FakeChannelVideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_repository::{
        FakePlaylistRepository, PlaylistRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::{
        FakePlaylistVideoRepository, PlaylistVideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_task_repository::{
        SqliteTaskRepository, TaskRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_repository::{
        FakeVideoRepository, VideoRepository,
    };
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use rusqlite::Connection;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn it_should_return_non_completed_tasks() {
        let tasks = task_repository_with(&[Task::ReconcilePlaylist {
            playlist_id: "PL1".to_string(),
        }]);

        let response = list(task_view_searcher(tasks, Repositories::default())).await;

        assert_eq!(response, Ok(vec![pending_task("reconcile_playlist", &[])]));
    }

    #[tokio::test]
    async fn it_should_return_an_empty_list_when_no_tasks_exist() {
        let response = list(task_view_searcher(sqlite_repo(), Repositories::default())).await;

        assert_eq!(response, Ok(vec![]));
    }

    #[tokio::test]
    async fn it_should_include_a_pending_task_whose_run_at_is_in_the_future() {
        let future = fixed_timestamp() + chrono::Duration::seconds(60);
        let tasks = sqlite_repo();
        tasks
            .schedule(
                &Task::ReconcilePlaylist {
                    playlist_id: "PL1".to_string(),
                },
                future,
            )
            .unwrap();

        let response = list(task_view_searcher(tasks, Repositories::default())).await;

        assert_eq!(
            response,
            Ok(vec![TaskResponse {
                run_at: future,
                ..pending_task("reconcile_playlist", &[])
            }])
        );
    }

    #[tokio::test]
    async fn it_should_include_the_playlist_name_for_a_pending_reconcile_playlist_task() {
        let tasks = task_repository_with(&[Task::ReconcilePlaylist {
            playlist_id: "PL1".to_string(),
        }]);
        let playlists = FakePlaylistRepository::default();
        playlists.insert(&playlist("PL1", "My Playlist")).unwrap();

        let response = list(task_view_searcher(
            tasks,
            Repositories {
                playlists,
                ..Repositories::default()
            },
        ))
        .await;

        assert_eq!(
            response,
            Ok(vec![pending_task(
                "reconcile_playlist",
                &[("playlist_name", "My Playlist")]
            )])
        );
    }

    #[tokio::test]
    async fn it_should_include_the_channel_name_for_a_pending_reconcile_channel_task() {
        let tasks = task_repository_with(&[Task::ReconcileChannel {
            channel_id: "@somechannel".to_string(),
        }]);
        let channels = FakeChannelRepository::default();
        channels
            .insert(&channel("@somechannel", "Some Channel"))
            .unwrap();

        let response = list(task_view_searcher(
            tasks,
            Repositories {
                channels,
                ..Repositories::default()
            },
        ))
        .await;

        assert_eq!(
            response,
            Ok(vec![pending_task(
                "reconcile_channel",
                &[("channel_name", "Some Channel")]
            )])
        );
    }

    #[tokio::test]
    async fn it_should_include_the_video_title_and_playlist_name_for_a_pending_download_video_task()
    {
        let tasks = task_repository_with(&[Task::DownloadVideo {
            video_id: "rec1".to_string(),
            quality: "high".to_string(),
            output_dir: "/videos/my-playlist".to_string(),
        }]);
        let playlists = FakePlaylistRepository::default();
        playlists.insert(&playlist("PL1", "My Playlist")).unwrap();
        let videos = FakeVideoRepository::default();
        videos.save(&video("rec1", "My Video")).unwrap();
        let playlist_videos = FakePlaylistVideoRepository::default();
        playlist_videos
            .save(&PlaylistVideo::create(
                PlaylistId::new("PL1").unwrap(),
                VideoRecordId::new("rec1").unwrap(),
                fixed_timestamp(),
            ))
            .unwrap();

        let response = list(task_view_searcher(
            tasks,
            Repositories {
                playlists,
                videos,
                playlist_videos,
                ..Repositories::default()
            },
        ))
        .await;

        assert_eq!(
            response,
            Ok(vec![pending_task(
                "download_video",
                &[
                    ("video_title", "My Video"),
                    ("playlist_name", "My Playlist")
                ]
            )])
        );
    }

    #[tokio::test]
    async fn it_should_include_the_video_title_and_channel_name_for_a_channel_owned_download_video_task()
     {
        let tasks = task_repository_with(&[Task::DownloadVideo {
            video_id: "rec1".to_string(),
            quality: "high".to_string(),
            output_dir: "/videos/somechannel".to_string(),
        }]);
        let channels = FakeChannelRepository::default();
        channels
            .insert(&channel("@somechannel", "Some Channel"))
            .unwrap();
        let videos = FakeVideoRepository::default();
        videos.save(&video("rec1", "My Video")).unwrap();
        let channel_videos = FakeChannelVideoRepository::default();
        channel_videos
            .save(&ChannelVideo::create(
                ChannelHandle::new("@somechannel").unwrap(),
                VideoRecordId::new("rec1").unwrap(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();

        let response = list(task_view_searcher(
            tasks,
            Repositories {
                channels,
                videos,
                channel_videos,
                ..Repositories::default()
            },
        ))
        .await;

        assert_eq!(
            response,
            Ok(vec![pending_task(
                "download_video",
                &[
                    ("video_title", "My Video"),
                    ("channel_name", "Some Channel")
                ]
            )])
        );
    }

    #[tokio::test]
    async fn it_should_include_the_filename_for_a_pending_delete_video_file_task() {
        let tasks = task_repository_with(&[Task::DeleteVideoFile {
            filename: Some("My Video.mp4".to_string()),
            thumbnail_filename: Some("My Video.jpg".to_string()),
            output_dir: "/videos/music".to_string(),
        }]);

        let response = list(task_view_searcher(tasks, Repositories::default())).await;

        assert_eq!(
            response,
            Ok(vec![pending_task(
                "delete_video_file",
                &[("filename", "My Video.mp4")]
            )])
        );
    }

    #[tokio::test]
    async fn it_should_include_the_path_for_a_pending_delete_playlist_files_task() {
        let tasks = task_repository_with(&[Task::DeletePlaylistFiles {
            playlist_id: "PL1".to_string(),
            path: "music/chill".to_string(),
        }]);

        let response = list(task_view_searcher(tasks, Repositories::default())).await;

        assert_eq!(
            response,
            Ok(vec![pending_task(
                "delete_playlist_files",
                &[("path", "music/chill")]
            )])
        );
    }

    #[tokio::test]
    async fn it_should_include_the_path_for_a_pending_delete_channel_files_task() {
        let tasks = task_repository_with(&[Task::DeleteChannelFiles {
            channel_id: "@somechannel".to_string(),
            path: "creators/somechannel".to_string(),
        }]);

        let response = list(task_view_searcher(tasks, Repositories::default())).await;

        assert_eq!(
            response,
            Ok(vec![pending_task(
                "delete_channel_files",
                &[("path", "creators/somechannel")]
            )])
        );
    }

    #[tokio::test]
    async fn it_should_return_ok_and_an_empty_payload_when_the_referenced_playlist_no_longer_exists()
     {
        let tasks = task_repository_with(&[Task::ReconcilePlaylist {
            playlist_id: "PL1".to_string(),
        }]);

        let response = list(task_view_searcher(tasks, Repositories::default())).await;

        assert_eq!(response, Ok(vec![pending_task("reconcile_playlist", &[])]));
    }

    #[derive(Default)]
    struct Repositories {
        playlists: FakePlaylistRepository,
        channels: FakeChannelRepository,
        videos: FakeVideoRepository,
        playlist_videos: FakePlaylistVideoRepository,
        channel_videos: FakeChannelVideoRepository,
    }

    fn task_view_searcher(
        task_repository: Arc<dyn TaskRepository>,
        repositories: Repositories,
    ) -> TaskViewSearcher {
        TaskViewSearcher::new(
            task_repository,
            Arc::new(repositories.playlists),
            Arc::new(repositories.channels),
            Arc::new(repositories.videos),
            Arc::new(repositories.playlist_videos),
            Arc::new(repositories.channel_videos),
        )
    }

    async fn list(searcher: TaskViewSearcher) -> Result<Vec<TaskResponse>, ApiError> {
        list_tasks(State(searcher)).await.map(|Json(tasks)| tasks)
    }

    fn pending_task(task_type: &str, payload: &[(&str, &str)]) -> TaskResponse {
        TaskResponse {
            id: 1,
            task_type: task_type.to_string(),
            status: "pending".to_string(),
            retries: 0,
            run_at: fixed_timestamp(),
            created_at: fixed_timestamp(),
            last_error: None,
            payload: payload
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect::<HashMap<_, _>>(),
        }
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn sqlite_repo() -> Arc<dyn TaskRepository> {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::shared::sqlite_migrations::apply(&mut conn).unwrap();
        Arc::new(SqliteTaskRepository::new(
            Arc::new(Mutex::new(conn)),
            Arc::new(FixedClock(fixed_timestamp())),
        ))
    }

    fn task_repository_with(tasks: &[Task]) -> Arc<dyn TaskRepository> {
        let repository = sqlite_repo();
        for task in tasks {
            repository.schedule(task, fixed_timestamp()).unwrap();
        }
        repository
    }

    fn playlist(id: &str, name: &str) -> Playlist {
        Playlist::create(
            PlaylistId::new(id).unwrap(),
            PlaylistName::new(name).unwrap(),
            PlaylistPath::new("my-playlist").unwrap(),
            Quality::High,
            PlaylistKind::YoutubeLinked,
            fixed_timestamp(),
        )
    }

    fn channel(id: &str, name: &str) -> Channel {
        Channel::create(
            ChannelHandle::new(id).unwrap(),
            name,
            "UC123",
            Quality::High,
            VideoLimit::new(10).unwrap(),
            PlaylistPath::new("creators/somechannel").unwrap(),
            None,
            fixed_timestamp(),
        )
    }

    fn video(id: &str, title: &str) -> Video {
        Video {
            id: VideoRecordId::new(id).unwrap(),
            youtube_id: VideoId::new("yt1").unwrap(),
            title: title.to_string(),
            status: VideoStatus::Pending,
            quality: None,
            filename: None,
            thumbnail_filename: None,
            duration_seconds: None,
            created_at: fixed_timestamp(),
            updated_at: fixed_timestamp(),
        }
    }
}
