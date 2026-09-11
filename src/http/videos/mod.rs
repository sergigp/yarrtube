pub mod dto;

use super::AppState;
use super::error::error_response;
use crate::domain::shared::PlaylistId;
use crate::domain::video::ListVideosError;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use dto::VideoResponse;

pub async fn list_videos_for_playlist(
    State(state): State<AppState>,
    Path(playlist_id): Path<String>,
) -> Response {
    let playlist_id = match PlaylistId::new(playlist_id) {
        Ok(id) => id,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, e.to_string()),
    };

    match state.video_service.list_videos(&playlist_id) {
        Ok(videos) => {
            let response: Vec<VideoResponse> =
                videos.into_iter().map(VideoResponse::from).collect();
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e @ ListVideosError::PlaylistNotFound(_)) => {
            error_response(StatusCode::BAD_REQUEST, e.to_string())
        }
        Err(e @ ListVideosError::Repository(_)) => {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::playlist::{
        Playlist, PlaylistKind, PlaylistName, PlaylistPath, PlaylistService,
    };
    use crate::domain::shared::{Quality, VideoId};
    use crate::domain::video::{Video, VideoService};
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_playlist_repository::{
        FakePlaylistRepository, PlaylistRepository,
    };
    use crate::infrastructure::repositories::sqlite_task_repository::FakeTaskRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::{
        FakeVideoRepository, VideoRepository,
    };
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
    use std::sync::Arc;
    use tower::ServiceExt;

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn test_router(
        playlist_repository: Arc<FakePlaylistRepository>,
        video_repository: Arc<FakeVideoRepository>,
    ) -> axum::Router {
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let playlist_service = PlaylistService::new(
            playlist_repository.clone(),
            Arc::new(FakeYoutubePlaylistRepository { exists: true }),
            event_publisher.clone() as Arc<dyn EventPublisher>,
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let video_service = VideoService::new(
            playlist_repository,
            video_repository,
            Arc::new(FakeYoutubePlaylistItemsRepository::default()),
            Arc::new(FakeYoutubeVideoRepository::default()),
            event_publisher as Arc<dyn EventPublisher>,
            Arc::new(FakeTaskRepository::default()),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );
        let state = AppState {
            playlist_service,
            video_service,
            task_service: crate::domain::task::TaskService::new(Arc::new(
                FakeTaskRepository::default(),
            )),
        };
        let inner = Router::new()
            .route("/playlists/{id}/videos", get(list_videos_for_playlist))
            .with_state(state);
        Router::new().nest("/api", inner)
    }

    async fn body_json(response: Response) -> serde_json::Value {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    fn playlist(id: &str) -> Playlist {
        Playlist::create(
            PlaylistId::new(id).unwrap(),
            PlaylistName::new("My Playlist").unwrap(),
            PlaylistPath::new("music").unwrap(),
            Quality::High,
            PlaylistKind::YoutubeLinked,
            fixed_timestamp(),
        )
    }

    fn request(playlist_id: &str) -> Request<Body> {
        Request::builder()
            .method("GET")
            .uri(format!("/api/playlists/{playlist_id}/videos"))
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn it_should_return_the_playlists_videos() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let video_repository = Arc::new(FakeVideoRepository::default());
        video_repository
            .save(&Video::create(
                PlaylistId::new("PL1").unwrap(),
                VideoId::new("vid1").unwrap(),
                "My Video",
                fixed_timestamp(),
            ))
            .unwrap();
        let router = test_router(playlist_repository, video_repository);

        let response = router.oneshot(request("PL1")).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let videos = body.as_array().unwrap();
        assert_eq!(videos.len(), 1);
        assert_eq!(videos[0]["id"], "vid1");
        assert_eq!(videos[0]["title"], "My Video");
        assert_eq!(videos[0]["status"], "PENDING");
    }

    #[tokio::test]
    async fn it_should_return_an_empty_list_when_the_playlist_has_no_videos() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let router = test_router(
            playlist_repository,
            Arc::new(FakeVideoRepository::default()),
        );

        let response = router.oneshot(request("PL1")).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body, serde_json::json!([]));
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_playlist_does_not_exist() {
        let router = test_router(
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakeVideoRepository::default()),
        );

        let response = router.oneshot(request("PL404")).await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
