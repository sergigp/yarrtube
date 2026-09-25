pub mod dto;

use super::blocking::run_blocking;
use super::error::ApiError;
use super::validation::{MISSING_QUALITY, required};
use super::videos::update_watch_state_error;
use crate::domain::channel::{ChannelHandle, CreateChannelError, DeleteChannelError, VideoLimit};
use crate::domain::playlist::PlaylistPath;
use crate::domain::services::{
    ChannelCreator, ChannelCreatorApi, ChannelDeleter, ChannelDeleterApi, ChannelSearcher,
    ChannelSearcherApi, ChannelVideoReconciler, ChannelVideoReconcilerApi, CreateChannelOutcome,
    VideoWatchStateUpdater, VideoWatchStateUpdaterApi,
};
use crate::domain::shared::Quality;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use dto::{ChannelListItemResponse, ChannelResponse, CreateChannelRequest};

const MISSING_VIDEO_LIMIT: &str = "Video limit must be a positive integer (missing)";
const MISSING_PATH: &str = "Channel path must not be empty";

pub async fn create_channel(
    State(channel_creator): State<ChannelCreator>,
    Json(request): Json<CreateChannelRequest>,
) -> Result<(StatusCode, Json<ChannelResponse>), ApiError> {
    let id = ChannelHandle::from_url_or_handle(request.channel.unwrap_or_default())?;
    let quality = Quality::new(required(request.quality, MISSING_QUALITY)?)?;
    let video_limit = VideoLimit::new(required(request.video_limit, MISSING_VIDEO_LIMIT)?)?;
    let path = PlaylistPath::new(required(request.path, MISSING_PATH)?)?;

    let outcome =
        run_blocking(move || channel_creator.create(id, quality, video_limit, path)).await?;

    match outcome {
        Ok(CreateChannelOutcome::Created(channel)) => {
            Ok((StatusCode::CREATED, Json(ChannelResponse::from(channel))))
        }
        Ok(CreateChannelOutcome::AlreadyExisted(channel)) => {
            Ok((StatusCode::OK, Json(ChannelResponse::from(channel))))
        }
        Err(e @ CreateChannelError::YoutubeChannelNotFound(_)) => Err(ApiError::bad_request(e)),
        Err(e @ CreateChannelError::Lookup(_)) => Err(ApiError::new(StatusCode::BAD_GATEWAY, e)),
        Err(e @ CreateChannelError::Repository(_)) => Err(ApiError::internal(e)),
    }
}

pub async fn delete_channel(
    State(channel_deleter): State<ChannelDeleter>,
    Path(handle): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = ChannelHandle::new(handle)?;

    match run_blocking(move || channel_deleter.delete(id)).await? {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(e @ DeleteChannelError::NotFound(_)) => Err(ApiError::bad_request(e)),
        Err(e @ DeleteChannelError::Repository(_)) => Err(ApiError::internal(e)),
    }
}

pub async fn list_channels(
    State(channel_searcher): State<ChannelSearcher>,
) -> Result<Json<Vec<ChannelListItemResponse>>, ApiError> {
    let channels = run_blocking(move || channel_searcher.search_all())
        .await?
        .map_err(ApiError::internal)?;
    Ok(Json(
        channels
            .into_iter()
            .map(ChannelListItemResponse::from)
            .collect(),
    ))
}

pub async fn reconcile_channel(
    State(channel_video_reconciler): State<ChannelVideoReconciler>,
    Path(handle): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = ChannelHandle::new(handle)?;

    run_blocking(move || channel_video_reconciler.force_reconcile(id))
        .await?
        .map_err(ApiError::internal)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn mark_channel_watched(
    State(video_watch_state_updater): State<VideoWatchStateUpdater>,
    Path(handle): Path<String>,
) -> Result<StatusCode, ApiError> {
    let channel_id = ChannelHandle::new(handle)?;

    run_blocking(move || video_watch_state_updater.mark_channel_watched(&channel_id))
        .await?
        .map_err(update_watch_state_error)?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::Channel;
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::event::{DomainEvent, ScheduledEvent};
    use crate::domain::playlist::PlaylistId;
    use crate::domain::playlist_video::PlaylistVideo;
    use crate::domain::services::ThumbnailFetcher;
    use crate::domain::video::Video;
    use crate::domain::video::VideoId;
    use crate::infrastructure::repositories::filesystem_channel_avatar_repository::FakeChannelAvatarRepository;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_channel_repository::{
        ChannelRepository, SqliteChannelRepository,
    };
    use crate::infrastructure::repositories::sqlite_channel_video_repository::{
        ChannelVideoRepository, SqliteChannelVideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::{
        PlaylistVideoRepository, SqlitePlaylistVideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_task_repository::{
        SqliteTaskRepository, TaskRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_metadata_repository::SqliteVideoMetadataRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::{
        SqliteVideoRepository, VideoRepository,
    };
    use crate::infrastructure::repositories::youtube_channel_repository::{
        FakeYoutubeChannelRepository, ResolvedChannel, YoutubeChannelRepository,
    };
    use crate::infrastructure::repositories::youtube_channel_videos_repository::{
        ChannelVideoListing, FakeChannelVideosRepository,
    };
    use crate::infrastructure::repositories::youtube_metadata_repository::FakeYoutubeMetadataRepository;
    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
    use crate::infrastructure::shared::domain_events::event_publisher::SqliteEventPublisher;
    use crate::infrastructure::shared::domain_events::event_repository::{
        EventRepository, SqliteEventRepository,
    };
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use rusqlite::Connection;
    use std::sync::Arc;

    const AVATAR_URL: &str = "https://yt3.ggpht.com/avatar.jpg";
    const OTHER_AVATAR_URL: &str = "https://yt3.ggpht.com/other-avatar.jpg";

    #[tokio::test]
    async fn it_should_create_a_channel() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        let channel_creator = ChannelCreator::new(
            channel_repository.clone(),
            resolving(Some(resolved_channel())),
            Arc::new(FakeChannelAvatarRepository::default()),
            event_publisher(&db),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = create(channel_creator, create_request("@somechannel")).await;

        assert_eq!(response, Ok((StatusCode::CREATED, some_channel_response())));
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![channel("@somechannel")]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(1, channel_created("@somechannel"))]
        );
    }

    #[tokio::test]
    async fn it_should_create_a_channel_from_a_youtube_url() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        let channel_creator = ChannelCreator::new(
            channel_repository.clone(),
            resolving(Some(resolved_channel())),
            Arc::new(FakeChannelAvatarRepository::default()),
            event_publisher(&db),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = create(
            channel_creator,
            create_request("https://www.youtube.com/@somechannel"),
        )
        .await;

        assert_eq!(response, Ok((StatusCode::CREATED, some_channel_response())));
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![channel("@somechannel")]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(1, channel_created("@somechannel"))]
        );
    }

    #[tokio::test]
    async fn it_should_store_the_channel_avatar() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let avatar_repository = Arc::new(FakeChannelAvatarRepository::default());
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        let channel_creator = ChannelCreator::new(
            channel_repository.clone(),
            resolving(Some(resolved_channel_with_avatar())),
            avatar_repository.clone(),
            event_publisher(&db),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = create(channel_creator, create_request("@somechannel")).await;

        assert_eq!(
            response,
            Ok((
                StatusCode::CREATED,
                ChannelResponse {
                    avatar_filename: Some("@somechannel.jpg".to_string()),
                    ..some_channel_response()
                }
            ))
        );
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![Channel {
                avatar_filename: Some("@somechannel.jpg".to_string()),
                ..channel("@somechannel")
            }]
        );
        assert_eq!(
            avatar_repository.avatars(),
            vec![("@somechannel.jpg".to_string(), AVATAR_URL.to_string())]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(1, channel_created("@somechannel"))]
        );
    }

    #[tokio::test]
    async fn it_should_skip_avatar_if_channel_has_none() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let avatar_repository = Arc::new(FakeChannelAvatarRepository::default());
        let channel_creator = ChannelCreator::new(
            channel_repository.clone(),
            resolving(Some(resolved_channel())),
            avatar_repository.clone(),
            event_publisher(&db),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = create(channel_creator, create_request("@somechannel")).await;

        assert_eq!(response, Ok((StatusCode::CREATED, some_channel_response())));
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![channel("@somechannel")]
        );
        assert_eq!(avatar_repository.avatars(), vec![]);
    }

    #[tokio::test]
    async fn it_should_skip_avatar_if_unavailable() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let avatar_repository = Arc::new(FakeChannelAvatarRepository::unavailable());
        let channel_creator = ChannelCreator::new(
            channel_repository.clone(),
            resolving(Some(resolved_channel_with_avatar())),
            avatar_repository.clone(),
            event_publisher(&db),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = create(channel_creator, create_request("@somechannel")).await;

        assert_eq!(response, Ok((StatusCode::CREATED, some_channel_response())));
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![channel("@somechannel")]
        );
        assert_eq!(avatar_repository.avatars(), vec![]);
    }

    #[tokio::test]
    async fn it_should_create_the_channel_even_if_avatar_storage_fails() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let avatar_repository = Arc::new(FakeChannelAvatarRepository::failing());
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        let channel_creator = ChannelCreator::new(
            channel_repository.clone(),
            resolving(Some(resolved_channel_with_avatar())),
            avatar_repository.clone(),
            event_publisher(&db),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = create(channel_creator, create_request("@somechannel")).await;

        assert_eq!(response, Ok((StatusCode::CREATED, some_channel_response())));
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![channel("@somechannel")]
        );
        assert_eq!(avatar_repository.avatars(), vec![]);
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(1, channel_created("@somechannel"))]
        );
    }

    #[tokio::test]
    async fn it_should_return_the_existing_channel_if_already_created() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let channel_creator = ChannelCreator::new(
            channel_repository.clone(),
            resolving(Some(resolved_channel())),
            Arc::new(FakeChannelAvatarRepository::default()),
            event_publisher(&db),
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let request = CreateChannelRequest {
            quality: Some("low".to_string()),
            video_limit: Some(5),
            path: Some("different/path".to_string()),
            ..create_request("@somechannel")
        };

        let response = create(channel_creator, request).await;

        assert_eq!(response, Ok((StatusCode::OK, some_channel_response())));
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![channel("@somechannel")]
        );
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_publish_channel_created_only_once() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        let channel_creator = ChannelCreator::new(
            channel_repository.clone(),
            resolving(Some(resolved_channel())),
            Arc::new(FakeChannelAvatarRepository::default()),
            event_publisher(&db),
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let repeated_request = CreateChannelRequest {
            quality: Some("low".to_string()),
            video_limit: Some(5),
            ..create_request("@somechannel")
        };

        create(channel_creator.clone(), create_request("@somechannel"))
            .await
            .unwrap();
        create(channel_creator, repeated_request).await.unwrap();

        assert_eq!(
            channel_repository.list().unwrap(),
            vec![channel("@somechannel")]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(1, channel_created("@somechannel"))]
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_create_if_channel_missing() {
        let request = CreateChannelRequest {
            channel: None,
            ..create_request("@somechannel")
        };

        let response = create(any_channel_creator(), request).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Channel handle or URL must not be empty"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_create_if_invalid_channel_provided() {
        let response = create(any_channel_creator(), create_request("somechannel")).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Channel handle must start with \"@\" (got \"somechannel\")"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_create_if_quality_missing() {
        let request = CreateChannelRequest {
            quality: None,
            ..create_request("@somechannel")
        };

        let response = create(any_channel_creator(), request).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Quality must be one of \"high\", \"mid\", or \"low\" (missing)"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_create_if_video_limit_missing() {
        let request = CreateChannelRequest {
            video_limit: None,
            ..create_request("@somechannel")
        };

        let response = create(any_channel_creator(), request).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Video limit must be a positive integer (missing)"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_create_if_path_missing() {
        let request = CreateChannelRequest {
            path: None,
            ..create_request("@somechannel")
        };

        let response = create(any_channel_creator(), request).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request("Channel path must not be empty"))
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_create_if_channel_not_found_on_youtube() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        let channel_creator = ChannelCreator::new(
            channel_repository.clone(),
            resolving(None),
            Arc::new(FakeChannelAvatarRepository::default()),
            event_publisher(&db),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = create(channel_creator, create_request("@missing")).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "YouTube channel @missing does not exist or is not accessible"
            ))
        );
        assert_eq!(channel_repository.list().unwrap(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_fail_to_create_if_youtube_lookup_fails() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        let channel_creator = ChannelCreator::new(
            channel_repository.clone(),
            Arc::new(FailingYoutubeChannelRepository),
            Arc::new(FakeChannelAvatarRepository::default()),
            event_publisher(&db),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = create(channel_creator, create_request("@somechannel")).await;

        assert_eq!(
            response,
            Err(ApiError::new(
                StatusCode::BAD_GATEWAY,
                "YouTube API request failed"
            ))
        );
        assert_eq!(channel_repository.list().unwrap(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_delete_a_channel() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let avatar_repository = Arc::new(FakeChannelAvatarRepository::default());
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let channel_deleter = ChannelDeleter::new(
            channel_repository.clone(),
            Arc::new(SqliteVideoRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            avatar_repository.clone(),
            event_publisher(&db),
        );

        let response = delete(channel_deleter, "@somechannel").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(channel_repository.list().unwrap(), vec![]);
        assert_eq!(avatar_repository.avatars(), vec![]);
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(1, channel_deleted("@somechannel"))]
        );
    }

    #[tokio::test]
    async fn it_should_delete_only_the_deleted_channel_videos() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        channel_repository
            .insert(&channel("@otherchannel"))
            .unwrap();
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            "@somechannel",
            "yt1",
            0,
        );
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            "@somechannel",
            "yt2",
            1,
        );
        let (other_video, other_channel_video) = save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            "@otherchannel",
            "yt3",
            0,
        );
        let channel_deleter = ChannelDeleter::new(
            channel_repository.clone(),
            video_repository.clone(),
            channel_video_repository.clone(),
            Arc::new(FakeChannelAvatarRepository::default()),
            event_publisher(&db),
        );

        let response = delete(channel_deleter, "@somechannel").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![channel("@otherchannel")]
        );
        assert_eq!(video_repository.list().unwrap(), vec![other_video]);
        assert_eq!(
            channel_video_repository
                .list_for_channel(&handle("@somechannel"))
                .unwrap(),
            vec![]
        );
        assert_eq!(
            channel_video_repository
                .list_for_channel(&handle("@otherchannel"))
                .unwrap(),
            vec![other_channel_video]
        );
    }

    #[tokio::test]
    async fn it_should_delete_the_channel_avatar() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let avatar_repository = Arc::new(FakeChannelAvatarRepository::with_avatars(&[
            ("@somechannel.jpg", AVATAR_URL),
            ("@otherchannel.jpg", OTHER_AVATAR_URL),
        ]));
        channel_repository
            .insert(&Channel {
                avatar_filename: Some("@somechannel.jpg".to_string()),
                ..channel("@somechannel")
            })
            .unwrap();
        let channel_deleter = ChannelDeleter::new(
            channel_repository.clone(),
            Arc::new(SqliteVideoRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            avatar_repository.clone(),
            event_publisher(&db),
        );

        let response = delete(channel_deleter, "@somechannel").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(channel_repository.list().unwrap(), vec![]);
        assert_eq!(
            avatar_repository.avatars(),
            vec![(
                "@otherchannel.jpg".to_string(),
                OTHER_AVATAR_URL.to_string()
            )]
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_delete_a_missing_channel() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let channel_deleter = ChannelDeleter::new(
            channel_repository.clone(),
            Arc::new(SqliteVideoRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            Arc::new(FakeChannelAvatarRepository::default()),
            event_publisher(&db),
        );

        let response = delete(channel_deleter, "@missing").await;

        assert_eq!(
            response,
            Err(ApiError::bad_request("channel @missing not found"))
        );
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![channel("@somechannel")]
        );
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_fail_to_delete_if_invalid_handle_provided() {
        let response = delete(any_channel_deleter(), "noatsign").await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Channel handle must start with \"@\" (got \"noatsign\")"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_list_no_channels() {
        let db = TestDatabase::new();
        let channel_searcher = ChannelSearcher::new(
            Arc::new(SqliteChannelRepository::new(db.connection())),
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            Arc::new(SqliteVideoRepository::new(db.connection())),
        );

        let response = list(channel_searcher).await;

        assert_eq!(response, Ok(vec![]));
    }

    #[tokio::test]
    async fn it_should_list_all_channels() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let channel_searcher = ChannelSearcher::new(
            channel_repository,
            Arc::new(SqliteChannelVideoRepository::new(db.connection())),
            Arc::new(SqliteVideoRepository::new(db.connection())),
        );

        let response = list(channel_searcher).await;

        assert_eq!(response, Ok(vec![some_channel_list_item_response()]));
    }

    #[tokio::test]
    async fn it_should_count_unwatched_downloaded_videos_when_listing_channels() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        for (youtube_id, position) in [("vid_unwatched_1", 0), ("vid_unwatched_2", 1)] {
            save_downloaded_channel_video(
                video_repository.as_ref(),
                channel_video_repository.as_ref(),
                "@somechannel",
                youtube_id,
                position,
            );
        }
        let watched = save_downloaded_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            "@somechannel",
            "vid_watched",
            2,
        );
        video_repository
            .update(&watched.mark_watched(fixed_timestamp()))
            .unwrap();
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            "@somechannel",
            "vid_pending",
            3,
        );
        let channel_searcher = ChannelSearcher::new(
            channel_repository,
            channel_video_repository,
            video_repository,
        );

        let response = list(channel_searcher).await;

        assert_eq!(
            response,
            Ok(vec![ChannelListItemResponse {
                unwatched_count: 2,
                ..some_channel_list_item_response()
            }])
        );
    }

    #[tokio::test]
    async fn it_should_add_new_videos_on_reconcile() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let reconciler = channel_video_reconciler(
            &db,
            channel_repository,
            video_repository.clone(),
            channel_video_repository.clone(),
            vec![listed_video("yt1", "One", 0)],
            task_repository.clone(),
        );

        let response = reconcile(reconciler, "@somechannel").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        let videos = video_repository.list().unwrap();
        let video_id = videos[0].id.clone();
        assert_eq!(
            videos,
            vec![Video {
                id: video_id.clone(),
                ..Video::create(VideoId::new("yt1").unwrap(), "One", fixed_timestamp())
            }]
        );
        assert_eq!(
            channel_video_repository
                .list_for_channel(&handle("@somechannel"))
                .unwrap(),
            vec![ChannelVideo {
                id: 1,
                ..ChannelVideo::create(
                    handle("@somechannel"),
                    video_id.clone(),
                    0,
                    fixed_timestamp()
                )
            }]
        );
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(
                1,
                DomainEvent::VideoAddedToChannel {
                    channel_id: "@somechannel".to_string(),
                    video_id: video_id.as_str().to_string(),
                }
            )]
        );
    }

    #[tokio::test]
    async fn it_should_evict_videos_no_longer_listed_on_reconcile() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let (evicted, _) = save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            "@somechannel",
            "yt_old",
            0,
        );
        let reconciler = channel_video_reconciler(
            &db,
            channel_repository,
            video_repository.clone(),
            channel_video_repository.clone(),
            Vec::new(),
            Arc::new(SqliteTaskRepository::new(
                db.shared_connection(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
        );

        let response = reconcile(reconciler, "@somechannel").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(
            channel_video_repository
                .list_for_channel(&handle("@somechannel"))
                .unwrap(),
            vec![]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(
                1,
                DomainEvent::VideoRemovedFromChannel {
                    channel_id: "@somechannel".to_string(),
                    video_id: evicted.id.as_str().to_string(),
                    title: "Video yt_old".to_string(),
                    filename: None,
                    thumbnail_filename: None,
                    was_downloaded: false,
                }
            )]
        );
    }

    #[tokio::test]
    async fn it_should_be_idempotent_on_repeated_reconciles() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let reconciler = channel_video_reconciler(
            &db,
            channel_repository,
            video_repository.clone(),
            channel_video_repository.clone(),
            vec![listed_video("yt1", "One", 0)],
            Arc::new(SqliteTaskRepository::new(
                db.shared_connection(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
        );

        reconcile(reconciler.clone(), "@somechannel").await.unwrap();
        let videos_after_first_reconcile = video_repository.list().unwrap();
        let channel_videos_after_first_reconcile = channel_video_repository
            .list_for_channel(&handle("@somechannel"))
            .unwrap();
        let events_after_first_reconcile = event_repository.list_eligible().unwrap();
        let response = reconcile(reconciler, "@somechannel").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            video_repository.list().unwrap(),
            videos_after_first_reconcile
        );
        assert_eq!(
            channel_video_repository
                .list_for_channel(&handle("@somechannel"))
                .unwrap(),
            channel_videos_after_first_reconcile
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            events_after_first_reconcile
        );
    }

    #[tokio::test]
    async fn it_should_ignore_reconcile_of_a_missing_channel() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        let reconciler = channel_video_reconciler(
            &db,
            Arc::new(SqliteChannelRepository::new(db.connection())),
            video_repository.clone(),
            channel_video_repository.clone(),
            vec![listed_video("yt1", "One", 0)],
            task_repository.clone(),
        );

        let response = reconcile(reconciler, "@missing").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(
            channel_video_repository
                .list_for_channel(&handle("@missing"))
                .unwrap(),
            vec![]
        );
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_fail_to_reconcile_if_invalid_handle_provided() {
        let response = reconcile(any_channel_video_reconciler(), "noatsign").await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Channel handle must start with \"@\" (got \"noatsign\")"
            ))
        );
    }

    /// A creator for tests whose request is rejected before reaching it. Its
    /// repositories sit on an unmigrated in-memory database, so a request that
    /// wrongly got through would fail loudly instead of passing.
    #[tokio::test]
    async fn it_should_mark_every_downloaded_channel_video_watched() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let playlist_video_repository = SqlitePlaylistVideoRepository::new(db.connection());
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.connection()));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let downloaded = save_downloaded_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            "@somechannel",
            "vid_downloaded",
            0,
        );
        let (pending, _) = save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            "@somechannel",
            "vid_pending",
            1,
        );
        let shared = save_downloaded_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            "@somechannel",
            "vid_shared",
            2,
        );
        let playlist_copy = downloaded_video("vid_shared");
        video_repository.save(&playlist_copy).unwrap();
        playlist_video_repository
            .save(&PlaylistVideo::create(
                PlaylistId::new("PL1").unwrap(),
                playlist_copy.id.clone(),
                fixed_timestamp(),
            ))
            .unwrap();
        let video_watch_state_updater = VideoWatchStateUpdater::new(
            video_repository.clone(),
            channel_repository,
            channel_video_repository,
            Arc::new(FixedClock(watched_timestamp())),
        );

        let response = mark_watched(video_watch_state_updater, "@somechannel").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                downloaded.mark_watched(watched_timestamp()),
                pending,
                shared.mark_watched(watched_timestamp()),
                playlist_copy.mark_watched(watched_timestamp()),
            ]
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_mark_watched_a_missing_channel() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.connection()));
        let video = save_downloaded_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            "@missing",
            "vid1",
            0,
        );
        let video_watch_state_updater = VideoWatchStateUpdater::new(
            video_repository.clone(),
            Arc::new(SqliteChannelRepository::new(db.connection())),
            channel_video_repository,
            Arc::new(FixedClock(watched_timestamp())),
        );

        let response = mark_watched(video_watch_state_updater, "@missing").await;

        assert_eq!(
            response,
            Err(ApiError::bad_request("channel @missing not found"))
        );
        assert_eq!(video_repository.list().unwrap(), vec![video]);
    }

    #[tokio::test]
    async fn it_should_fail_to_mark_watched_if_invalid_handle_provided() {
        let response = mark_watched(any_video_watch_state_updater(), "noatsign").await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Channel handle must start with \"@\" (got \"noatsign\")"
            ))
        );
    }

    fn any_channel_creator() -> ChannelCreator {
        ChannelCreator::new(
            Arc::new(SqliteChannelRepository::new(unused_connection())),
            resolving(Some(resolved_channel())),
            Arc::new(FakeChannelAvatarRepository::default()),
            unused_event_publisher(),
            Arc::new(FixedClock(fixed_timestamp())),
        )
    }

    /// A deleter for tests whose request is rejected before reaching it (see
    /// `any_channel_creator`).
    fn any_channel_deleter() -> ChannelDeleter {
        ChannelDeleter::new(
            Arc::new(SqliteChannelRepository::new(unused_connection())),
            Arc::new(SqliteVideoRepository::new(unused_connection())),
            Arc::new(SqliteChannelVideoRepository::new(unused_connection())),
            Arc::new(FakeChannelAvatarRepository::default()),
            unused_event_publisher(),
        )
    }

    /// Builds a reconciler around the repositories a test seeds and asserts;
    /// the remaining ports (metadata, files, thumbnails) are ones no channel
    /// reconcile test observes. Events go to `db`'s outbox table.
    fn channel_video_reconciler(
        db: &TestDatabase,
        channel_repository: Arc<SqliteChannelRepository>,
        video_repository: Arc<SqliteVideoRepository>,
        channel_video_repository: Arc<SqliteChannelVideoRepository>,
        listed_videos: Vec<ChannelVideoListing>,
        task_repository: Arc<SqliteTaskRepository>,
    ) -> ChannelVideoReconciler {
        let thumbnail_fetcher = Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        ChannelVideoReconciler::new(
            channel_repository,
            video_repository,
            channel_video_repository,
            Arc::new(FakeChannelVideosRepository::with_videos(listed_videos)),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(db.connection())),
            event_publisher(db),
            task_repository,
            Arc::new(FakeVideoFileRepository::default()),
            thumbnail_fetcher,
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        )
    }

    fn any_channel_video_reconciler() -> ChannelVideoReconciler {
        let video_repository = Arc::new(SqliteVideoRepository::new(unused_connection()));
        ChannelVideoReconciler::new(
            Arc::new(SqliteChannelRepository::new(unused_connection())),
            video_repository.clone(),
            Arc::new(SqliteChannelVideoRepository::new(unused_connection())),
            Arc::new(FakeChannelVideosRepository::with_videos(Vec::new())),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(unused_connection())),
            unused_event_publisher(),
            Arc::new(SqliteTaskRepository::new(
                Arc::new(std::sync::Mutex::new(unused_connection())),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(ThumbnailFetcher::new(
                video_repository,
                Arc::new(FakeVideoDownloaderRepository::default()),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        )
    }

    struct FailingYoutubeChannelRepository;

    impl YoutubeChannelRepository for FailingYoutubeChannelRepository {
        fn resolve(&self, _handle: &ChannelHandle) -> anyhow::Result<Option<ResolvedChannel>> {
            anyhow::bail!("YouTube API request failed")
        }
    }

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

    fn unused_event_publisher() -> Arc<SqliteEventPublisher> {
        Arc::new(SqliteEventPublisher::new(
            Arc::new(std::sync::Mutex::new(unused_connection())),
            Arc::new(FixedClock(fixed_timestamp())),
        ))
    }

    fn event_publisher(db: &TestDatabase) -> Arc<SqliteEventPublisher> {
        Arc::new(SqliteEventPublisher::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ))
    }

    /// The outbox row `SqliteEventPublisher` writes for `event`, as read back
    /// before the consumer has dispatched it.
    fn pending_event(id: i64, event: DomainEvent) -> ScheduledEvent {
        ScheduledEvent {
            id,
            event_type: event.event_type().to_string(),
            payload: event.payload().to_string(),
            retries: 0,
            created_at: fixed_timestamp(),
            updated_at: fixed_timestamp(),
            last_error: None,
        }
    }

    fn watched_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_800_000_000, 0).unwrap()
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn handle(value: &str) -> ChannelHandle {
        ChannelHandle::new(value).unwrap()
    }

    fn resolved_channel() -> ResolvedChannel {
        ResolvedChannel {
            youtube_channel_id: "UC123".to_string(),
            title: "Some Channel".to_string(),
            avatar_url: None,
        }
    }

    fn resolved_channel_with_avatar() -> ResolvedChannel {
        ResolvedChannel {
            avatar_url: Some(AVATAR_URL.to_string()),
            ..resolved_channel()
        }
    }

    fn resolving(resolved: Option<ResolvedChannel>) -> Arc<dyn YoutubeChannelRepository> {
        Arc::new(FakeYoutubeChannelRepository { resolved })
    }

    fn channel(channel_handle: &str) -> Channel {
        Channel::create(
            handle(channel_handle),
            "Some Channel",
            "UC123",
            Quality::High,
            VideoLimit::new(10).unwrap(),
            PlaylistPath::new("creators/somechannel").unwrap(),
            None,
            fixed_timestamp(),
        )
    }

    /// Saves a video and its channel membership, returning both as stored
    /// (the channel video with its storage-assigned `id`).
    fn save_channel_video(
        video_repository: &dyn VideoRepository,
        channel_video_repository: &dyn ChannelVideoRepository,
        channel_handle: &str,
        youtube_id: &str,
        position: i64,
    ) -> (Video, ChannelVideo) {
        let video = Video::create(
            VideoId::new(youtube_id).unwrap(),
            format!("Video {youtube_id}"),
            fixed_timestamp(),
        );
        video_repository.save(&video).unwrap();
        channel_video_repository
            .save(&ChannelVideo::create(
                handle(channel_handle),
                video.id.clone(),
                position,
                fixed_timestamp(),
            ))
            .unwrap();
        let channel_video = channel_video_repository
            .find_by_video(&video.id)
            .unwrap()
            .unwrap();
        (video, channel_video)
    }

    /// Saves a downloaded video and its channel membership, returning the
    /// video as stored.
    fn save_downloaded_channel_video(
        video_repository: &dyn VideoRepository,
        channel_video_repository: &dyn ChannelVideoRepository,
        channel_handle: &str,
        youtube_id: &str,
        position: i64,
    ) -> Video {
        let video = downloaded_video(youtube_id);
        video_repository.save(&video).unwrap();
        channel_video_repository
            .save(&ChannelVideo::create(
                handle(channel_handle),
                video.id.clone(),
                position,
                fixed_timestamp(),
            ))
            .unwrap();
        video
    }

    fn downloaded_video(youtube_id: &str) -> Video {
        Video::create(
            VideoId::new(youtube_id).unwrap(),
            format!("Video {youtube_id}"),
            fixed_timestamp(),
        )
        .mark_downloaded(
            Quality::High,
            format!("Video {youtube_id}.mp4"),
            None,
            Some(100),
            fixed_timestamp(),
        )
    }

    fn listed_video(youtube_id: &str, title: &str, position: i64) -> ChannelVideoListing {
        ChannelVideoListing {
            youtube_id: youtube_id.to_string(),
            title: title.to_string(),
            position,
        }
    }

    fn channel_created(channel_handle: &str) -> DomainEvent {
        DomainEvent::ChannelCreated {
            channel_id: channel_handle.to_string(),
        }
    }

    fn channel_deleted(channel_handle: &str) -> DomainEvent {
        DomainEvent::ChannelDeleted {
            channel_id: channel_handle.to_string(),
            path: "creators/somechannel".to_string(),
        }
    }

    fn create_request(channel: &str) -> CreateChannelRequest {
        CreateChannelRequest {
            channel: Some(channel.to_string()),
            quality: Some("high".to_string()),
            video_limit: Some(10),
            path: Some("creators/somechannel".to_string()),
        }
    }

    fn some_channel_response() -> ChannelResponse {
        ChannelResponse {
            id: "@somechannel".to_string(),
            name: "Some Channel".to_string(),
            youtube_channel_id: "UC123".to_string(),
            quality: "high".to_string(),
            video_limit: 10,
            path: "creators/somechannel".to_string(),
            avatar_filename: None,
            created_at: fixed_timestamp(),
        }
    }

    fn some_channel_list_item_response() -> ChannelListItemResponse {
        ChannelListItemResponse {
            id: "@somechannel".to_string(),
            name: "Some Channel".to_string(),
            path: "creators/somechannel".to_string(),
            avatar_filename: None,
            unwatched_count: 0,
        }
    }

    async fn create(
        channel_creator: ChannelCreator,
        request: CreateChannelRequest,
    ) -> Result<(StatusCode, ChannelResponse), ApiError> {
        create_channel(State(channel_creator), Json(request))
            .await
            .map(|(status, Json(channel))| (status, channel))
    }

    async fn delete(
        channel_deleter: ChannelDeleter,
        channel_handle: &str,
    ) -> Result<StatusCode, ApiError> {
        delete_channel(State(channel_deleter), Path(channel_handle.to_string())).await
    }

    async fn list(
        channel_searcher: ChannelSearcher,
    ) -> Result<Vec<ChannelListItemResponse>, ApiError> {
        list_channels(State(channel_searcher))
            .await
            .map(|Json(channels)| channels)
    }

    async fn mark_watched(
        video_watch_state_updater: VideoWatchStateUpdater,
        channel_handle: &str,
    ) -> Result<StatusCode, ApiError> {
        mark_channel_watched(
            State(video_watch_state_updater),
            Path(channel_handle.to_string()),
        )
        .await
    }

    async fn reconcile(
        reconciler: ChannelVideoReconciler,
        channel_handle: &str,
    ) -> Result<StatusCode, ApiError> {
        reconcile_channel(State(reconciler), Path(channel_handle.to_string())).await
    }
}
