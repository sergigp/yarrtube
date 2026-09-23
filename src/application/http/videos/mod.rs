pub mod dto;

use super::blocking::run_blocking;
use super::error::ApiError;
use crate::domain::channel::ChannelHandle;
use crate::domain::services::VideoSearcher;
use crate::domain::shared::PlaylistId;
use crate::domain::video::ListVideosError;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use dto::{RecentVideoResponse, VideoResponse};
use serde::Deserialize;

const DEFAULT_RECENT_VIDEOS_LIMIT: usize = 20;
const MAX_RECENT_VIDEOS_LIMIT: usize = 100;

#[derive(Debug, Deserialize)]
pub struct ListRecentVideosQuery {
    #[serde(default)]
    pub limit: Option<usize>,
}

pub async fn list_videos_for_playlist(
    State(video_searcher): State<VideoSearcher>,
    Path(playlist_id): Path<String>,
) -> Result<Response, ApiError> {
    let playlist_id = PlaylistId::new(playlist_id).map_err(ApiError::bad_request)?;

    let videos = run_blocking(move || video_searcher.list(&playlist_id))
        .await?
        .map_err(list_videos_error)?;
    let response: Vec<VideoResponse> = videos.into_iter().map(VideoResponse::from).collect();
    Ok((StatusCode::OK, Json(response)).into_response())
}

pub async fn list_videos_for_channel(
    State(video_searcher): State<VideoSearcher>,
    Path(handle): Path<String>,
) -> Result<Response, ApiError> {
    let channel_id = ChannelHandle::new(handle).map_err(ApiError::bad_request)?;

    let videos = run_blocking(move || video_searcher.list_for_channel(&channel_id))
        .await?
        .map_err(list_videos_error)?;
    let response: Vec<VideoResponse> = videos.into_iter().map(VideoResponse::from).collect();
    Ok((StatusCode::OK, Json(response)).into_response())
}

pub async fn list_recent_videos(
    State(video_searcher): State<VideoSearcher>,
    Query(query): Query<ListRecentVideosQuery>,
) -> Result<Response, ApiError> {
    let limit = query
        .limit
        .unwrap_or(DEFAULT_RECENT_VIDEOS_LIMIT)
        .min(MAX_RECENT_VIDEOS_LIMIT);

    let videos = run_blocking(move || video_searcher.list_recent(limit))
        .await?
        .map_err(list_videos_error)?;
    let response: Vec<RecentVideoResponse> =
        videos.into_iter().map(RecentVideoResponse::from).collect();
    Ok((StatusCode::OK, Json(response)).into_response())
}

fn list_videos_error(error: ListVideosError) -> ApiError {
    match error {
        e @ ListVideosError::PlaylistNotFound(_) => ApiError::bad_request(e),
        e @ ListVideosError::ChannelNotFound(_) => ApiError::bad_request(e),
        e @ ListVideosError::Repository(_) => ApiError::internal(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::{Channel, VideoLimit};
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::playlist::{Playlist, PlaylistKind, PlaylistName, PlaylistPath};
    use crate::domain::playlist_video::PlaylistVideo;
    use crate::domain::shared::{Quality, VideoId};
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
    use crate::infrastructure::repositories::sqlite_video_repository::{
        FakeVideoRepository, VideoRepository,
    };
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
        playlist_video_repository: Arc<FakePlaylistVideoRepository>,
        channel_repository: Arc<FakeChannelRepository>,
        channel_video_repository: Arc<FakeChannelVideoRepository>,
        video_repository: Arc<FakeVideoRepository>,
    ) -> axum::Router {
        let video_searcher = VideoSearcher::new(
            playlist_repository,
            playlist_video_repository,
            channel_repository,
            channel_video_repository,
            video_repository,
        );
        let api = Router::new()
            .route("/playlists/{id}/videos", get(list_videos_for_playlist))
            .route("/channels/{handle}/videos", get(list_videos_for_channel))
            .route("/videos/recent", get(list_recent_videos))
            .with_state(video_searcher);
        Router::new().nest("/api", api)
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

    fn channel(id: &str) -> Channel {
        Channel::create(
            ChannelHandle::new(id).unwrap(),
            "Some Channel",
            "UC123",
            Quality::High,
            VideoLimit::new(10).unwrap(),
            PlaylistPath::new("creators/somechannel").unwrap(),
            None,
            fixed_timestamp(),
        )
    }

    fn recent_request(query: &str) -> Request<Body> {
        let uri = if query.is_empty() {
            "/api/videos/recent".to_string()
        } else {
            format!("/api/videos/recent?{query}")
        };
        Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    }

    fn downloaded_video(youtube_id: &str, title: &str, created_at: DateTime<Utc>) -> Video {
        Video::create(VideoId::new(youtube_id).unwrap(), title, created_at).mark_downloaded(
            Quality::High,
            format!("{title}.mp4"),
            None,
            None,
            created_at,
        )
    }

    fn downloaded_video_with_thumbnail(
        youtube_id: &str,
        title: &str,
        created_at: DateTime<Utc>,
    ) -> Video {
        Video::create(VideoId::new(youtube_id).unwrap(), title, created_at).mark_downloaded(
            Quality::High,
            format!("{title}.mp4"),
            Some(format!("{title}.jpg")),
            None,
            created_at,
        )
    }

    fn request(playlist_id: &str) -> Request<Body> {
        Request::builder()
            .method("GET")
            .uri(format!("/api/playlists/{playlist_id}/videos"))
            .body(Body::empty())
            .unwrap()
    }

    fn channel_request(handle: &str) -> Request<Body> {
        Request::builder()
            .method("GET")
            .uri(format!("/api/channels/{handle}/videos"))
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn it_should_return_the_playlists_videos() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let playlist_video_repository = Arc::new(FakePlaylistVideoRepository::default());
        let video_repository = Arc::new(FakeVideoRepository::default());
        let video = Video::create(VideoId::new("vid1").unwrap(), "My Video", fixed_timestamp());
        video_repository.save(&video).unwrap();
        playlist_video_repository
            .save(&PlaylistVideo::create(
                PlaylistId::new("PL1").unwrap(),
                video.id.clone(),
                fixed_timestamp(),
            ))
            .unwrap();
        let router = test_router(
            playlist_repository,
            playlist_video_repository,
            Arc::new(FakeChannelRepository::default()),
            Arc::new(FakeChannelVideoRepository::default()),
            video_repository,
        );

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
    async fn it_should_return_videos_ordered_by_playlist_position() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let playlist_video_repository = Arc::new(FakePlaylistVideoRepository::default());
        let video_repository = Arc::new(FakeVideoRepository::default());
        for (youtube_id, title, position) in [
            ("vid_third", "Third", 2),
            ("vid_first", "First", 0),
            ("vid_second", "Second", 1),
        ] {
            let video = Video::create(VideoId::new(youtube_id).unwrap(), title, fixed_timestamp());
            video_repository.save(&video).unwrap();
            playlist_video_repository
                .save(&PlaylistVideo::create_with_position(
                    PlaylistId::new("PL1").unwrap(),
                    video.id.clone(),
                    position,
                    fixed_timestamp(),
                ))
                .unwrap();
        }
        let router = test_router(
            playlist_repository,
            playlist_video_repository,
            Arc::new(FakeChannelRepository::default()),
            Arc::new(FakeChannelVideoRepository::default()),
            video_repository,
        );

        let response = router.oneshot(request("PL1")).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let videos = body.as_array().unwrap();
        assert_eq!(
            videos.iter().map(|v| v["id"].clone()).collect::<Vec<_>>(),
            vec!["vid_first", "vid_second", "vid_third"]
        );
    }

    #[tokio::test]
    async fn it_should_return_an_empty_list_when_the_playlist_has_no_videos() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let router = test_router(
            playlist_repository,
            Arc::new(FakePlaylistVideoRepository::default()),
            Arc::new(FakeChannelRepository::default()),
            Arc::new(FakeChannelVideoRepository::default()),
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
            Arc::new(FakePlaylistVideoRepository::default()),
            Arc::new(FakeChannelRepository::default()),
            Arc::new(FakeChannelVideoRepository::default()),
            Arc::new(FakeVideoRepository::default()),
        );

        let response = router.oneshot(request("PL404")).await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_the_channels_videos_ordered_by_recency() {
        let channel_repository = Arc::new(FakeChannelRepository::default());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let channel_video_repository = Arc::new(FakeChannelVideoRepository::default());
        let video_repository = Arc::new(FakeVideoRepository::default());
        for (youtube_id, title, position) in
            [("vid_newest", "Newest", 0), ("vid_oldest", "Oldest", 1)]
        {
            let video = Video::create(VideoId::new(youtube_id).unwrap(), title, fixed_timestamp());
            video_repository.save(&video).unwrap();
            channel_video_repository
                .save(&ChannelVideo::create(
                    ChannelHandle::new("@somechannel").unwrap(),
                    video.id.clone(),
                    position,
                    fixed_timestamp(),
                ))
                .unwrap();
        }
        let router = test_router(
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakePlaylistVideoRepository::default()),
            channel_repository,
            channel_video_repository,
            video_repository,
        );

        let response = router
            .oneshot(channel_request("@somechannel"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let videos = body.as_array().unwrap();
        assert_eq!(
            videos.iter().map(|v| v["id"].clone()).collect::<Vec<_>>(),
            vec!["vid_newest", "vid_oldest"]
        );
    }

    #[tokio::test]
    async fn it_should_return_an_empty_list_when_the_channel_has_no_videos() {
        let channel_repository = Arc::new(FakeChannelRepository::default());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let router = test_router(
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakePlaylistVideoRepository::default()),
            channel_repository,
            Arc::new(FakeChannelVideoRepository::default()),
            Arc::new(FakeVideoRepository::default()),
        );

        let response = router
            .oneshot(channel_request("@somechannel"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body, serde_json::json!([]));
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_channel_does_not_exist() {
        let router = test_router(
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakePlaylistVideoRepository::default()),
            Arc::new(FakeChannelRepository::default()),
            Arc::new(FakeChannelVideoRepository::default()),
            Arc::new(FakeVideoRepository::default()),
        );

        let response = router.oneshot(channel_request("@missing")).await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_an_empty_list_when_no_downloaded_videos_exist() {
        let router = test_router(
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakePlaylistVideoRepository::default()),
            Arc::new(FakeChannelRepository::default()),
            Arc::new(FakeChannelVideoRepository::default()),
            Arc::new(FakeVideoRepository::default()),
        );

        let response = router.oneshot(recent_request("")).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body, serde_json::json!([]));
    }

    #[tokio::test]
    async fn it_should_combine_and_sort_recent_videos_from_playlists_and_channels() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let playlist_video_repository = Arc::new(FakePlaylistVideoRepository::default());
        let channel_repository = Arc::new(FakeChannelRepository::default());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let channel_video_repository = Arc::new(FakeChannelVideoRepository::default());
        let video_repository = Arc::new(FakeVideoRepository::default());

        let from_playlist = downloaded_video(
            "vid_from_playlist",
            "From Playlist",
            DateTime::<Utc>::from_timestamp(100, 0).unwrap(),
        );
        video_repository.save(&from_playlist).unwrap();
        playlist_video_repository
            .save(&PlaylistVideo::create(
                PlaylistId::new("PL1").unwrap(),
                from_playlist.id.clone(),
                fixed_timestamp(),
            ))
            .unwrap();

        let from_channel = downloaded_video_with_thumbnail(
            "vid_from_channel",
            "From Channel",
            DateTime::<Utc>::from_timestamp(200, 0).unwrap(),
        );
        video_repository.save(&from_channel).unwrap();
        channel_video_repository
            .save(&ChannelVideo::create(
                ChannelHandle::new("@somechannel").unwrap(),
                from_channel.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();

        let router = test_router(
            playlist_repository,
            playlist_video_repository,
            channel_repository,
            channel_video_repository,
            video_repository,
        );

        let response = router.oneshot(recent_request("")).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let videos = body.as_array().unwrap();
        assert_eq!(videos.len(), 2);
        assert_eq!(videos[0]["id"], "vid_from_channel");
        assert_eq!(videos[0]["source"]["kind"], "channel");
        assert_eq!(videos[0]["source"]["id"], "@somechannel");
        assert_eq!(videos[0]["source"]["path"], "creators/somechannel");
        assert_eq!(videos[0]["thumbnail_filename"], "From Channel.jpg");
        assert_eq!(videos[1]["id"], "vid_from_playlist");
        assert_eq!(videos[1]["source"]["kind"], "playlist");
        assert_eq!(videos[1]["source"]["id"], "PL1");
        assert_eq!(videos[1]["source"]["path"], "music");
        assert_eq!(videos[1]["thumbnail_filename"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn it_should_exclude_non_downloaded_videos_from_recent() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let playlist_video_repository = Arc::new(FakePlaylistVideoRepository::default());
        let video_repository = Arc::new(FakeVideoRepository::default());

        let pending = Video::create(
            VideoId::new("vid_pending").unwrap(),
            "Pending",
            fixed_timestamp(),
        );
        video_repository.save(&pending).unwrap();
        playlist_video_repository
            .save(&PlaylistVideo::create(
                PlaylistId::new("PL1").unwrap(),
                pending.id.clone(),
                fixed_timestamp(),
            ))
            .unwrap();

        let router = test_router(
            playlist_repository,
            playlist_video_repository,
            Arc::new(FakeChannelRepository::default()),
            Arc::new(FakeChannelVideoRepository::default()),
            video_repository,
        );

        let response = router.oneshot(recent_request("")).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body, serde_json::json!([]));
    }

    #[tokio::test]
    async fn it_should_list_a_video_tracked_by_two_sources_once_per_source() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let playlist_video_repository = Arc::new(FakePlaylistVideoRepository::default());
        let channel_repository = Arc::new(FakeChannelRepository::default());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let channel_video_repository = Arc::new(FakeChannelVideoRepository::default());
        let video_repository = Arc::new(FakeVideoRepository::default());

        let shared = downloaded_video("vid_shared", "Shared", fixed_timestamp());
        video_repository.save(&shared).unwrap();
        playlist_video_repository
            .save(&PlaylistVideo::create(
                PlaylistId::new("PL1").unwrap(),
                shared.id.clone(),
                fixed_timestamp(),
            ))
            .unwrap();
        channel_video_repository
            .save(&ChannelVideo::create(
                ChannelHandle::new("@somechannel").unwrap(),
                shared.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();

        let router = test_router(
            playlist_repository,
            playlist_video_repository,
            channel_repository,
            channel_video_repository,
            video_repository,
        );

        let response = router.oneshot(recent_request("")).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let videos = body.as_array().unwrap();
        assert_eq!(videos.len(), 2);
        assert!(videos.iter().all(|v| v["id"] == "vid_shared"));
        let kinds: Vec<&str> = videos
            .iter()
            .map(|v| v["source"]["kind"].as_str().unwrap())
            .collect();
        assert!(kinds.contains(&"playlist"));
        assert!(kinds.contains(&"channel"));
    }

    #[tokio::test]
    async fn it_should_apply_the_default_limit_of_20_when_omitted() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let playlist_video_repository = Arc::new(FakePlaylistVideoRepository::default());
        let video_repository = Arc::new(FakeVideoRepository::default());
        for i in 0..25 {
            let video = downloaded_video(
                &format!("vid{i}"),
                &format!("Video {i}"),
                DateTime::<Utc>::from_timestamp(i, 0).unwrap(),
            );
            video_repository.save(&video).unwrap();
            playlist_video_repository
                .save(&PlaylistVideo::create(
                    PlaylistId::new("PL1").unwrap(),
                    video.id.clone(),
                    fixed_timestamp(),
                ))
                .unwrap();
        }
        let router = test_router(
            playlist_repository,
            playlist_video_repository,
            Arc::new(FakeChannelRepository::default()),
            Arc::new(FakeChannelVideoRepository::default()),
            video_repository,
        );

        let response = router.oneshot(recent_request("")).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let videos = body.as_array().unwrap();
        assert_eq!(videos.len(), 20);
        assert_eq!(videos[0]["id"], "vid24");
        assert_eq!(videos[19]["id"], "vid5");
    }

    #[tokio::test]
    async fn it_should_narrow_the_result_with_an_explicit_limit() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let playlist_video_repository = Arc::new(FakePlaylistVideoRepository::default());
        let video_repository = Arc::new(FakeVideoRepository::default());
        for i in 0..3 {
            let video = downloaded_video(
                &format!("vid{i}"),
                &format!("Video {i}"),
                DateTime::<Utc>::from_timestamp(i, 0).unwrap(),
            );
            video_repository.save(&video).unwrap();
            playlist_video_repository
                .save(&PlaylistVideo::create(
                    PlaylistId::new("PL1").unwrap(),
                    video.id.clone(),
                    fixed_timestamp(),
                ))
                .unwrap();
        }
        let router = test_router(
            playlist_repository,
            playlist_video_repository,
            Arc::new(FakeChannelRepository::default()),
            Arc::new(FakeChannelVideoRepository::default()),
            video_repository,
        );

        let response = router.oneshot(recent_request("limit=2")).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let videos = body.as_array().unwrap();
        assert_eq!(videos.len(), 2);
        assert_eq!(videos[0]["id"], "vid2");
        assert_eq!(videos[1]["id"], "vid1");
    }

    #[tokio::test]
    async fn it_should_cap_the_limit_at_100() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let playlist_video_repository = Arc::new(FakePlaylistVideoRepository::default());
        let video_repository = Arc::new(FakeVideoRepository::default());
        for i in 0..105 {
            let video = downloaded_video(
                &format!("vid{i}"),
                &format!("Video {i}"),
                DateTime::<Utc>::from_timestamp(i, 0).unwrap(),
            );
            video_repository.save(&video).unwrap();
            playlist_video_repository
                .save(&PlaylistVideo::create(
                    PlaylistId::new("PL1").unwrap(),
                    video.id.clone(),
                    fixed_timestamp(),
                ))
                .unwrap();
        }
        let router = test_router(
            playlist_repository,
            playlist_video_repository,
            Arc::new(FakeChannelRepository::default()),
            Arc::new(FakeChannelVideoRepository::default()),
            video_repository,
        );

        let response = router.oneshot(recent_request("limit=1000")).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let videos = body.as_array().unwrap();
        assert_eq!(videos.len(), 100);
        assert_eq!(videos[0]["id"], "vid104");
        assert_eq!(videos[99]["id"], "vid5");
    }
}
