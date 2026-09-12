pub mod dto;

use super::AppState;
use super::error::error_response;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use dto::TaskResponse;

pub async fn list_tasks(State(state): State<AppState>) -> Response {
    match state.task_service.list_non_completed() {
        Ok(tasks) => {
            let response: Vec<TaskResponse> = tasks.into_iter().map(TaskResponse::from).collect();
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task::{Task, TaskService};
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_playlist_repository::FakePlaylistRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::{
        SqliteTaskRepository, TaskRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_repository::FakeVideoRepository;
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
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let playlist_service = crate::domain::playlist::PlaylistService::new(
            playlist_repository.clone(),
            Arc::new(FakeYoutubePlaylistRepository { exists: true }),
            event_publisher.clone() as Arc<dyn EventPublisher>,
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let video_service = crate::domain::video::VideoService::new(
            playlist_repository,
            Arc::new(FakeVideoRepository::default()),
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
}
