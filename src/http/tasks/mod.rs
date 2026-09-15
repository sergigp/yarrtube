pub mod dto;

use super::AppState;
use super::error::error_response;
use crate::domain::shared::{PlaylistId, VideoId};
use crate::domain::task::ScheduledTask;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use dto::TaskResponse;
use std::collections::HashMap;

pub async fn list_tasks(State(state): State<AppState>) -> Response {
    let tasks = match state.task_service.list_non_completed() {
        Ok(tasks) => tasks,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let referenced_ids: Vec<(PlaylistId, Option<VideoId>)> = tasks
        .iter()
        .filter_map(|task| task.referenced_ids().ok())
        .collect();

    let mut playlist_names: HashMap<PlaylistId, String> = HashMap::new();
    for playlist_id in referenced_ids.iter().map(|(playlist_id, _)| playlist_id) {
        if playlist_names.contains_key(playlist_id) {
            continue;
        }
        match state.playlist_service.find_playlist(playlist_id) {
            Ok(Some(playlist)) => {
                playlist_names.insert(playlist_id.clone(), playlist.name.as_str().to_string());
            }
            Ok(None) => {}
            Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        }
    }

    let mut video_titles: HashMap<(PlaylistId, VideoId), String> = HashMap::new();
    for (playlist_id, video_id) in referenced_ids
        .iter()
        .filter_map(|(playlist_id, video_id)| video_id.as_ref().map(|v| (playlist_id, v)))
    {
        let key = (playlist_id.clone(), video_id.clone());
        if video_titles.contains_key(&key) {
            continue;
        }
        match state.video_service.find(playlist_id, video_id) {
            Ok(Some(video)) => {
                video_titles.insert(key, video.title);
            }
            Ok(None) => {}
            Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        }
    }

    let response: Vec<TaskResponse> = tasks
        .into_iter()
        .map(|task| build_task_response(task, &playlist_names, &video_titles))
        .collect();
    (StatusCode::OK, Json(response)).into_response()
}

fn build_task_response(
    task: ScheduledTask,
    playlist_names: &HashMap<PlaylistId, String>,
    video_titles: &HashMap<(PlaylistId, VideoId), String>,
) -> TaskResponse {
    let Ok((playlist_id, video_id)) = task.referenced_ids() else {
        return TaskResponse::from_task_with_context(task, None, None, None);
    };

    let playlist_name = playlist_names.get(&playlist_id).cloned();
    let (video_id, video_title) = match video_id {
        Some(video_id) => {
            let title = video_titles
                .get(&(playlist_id.clone(), video_id.clone()))
                .cloned();
            (Some(video_id.as_str().to_string()), title)
        }
        None => (None, None),
    };

    TaskResponse::from_task_with_context(task, playlist_name, video_id, video_title)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::playlist::{Playlist, PlaylistKind, PlaylistName, PlaylistPath};
    use crate::domain::shared::Quality;
    use crate::domain::task::{Task, TaskService};
    use crate::domain::video::Video;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_channel_repository::FakeChannelRepository;
    use crate::infrastructure::repositories::sqlite_playlist_repository::{
        FakePlaylistRepository, PlaylistRepository,
    };
    use crate::infrastructure::repositories::sqlite_task_repository::{
        SqliteTaskRepository, TaskRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_repository::{
        FakeVideoRepository, VideoRepository,
    };
    use crate::infrastructure::repositories::youtube_channel_repository::FakeYoutubeChannelRepository;
    use crate::infrastructure::repositories::youtube_playlist_items_repository::FakeYoutubePlaylistItemsRepository;
    use crate::infrastructure::repositories::youtube_playlist_repository::FakeYoutubePlaylistRepository;
    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
    use crate::infrastructure::repositories::youtube_video_repository::FakeYoutubeVideoRepository;
    use crate::infrastructure::shared::domain_events::event_publisher::{
        EventPublisher, FakeEventPublisher,
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
            Arc::new(FakeVideoRepository::default()),
        )
    }

    fn test_router_with(
        task_repository: Arc<dyn TaskRepository>,
        playlist_repository: Arc<FakePlaylistRepository>,
        video_repository: Arc<FakeVideoRepository>,
    ) -> axum::Router {
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let playlist_service = crate::domain::playlist::PlaylistService::new(
            playlist_repository.clone(),
            video_repository.clone(),
            Arc::new(FakeYoutubePlaylistRepository { exists: true }),
            event_publisher.clone() as Arc<dyn EventPublisher>,
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let channel_service = crate::domain::channel::ChannelService::new(
            Arc::new(FakeChannelRepository::default()),
            Arc::new(FakeYoutubeChannelRepository { resolved: None }),
            event_publisher.clone() as Arc<dyn EventPublisher>,
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let video_service = crate::domain::video::VideoService::new(
            playlist_repository,
            video_repository,
            Arc::new(FakeYoutubePlaylistItemsRepository::default()),
            Arc::new(FakeYoutubeVideoRepository::default()),
            event_publisher as Arc<dyn EventPublisher>,
            task_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );
        let state = AppState {
            playlist_service,
            video_service,
            task_service: TaskService::new(task_repository),
            channel_service,
        };
        let inner = Router::new()
            .route("/tasks", get(list_tasks))
            .with_state(state);
        Router::new().nest("/api", inner)
    }

    async fn body_json(response: Response) -> serde_json::Value {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    fn sqlite_repo() -> Arc<dyn TaskRepository> {
        Arc::new(
            SqliteTaskRepository::new(
                Arc::new(Mutex::new(Connection::open_in_memory().unwrap())),
                Arc::new(FixedClock(fixed_timestamp())),
            )
            .unwrap(),
        )
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
            Arc::new(FakeVideoRepository::default()),
        );

        let response = router.oneshot(request()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let tasks = body.as_array().unwrap();
        assert_eq!(tasks[0]["playlist_name"], "My Playlist");
        assert_eq!(tasks[0]["video_id"], serde_json::Value::Null);
        assert_eq!(tasks[0]["video_title"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn it_should_include_the_playlist_name_and_video_details_for_a_pending_download_video_task()
     {
        let task_repository = sqlite_repo();
        task_repository
            .schedule(
                &Task::DownloadVideo {
                    playlist_id: "PL1".to_string(),
                    video_id: "vid1".to_string(),
                    quality: "high".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository
            .insert(&playlist("PL1", "My Playlist"))
            .unwrap();
        let video_repository = Arc::new(FakeVideoRepository::default());
        video_repository
            .save(&Video::create(
                PlaylistId::new("PL1").unwrap(),
                VideoId::new("vid1").unwrap(),
                "My Video",
                fixed_timestamp(),
            ))
            .unwrap();
        let router = test_router_with(task_repository, playlist_repository, video_repository);

        let response = router.oneshot(request()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let tasks = body.as_array().unwrap();
        assert_eq!(tasks[0]["playlist_name"], "My Playlist");
        assert_eq!(tasks[0]["video_id"], "vid1");
        assert_eq!(tasks[0]["video_title"], "My Video");
    }

    #[tokio::test]
    async fn it_should_return_ok_and_omit_the_name_when_the_referenced_playlist_no_longer_exists() {
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
        assert_eq!(tasks[0]["playlist_name"], serde_json::Value::Null);
    }
}
