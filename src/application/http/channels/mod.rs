pub mod dto;

use super::blocking::run_blocking;
use super::error::{ApiError, move_show_error};
use super::validation::{MISSING_QUALITY, required};
use super::videos::update_watch_state_error;
use crate::domain::channel::{
    ChannelHandle, CreateChannelError, DeleteChannelError, PreviewChannelError, UpdateChannelError,
    VideoLimit,
};
use crate::domain::playlist::PlaylistPath;
use crate::domain::services::{
    ChannelCreator, ChannelCreatorApi, ChannelDeleter, ChannelDeleterApi, ChannelPreviewer,
    ChannelPreviewerApi, ChannelUpdater, ChannelUpdaterApi, ChannelVideoReconciler,
    ChannelVideoReconcilerApi, ChannelViewSearcher, ChannelViewSearcherApi, CreateChannelOutcome,
    ShowMover, ShowMoverApi, VideoWatchStateUpdater, VideoWatchStateUpdaterApi,
};
use crate::domain::shared::{Quality, ReconcileInterval};
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use dto::{
    ChannelListItemResponse, ChannelPreviewResponse, ChannelResponse, CreateChannelRequest,
    MoveChannelRequest, PreviewChannelQuery, UpdateChannelRequest,
};

const MISSING_VIDEO_LIMIT: &str = "Video limit must be a positive integer (missing)";
const MISSING_PATH: &str = "Channel path must not be empty";
const NOTHING_TO_UPDATE: &str =
    "Request must change the quality, the video limit or the reconcile interval";

pub async fn create_channel(
    State(channel_creator): State<ChannelCreator>,
    Json(request): Json<CreateChannelRequest>,
) -> Result<(StatusCode, Json<ChannelResponse>), ApiError> {
    let id = ChannelHandle::from_url_or_handle(request.channel.unwrap_or_default())?;
    let quality = Quality::new(required(request.quality, MISSING_QUALITY)?)?;
    let video_limit = VideoLimit::new(required(request.video_limit, MISSING_VIDEO_LIMIT)?)?;
    let path = PlaylistPath::new(required(request.path, MISSING_PATH)?)?;
    let reconcile_interval = request
        .reconcile_interval_minutes
        .map(ReconcileInterval::new)
        .transpose()?
        .unwrap_or_else(ReconcileInterval::default_interval);

    let outcome = run_blocking(move || {
        channel_creator.create(id, quality, video_limit, reconcile_interval, path)
    })
    .await?;

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

pub async fn preview_channel(
    State(channel_previewer): State<ChannelPreviewer>,
    Query(query): Query<PreviewChannelQuery>,
) -> Result<Json<ChannelPreviewResponse>, ApiError> {
    let id = ChannelHandle::from_url_or_handle(query.channel.unwrap_or_default())?;

    match run_blocking(move || channel_previewer.preview(id)).await? {
        Ok(preview) => Ok(Json(ChannelPreviewResponse::from(preview))),
        Err(e @ PreviewChannelError::YoutubeChannelNotFound(_)) => {
            Err(ApiError::new(StatusCode::NOT_FOUND, e))
        }
        Err(e @ PreviewChannelError::Lookup(_)) => Err(ApiError::new(StatusCode::BAD_GATEWAY, e)),
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

pub async fn update_channel(
    State(channel_updater): State<ChannelUpdater>,
    Path(handle): Path<String>,
    Json(request): Json<UpdateChannelRequest>,
) -> Result<Json<ChannelResponse>, ApiError> {
    let id = ChannelHandle::new(handle)?;
    let quality = request.quality.map(Quality::new).transpose()?;
    let video_limit = request.video_limit.map(VideoLimit::new).transpose()?;
    let reconcile_interval = request
        .reconcile_interval_minutes
        .map(ReconcileInterval::new)
        .transpose()?;
    if quality.is_none() && video_limit.is_none() && reconcile_interval.is_none() {
        return Err(ApiError::bad_request(NOTHING_TO_UPDATE));
    }

    match run_blocking(move || {
        channel_updater.update_settings(id, quality, video_limit, reconcile_interval)
    })
    .await?
    {
        Ok(channel) => Ok(Json(ChannelResponse::from(channel))),
        Err(e @ UpdateChannelError::NotFound(_)) => Err(ApiError::new(StatusCode::NOT_FOUND, e)),
        Err(e @ UpdateChannelError::Repository(_)) => Err(ApiError::internal(e)),
    }
}

pub async fn move_channel(
    State(show_mover): State<ShowMover>,
    Path(handle): Path<String>,
    Json(request): Json<MoveChannelRequest>,
) -> Result<Json<ChannelResponse>, ApiError> {
    let id = ChannelHandle::new(handle)?;
    let path = PlaylistPath::new(required(request.path, MISSING_PATH)?)?;

    run_blocking(move || show_mover.move_channel(id, path))
        .await?
        .map(|channel| Json(ChannelResponse::from(channel)))
        .map_err(move_show_error)
}

pub async fn list_channels(
    State(channel_view_searcher): State<ChannelViewSearcher>,
) -> Result<Json<Vec<ChannelListItemResponse>>, ApiError> {
    let channels = run_blocking(move || channel_view_searcher.search_all())
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
    use crate::domain::channel::{ChannelCreated, ChannelDeleted};
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::channel_video::{VideoAddedToChannel, VideoRemovedFromChannel};
    use crate::domain::event::{DomainEvent, ScheduledEvent};
    use crate::domain::playlist::PlaylistId;
    use crate::domain::playlist_video::PlaylistVideo;
    use crate::domain::services::CategoryVideoCollector;
    use crate::domain::services::ChannelAvatarRestorer;
    use crate::domain::services::InternalVideoReconciler;
    use crate::domain::services::MetadataGenerator;
    use crate::domain::services::ShowFileWriter;
    use crate::domain::services::SubtitleFetcher;
    use crate::domain::services::ThumbnailFetcher;
    use crate::domain::services::VideoRelocator;
    use crate::domain::show::ShowMoved;
    use crate::domain::task::{ScheduledTask, Task, TaskStatus};
    use crate::domain::video::VideoId;
    use crate::domain::video::{PlaybackPosition, Video};
    use crate::domain::video_transcript::{
        CaptionLanguage, CaptionSource, CaptionTrack, TranscriptText, VideoTranscript,
    };
    use crate::infrastructure::repositories::filesystem_channel_avatar_repository::FakeChannelAvatarRepository;
    use crate::infrastructure::repositories::filesystem_show_file_repository::FakeShowFileRepository;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_category_repository::SqliteCategoryRepository;
    use crate::infrastructure::repositories::sqlite_channel_repository::{
        ChannelRepository, SqliteChannelRepository,
    };
    use crate::infrastructure::repositories::sqlite_channel_video_repository::{
        ChannelVideoRepository, SqliteChannelVideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_repository::SqlitePlaylistRepository;
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
    use crate::infrastructure::repositories::sqlite_video_transcript_repository::{
        SqliteVideoTranscriptRepository, VideoTranscriptRepository,
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
    use crate::infrastructure::shared::show_locks::ShowLocks;
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
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
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
    async fn it_should_create_a_channel_with_the_default_interval() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let channel_creator = ChannelCreator::new(
            channel_repository.clone(),
            resolving(Some(resolved_channel())),
            Arc::new(FakeChannelAvatarRepository::default()),
            event_publisher(&db),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let request = CreateChannelRequest {
            reconcile_interval_minutes: None,
            ..create_request("@somechannel")
        };
        let response = create(channel_creator, request).await;

        assert_eq!(
            response,
            Ok((
                StatusCode::CREATED,
                ChannelResponse {
                    reconcile_interval_minutes: 180,
                    ..some_channel_response()
                }
            ))
        );
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![
                channel("@somechannel")
                    .with_reconcile_interval(ReconcileInterval::new(180).unwrap())
            ]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(1, channel_created("@somechannel"))]
        );
    }

    #[tokio::test]
    async fn it_should_create_a_channel_with_its_interval() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let channel_creator = ChannelCreator::new(
            channel_repository.clone(),
            resolving(Some(resolved_channel())),
            Arc::new(FakeChannelAvatarRepository::default()),
            event_publisher(&db),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let request = CreateChannelRequest {
            reconcile_interval_minutes: Some(30),
            ..create_request("@somechannel")
        };
        let response = create(channel_creator, request).await;

        assert_eq!(
            response,
            Ok((
                StatusCode::CREATED,
                ChannelResponse {
                    reconcile_interval_minutes: 30,
                    ..some_channel_response()
                }
            ))
        );
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![
                channel("@somechannel")
                    .with_reconcile_interval(ReconcileInterval::new(30).unwrap())
            ]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(1, channel_created("@somechannel"))]
        );
    }

    #[tokio::test]
    async fn it_should_create_a_channel_from_a_youtube_url() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
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
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let avatar_repository = Arc::new(FakeChannelAvatarRepository::default());
        let event_repository = SqliteEventRepository::new(db.database());
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
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
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
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
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
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let avatar_repository = Arc::new(FakeChannelAvatarRepository::failing());
        let event_repository = SqliteEventRepository::new(db.database());
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
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
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
    async fn it_should_keep_the_interval_of_an_existing_channel_on_duplicate_create() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let existing =
            channel("@somechannel").with_reconcile_interval(ReconcileInterval::new(60).unwrap());
        channel_repository.insert(&existing).unwrap();
        let channel_creator = ChannelCreator::new(
            channel_repository.clone(),
            resolving(Some(resolved_channel())),
            Arc::new(FakeChannelAvatarRepository::default()),
            event_publisher(&db),
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let request = CreateChannelRequest {
            reconcile_interval_minutes: Some(30),
            ..create_request("@somechannel")
        };

        let response = create(channel_creator, request).await;

        assert_eq!(
            response,
            Ok((
                StatusCode::OK,
                ChannelResponse {
                    reconcile_interval_minutes: 60,
                    ..some_channel_response()
                }
            ))
        );
        assert_eq!(channel_repository.list().unwrap(), vec![existing]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_return_the_existing_channel_if_handle_differs_in_case() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let channel_creator = ChannelCreator::new(
            channel_repository.clone(),
            resolving(Some(resolved_channel())),
            Arc::new(FakeChannelAvatarRepository::default()),
            event_publisher(&db),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = create(channel_creator, create_request("@SomeChannel")).await;

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
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
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
    async fn it_should_fail_to_create_if_invalid_interval_provided() {
        let request = CreateChannelRequest {
            reconcile_interval_minutes: Some(0),
            ..create_request("@somechannel")
        };

        let response = create(any_channel_creator(), request).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Reconcile interval must be at least 1 minute (got 0)"
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
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
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
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
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
    async fn it_should_preview_a_channel() {
        let db = TestDatabase::new();
        let channel_repository = SqliteChannelRepository::new(db.database());
        let event_repository = SqliteEventRepository::new(db.database());
        let task_repository =
            SqliteTaskRepository::new(db.database(), Arc::new(FixedClock(fixed_timestamp())));
        let channel_previewer =
            ChannelPreviewer::new(resolving(Some(resolved_channel_with_avatar())));

        let response = preview(channel_previewer, preview_query("@somechannel")).await;

        assert_eq!(response, Ok(channel_preview_response()));
        assert_eq!(channel_repository.list().unwrap(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_preview_a_channel_from_a_youtube_url() {
        let channel_previewer =
            ChannelPreviewer::new(resolving(Some(resolved_channel_with_avatar())));

        let response = preview(
            channel_previewer,
            preview_query("https://www.youtube.com/@somechannel/videos"),
        )
        .await;

        assert_eq!(response, Ok(channel_preview_response()));
    }

    #[tokio::test]
    async fn it_should_preview_a_channel_without_an_avatar() {
        let channel_previewer = ChannelPreviewer::new(resolving(Some(resolved_channel())));

        let response = preview(channel_previewer, preview_query("@somechannel")).await;

        assert_eq!(
            response,
            Ok(ChannelPreviewResponse {
                avatar_url: None,
                ..channel_preview_response()
            })
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_preview_if_invalid_channel_provided() {
        let response = preview(any_channel_previewer(), preview_query("somechannel")).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Channel handle must start with \"@\" (got \"somechannel\")"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_preview_if_channel_missing() {
        let query = PreviewChannelQuery { channel: None };

        let response = preview(any_channel_previewer(), query).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Channel handle or URL must not be empty"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_preview_if_channel_not_found_on_youtube() {
        let channel_previewer = ChannelPreviewer::new(resolving(None));

        let response = preview(channel_previewer, preview_query("@somechannel")).await;

        assert_eq!(
            response,
            Err(ApiError::new(
                StatusCode::NOT_FOUND,
                "YouTube channel @somechannel does not exist or is not accessible"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_preview_if_youtube_lookup_fails() {
        let channel_previewer = ChannelPreviewer::new(Arc::new(FailingYoutubeChannelRepository));

        let response = preview(channel_previewer, preview_query("@somechannel")).await;

        assert_eq!(
            response,
            Err(ApiError::new(
                StatusCode::BAD_GATEWAY,
                "YouTube API request failed"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_delete_a_channel() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let avatar_repository = Arc::new(FakeChannelAvatarRepository::default());
        let event_repository = SqliteEventRepository::new(db.database());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let channel_deleter = ChannelDeleter::new(
            channel_repository.clone(),
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            avatar_repository.clone(),
            event_publisher(&db),
            Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
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
    async fn it_should_remove_the_pending_reconcile_of_a_deleted_channel() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.database());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        task_repository
            .schedule(
                &reconcile_task("@somechannel"),
                fixed_timestamp() + chrono::Duration::hours(3),
            )
            .unwrap();
        task_repository
            .schedule(
                &reconcile_task("@otherchannel"),
                fixed_timestamp() + chrono::Duration::hours(3),
            )
            .unwrap();
        let channel_deleter = ChannelDeleter::new(
            channel_repository.clone(),
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(FakeChannelAvatarRepository::default()),
            event_publisher(&db),
            task_repository.clone(),
        );

        let response = delete(channel_deleter, "@somechannel").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(channel_repository.list().unwrap(), vec![]);
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                2,
                &reconcile_task("@otherchannel"),
                fixed_timestamp() + chrono::Duration::hours(3),
            )]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(1, channel_deleted("@somechannel"))]
        );
    }

    #[tokio::test]
    async fn it_should_remove_every_pending_reconcile_of_a_deleted_channel() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.database());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        task_repository
            .schedule(
                &reconcile_task("@somechannel"),
                fixed_timestamp() + chrono::Duration::hours(1),
            )
            .unwrap();
        task_repository
            .schedule(
                &reconcile_task("@somechannel"),
                fixed_timestamp() + chrono::Duration::hours(3),
            )
            .unwrap();
        let channel_deleter = ChannelDeleter::new(
            channel_repository.clone(),
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(FakeChannelAvatarRepository::default()),
            event_publisher(&db),
            task_repository.clone(),
        );

        let response = delete(channel_deleter, "@somechannel").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(channel_repository.list().unwrap(), vec![]);
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(1, channel_deleted("@somechannel"))]
        );
    }

    #[tokio::test]
    async fn it_should_delete_only_the_deleted_channel_videos() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
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
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            channel_video_repository.clone(),
            Arc::new(FakeChannelAvatarRepository::default()),
            event_publisher(&db),
            Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
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
    async fn it_should_delete_the_transcripts_of_the_deleted_channel_videos() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_transcript_repository =
            Arc::new(SqliteVideoTranscriptRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let (video, _) = save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            "@somechannel",
            "yt1",
            0,
        );
        video_transcript_repository
            .save(&video_transcript(&video))
            .unwrap();
        let channel_deleter = ChannelDeleter::new(
            channel_repository.clone(),
            video_repository.clone(),
            video_transcript_repository.clone(),
            channel_video_repository.clone(),
            Arc::new(FakeChannelAvatarRepository::default()),
            event_publisher(&db),
            Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
        );

        let response = delete(channel_deleter, "@somechannel").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(video_transcript_repository.find(&video.id).unwrap(), None);
    }

    #[tokio::test]
    async fn it_should_delete_the_channel_avatar() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
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
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            avatar_repository.clone(),
            event_publisher(&db),
            Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
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
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let channel_deleter = ChannelDeleter::new(
            channel_repository.clone(),
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(FakeChannelAvatarRepository::default()),
            event_publisher(&db),
            Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
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
    async fn it_should_update_a_channels_quality() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let channel_updater = ChannelUpdater::new(
            channel_repository.clone(),
            Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let request = UpdateChannelRequest {
            quality: Some("low".to_string()),
            ..update_request()
        };
        let response = update(channel_updater, "@somechannel", request).await;

        assert_eq!(
            response,
            Ok(ChannelResponse {
                quality: "low".to_string(),
                ..some_channel_response()
            })
        );
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![channel("@somechannel").with_quality(Quality::Low)]
        );
    }

    #[tokio::test]
    async fn it_should_update_a_channels_video_limit() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let channel_updater = ChannelUpdater::new(
            channel_repository.clone(),
            Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let request = UpdateChannelRequest {
            video_limit: Some(20),
            ..update_request()
        };
        let response = update(channel_updater, "@somechannel", request).await;

        assert_eq!(
            response,
            Ok(ChannelResponse {
                video_limit: 20,
                ..some_channel_response()
            })
        );
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![channel("@somechannel").with_video_limit(VideoLimit::new(20).unwrap())]
        );
    }

    #[tokio::test]
    async fn it_should_update_a_channels_interval() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let channel_updater = ChannelUpdater::new(
            channel_repository.clone(),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let request = UpdateChannelRequest {
            reconcile_interval_minutes: Some(60),
            ..update_request()
        };
        let response = update(channel_updater, "@somechannel", request).await;

        assert_eq!(
            response,
            Ok(ChannelResponse {
                reconcile_interval_minutes: 60,
                ..some_channel_response()
            })
        );
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![
                channel("@somechannel")
                    .with_reconcile_interval(ReconcileInterval::new(60).unwrap())
            ]
        );
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_bring_the_next_reconcile_forward_if_interval_shortened() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        task_repository
            .schedule(
                &reconcile_task("@somechannel"),
                fixed_timestamp() + chrono::Duration::hours(20),
            )
            .unwrap();
        let channel_updater = ChannelUpdater::new(
            channel_repository.clone(),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let request = UpdateChannelRequest {
            reconcile_interval_minutes: Some(60),
            ..update_request()
        };
        let response = update(channel_updater, "@somechannel", request).await;

        assert_eq!(
            response,
            Ok(ChannelResponse {
                reconcile_interval_minutes: 60,
                ..some_channel_response()
            })
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &reconcile_task("@somechannel"),
                fixed_timestamp() + chrono::Duration::minutes(60),
            )]
        );
    }

    #[tokio::test]
    async fn it_should_stamp_the_brought_forward_reconcile_with_the_update_time() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp() - chrono::Duration::hours(1))),
        ));
        task_repository
            .schedule(
                &reconcile_task("@somechannel"),
                fixed_timestamp() + chrono::Duration::hours(20),
            )
            .unwrap();
        let channel_updater = ChannelUpdater::new(
            channel_repository.clone(),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let request = UpdateChannelRequest {
            reconcile_interval_minutes: Some(60),
            ..update_request()
        };
        let response = update(channel_updater, "@somechannel", request).await;

        assert_eq!(
            response,
            Ok(ChannelResponse {
                reconcile_interval_minutes: 60,
                ..some_channel_response()
            })
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![ScheduledTask {
                created_at: fixed_timestamp() - chrono::Duration::hours(1),
                ..pending_task(
                    1,
                    &reconcile_task("@somechannel"),
                    fixed_timestamp() + chrono::Duration::minutes(60),
                )
            }]
        );
    }

    #[tokio::test]
    async fn it_should_keep_the_next_reconcile_if_interval_lengthened() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        task_repository
            .schedule(
                &reconcile_task("@somechannel"),
                fixed_timestamp() + chrono::Duration::minutes(30),
            )
            .unwrap();
        let channel_updater = ChannelUpdater::new(
            channel_repository.clone(),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let request = UpdateChannelRequest {
            reconcile_interval_minutes: Some(1440),
            ..update_request()
        };
        let response = update(channel_updater, "@somechannel", request).await;

        assert_eq!(
            response,
            Ok(ChannelResponse {
                reconcile_interval_minutes: 1440,
                ..some_channel_response()
            })
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(
                1,
                &reconcile_task("@somechannel"),
                fixed_timestamp() + chrono::Duration::minutes(30),
            )]
        );
    }

    #[tokio::test]
    async fn it_should_update_both_settings_of_a_channel() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let channel_updater = ChannelUpdater::new(
            channel_repository.clone(),
            Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let request = UpdateChannelRequest {
            quality: Some("mid".to_string()),
            video_limit: Some(3),
            ..update_request()
        };
        let response = update(channel_updater, "@somechannel", request).await;

        assert_eq!(
            response,
            Ok(ChannelResponse {
                quality: "mid".to_string(),
                video_limit: 3,
                ..some_channel_response()
            })
        );
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![
                channel("@somechannel")
                    .with_quality(Quality::Mid)
                    .with_video_limit(VideoLimit::new(3).unwrap())
            ]
        );
    }

    #[tokio::test]
    async fn it_should_leave_a_channel_unchanged_if_settings_already_set() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let channel_updater = ChannelUpdater::new(
            channel_repository.clone(),
            Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let request = UpdateChannelRequest {
            quality: Some("high".to_string()),
            video_limit: Some(10),
            ..update_request()
        };
        let response = update(channel_updater, "@somechannel", request).await;

        assert_eq!(response, Ok(some_channel_response()));
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![channel("@somechannel")]
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_update_an_unknown_channel() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let channel_updater = ChannelUpdater::new(
            channel_repository.clone(),
            Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let request = UpdateChannelRequest {
            quality: Some("low".to_string()),
            ..update_request()
        };
        let response = update(channel_updater, "@missing", request).await;

        assert_eq!(
            response,
            Err(ApiError::new(
                StatusCode::NOT_FOUND,
                "channel @missing not found"
            ))
        );
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![channel("@somechannel")]
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_update_if_nothing_to_update() {
        let response = update(any_channel_updater(), "@somechannel", update_request()).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Request must change the quality, the video limit or the reconcile interval"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_update_if_invalid_quality_provided() {
        let request = UpdateChannelRequest {
            quality: Some("ultra".to_string()),
            ..update_request()
        };
        let response = update(any_channel_updater(), "@somechannel", request).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Quality must be one of \"high\", \"mid\", or \"low\" (got \"ultra\")"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_update_if_invalid_video_limit_provided() {
        let request = UpdateChannelRequest {
            video_limit: Some(1001),
            ..update_request()
        };
        let response = update(any_channel_updater(), "@somechannel", request).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Video limit must be between 1 and 1000 (got 1001)"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_update_if_invalid_interval_provided() {
        let request = UpdateChannelRequest {
            reconcile_interval_minutes: Some(0),
            ..update_request()
        };
        let response = update(any_channel_updater(), "@somechannel", request).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Reconcile interval must be at least 1 minute (got 0)"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_update_if_invalid_handle_provided() {
        let request = UpdateChannelRequest {
            quality: Some("low".to_string()),
            ..update_request()
        };
        let response = update(any_channel_updater(), "noatsign", request).await;

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
        let channel_view_searcher = ChannelViewSearcher::new(
            Arc::new(SqliteChannelRepository::new(db.database())),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(SqliteEventRepository::new(db.database())),
            "/videos",
        );

        let response = list(channel_view_searcher).await;

        assert_eq!(response, Ok(vec![]));
    }

    #[tokio::test]
    async fn it_should_list_a_channel_with_a_pending_video_as_work_in_progress() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            "@somechannel",
            "vid_pending",
            0,
        );
        let channel_view_searcher = ChannelViewSearcher::new(
            channel_repository,
            channel_video_repository,
            video_repository,
            Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(SqliteEventRepository::new(db.database())),
            "/videos",
        );

        let response = list(channel_view_searcher).await;

        assert_eq!(
            response,
            Ok(vec![ChannelListItemResponse {
                work_in_progress: true,
                ..some_channel_list_item_response()
            }])
        );
    }

    #[tokio::test]
    async fn it_should_list_all_channels() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let channel_view_searcher = ChannelViewSearcher::new(
            channel_repository,
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(SqliteEventRepository::new(db.database())),
            "/videos",
        );

        let response = list(channel_view_searcher).await;

        assert_eq!(response, Ok(vec![some_channel_list_item_response()]));
    }

    #[tokio::test]
    async fn it_should_list_channels_with_their_quality_and_video_limit() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        channel_repository
            .insert(&Channel {
                name: "A".to_string(),
                ..channel("@a")
            })
            .unwrap();
        channel_repository
            .insert(&Channel {
                name: "B".to_string(),
                ..channel("@b")
                    .with_quality(Quality::Low)
                    .with_video_limit(VideoLimit::new(25).unwrap())
            })
            .unwrap();
        let channel_view_searcher = ChannelViewSearcher::new(
            channel_repository,
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(SqliteEventRepository::new(db.database())),
            "/videos",
        );

        let response = list(channel_view_searcher).await;

        assert_eq!(
            response,
            Ok(vec![
                ChannelListItemResponse {
                    id: "@a".to_string(),
                    name: "A".to_string(),
                    ..some_channel_list_item_response()
                },
                ChannelListItemResponse {
                    id: "@b".to_string(),
                    name: "B".to_string(),
                    quality: "low".to_string(),
                    video_limit: 25,
                    ..some_channel_list_item_response()
                },
            ])
        );
    }

    #[tokio::test]
    async fn it_should_list_channels_with_their_interval() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        channel_repository
            .insert(
                &channel("@somechannel")
                    .with_reconcile_interval(ReconcileInterval::new(30).unwrap()),
            )
            .unwrap();
        let channel_view_searcher = ChannelViewSearcher::new(
            channel_repository,
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(SqliteEventRepository::new(db.database())),
            "/videos",
        );

        let response = list(channel_view_searcher).await;

        assert_eq!(
            response,
            Ok(vec![ChannelListItemResponse {
                reconcile_interval_minutes: 30,
                ..some_channel_list_item_response()
            }])
        );
    }

    #[tokio::test]
    async fn it_should_list_channels_sorted_by_name_ignoring_case() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        for (channel_handle, name) in [
            ("@veritasium", "veritasium"),
            ("@kurzgesagt", "Kurzgesagt"),
            ("@3blue1brown", "3Blue1Brown"),
        ] {
            channel_repository
                .insert(&Channel {
                    name: name.to_string(),
                    ..channel(channel_handle)
                })
                .unwrap();
        }
        let channel_view_searcher = ChannelViewSearcher::new(
            channel_repository,
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(SqliteEventRepository::new(db.database())),
            "/videos",
        );

        let response = list(channel_view_searcher).await;

        assert_eq!(
            response,
            Ok(vec![
                ChannelListItemResponse {
                    id: "@3blue1brown".to_string(),
                    name: "3Blue1Brown".to_string(),
                    ..some_channel_list_item_response()
                },
                ChannelListItemResponse {
                    id: "@kurzgesagt".to_string(),
                    name: "Kurzgesagt".to_string(),
                    ..some_channel_list_item_response()
                },
                ChannelListItemResponse {
                    id: "@veritasium".to_string(),
                    name: "veritasium".to_string(),
                    ..some_channel_list_item_response()
                },
            ])
        );
    }

    #[tokio::test]
    async fn it_should_count_unwatched_downloaded_videos_when_listing_channels() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
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
        let channel_view_searcher = ChannelViewSearcher::new(
            channel_repository,
            channel_video_repository,
            video_repository,
            Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(SqliteEventRepository::new(db.database())),
            "/videos",
        );

        let response = list(channel_view_searcher).await;

        assert_eq!(
            response,
            Ok(vec![ChannelListItemResponse {
                unwatched_count: 2,
                work_in_progress: true,
                ..some_channel_list_item_response()
            }])
        );
    }

    #[tokio::test]
    async fn it_should_add_new_videos_on_reconcile() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.database());
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
                DomainEvent::VideoAddedToChannel(VideoAddedToChannel {
                    channel_id: "@somechannel".to_string(),
                    video_id: video_id.as_str().to_string(),
                })
            )]
        );
    }

    #[tokio::test]
    async fn it_should_evict_videos_no_longer_listed_on_reconcile() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
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
                db.database(),
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
                DomainEvent::VideoRemovedFromChannel(VideoRemovedFromChannel {
                    channel_id: "@somechannel".to_string(),
                    video_id: evicted.id.as_str().to_string(),
                    title: "Video yt_old".to_string(),
                    filename: None,
                    thumbnail_filename: None,
                    subtitle_filename: None,
                    was_downloaded: false,
                })
            )]
        );
    }

    #[tokio::test]
    async fn it_should_be_idempotent_on_repeated_reconciles() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let reconciler = channel_video_reconciler(
            &db,
            channel_repository,
            video_repository.clone(),
            channel_video_repository.clone(),
            vec![listed_video("yt1", "One", 0)],
            Arc::new(SqliteTaskRepository::new(
                db.database(),
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
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_repository = SqliteEventRepository::new(db.database());
        let reconciler = channel_video_reconciler(
            &db,
            Arc::new(SqliteChannelRepository::new(db.database())),
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
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let playlist_video_repository = SqlitePlaylistVideoRepository::new(db.database());
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
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
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let video_watch_state_updater = VideoWatchStateUpdater::new(
            video_repository.clone(),
            channel_repository,
            channel_video_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            Arc::new(FixedClock(watched_timestamp())),
            Arc::new(SqliteCategoryRepository::new(db.database())),
            category_video_collector(&db),
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
    async fn it_should_not_change_the_last_played_time_when_marking_a_channel_watched() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let played_at = DateTime::<Utc>::from_timestamp(1_750_000_000, 0).unwrap();
        let video = Video {
            playback_position: PlaybackPosition::new(40).unwrap(),
            last_played_at: Some(played_at),
            ..downloaded_video("vid1")
        };
        video_repository.save(&video).unwrap();
        channel_video_repository
            .save(&ChannelVideo::create(
                handle("@somechannel"),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let video_watch_state_updater = VideoWatchStateUpdater::new(
            video_repository.clone(),
            channel_repository,
            channel_video_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            Arc::new(FixedClock(watched_timestamp())),
            Arc::new(SqliteCategoryRepository::new(db.database())),
            category_video_collector(&db),
        );

        let response = mark_watched(video_watch_state_updater, "@somechannel").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                watched_at: Some(watched_timestamp()),
                playback_position: PlaybackPosition::start(),
                last_played_at: Some(played_at),
                ..video
            }]
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_mark_watched_a_missing_channel() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let video = save_downloaded_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            "@missing",
            "vid1",
            0,
        );
        let video_watch_state_updater = VideoWatchStateUpdater::new(
            video_repository.clone(),
            Arc::new(SqliteChannelRepository::new(db.database())),
            channel_video_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            Arc::new(FixedClock(watched_timestamp())),
            Arc::new(SqliteCategoryRepository::new(db.database())),
            category_video_collector(&db),
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

    #[tokio::test]
    async fn it_should_move_a_channel() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository =
            Arc::new(FakeVideoFileRepository::default().with_dir("/videos/creators/somechannel"));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        let show_mover = show_mover(&db, video_file_repository.clone());

        let response = move_(
            show_mover,
            "@somechannel",
            move_request("science/somechannel"),
        )
        .await;

        assert_eq!(
            response,
            Ok(ChannelResponse {
                path: "science/somechannel".to_string(),
                ..some_channel_response()
            })
        );
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![
                channel("@somechannel")
                    .with_path(PlaylistPath::new("science/somechannel").unwrap())
            ]
        );
        assert_eq!(
            video_file_repository.renamed_dirs(),
            vec![(
                std::path::PathBuf::from("/videos/creators/somechannel"),
                std::path::PathBuf::from("/videos/science/somechannel")
            )]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(
                1,
                DomainEvent::ShowMoved(ShowMoved {
                    from: "/videos/creators/somechannel".to_string(),
                    to: "/videos/science/somechannel".to_string(),
                })
            )]
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_move_a_missing_channel() {
        let db = TestDatabase::new();
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository =
            Arc::new(FakeVideoFileRepository::default().with_dir("/videos/creators/somechannel"));
        let show_mover = show_mover(&db, video_file_repository.clone());

        let response = move_(show_mover, "@missing", move_request("science/somechannel")).await;

        assert_eq!(
            response,
            Err(ApiError::new(
                StatusCode::NOT_FOUND,
                "playlist or channel not found"
            ))
        );
        assert_eq!(video_file_repository.renamed_dirs(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_fail_to_move_if_a_channel_video_is_pending() {
        let db = TestDatabase::new();
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = SqliteVideoRepository::new(db.database());
        let channel_video_repository = SqliteChannelVideoRepository::new(db.database());
        let event_repository = SqliteEventRepository::new(db.database());
        let video_file_repository =
            Arc::new(FakeVideoFileRepository::default().with_dir("/videos/creators/somechannel"));
        channel_repository.insert(&channel("@somechannel")).unwrap();
        save_downloaded_channel_video(
            &video_repository,
            &channel_video_repository,
            "@somechannel",
            "vid_downloaded",
            0,
        );
        save_channel_video(
            &video_repository,
            &channel_video_repository,
            "@somechannel",
            "vid_pending",
            1,
        );
        let show_mover = show_mover(&db, video_file_repository.clone());

        let response = move_(
            show_mover,
            "@somechannel",
            move_request("science/somechannel"),
        )
        .await;

        assert_eq!(
            response,
            Err(ApiError::new(
                StatusCode::CONFLICT,
                "playlist or channel still has work in progress, try again once its downloads finish"
            ))
        );
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![channel("@somechannel")]
        );
        assert_eq!(video_file_repository.renamed_dirs(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
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

    /// A previewer for tests whose request is rejected before reaching it. Its
    /// lookup fails, so a request that wrongly reaches YouTube fails loudly.
    fn any_channel_previewer() -> ChannelPreviewer {
        ChannelPreviewer::new(Arc::new(FailingYoutubeChannelRepository))
    }

    /// A deleter for tests whose request is rejected before reaching it (see
    /// `any_channel_creator`).
    fn any_channel_deleter() -> ChannelDeleter {
        ChannelDeleter::new(
            Arc::new(SqliteChannelRepository::new(unused_connection())),
            Arc::new(SqliteVideoRepository::new(unused_connection())),
            Arc::new(SqliteVideoTranscriptRepository::new(unused_connection())),
            Arc::new(SqliteChannelVideoRepository::new(unused_connection())),
            Arc::new(FakeChannelAvatarRepository::default()),
            unused_event_publisher(),
            Arc::new(SqliteTaskRepository::new(
                unused_connection(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
        )
    }

    fn any_channel_updater() -> ChannelUpdater {
        ChannelUpdater::new(
            Arc::new(SqliteChannelRepository::new(unused_connection())),
            Arc::new(SqliteTaskRepository::new(
                unused_connection(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
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
        let subtitle_fetcher = Arc::new(SubtitleFetcher::new(
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            Arc::new(FakeVideoDownloaderRepository::default()),
            Arc::new(FakeVideoFileRepository::default()),
            task_repository.clone(),
            Arc::new(SqliteEventPublisher::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
        ));
        let thumbnail_fetcher = Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::default()),
            Arc::new(FakeVideoFileRepository::default()),
            task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            "/videos",
        ));
        ChannelVideoReconciler::new(
            channel_repository.clone(),
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(db.database())),
            channel_video_repository,
            Arc::new(FakeChannelVideosRepository::with_videos(listed_videos)),
            event_publisher(db),
            task_repository.clone(),
            Arc::new(InternalVideoReconciler::new(
                video_repository,
                Arc::new(MetadataGenerator::new(
                    Arc::new(FakeYoutubeMetadataRepository::default()),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                task_repository,
                Arc::new(FakeVideoFileRepository::default()),
                subtitle_fetcher,
                thumbnail_fetcher,
                Arc::new(VideoRelocator::new(
                    Arc::new(SqliteVideoRepository::new(db.database())),
                    Arc::new(FakeVideoFileRepository::default()),
                    Arc::new(SqliteVideoMetadataRepository::new(db.database())),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(ShowFileWriter::new(
                Arc::new(FakeShowFileRepository::default()),
                Arc::new(FakeChannelAvatarRepository::default()),
                "/videos",
            )),
            Arc::new(ChannelAvatarRestorer::new(
                channel_repository,
                Arc::new(FakeYoutubeChannelRepository { resolved: None }),
                Arc::new(FakeChannelAvatarRepository::default()),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(ShowLocks::default()),
        )
    }

    fn any_channel_video_reconciler() -> ChannelVideoReconciler {
        let video_repository = Arc::new(SqliteVideoRepository::new(unused_connection()));
        let task_repository = Arc::new(SqliteTaskRepository::new(
            unused_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        ChannelVideoReconciler::new(
            Arc::new(SqliteChannelRepository::new(unused_connection())),
            video_repository.clone(),
            Arc::new(SqliteVideoTranscriptRepository::new(unused_connection())),
            Arc::new(SqliteChannelVideoRepository::new(unused_connection())),
            Arc::new(FakeChannelVideosRepository::with_videos(Vec::new())),
            unused_event_publisher(),
            task_repository.clone(),
            Arc::new(InternalVideoReconciler::new(
                video_repository.clone(),
                Arc::new(MetadataGenerator::new(
                    Arc::new(FakeYoutubeMetadataRepository::default()),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                Arc::new(SqliteVideoMetadataRepository::new(unused_connection())),
                task_repository.clone(),
                Arc::new(FakeVideoFileRepository::default()),
                Arc::new(SubtitleFetcher::new(
                    video_repository.clone(),
                    Arc::new(SqliteVideoTranscriptRepository::new(unused_connection())),
                    Arc::new(FakeVideoDownloaderRepository::default()),
                    Arc::new(FakeVideoFileRepository::default()),
                    task_repository.clone(),
                    Arc::new(SqliteEventPublisher::new(
                        unused_connection(),
                        Arc::new(FixedClock(fixed_timestamp())),
                    )),
                    Arc::new(FixedClock(fixed_timestamp())),
                    "/videos",
                )),
                Arc::new(ThumbnailFetcher::new(
                    video_repository,
                    Arc::new(FakeVideoDownloaderRepository::default()),
                    Arc::new(FakeVideoFileRepository::default()),
                    task_repository.clone(),
                    Arc::new(FixedClock(fixed_timestamp())),
                    "/videos",
                )),
                Arc::new(VideoRelocator::new(
                    Arc::new(SqliteVideoRepository::new(unused_connection())),
                    Arc::new(FakeVideoFileRepository::default()),
                    Arc::new(SqliteVideoMetadataRepository::new(unused_connection())),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                Arc::new(FixedClock(fixed_timestamp())),
                "/videos",
            )),
            Arc::new(ShowFileWriter::new(
                Arc::new(FakeShowFileRepository::default()),
                Arc::new(FakeChannelAvatarRepository::default()),
                "/videos",
            )),
            Arc::new(ChannelAvatarRestorer::new(
                Arc::new(SqliteChannelRepository::new(unused_connection())),
                Arc::new(FakeYoutubeChannelRepository { resolved: None }),
                Arc::new(FakeChannelAvatarRepository::default()),
            )),
            Arc::new(FixedClock(fixed_timestamp())),
            Arc::new(ShowLocks::default()),
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
            Arc::new(SqlitePlaylistRepository::new(unused_connection())),
            Arc::new(SqlitePlaylistVideoRepository::new(unused_connection())),
            Arc::new(FixedClock(watched_timestamp())),
            Arc::new(SqliteCategoryRepository::new(unused_connection())),
            any_category_video_collector(),
        )
    }

    /// The collector the updater reads category videos through, on `db` like
    /// the repositories each test seeds.
    fn category_video_collector(db: &TestDatabase) -> CategoryVideoCollector {
        CategoryVideoCollector::new(
            Arc::new(SqliteChannelRepository::new(db.database())),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            Arc::new(SqliteVideoRepository::new(db.database())),
        )
    }

    /// A collector on an unmigrated in-memory database.
    fn any_category_video_collector() -> CategoryVideoCollector {
        CategoryVideoCollector::new(
            Arc::new(SqliteChannelRepository::new(unused_connection())),
            Arc::new(SqliteChannelVideoRepository::new(unused_connection())),
            Arc::new(SqlitePlaylistRepository::new(unused_connection())),
            Arc::new(SqlitePlaylistVideoRepository::new(unused_connection())),
            Arc::new(SqliteVideoRepository::new(unused_connection())),
        )
    }

    fn unused_connection() -> crate::infrastructure::shared::sqlite_connection::Database {
        crate::infrastructure::shared::sqlite_connection::Database::single(
            Connection::open_in_memory().unwrap(),
        )
    }

    fn unused_event_publisher() -> Arc<SqliteEventPublisher> {
        Arc::new(SqliteEventPublisher::new(
            unused_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ))
    }

    fn event_publisher(db: &TestDatabase) -> Arc<SqliteEventPublisher> {
        Arc::new(SqliteEventPublisher::new(
            db.database(),
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

    fn reconcile_task(channel_handle: &str) -> Task {
        Task::ReconcileChannel {
            channel_id: channel_handle.to_string(),
        }
    }

    fn pending_task(id: i64, task: &Task, run_at: DateTime<Utc>) -> ScheduledTask {
        ScheduledTask {
            id,
            task_type: task.task_type().to_string(),
            payload: task.payload().to_string(),
            status: TaskStatus::Pending,
            retries: 0,
            run_at,
            created_at: fixed_timestamp(),
            updated_at: fixed_timestamp(),
            last_error: None,
        }
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
            ReconcileInterval::default_interval(),
            PlaylistPath::new("creators/somechannel").unwrap(),
            None,
            fixed_timestamp(),
        )
    }

    fn video_transcript(video: &Video) -> VideoTranscript {
        VideoTranscript::create(
            video.id.clone(),
            &CaptionTrack {
                key: "en".to_string(),
                language: CaptionLanguage::new("en").unwrap(),
                source: CaptionSource::Manual,
            },
            TranscriptText::new("Hello there"),
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
        DomainEvent::ChannelCreated(ChannelCreated {
            channel_id: channel_handle.to_string(),
        })
    }

    fn channel_deleted(channel_handle: &str) -> DomainEvent {
        DomainEvent::ChannelDeleted(ChannelDeleted {
            channel_id: channel_handle.to_string(),
            name: "Some Channel".to_string(),
            path: "creators/somechannel".to_string(),
        })
    }

    fn create_request(channel: &str) -> CreateChannelRequest {
        CreateChannelRequest {
            channel: Some(channel.to_string()),
            quality: Some("high".to_string()),
            video_limit: Some(10),
            path: Some("creators/somechannel".to_string()),
            reconcile_interval_minutes: None,
        }
    }

    fn preview_query(channel: &str) -> PreviewChannelQuery {
        PreviewChannelQuery {
            channel: Some(channel.to_string()),
        }
    }

    fn channel_preview_response() -> ChannelPreviewResponse {
        ChannelPreviewResponse {
            id: "@somechannel".to_string(),
            title: "Some Channel".to_string(),
            avatar_url: Some(AVATAR_URL.to_string()),
        }
    }

    fn update_request() -> UpdateChannelRequest {
        UpdateChannelRequest {
            quality: None,
            video_limit: None,
            reconcile_interval_minutes: None,
        }
    }

    fn some_channel_response() -> ChannelResponse {
        ChannelResponse {
            id: "@somechannel".to_string(),
            name: "Some Channel".to_string(),
            youtube_channel_id: "UC123".to_string(),
            quality: "high".to_string(),
            video_limit: 10,
            reconcile_interval_minutes: 180,
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
            quality: "high".to_string(),
            video_limit: 10,
            reconcile_interval_minutes: 180,
            avatar_filename: None,
            unwatched_count: 0,
            work_in_progress: false,
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

    async fn preview(
        channel_previewer: ChannelPreviewer,
        query: PreviewChannelQuery,
    ) -> Result<ChannelPreviewResponse, ApiError> {
        preview_channel(State(channel_previewer), Query(query))
            .await
            .map(|Json(preview)| preview)
    }

    async fn delete(
        channel_deleter: ChannelDeleter,
        channel_handle: &str,
    ) -> Result<StatusCode, ApiError> {
        delete_channel(State(channel_deleter), Path(channel_handle.to_string())).await
    }

    async fn update(
        channel_updater: ChannelUpdater,
        channel_handle: &str,
        request: UpdateChannelRequest,
    ) -> Result<ChannelResponse, ApiError> {
        update_channel(
            State(channel_updater),
            Path(channel_handle.to_string()),
            Json(request),
        )
        .await
        .map(|Json(channel)| channel)
    }

    async fn list(
        channel_view_searcher: ChannelViewSearcher,
    ) -> Result<Vec<ChannelListItemResponse>, ApiError> {
        list_channels(State(channel_view_searcher))
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

    async fn move_(
        show_mover: ShowMover,
        channel_handle: &str,
        request: MoveChannelRequest,
    ) -> Result<ChannelResponse, ApiError> {
        move_channel(
            State(show_mover),
            Path(channel_handle.to_string()),
            Json(request),
        )
        .await
        .map(|Json(channel)| channel)
    }

    /// Builds a mover on `db` (where each test seeds and asserts sources,
    /// videos, tasks and events) under the videos root `/videos`, with its
    /// own show locks, nobody else holds.
    fn show_mover(
        db: &TestDatabase,
        video_file_repository: Arc<FakeVideoFileRepository>,
    ) -> ShowMover {
        ShowMover::new(
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            Arc::new(SqliteChannelRepository::new(db.database())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(SqliteTaskRepository::new(
                db.database(),
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(SqliteEventRepository::new(db.database())),
            video_file_repository,
            event_publisher(db),
            Arc::new(ShowLocks::default()),
            "/videos",
        )
    }

    fn move_request(path: &str) -> MoveChannelRequest {
        MoveChannelRequest {
            path: Some(path.to_string()),
        }
    }

    async fn reconcile(
        reconciler: ChannelVideoReconciler,
        channel_handle: &str,
    ) -> Result<StatusCode, ApiError> {
        reconcile_channel(State(reconciler), Path(channel_handle.to_string())).await
    }
}
