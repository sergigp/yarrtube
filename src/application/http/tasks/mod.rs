pub mod dto;

use super::AppState;
use super::error::error_response;
use crate::domain::channel::ChannelHandle;
use crate::domain::shared::PlaylistId;
use crate::domain::task::{ScheduledTask, TaskContainer};
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

    let referenced_containers: Vec<TaskContainer> = tasks
        .iter()
        .filter_map(|task| task.referenced_container())
        .collect();

    let mut playlist_names: HashMap<PlaylistId, String> = HashMap::new();
    for playlist_id in referenced_containers.iter().filter_map(|c| match c {
        TaskContainer::Playlist(id) => Some(id),
        TaskContainer::Channel(_) => None,
    }) {
        if playlist_names.contains_key(playlist_id) {
            continue;
        }
        match state.playlist_searcher.search(playlist_id) {
            Ok(Some(playlist)) => {
                playlist_names.insert(playlist_id.clone(), playlist.name.as_str().to_string());
            }
            Ok(None) => {}
            Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        }
    }

    let mut channel_names: HashMap<ChannelHandle, String> = HashMap::new();
    for channel_id in referenced_containers.iter().filter_map(|c| match c {
        TaskContainer::Channel(id) => Some(id),
        TaskContainer::Playlist(_) => None,
    }) {
        if channel_names.contains_key(channel_id) {
            continue;
        }
        match state.channel_service.find_channel(channel_id) {
            Ok(Some(channel)) => {
                channel_names.insert(channel_id.clone(), channel.name.clone());
            }
            Ok(None) => {}
            Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        }
    }

    let response: Vec<TaskResponse> = tasks
        .into_iter()
        .map(|task| build_task_response(task, &playlist_names, &channel_names))
        .collect();
    (StatusCode::OK, Json(response)).into_response()
}

fn build_task_response(
    task: ScheduledTask,
    playlist_names: &HashMap<PlaylistId, String>,
    channel_names: &HashMap<ChannelHandle, String>,
) -> TaskResponse {
    match task.referenced_container() {
        Some(TaskContainer::Playlist(playlist_id)) => {
            let playlist_name = playlist_names.get(&playlist_id).cloned();
            TaskResponse::from_task_with_context(task, playlist_name, None)
        }
        Some(TaskContainer::Channel(channel_id)) => {
            let channel_name = channel_names.get(&channel_id).cloned();
            TaskResponse::from_task_with_context(task, None, channel_name)
        }
        None => TaskResponse::from_task_with_context(task, None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::{Channel, VideoLimit};
    use crate::domain::playlist::{Playlist, PlaylistKind, PlaylistName, PlaylistPath};
    use crate::domain::shared::Quality;
    use crate::domain::task::{Task, TaskService};
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_channel_repository::{
        ChannelRepository, FakeChannelRepository,
    };
    use crate::infrastructure::repositories::sqlite_channel_video_repository::FakeChannelVideoRepository;
    use crate::infrastructure::repositories::sqlite_playlist_repository::{
        FakePlaylistRepository, PlaylistRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::FakePlaylistVideoRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::{
        SqliteTaskRepository, TaskRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_repository::FakeVideoRepository;
    use crate::infrastructure::repositories::youtube_channel_repository::FakeYoutubeChannelRepository;
    use crate::infrastructure::repositories::youtube_channel_videos_repository::FakeChannelVideosRepository;
    use crate::infrastructure::repositories::youtube_playlist_items_repository::FakeYoutubePlaylistItemsRepository;
    use crate::infrastructure::repositories::youtube_playlist_repository::FakeYoutubePlaylistRepository;

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
            Arc::new(FakeChannelRepository::default()),
        )
    }

    fn test_router_with(
        task_repository: Arc<dyn TaskRepository>,
        playlist_repository: Arc<FakePlaylistRepository>,
        channel_repository: Arc<FakeChannelRepository>,
    ) -> axum::Router {
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let video_repository = Arc::new(FakeVideoRepository::default());
        let playlist_video_repository = Arc::new(FakePlaylistVideoRepository::default());
        let channel_video_repository = Arc::new(FakeChannelVideoRepository::default());
        let playlist_creator = crate::domain::services::PlaylistCreator::new(
            playlist_repository.clone(),
            Arc::new(FakeYoutubePlaylistRepository { exists: true }),
            event_publisher.clone() as Arc<dyn EventPublisher>,
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let playlist_deleter = crate::domain::services::PlaylistDeleter::new(
            playlist_repository.clone(),
            video_repository.clone(),
            playlist_video_repository.clone(),
            event_publisher.clone() as Arc<dyn EventPublisher>,
        );
        let playlist_searcher =
            crate::domain::services::PlaylistSearcher::new(playlist_repository.clone());
        let channel_service = crate::domain::channel::ChannelService::new(
            channel_repository.clone(),
            Arc::new(FakeYoutubeChannelRepository { resolved: None }),
            Arc::new(FakeVideoRepository::default()),
            channel_video_repository.clone(),
            event_publisher.clone() as Arc<dyn EventPublisher>,
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let channel_video_reconciler = crate::domain::services::ChannelVideoReconciler::new(
            channel_repository,
            Arc::new(FakeVideoRepository::default()),
            channel_video_repository,
            Arc::new(FakeChannelVideosRepository::default()),
            event_publisher.clone() as Arc<dyn EventPublisher>,
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );
        let video_reconciler = crate::domain::services::VideoReconciler::new(
            playlist_repository.clone(),
            video_repository.clone(),
            playlist_video_repository.clone(),
            Arc::new(FakeYoutubePlaylistItemsRepository::default()),
            event_publisher.clone() as Arc<dyn EventPublisher>,
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );
        let custom_playlist_video_adder = crate::domain::services::CustomPlaylistVideoAdder::new(
            playlist_repository.clone(),
            video_repository.clone(),
            playlist_video_repository.clone(),
            Arc::new(FakeYoutubeVideoRepository::default()),
            event_publisher.clone() as Arc<dyn EventPublisher>,
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let custom_playlist_video_remover =
            crate::domain::services::CustomPlaylistVideoRemover::new(
                playlist_repository.clone(),
                video_repository.clone(),
                playlist_video_repository.clone(),
                event_publisher.clone() as Arc<dyn EventPublisher>,
            );
        let video_searcher = crate::domain::services::VideoSearcher::new(
            playlist_repository,
            playlist_video_repository,
            Arc::new(FakeChannelRepository::default()),
            Arc::new(FakeChannelVideoRepository::default()),
            video_repository,
        );
        let state = AppState {
            playlist_creator,
            playlist_deleter,
            playlist_searcher,
            video_reconciler,
            custom_playlist_video_adder,
            custom_playlist_video_remover,
            video_searcher,
            task_service: TaskService::new(task_repository),
            channel_service,
            channel_video_reconciler,
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

    fn channel(id: &str, name: &str) -> Channel {
        Channel::create(
            ChannelHandle::new(id).unwrap(),
            name,
            "UC123",
            Quality::High,
            VideoLimit::new(10).unwrap(),
            PlaylistPath::new("creators/somechannel").unwrap(),
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
            Arc::new(FakeChannelRepository::default()),
        );

        let response = router.oneshot(request()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let tasks = body.as_array().unwrap();
        assert_eq!(tasks[0]["playlist_name"], "My Playlist");
        assert_eq!(tasks[0]["channel_name"], serde_json::Value::Null);
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
        );

        let response = router.oneshot(request()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let tasks = body.as_array().unwrap();
        assert_eq!(tasks[0]["channel_name"], "Some Channel");
        assert_eq!(tasks[0]["playlist_name"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn it_should_show_no_container_name_for_a_pending_download_video_task() {
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
        let router = test_router(task_repository);

        let response = router.oneshot(request()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let tasks = body.as_array().unwrap();
        assert_eq!(tasks[0]["playlist_name"], serde_json::Value::Null);
        assert_eq!(tasks[0]["channel_name"], serde_json::Value::Null);
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
