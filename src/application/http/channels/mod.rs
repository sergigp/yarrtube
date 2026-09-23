pub mod dto;

use super::blocking::run_blocking;
use super::error::ApiError;
use super::validation::{MISSING_QUALITY, required};
use crate::domain::channel::{
    ChannelHandle, ChannelService, CreateChannelError, CreateChannelOutcome, DeleteChannelError,
    VideoLimit,
};
use crate::domain::playlist::PlaylistPath;
use crate::domain::services::ChannelVideoReconciler;
use crate::domain::shared::Quality;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use dto::{ChannelResponse, CreateChannelRequest};

const MISSING_VIDEO_LIMIT: &str = "Video limit must be a positive integer (missing)";
const MISSING_PATH: &str = "Channel path must not be empty";

pub async fn create_channel(
    State(channel_service): State<ChannelService>,
    Json(request): Json<CreateChannelRequest>,
) -> Result<(StatusCode, Json<ChannelResponse>), ApiError> {
    let id = ChannelHandle::from_url_or_handle(request.channel.unwrap_or_default())
        .map_err(ApiError::bad_request)?;
    let quality =
        Quality::new(required(request.quality, MISSING_QUALITY)?).map_err(ApiError::bad_request)?;
    let video_limit = VideoLimit::new(required(request.video_limit, MISSING_VIDEO_LIMIT)?)
        .map_err(ApiError::bad_request)?;
    let path =
        PlaylistPath::new(required(request.path, MISSING_PATH)?).map_err(ApiError::bad_request)?;

    let outcome =
        run_blocking(move || channel_service.create_channel(id, quality, video_limit, path))
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

pub async fn delete_channel(
    State(channel_service): State<ChannelService>,
    Path(handle): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = ChannelHandle::new(handle).map_err(ApiError::bad_request)?;

    match run_blocking(move || channel_service.delete_channel(id)).await? {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(e @ DeleteChannelError::NotFound(_)) => Err(ApiError::bad_request(e)),
        Err(e @ DeleteChannelError::Repository(_)) => Err(ApiError::internal(e)),
    }
}

pub async fn list_channels(
    State(channel_service): State<ChannelService>,
) -> Result<Json<Vec<ChannelResponse>>, ApiError> {
    let channels = run_blocking(move || channel_service.list_channels())
        .await?
        .map_err(ApiError::internal)?;
    Ok(Json(
        channels.into_iter().map(ChannelResponse::from).collect(),
    ))
}

pub async fn reconcile_channel(
    State(channel_video_reconciler): State<ChannelVideoReconciler>,
    Path(handle): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = ChannelHandle::new(handle).map_err(ApiError::bad_request)?;

    run_blocking(move || channel_video_reconciler.force_reconcile(id))
        .await?
        .map_err(ApiError::internal)?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::Channel;
    use crate::domain::event::DomainEvent;
    use crate::domain::services::ThumbnailFetcher;
    use crate::infrastructure::repositories::filesystem_channel_avatar_repository::FakeChannelAvatarRepository;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_channel_repository::{
        ChannelRepository, FakeChannelRepository,
    };
    use crate::infrastructure::repositories::sqlite_channel_video_repository::FakeChannelVideoRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::FakeTaskRepository;
    use crate::infrastructure::repositories::sqlite_video_metadata_repository::FakeVideoMetadataRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::FakeVideoRepository;
    use crate::infrastructure::repositories::youtube_channel_repository::{
        FakeYoutubeChannelRepository, ResolvedChannel, YoutubeChannelRepository,
    };
    use crate::infrastructure::repositories::youtube_channel_videos_repository::{
        ChannelVideoListing, FakeChannelVideosRepository,
    };
    use crate::infrastructure::repositories::youtube_metadata_repository::FakeYoutubeMetadataRepository;
    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
    use crate::infrastructure::shared::domain_events::event_publisher::FakeEventPublisher;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use std::sync::Arc;

    #[tokio::test]
    async fn it_should_return_201_when_creating_a_new_channel_from_a_bare_handle() {
        let (service, _event_publisher) = channel_service();

        let response = create(service, create_request("@somechannel")).await;

        assert_eq!(response, Ok((StatusCode::CREATED, some_channel_response())));
    }

    #[tokio::test]
    async fn it_should_return_201_when_creating_a_channel_from_a_url() {
        let (service, _event_publisher) = channel_service();

        let response = create(
            service,
            create_request("https://www.youtube.com/@somechannel"),
        )
        .await;

        assert_eq!(response, Ok((StatusCode::CREATED, some_channel_response())));
    }

    #[tokio::test]
    async fn it_should_return_the_stored_avatar_filename_when_the_channel_has_an_avatar() {
        let (service, _event_publisher) = channel_service_with(
            resolving(Some(ResolvedChannel {
                avatar_url: Some("https://yt3.ggpht.com/avatar.jpg".to_string()),
                ..resolved_channel()
            })),
            FakeChannelRepository::default(),
            FakeChannelAvatarRepository::with_stored_filename("@somechannel.jpg"),
        );

        let response = create(service, create_request("@somechannel")).await;

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
    }

    #[tokio::test]
    async fn it_should_return_200_with_the_existing_channel_when_creating_a_channel_that_already_exists()
     {
        let (service, _event_publisher) = channel_service_with(
            resolving(Some(resolved_channel())),
            repository_with(&[channel("@somechannel", "creators/somechannel")]),
            FakeChannelAvatarRepository::default(),
        );

        let response = create(
            service,
            CreateChannelRequest {
                quality: Some("low".to_string()),
                video_limit: Some(5),
                path: Some("different/path".to_string()),
                ..create_request("@somechannel")
            },
        )
        .await;

        assert_eq!(response, Ok((StatusCode::OK, some_channel_response())));
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_channel_value_is_missing() {
        let response = create_rejection(CreateChannelRequest {
            channel: None,
            ..create_request("@somechannel")
        })
        .await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Channel handle or URL must not be empty"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_channel_value_is_empty() {
        let response = create_rejection(create_request("")).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Channel handle or URL must not be empty"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_handle_is_missing_its_leading_at() {
        let response = create_rejection(create_request("somechannel")).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Channel handle must start with \"@\" (got \"somechannel\")"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_url_is_not_a_recognized_youtube_url() {
        let response = create_rejection(create_request("https://example.com/@somechannel")).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "\"https://example.com/@somechannel\" is not a recognized YouTube channel URL"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_youtube_url_is_missing_a_handle() {
        let response = create_rejection(create_request(
            "https://www.youtube.com/channel/UCabcdefghijklmnopqrstuv",
        ))
        .await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "YouTube URL is missing a channel handle (got \"https://www.youtube.com/channel/UCabcdefghijklmnopqrstuv\")"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_youtube_channel_does_not_exist() {
        let (service, _event_publisher) = channel_service_with(
            resolving(None),
            FakeChannelRepository::default(),
            FakeChannelAvatarRepository::default(),
        );

        let response = create(service, create_request("@missing")).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "YouTube channel @missing does not exist or is not accessible"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_502_when_the_youtube_lookup_fails() {
        let (service, _event_publisher) = channel_service_with(
            Arc::new(FailingYoutubeChannelRepository),
            FakeChannelRepository::default(),
            FakeChannelAvatarRepository::default(),
        );

        let response = create(service, create_request("@somechannel")).await;

        assert_eq!(
            response,
            Err(ApiError::new(
                StatusCode::BAD_GATEWAY,
                "YouTube API request failed"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_quality_is_missing() {
        let response = create_rejection(CreateChannelRequest {
            quality: None,
            ..create_request("@somechannel")
        })
        .await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Quality must be one of \"high\", \"mid\", or \"low\" (missing)"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_quality_is_invalid() {
        let response = create_rejection(CreateChannelRequest {
            quality: Some("ultra".to_string()),
            ..create_request("@somechannel")
        })
        .await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Quality must be one of \"high\", \"mid\", or \"low\" (got \"ultra\")"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_video_limit_is_missing() {
        let response = create_rejection(CreateChannelRequest {
            video_limit: None,
            ..create_request("@somechannel")
        })
        .await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Video limit must be a positive integer (missing)"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_video_limit_is_not_positive() {
        let response = create_rejection(CreateChannelRequest {
            video_limit: Some(0),
            ..create_request("@somechannel")
        })
        .await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Video limit must be a positive integer (got 0)"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_path_is_missing() {
        let response = create_rejection(CreateChannelRequest {
            path: None,
            ..create_request("@somechannel")
        })
        .await;

        assert_eq!(
            response,
            Err(ApiError::bad_request("Channel path must not be empty"))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_path_is_absolute() {
        let response = create_rejection(CreateChannelRequest {
            path: Some("/absolute/path".to_string()),
            ..create_request("@somechannel")
        })
        .await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Playlist path must not be an absolute path"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_path_contains_a_parent_traversal_segment() {
        let response = create_rejection(CreateChannelRequest {
            path: Some("a/../b".to_string()),
            ..create_request("@somechannel")
        })
        .await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Playlist path must not contain \"..\" segments"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_path_contains_an_empty_segment() {
        let response = create_rejection(CreateChannelRequest {
            path: Some("a//b".to_string()),
            ..create_request("@somechannel")
        })
        .await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Playlist path must not contain empty segments"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_record_a_channel_created_event_only_once_for_repeated_creation() {
        let (service, event_publisher) = channel_service();

        create(service.clone(), create_request("@somechannel"))
            .await
            .unwrap();
        create(
            service,
            CreateChannelRequest {
                quality: Some("low".to_string()),
                video_limit: Some(5),
                ..create_request("@somechannel")
            },
        )
        .await
        .unwrap();

        assert_eq!(
            *event_publisher.published.lock().unwrap(),
            vec![DomainEvent::ChannelCreated {
                channel_id: "@somechannel".to_string()
            }]
        );
    }

    #[tokio::test]
    async fn it_should_return_204_and_record_a_channel_deleted_event_when_deleting_an_existing_channel()
     {
        let (service, event_publisher) = channel_service_with(
            resolving(Some(resolved_channel())),
            repository_with(&[channel("@somechannel", "creators/somechannel")]),
            FakeChannelAvatarRepository::default(),
        );

        let response = delete_channel(State(service), Path("@somechannel".to_string())).await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            *event_publisher.published.lock().unwrap(),
            vec![DomainEvent::ChannelDeleted {
                channel_id: "@somechannel".to_string(),
                path: "creators/somechannel".to_string(),
            }]
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_deleting_a_missing_channel() {
        let (service, event_publisher) = channel_service();

        let response = delete_channel(State(service), Path("@missing".to_string())).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request("channel @missing not found"))
        );
        assert!(event_publisher.published.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn it_should_return_an_empty_array_when_no_channels_exist() {
        let (service, _event_publisher) = channel_service();

        let response = list_channels(State(service))
            .await
            .map(|Json(channels)| channels);

        assert_eq!(response, Ok(vec![]));
    }

    #[tokio::test]
    async fn it_should_return_all_existing_channels() {
        let (service, _event_publisher) = channel_service_with(
            resolving(Some(resolved_channel())),
            repository_with(&[channel("@somechannel", "creators/somechannel")]),
            FakeChannelAvatarRepository::default(),
        );

        let response = list_channels(State(service))
            .await
            .map(|Json(channels)| channels);

        assert_eq!(response, Ok(vec![some_channel_response()]));
    }

    #[tokio::test]
    async fn it_should_return_204_when_reconciling_an_existing_channel() {
        let reconciler = channel_video_reconciler(
            repository_with(&[channel("@somechannel", "creators/somechannel")]),
            vec![ChannelVideoListing {
                youtube_id: "yt1".to_string(),
                title: "One".to_string(),
                position: 0,
            }],
        );

        let response = reconcile_channel(State(reconciler), Path("@somechannel".to_string())).await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
    }

    #[tokio::test]
    async fn it_should_return_204_on_repeated_reconciles_of_the_same_channel() {
        let reconciler = channel_video_reconciler(
            repository_with(&[channel("@somechannel", "creators/somechannel")]),
            Vec::new(),
        );

        reconcile_channel(State(reconciler.clone()), Path("@somechannel".to_string()))
            .await
            .unwrap();
        let response = reconcile_channel(State(reconciler), Path("@somechannel".to_string())).await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
    }

    #[tokio::test]
    async fn it_should_return_204_and_do_nothing_when_reconciling_a_nonexistent_channel() {
        let reconciler = channel_video_reconciler(FakeChannelRepository::default(), Vec::new());

        let response = reconcile_channel(State(reconciler), Path("@missing".to_string())).await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
    }

    #[tokio::test]
    async fn it_should_return_400_when_reconciling_with_an_invalid_handle() {
        let reconciler = channel_video_reconciler(FakeChannelRepository::default(), Vec::new());

        let response = reconcile_channel(State(reconciler), Path("noatsign".to_string())).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Channel handle must start with \"@\" (got \"noatsign\")"
            ))
        );
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn resolved_channel() -> ResolvedChannel {
        ResolvedChannel {
            youtube_channel_id: "UC123".to_string(),
            title: "Some Channel".to_string(),
            avatar_url: None,
        }
    }

    fn resolving(resolved: Option<ResolvedChannel>) -> Arc<dyn YoutubeChannelRepository> {
        Arc::new(FakeYoutubeChannelRepository { resolved })
    }

    struct FailingYoutubeChannelRepository;

    impl YoutubeChannelRepository for FailingYoutubeChannelRepository {
        fn resolve(&self, _handle: &ChannelHandle) -> anyhow::Result<Option<ResolvedChannel>> {
            anyhow::bail!("YouTube API request failed")
        }
    }

    fn channel_service() -> (ChannelService, Arc<FakeEventPublisher>) {
        channel_service_with(
            resolving(Some(resolved_channel())),
            FakeChannelRepository::default(),
            FakeChannelAvatarRepository::default(),
        )
    }

    fn channel_service_with(
        lookup: Arc<dyn YoutubeChannelRepository>,
        channel_repository: FakeChannelRepository,
        avatar_repository: FakeChannelAvatarRepository,
    ) -> (ChannelService, Arc<FakeEventPublisher>) {
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let channel_service = ChannelService::new(
            Arc::new(channel_repository),
            lookup,
            Arc::new(avatar_repository),
            Arc::new(FakeVideoRepository::default()),
            Arc::new(FakeChannelVideoRepository::default()),
            event_publisher.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );
        (channel_service, event_publisher)
    }

    fn channel_video_reconciler(
        channel_repository: FakeChannelRepository,
        channel_videos: Vec<ChannelVideoListing>,
    ) -> ChannelVideoReconciler {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let thumbnail_fetcher = Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        ChannelVideoReconciler::new(
            Arc::new(channel_repository),
            video_repository,
            Arc::new(FakeChannelVideoRepository::default()),
            Arc::new(FakeChannelVideosRepository::with_videos(channel_videos)),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(FakeVideoMetadataRepository::default()),
            Arc::new(FakeEventPublisher::default()),
            Arc::new(FakeTaskRepository::default()),
            Arc::new(FakeVideoFileRepository::default()),
            thumbnail_fetcher,
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        )
    }

    fn channel(handle: &str, path: &str) -> Channel {
        Channel::create(
            ChannelHandle::new(handle).unwrap(),
            "Some Channel",
            "UC123",
            Quality::High,
            VideoLimit::new(10).unwrap(),
            PlaylistPath::new(path).unwrap(),
            None,
            fixed_timestamp(),
        )
    }

    fn repository_with(channels: &[Channel]) -> FakeChannelRepository {
        let repository = FakeChannelRepository::default();
        for channel in channels {
            repository.insert(channel).unwrap();
        }
        repository
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

    async fn create(
        service: ChannelService,
        request: CreateChannelRequest,
    ) -> Result<(StatusCode, ChannelResponse), ApiError> {
        create_channel(State(service), Json(request))
            .await
            .map(|(status, Json(channel))| (status, channel))
    }

    async fn create_rejection(
        request: CreateChannelRequest,
    ) -> Result<(StatusCode, ChannelResponse), ApiError> {
        let (service, _event_publisher) = channel_service();
        create(service, request).await
    }
}
