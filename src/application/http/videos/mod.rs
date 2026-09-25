pub mod dto;

use super::blocking::run_blocking;
use super::error::ApiError;
use super::validation::{MISSING_POSITION, required};
use crate::domain::channel::ChannelHandle;
use crate::domain::playlist::PlaylistId;
use crate::domain::services::{
    VideoSearcher, VideoSearcherApi, VideoWatchStateUpdater, VideoWatchStateUpdaterApi,
};
use crate::domain::video::{ListVideosError, PlaybackPosition, UpdateWatchStateError, VideoId};
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use dto::{RecentVideoResponse, RecordProgressRequest, VideoResponse};
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
) -> Result<Json<Vec<VideoResponse>>, ApiError> {
    let playlist_id = PlaylistId::new(playlist_id)?;

    let videos = run_blocking(move || video_searcher.list(&playlist_id))
        .await?
        .map_err(list_videos_error)?;
    Ok(Json(videos.into_iter().map(VideoResponse::from).collect()))
}

pub async fn list_videos_for_channel(
    State(video_searcher): State<VideoSearcher>,
    Path(handle): Path<String>,
) -> Result<Json<Vec<VideoResponse>>, ApiError> {
    let channel_id = ChannelHandle::new(handle)?;

    let videos = run_blocking(move || video_searcher.list_for_channel(&channel_id))
        .await?
        .map_err(list_videos_error)?;
    Ok(Json(videos.into_iter().map(VideoResponse::from).collect()))
}

pub async fn list_recent_videos(
    State(video_searcher): State<VideoSearcher>,
    Query(query): Query<ListRecentVideosQuery>,
) -> Result<Json<Vec<RecentVideoResponse>>, ApiError> {
    let limit = query
        .limit
        .unwrap_or(DEFAULT_RECENT_VIDEOS_LIMIT)
        .min(MAX_RECENT_VIDEOS_LIMIT);

    let videos = run_blocking(move || video_searcher.list_recent(limit))
        .await?
        .map_err(list_videos_error)?;
    Ok(Json(
        videos.into_iter().map(RecentVideoResponse::from).collect(),
    ))
}

pub async fn record_video_progress(
    State(video_watch_state_updater): State<VideoWatchStateUpdater>,
    Path(youtube_id): Path<String>,
    Json(request): Json<RecordProgressRequest>,
) -> Result<StatusCode, ApiError> {
    let youtube_id = VideoId::new(youtube_id)?;
    let position = PlaybackPosition::new(required(request.position_seconds, MISSING_POSITION)?)?;

    run_blocking(move || {
        video_watch_state_updater.update(&youtube_id, position, request.duration_seconds)
    })
    .await?
    .map_err(update_watch_state_error)?;

    Ok(StatusCode::NO_CONTENT)
}

pub fn update_watch_state_error(error: UpdateWatchStateError) -> ApiError {
    match error {
        e @ UpdateWatchStateError::VideoNotFound(_) => ApiError::bad_request(e),
        e @ UpdateWatchStateError::ChannelNotFound(_) => ApiError::bad_request(e),
        e @ UpdateWatchStateError::Repository(_) => ApiError::internal(e),
    }
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
    use crate::domain::shared::Quality;
    use crate::domain::video::Video;
    use crate::infrastructure::repositories::sqlite_channel_repository::{
        ChannelRepository, SqliteChannelRepository,
    };
    use crate::infrastructure::repositories::sqlite_channel_video_repository::{
        ChannelVideoRepository, SqliteChannelVideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_repository::{
        PlaylistRepository, SqlitePlaylistRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::{
        PlaylistVideoRepository, SqlitePlaylistVideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_repository::{
        SqliteVideoRepository, VideoRepository,
    };
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use dto::RecentVideoSourceResponse;
    use rusqlite::Connection;
    use std::sync::Arc;

    #[tokio::test]
    async fn it_should_list_playlist_videos() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let video = Video::create(VideoId::new("vid1").unwrap(), "My Video", fixed_timestamp());
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            "PL1",
            &video,
        );
        let video_searcher = VideoSearcher::new(
            playlist_repository,
            playlist_video_repository,
            Arc::new(SqliteChannelRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            video_repository,
        );

        let response = list_for_playlist(video_searcher, "PL1").await;

        assert_eq!(
            response,
            Ok(vec![pending_video_response("vid1", "My Video")])
        );
    }

    #[tokio::test]
    async fn it_should_include_download_details_of_downloaded_videos() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let video = Video::create(VideoId::new("vid1").unwrap(), "My Video", fixed_timestamp())
            .start_download(fixed_timestamp())
            .mark_downloaded(
                Quality::High,
                "My Video.mp4",
                Some("My Video.jpg".to_string()),
                Some(223),
                fixed_timestamp(),
            );
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            "PL1",
            &video,
        );
        let video_searcher = VideoSearcher::new(
            playlist_repository,
            playlist_video_repository,
            Arc::new(SqliteChannelRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            video_repository,
        );

        let response = list_for_playlist(video_searcher, "PL1").await;

        assert_eq!(
            response,
            Ok(vec![VideoResponse {
                status: "DOWNLOADED".to_string(),
                quality: Some("high".to_string()),
                filename: Some("My Video.mp4".to_string()),
                thumbnail_filename: Some("My Video.jpg".to_string()),
                duration_seconds: Some(223),
                ..pending_video_response("vid1", "My Video")
            }])
        );
    }

    #[tokio::test]
    async fn it_should_list_playlist_videos_by_position() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        playlist_repository.insert(&playlist("PL1")).unwrap();
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
        let video_searcher = VideoSearcher::new(
            playlist_repository,
            playlist_video_repository,
            Arc::new(SqliteChannelRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            video_repository,
        );

        let response = list_for_playlist(video_searcher, "PL1").await;

        assert_eq!(
            response,
            Ok(vec![
                pending_video_response("vid_first", "First"),
                pending_video_response("vid_second", "Second"),
                pending_video_response("vid_third", "Third"),
            ])
        );
    }

    #[tokio::test]
    async fn it_should_include_watch_state_when_listing_playlist_videos() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        for (video, position) in [
            (
                Video::create(
                    VideoId::new("vid_watched").unwrap(),
                    "Watched",
                    fixed_timestamp(),
                )
                .mark_watched(watched_timestamp()),
                0,
            ),
            (
                Video {
                    playback_position: PlaybackPosition::new(42).unwrap(),
                    ..Video::create(
                        VideoId::new("vid_partly").unwrap(),
                        "Partly",
                        fixed_timestamp(),
                    )
                },
                1,
            ),
        ] {
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
        let video_searcher = VideoSearcher::new(
            playlist_repository,
            playlist_video_repository,
            Arc::new(SqliteChannelRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            video_repository,
        );

        let response = list_for_playlist(video_searcher, "PL1").await;

        assert_eq!(
            response,
            Ok(vec![
                VideoResponse {
                    watched: true,
                    ..pending_video_response("vid_watched", "Watched")
                },
                VideoResponse {
                    position_seconds: 42,
                    ..pending_video_response("vid_partly", "Partly")
                },
            ])
        );
    }

    #[tokio::test]
    async fn it_should_list_no_videos_for_an_empty_playlist() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let video_searcher = VideoSearcher::new(
            playlist_repository,
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection())),
            Arc::new(SqliteChannelRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            video_repository,
        );

        let response = list_for_playlist(video_searcher, "PL1").await;

        assert_eq!(response, Ok(vec![]));
    }

    #[tokio::test]
    async fn it_should_fail_if_playlist_not_found() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));

        let video_searcher = VideoSearcher::new(
            Arc::new(SqlitePlaylistRepository::new(db.connection())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection())),
            Arc::new(SqliteChannelRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            video_repository,
        );

        let response = list_for_playlist(video_searcher, "PL404").await;

        assert_eq!(
            response,
            Err(ApiError::bad_request("playlist PL404 not found"))
        );
    }

    #[tokio::test]
    async fn it_should_fail_if_invalid_playlist_id_provided() {
        let response = list_for_playlist(any_video_searcher(), "   ").await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "YouTube playlist ID must not be empty"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_list_channel_videos_by_recency() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        channel_repository
            .insert(&channel("@somechannel", None))
            .unwrap();
        for (youtube_id, title, position) in
            [("vid_newest", "Newest", 0), ("vid_oldest", "Oldest", 1)]
        {
            let video = Video::create(VideoId::new(youtube_id).unwrap(), title, fixed_timestamp());
            save_channel_video(
                video_repository.as_ref(),
                channel_video_repository.as_ref(),
                "@somechannel",
                &video,
                position,
            );
        }
        let video_searcher = VideoSearcher::new(
            Arc::new(SqlitePlaylistRepository::new(db.connection())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection())),
            channel_repository,
            channel_video_repository,
            video_repository,
        );

        let response = list_for_channel(video_searcher, "@somechannel").await;

        assert_eq!(
            response,
            Ok(vec![
                pending_video_response("vid_newest", "Newest"),
                pending_video_response("vid_oldest", "Oldest"),
            ])
        );
    }

    #[tokio::test]
    async fn it_should_include_watch_state_when_listing_channel_videos() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        channel_repository
            .insert(&channel("@somechannel", None))
            .unwrap();
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            "@somechannel",
            &Video::create(
                VideoId::new("vid_watched").unwrap(),
                "Watched",
                fixed_timestamp(),
            )
            .mark_watched(watched_timestamp()),
            0,
        );
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            "@somechannel",
            &Video {
                playback_position: PlaybackPosition::new(42).unwrap(),
                ..Video::create(
                    VideoId::new("vid_partly").unwrap(),
                    "Partly",
                    fixed_timestamp(),
                )
            },
            1,
        );
        let video_searcher = VideoSearcher::new(
            Arc::new(SqlitePlaylistRepository::new(db.connection())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection())),
            channel_repository,
            channel_video_repository,
            video_repository,
        );

        let response = list_for_channel(video_searcher, "@somechannel").await;

        assert_eq!(
            response,
            Ok(vec![
                VideoResponse {
                    watched: true,
                    ..pending_video_response("vid_watched", "Watched")
                },
                VideoResponse {
                    position_seconds: 42,
                    ..pending_video_response("vid_partly", "Partly")
                },
            ])
        );
    }

    #[tokio::test]
    async fn it_should_list_no_videos_for_an_empty_channel() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        channel_repository
            .insert(&channel("@somechannel", None))
            .unwrap();
        let video_searcher = VideoSearcher::new(
            Arc::new(SqlitePlaylistRepository::new(db.connection())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection())),
            channel_repository,
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            video_repository,
        );

        let response = list_for_channel(video_searcher, "@somechannel").await;

        assert_eq!(response, Ok(vec![]));
    }

    #[tokio::test]
    async fn it_should_fail_if_channel_not_found() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));

        let video_searcher = VideoSearcher::new(
            Arc::new(SqlitePlaylistRepository::new(db.connection())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection())),
            Arc::new(SqliteChannelRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            video_repository,
        );

        let response = list_for_channel(video_searcher, "@missing").await;

        assert_eq!(
            response,
            Err(ApiError::bad_request("channel @missing not found"))
        );
    }

    #[tokio::test]
    async fn it_should_fail_if_invalid_handle_provided() {
        let response = list_for_channel(any_video_searcher(), "somechannel").await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Channel handle must start with \"@\" (got \"somechannel\")"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_list_no_recent_videos_if_nothing_downloaded() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));

        let video_searcher = VideoSearcher::new(
            Arc::new(SqlitePlaylistRepository::new(db.connection())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection())),
            Arc::new(SqliteChannelRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            video_repository,
        );

        let response = list_recent(video_searcher, ListRecentVideosQuery { limit: None }).await;

        assert_eq!(response, Ok(vec![]));
    }

    #[tokio::test]
    async fn it_should_merge_recent_videos_from_playlists_and_channels() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        channel_repository
            .insert(&channel("@somechannel", None))
            .unwrap();
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            "PL1",
            &downloaded_video("vid_from_playlist", "From Playlist", None, 100),
        );
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            "@somechannel",
            &downloaded_video(
                "vid_from_channel",
                "From Channel",
                Some("From Channel.jpg"),
                200,
            ),
            0,
        );
        let video_searcher = VideoSearcher::new(
            playlist_repository,
            playlist_video_repository,
            channel_repository,
            channel_video_repository,
            video_repository,
        );

        let response = list_recent(video_searcher, ListRecentVideosQuery { limit: None }).await;

        assert_eq!(
            response,
            Ok(vec![
                RecentVideoResponse {
                    thumbnail_filename: Some("From Channel.jpg".to_string()),
                    ..recent_video_response(
                        "vid_from_channel",
                        "From Channel",
                        channel_source(None)
                    )
                },
                recent_video_response("vid_from_playlist", "From Playlist", playlist_source()),
            ])
        );
    }

    #[tokio::test]
    async fn it_should_include_duration_in_recent_videos() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        let video = Video::create(VideoId::new("vid1").unwrap(), "My Video", fixed_timestamp())
            .mark_downloaded(
                Quality::High,
                "My Video.mp4",
                None,
                Some(223),
                fixed_timestamp(),
            );
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            "PL1",
            &video,
        );
        let video_searcher = VideoSearcher::new(
            playlist_repository,
            playlist_video_repository,
            Arc::new(SqliteChannelRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            video_repository,
        );

        let response = list_recent(video_searcher, ListRecentVideosQuery { limit: None }).await;

        assert_eq!(
            response,
            Ok(vec![RecentVideoResponse {
                duration_seconds: Some(223),
                ..recent_video_response("vid1", "My Video", playlist_source())
            }])
        );
    }

    #[tokio::test]
    async fn it_should_include_channel_avatar_in_recent_videos() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        channel_repository
            .insert(&channel("@somechannel", Some("@somechannel.jpg")))
            .unwrap();
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            "@somechannel",
            &downloaded_video("vid1", "My Video", None, 100),
            0,
        );
        let video_searcher = VideoSearcher::new(
            Arc::new(SqlitePlaylistRepository::new(db.connection())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection())),
            channel_repository,
            channel_video_repository,
            video_repository,
        );

        let response = list_recent(video_searcher, ListRecentVideosQuery { limit: None }).await;

        assert_eq!(
            response,
            Ok(vec![recent_video_response(
                "vid1",
                "My Video",
                channel_source(Some("@somechannel.jpg"))
            )])
        );
    }

    #[tokio::test]
    async fn it_should_include_whether_recent_videos_were_watched() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            "PL1",
            &downloaded_video("vid1", "My Video", None, 100).mark_watched(watched_timestamp()),
        );
        let video_searcher = VideoSearcher::new(
            playlist_repository,
            playlist_video_repository,
            Arc::new(SqliteChannelRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            video_repository,
        );

        let response = list_recent(video_searcher, ListRecentVideosQuery { limit: None }).await;

        assert_eq!(
            response,
            Ok(vec![RecentVideoResponse {
                watched: true,
                ..recent_video_response("vid1", "My Video", playlist_source())
            }])
        );
    }

    #[tokio::test]
    async fn it_should_exclude_not_downloaded_videos_from_recent() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            "PL1",
            &Video::create(
                VideoId::new("vid_pending").unwrap(),
                "Pending",
                fixed_timestamp(),
            ),
        );
        let video_searcher = VideoSearcher::new(
            playlist_repository,
            playlist_video_repository,
            Arc::new(SqliteChannelRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            video_repository,
        );

        let response = list_recent(video_searcher, ListRecentVideosQuery { limit: None }).await;

        assert_eq!(response, Ok(vec![]));
    }

    #[tokio::test]
    async fn it_should_list_a_recent_video_once_per_source() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        channel_repository
            .insert(&channel("@somechannel", None))
            .unwrap();
        let shared = downloaded_video("vid_shared", "Shared", None, 100);
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            "PL1",
            &shared,
        );
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            "@somechannel",
            &shared,
            0,
        );
        let video_searcher = VideoSearcher::new(
            playlist_repository,
            playlist_video_repository,
            channel_repository,
            channel_video_repository,
            video_repository,
        );

        let response = list_recent(video_searcher, ListRecentVideosQuery { limit: None }).await;

        assert_eq!(
            response,
            Ok(vec![
                recent_video_response("vid_shared", "Shared", playlist_source()),
                recent_video_response("vid_shared", "Shared", channel_source(None)),
            ])
        );
    }

    #[tokio::test]
    async fn it_should_default_recent_limit_to_20() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        save_numbered_playlist_videos(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            25,
        );
        let video_searcher = VideoSearcher::new(
            playlist_repository,
            playlist_video_repository,
            Arc::new(SqliteChannelRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            video_repository,
        );

        let response = list_recent(video_searcher, ListRecentVideosQuery { limit: None }).await;

        assert_eq!(response, Ok(numbered_recent_videos((5..25).rev())));
    }

    #[tokio::test]
    async fn it_should_honor_an_explicit_recent_limit() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        save_numbered_playlist_videos(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            3,
        );
        let video_searcher = VideoSearcher::new(
            playlist_repository,
            playlist_video_repository,
            Arc::new(SqliteChannelRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            video_repository,
        );

        let response = list_recent(video_searcher, ListRecentVideosQuery { limit: Some(2) }).await;

        assert_eq!(response, Ok(numbered_recent_videos((1..3).rev())));
    }

    #[tokio::test]
    async fn it_should_cap_recent_limit_at_100() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        playlist_repository.insert(&playlist("PL1")).unwrap();
        save_numbered_playlist_videos(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            105,
        );
        let video_searcher = VideoSearcher::new(
            playlist_repository,
            playlist_video_repository,
            Arc::new(SqliteChannelRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            video_repository,
        );

        let response =
            list_recent(video_searcher, ListRecentVideosQuery { limit: Some(1000) }).await;

        assert_eq!(response, Ok(numbered_recent_videos((5..105).rev())));
    }

    #[tokio::test]
    async fn it_should_record_playback_progress() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let video = video_with_duration("vid1", Some(100));
        video_repository.save(&video).unwrap();
        let video_watch_state_updater = VideoWatchStateUpdater::new(
            video_repository.clone(),
            Arc::new(SqliteChannelRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            Arc::new(FixedClock(watched_timestamp())),
        );
        let request = progress_request(30);

        let response = record_progress(video_watch_state_updater, "vid1", request).await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                playback_position: PlaybackPosition::new(30).unwrap(),
                ..video
            }]
        );
    }

    #[tokio::test]
    async fn it_should_mark_the_video_watched_at_90_percent() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let video = Video {
            playback_position: PlaybackPosition::new(60).unwrap(),
            ..video_with_duration("vid1", Some(100))
        };
        video_repository.save(&video).unwrap();
        let video_watch_state_updater = VideoWatchStateUpdater::new(
            video_repository.clone(),
            Arc::new(SqliteChannelRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            Arc::new(FixedClock(watched_timestamp())),
        );
        let request = progress_request(90);

        let response = record_progress(video_watch_state_updater, "vid1", request).await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                watched_at: Some(watched_timestamp()),
                playback_position: PlaybackPosition::start(),
                ..video
            }]
        );
    }

    #[tokio::test]
    async fn it_should_use_the_reported_duration_if_none_is_recorded() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let video = video_with_duration("vid1", None);
        video_repository.save(&video).unwrap();
        let video_watch_state_updater = VideoWatchStateUpdater::new(
            video_repository.clone(),
            Arc::new(SqliteChannelRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            Arc::new(FixedClock(watched_timestamp())),
        );
        let request = RecordProgressRequest {
            duration_seconds: Some(100),
            ..progress_request(95)
        };

        let response = record_progress(video_watch_state_updater, "vid1", request).await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                watched_at: Some(watched_timestamp()),
                ..video
            }]
        );
    }

    #[tokio::test]
    async fn it_should_only_record_the_position_if_duration_is_unknown() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let video = video_with_duration("vid1", None);
        video_repository.save(&video).unwrap();
        let video_watch_state_updater = VideoWatchStateUpdater::new(
            video_repository.clone(),
            Arc::new(SqliteChannelRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            Arc::new(FixedClock(watched_timestamp())),
        );
        let request = RecordProgressRequest {
            duration_seconds: Some(0),
            ..progress_request(500)
        };

        let response = record_progress(video_watch_state_updater, "vid1", request).await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                playback_position: PlaybackPosition::new(500).unwrap(),
                ..video
            }]
        );
    }

    #[tokio::test]
    async fn it_should_keep_a_watched_video_watched_early_in_a_rewatch() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let video = video_with_duration("vid1", Some(100)).mark_watched(fixed_timestamp());
        video_repository.save(&video).unwrap();
        let video_watch_state_updater = VideoWatchStateUpdater::new(
            video_repository.clone(),
            Arc::new(SqliteChannelRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            Arc::new(FixedClock(watched_timestamp())),
        );
        let request = progress_request(10);

        let response = record_progress(video_watch_state_updater, "vid1", request).await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(video_repository.list().unwrap(), vec![video]);
    }

    #[tokio::test]
    async fn it_should_mark_a_watched_video_unwatched_past_10_percent_of_a_rewatch() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let video = video_with_duration("vid1", Some(100)).mark_watched(fixed_timestamp());
        video_repository.save(&video).unwrap();
        let video_watch_state_updater = VideoWatchStateUpdater::new(
            video_repository.clone(),
            Arc::new(SqliteChannelRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            Arc::new(FixedClock(watched_timestamp())),
        );
        let request = progress_request(11);

        let response = record_progress(video_watch_state_updater, "vid1", request).await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                watched_at: None,
                playback_position: PlaybackPosition::new(11).unwrap(),
                ..video
            }]
        );
    }

    #[tokio::test]
    async fn it_should_keep_a_watched_video_watched_when_playing_on_past_90_percent() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let video = video_with_duration("vid1", Some(100)).mark_watched(fixed_timestamp());
        video_repository.save(&video).unwrap();
        let video_watch_state_updater = VideoWatchStateUpdater::new(
            video_repository.clone(),
            Arc::new(SqliteChannelRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            Arc::new(FixedClock(watched_timestamp())),
        );
        let request = progress_request(95);

        let response = record_progress(video_watch_state_updater, "vid1", request).await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(video_repository.list().unwrap(), vec![video]);
    }

    #[tokio::test]
    async fn it_should_record_progress_on_every_copy_of_the_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let playlist_repository = SqlitePlaylistRepository::new(db.connection());
        playlist_repository.insert(&playlist("PL1")).unwrap();
        channel_repository
            .insert(&channel("@somechannel", None))
            .unwrap();
        let channel_copy = video_with_duration("vid1", Some(100));
        let playlist_copy = video_with_duration("vid1", Some(100));
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            "@somechannel",
            &channel_copy,
            0,
        );
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            "PL1",
            &playlist_copy,
        );
        let video_watch_state_updater = VideoWatchStateUpdater::new(
            video_repository.clone(),
            channel_repository,
            channel_video_repository,
            Arc::new(FixedClock(watched_timestamp())),
        );
        let request = progress_request(30);

        let response = record_progress(video_watch_state_updater, "vid1", request).await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                Video {
                    playback_position: PlaybackPosition::new(30).unwrap(),
                    ..channel_copy
                },
                Video {
                    playback_position: PlaybackPosition::new(30).unwrap(),
                    ..playlist_copy
                },
            ]
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_record_progress_of_an_unknown_video() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let video = video_with_duration("vid1", Some(100));
        video_repository.save(&video).unwrap();
        let video_watch_state_updater = VideoWatchStateUpdater::new(
            video_repository.clone(),
            Arc::new(SqliteChannelRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            Arc::new(FixedClock(watched_timestamp())),
        );
        let request = progress_request(30);

        let response = record_progress(video_watch_state_updater, "x", request).await;

        assert_eq!(response, Err(ApiError::bad_request("video x not found")));
        assert_eq!(video_repository.list().unwrap(), vec![video]);
    }

    #[tokio::test]
    async fn it_should_fail_to_record_progress_if_position_missing() {
        let request = RecordProgressRequest {
            position_seconds: None,
            ..progress_request(30)
        };

        let response = record_progress(any_video_watch_state_updater(), "vid1", request).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Playback position must not be negative (missing)"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_record_progress_if_invalid_position_provided() {
        let request = progress_request(-1);

        let response = record_progress(any_video_watch_state_updater(), "vid1", request).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Playback position must not be negative (got -1)"
            ))
        );
    }

    /// A searcher for tests whose request is rejected before reaching it. Its
    /// repositories sit on an unmigrated in-memory database, so a request that
    /// wrongly got through would fail loudly instead of passing.
    fn any_video_searcher() -> VideoSearcher {
        VideoSearcher::new(
            Arc::new(SqlitePlaylistRepository::new(unused_connection())),
            Arc::new(SqlitePlaylistVideoRepository::new(unused_connection())),
            Arc::new(SqliteChannelRepository::new(unused_connection())),
            Arc::new(SqliteChannelVideoRepository::new(unused_connection())),
            Arc::new(SqliteVideoRepository::new(unused_connection())),
        )
    }

    /// An updater for tests whose request is rejected before reaching it, on
    /// an unmigrated in-memory database like `any_video_searcher`.
    fn any_video_watch_state_updater() -> VideoWatchStateUpdater {
        VideoWatchStateUpdater::new(
            Arc::new(SqliteVideoRepository::new(unused_connection())),
            Arc::new(SqliteChannelRepository::new(unused_connection())),
            Arc::new(SqliteChannelVideoRepository::new(unused_connection())),
            Arc::new(FixedClock(watched_timestamp())),
        )
    }

    fn unused_connection() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    fn save_playlist_video(
        video_repository: &dyn VideoRepository,
        playlist_video_repository: &dyn PlaylistVideoRepository,
        playlist_id: &str,
        video: &Video,
    ) {
        video_repository.save(video).unwrap();
        playlist_video_repository
            .save(&PlaylistVideo::create(
                PlaylistId::new(playlist_id).unwrap(),
                video.id.clone(),
                fixed_timestamp(),
            ))
            .unwrap();
    }

    fn save_channel_video(
        video_repository: &dyn VideoRepository,
        channel_video_repository: &dyn ChannelVideoRepository,
        handle: &str,
        video: &Video,
        position: i64,
    ) {
        video_repository.save(video).unwrap();
        channel_video_repository
            .save(&ChannelVideo::create(
                ChannelHandle::new(handle).unwrap(),
                video.id.clone(),
                position,
                fixed_timestamp(),
            ))
            .unwrap();
    }

    /// Saves `count` downloaded videos to playlist `PL1`, video `i` created
    /// `i` seconds after the epoch so recency order is the reverse of `i`.
    fn save_numbered_playlist_videos(
        video_repository: &dyn VideoRepository,
        playlist_video_repository: &dyn PlaylistVideoRepository,
        count: i64,
    ) {
        for i in 0..count {
            save_playlist_video(
                video_repository,
                playlist_video_repository,
                "PL1",
                &downloaded_video(&format!("vid{i}"), &format!("Video {i}"), None, i),
            );
        }
    }

    fn numbered_recent_videos(numbers: impl Iterator<Item = i64>) -> Vec<RecentVideoResponse> {
        numbers
            .map(|i| {
                recent_video_response(&format!("vid{i}"), &format!("Video {i}"), playlist_source())
            })
            .collect()
    }

    fn pending_video_response(youtube_id: &str, title: &str) -> VideoResponse {
        VideoResponse {
            id: youtube_id.to_string(),
            title: title.to_string(),
            status: "PENDING".to_string(),
            quality: None,
            filename: None,
            thumbnail_filename: None,
            duration_seconds: None,
            created_at: fixed_timestamp(),
            updated_at: fixed_timestamp(),
            watched: false,
            position_seconds: 0,
        }
    }

    fn recent_video_response(
        youtube_id: &str,
        title: &str,
        source: RecentVideoSourceResponse,
    ) -> RecentVideoResponse {
        RecentVideoResponse {
            id: youtube_id.to_string(),
            title: title.to_string(),
            thumbnail_filename: None,
            duration_seconds: None,
            watched: false,
            source,
        }
    }

    fn playlist_source() -> RecentVideoSourceResponse {
        RecentVideoSourceResponse {
            kind: "playlist".to_string(),
            id: "PL1".to_string(),
            path: "music".to_string(),
            avatar_filename: None,
        }
    }

    fn channel_source(avatar_filename: Option<&str>) -> RecentVideoSourceResponse {
        RecentVideoSourceResponse {
            kind: "channel".to_string(),
            id: "@somechannel".to_string(),
            path: "creators/somechannel".to_string(),
            avatar_filename: avatar_filename.map(str::to_string),
        }
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
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

    fn channel(id: &str, avatar_filename: Option<&str>) -> Channel {
        Channel::create(
            ChannelHandle::new(id).unwrap(),
            "Some Channel",
            "UC123",
            Quality::High,
            VideoLimit::new(10).unwrap(),
            PlaylistPath::new("creators/somechannel").unwrap(),
            avatar_filename.map(str::to_string),
            fixed_timestamp(),
        )
    }

    fn downloaded_video(
        youtube_id: &str,
        title: &str,
        thumbnail_filename: Option<&str>,
        created_at_seconds: i64,
    ) -> Video {
        let created_at = DateTime::<Utc>::from_timestamp(created_at_seconds, 0).unwrap();
        Video::create(VideoId::new(youtube_id).unwrap(), title, created_at).mark_downloaded(
            Quality::High,
            format!("{title}.mp4"),
            thumbnail_filename.map(str::to_string),
            None,
            created_at,
        )
    }

    fn video_with_duration(youtube_id: &str, duration_seconds: Option<i64>) -> Video {
        Video::create(
            VideoId::new(youtube_id).unwrap(),
            "My Video",
            fixed_timestamp(),
        )
        .mark_downloaded(
            Quality::High,
            "My Video.mp4",
            None,
            duration_seconds,
            fixed_timestamp(),
        )
    }

    fn progress_request(position_seconds: i64) -> RecordProgressRequest {
        RecordProgressRequest {
            position_seconds: Some(position_seconds),
            duration_seconds: None,
        }
    }

    fn watched_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_800_000_000, 0).unwrap()
    }

    async fn record_progress(
        video_watch_state_updater: VideoWatchStateUpdater,
        youtube_id: &str,
        request: RecordProgressRequest,
    ) -> Result<StatusCode, ApiError> {
        record_video_progress(
            State(video_watch_state_updater),
            Path(youtube_id.to_string()),
            Json(request),
        )
        .await
    }

    async fn list_for_playlist(
        video_searcher: VideoSearcher,
        playlist_id: &str,
    ) -> Result<Vec<VideoResponse>, ApiError> {
        list_videos_for_playlist(State(video_searcher), Path(playlist_id.to_string()))
            .await
            .map(|Json(videos)| videos)
    }

    async fn list_for_channel(
        video_searcher: VideoSearcher,
        handle: &str,
    ) -> Result<Vec<VideoResponse>, ApiError> {
        list_videos_for_channel(State(video_searcher), Path(handle.to_string()))
            .await
            .map(|Json(videos)| videos)
    }

    async fn list_recent(
        video_searcher: VideoSearcher,
        query: ListRecentVideosQuery,
    ) -> Result<Vec<RecentVideoResponse>, ApiError> {
        list_recent_videos(State(video_searcher), Query(query))
            .await
            .map(|Json(videos)| videos)
    }
}
