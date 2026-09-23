pub mod dto;

use super::blocking::run_blocking;
use super::error::ApiError;
use crate::domain::services::TaskViewSearcher;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use dto::TaskResponse;

pub async fn list_tasks(
    State(task_view_searcher): State<TaskViewSearcher>,
) -> Result<Response, ApiError> {
    let views = run_blocking(move || task_view_searcher.search_all_pending())
        .await?
        .map_err(ApiError::internal)?;

    let response: Vec<TaskResponse> = views.into_iter().map(TaskResponse::from).collect();
    Ok((StatusCode::OK, Json(response)).into_response())
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
    use crate::domain::video::Video;
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
    use axum::Router;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use axum::routing::get;
    use chrono::{DateTime, Utc};
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};
    use tower::ServiceExt;

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn test_router(task_repository: Arc<dyn TaskRepository>) -> axum::Router {
        test_router_with(
            task_repository,
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakeChannelRepository::default()),
            Arc::new(FakeVideoRepository::default()),
            Arc::new(FakePlaylistVideoRepository::default()),
            Arc::new(FakeChannelVideoRepository::default()),
        )
    }

    fn test_router_with(
        task_repository: Arc<dyn TaskRepository>,
        playlist_repository: Arc<FakePlaylistRepository>,
        channel_repository: Arc<FakeChannelRepository>,
        video_repository: Arc<FakeVideoRepository>,
        playlist_video_repository: Arc<FakePlaylistVideoRepository>,
        channel_video_repository: Arc<FakeChannelVideoRepository>,
    ) -> axum::Router {
        let task_view_searcher = TaskViewSearcher::new(
            task_repository,
            playlist_repository,
            channel_repository,
            video_repository,
            playlist_video_repository,
            channel_video_repository,
        );
        Router::new()
            .route("/api/tasks", get(list_tasks))
            .with_state(task_view_searcher)
    }

    async fn body_json(response: Response) -> serde_json::Value {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    fn sqlite_repo() -> Arc<dyn TaskRepository> {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::shared::sqlite_migrations::apply(&mut conn).unwrap();
        Arc::new(SqliteTaskRepository::new(
            Arc::new(Mutex::new(conn)),
            Arc::new(FixedClock(fixed_timestamp())),
        ))
    }

    fn request() -> Request<Body> {
        Request::builder()
            .method("GET")
            .uri("/api/tasks")
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn it_should_return_non_completed_tasks() {
        let task_repository = sqlite_repo();
        task_repository
            .schedule(
                &Task::ReconcilePlaylist {
                    playlist_id: "PL1".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();
        let router = test_router(task_repository);

        let response = router.oneshot(request()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let tasks = body.as_array().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0]["task_type"], "reconcile_playlist");
        assert_eq!(tasks[0]["status"], "pending");
    }

    #[tokio::test]
    async fn it_should_return_an_empty_list_when_no_tasks_exist() {
        let router = test_router(sqlite_repo());

        let response = router.oneshot(request()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body, serde_json::json!([]));
    }

    #[tokio::test]
    async fn it_should_include_a_pending_task_whose_run_at_is_in_the_future() {
        let task_repository = sqlite_repo();
        let future = fixed_timestamp() + chrono::Duration::seconds(60);
        task_repository
            .schedule(
                &Task::ReconcilePlaylist {
                    playlist_id: "PL1".to_string(),
                },
                future,
            )
            .unwrap();
        let router = test_router(task_repository);

        let response = router.oneshot(request()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let tasks = body.as_array().unwrap();
        assert_eq!(tasks.len(), 1);
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
            status: crate::domain::video::VideoStatus::Pending,
            quality: None,
            filename: None,
            thumbnail_filename: None,
            duration_seconds: None,
            created_at: fixed_timestamp(),
            updated_at: fixed_timestamp(),
        }
    }

    #[tokio::test]
    async fn it_should_include_the_playlist_name_for_a_pending_reconcile_playlist_task() {
        let task_repository = sqlite_repo();
        task_repository
            .schedule(
                &Task::ReconcilePlaylist {
                    playlist_id: "PL1".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository
            .insert(&playlist("PL1", "My Playlist"))
            .unwrap();
        let router = test_router_with(
            task_repository,
            playlist_repository,
            Arc::new(FakeChannelRepository::default()),
            Arc::new(FakeVideoRepository::default()),
            Arc::new(FakePlaylistVideoRepository::default()),
            Arc::new(FakeChannelVideoRepository::default()),
        );

        let response = router.oneshot(request()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let tasks = body.as_array().unwrap();
        assert_eq!(tasks[0]["payload"]["playlist_name"], "My Playlist");
    }

    #[tokio::test]
    async fn it_should_include_the_channel_name_for_a_pending_reconcile_channel_task() {
        let task_repository = sqlite_repo();
        task_repository
            .schedule(
                &Task::ReconcileChannel {
                    channel_id: "@somechannel".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();
        let channel_repository = Arc::new(FakeChannelRepository::default());
        channel_repository
            .insert(&channel("@somechannel", "Some Channel"))
            .unwrap();
        let router = test_router_with(
            task_repository,
            Arc::new(FakePlaylistRepository::default()),
            channel_repository,
            Arc::new(FakeVideoRepository::default()),
            Arc::new(FakePlaylistVideoRepository::default()),
            Arc::new(FakeChannelVideoRepository::default()),
        );

        let response = router.oneshot(request()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let tasks = body.as_array().unwrap();
        assert_eq!(tasks[0]["payload"]["channel_name"], "Some Channel");
    }

    #[tokio::test]
    async fn it_should_include_the_video_title_and_playlist_name_for_a_pending_download_video_task()
    {
        let task_repository = sqlite_repo();
        task_repository
            .schedule(
                &Task::DownloadVideo {
                    video_id: "rec1".to_string(),
                    quality: "high".to_string(),
                    output_dir: "/videos/my-playlist".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository
            .insert(&playlist("PL1", "My Playlist"))
            .unwrap();
        let video_repository = Arc::new(FakeVideoRepository::default());
        video_repository.save(&video("rec1", "My Video")).unwrap();
        let playlist_video_repository = Arc::new(FakePlaylistVideoRepository::default());
        playlist_video_repository
            .save(&PlaylistVideo::create(
                PlaylistId::new("PL1").unwrap(),
                VideoRecordId::new("rec1").unwrap(),
                fixed_timestamp(),
            ))
            .unwrap();
        let router = test_router_with(
            task_repository,
            playlist_repository,
            Arc::new(FakeChannelRepository::default()),
            video_repository,
            playlist_video_repository,
            Arc::new(FakeChannelVideoRepository::default()),
        );

        let response = router.oneshot(request()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let tasks = body.as_array().unwrap();
        assert_eq!(tasks[0]["payload"]["video_title"], "My Video");
        assert_eq!(tasks[0]["payload"]["playlist_name"], "My Playlist");
    }

    #[tokio::test]
    async fn it_should_include_the_video_title_and_channel_name_for_a_channel_owned_download_video_task()
     {
        let task_repository = sqlite_repo();
        task_repository
            .schedule(
                &Task::DownloadVideo {
                    video_id: "rec1".to_string(),
                    quality: "high".to_string(),
                    output_dir: "/videos/somechannel".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();
        let channel_repository = Arc::new(FakeChannelRepository::default());
        channel_repository
            .insert(&channel("@somechannel", "Some Channel"))
            .unwrap();
        let video_repository = Arc::new(FakeVideoRepository::default());
        video_repository.save(&video("rec1", "My Video")).unwrap();
        let channel_video_repository = Arc::new(FakeChannelVideoRepository::default());
        channel_video_repository
            .save(&ChannelVideo::create(
                ChannelHandle::new("@somechannel").unwrap(),
                VideoRecordId::new("rec1").unwrap(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let router = test_router_with(
            task_repository,
            Arc::new(FakePlaylistRepository::default()),
            channel_repository,
            video_repository,
            Arc::new(FakePlaylistVideoRepository::default()),
            channel_video_repository,
        );

        let response = router.oneshot(request()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let tasks = body.as_array().unwrap();
        assert_eq!(tasks[0]["payload"]["video_title"], "My Video");
        assert_eq!(tasks[0]["payload"]["channel_name"], "Some Channel");
    }

    #[tokio::test]
    async fn it_should_include_the_filename_for_a_pending_delete_video_file_task() {
        let task_repository = sqlite_repo();
        task_repository
            .schedule(
                &Task::DeleteVideoFile {
                    filename: Some("My Video.mp4".to_string()),
                    thumbnail_filename: Some("My Video.jpg".to_string()),
                    output_dir: "/videos/music".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();
        let router = test_router(task_repository);

        let response = router.oneshot(request()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let tasks = body.as_array().unwrap();
        assert_eq!(tasks[0]["payload"]["filename"], "My Video.mp4");
    }

    #[tokio::test]
    async fn it_should_include_the_path_for_a_pending_delete_playlist_files_task() {
        let task_repository = sqlite_repo();
        task_repository
            .schedule(
                &Task::DeletePlaylistFiles {
                    playlist_id: "PL1".to_string(),
                    path: "music/chill".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();
        let router = test_router(task_repository);

        let response = router.oneshot(request()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let tasks = body.as_array().unwrap();
        assert_eq!(tasks[0]["payload"]["path"], "music/chill");
    }

    #[tokio::test]
    async fn it_should_include_the_path_for_a_pending_delete_channel_files_task() {
        let task_repository = sqlite_repo();
        task_repository
            .schedule(
                &Task::DeleteChannelFiles {
                    channel_id: "@somechannel".to_string(),
                    path: "creators/somechannel".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();
        let router = test_router(task_repository);

        let response = router.oneshot(request()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let tasks = body.as_array().unwrap();
        assert_eq!(tasks[0]["payload"]["path"], "creators/somechannel");
    }

    #[tokio::test]
    async fn it_should_return_ok_and_an_empty_payload_when_the_referenced_playlist_no_longer_exists()
     {
        let task_repository = sqlite_repo();
        task_repository
            .schedule(
                &Task::ReconcilePlaylist {
                    playlist_id: "PL1".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();
        let router = test_router(task_repository);

        let response = router.oneshot(request()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let tasks = body.as_array().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0]["payload"], serde_json::json!({}));
    }
}
