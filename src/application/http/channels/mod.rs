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
    let id = ChannelHandle::from_url_or_handle(request.channel.unwrap_or_default())?;
    let quality = Quality::new(required(request.quality, MISSING_QUALITY)?)?;
    let video_limit = VideoLimit::new(required(request.video_limit, MISSING_VIDEO_LIMIT)?)?;
    let path = PlaylistPath::new(required(request.path, MISSING_PATH)?)?;

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
    let id = ChannelHandle::new(handle)?;

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
    let id = ChannelHandle::new(handle)?;

    run_blocking(move || channel_video_reconciler.force_reconcile(id))
        .await?
        .map_err(ApiError::internal)?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::Channel;
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::event::DomainEvent;
    use crate::domain::services::ThumbnailFetcher;
    use crate::domain::shared::VideoId;
    use crate::domain::video::Video;
    use crate::infrastructure::repositories::filesystem_channel_avatar_repository::FakeChannelAvatarRepository;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_channel_repository::{
        ChannelRepository, FakeChannelRepository,
    };
    use crate::infrastructure::repositories::sqlite_channel_video_repository::{
        ChannelVideoRepository, FakeChannelVideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_task_repository::FakeTaskRepository;
    use crate::infrastructure::repositories::sqlite_video_metadata_repository::FakeVideoMetadataRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::{
        FakeVideoRepository, VideoRepository,
    };
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

    const AVATAR_URL: &str = "https://yt3.ggpht.com/avatar.jpg";
    const OTHER_AVATAR_URL: &str = "https://yt3.ggpht.com/other-avatar.jpg";

    #[tokio::test]
    async fn it_should_return_201_and_store_the_channel_when_creating_a_new_channel_from_a_bare_handle()
     {
        let channel_repository = Arc::new(FakeChannelRepository::default());
        let video_repository = Arc::new(FakeVideoRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let service = ChannelService::new(
            channel_repository.clone(),
            resolving(Some(resolved_channel())),
            Arc::new(FakeChannelAvatarRepository::default()),
            video_repository.clone(),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
            event_publisher.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = create(service, create_request("@somechannel")).await;

        assert_eq!(response, Ok((StatusCode::CREATED, some_channel_response())));
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![channel("@somechannel")]
        );
        assert_eq!(
            *event_publisher.published.lock().unwrap(),
            vec![channel_created("@somechannel")]
        );
    }

    #[tokio::test]
    async fn it_should_return_201_and_store_the_channel_when_creating_a_channel_from_a_url() {
        let channel_repository = Arc::new(FakeChannelRepository::default());
        let video_repository = Arc::new(FakeVideoRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let service = ChannelService::new(
            channel_repository.clone(),
            resolving(Some(resolved_channel())),
            Arc::new(FakeChannelAvatarRepository::default()),
            video_repository.clone(),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
            event_publisher.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = create(
            service,
            create_request("https://www.youtube.com/@somechannel"),
        )
        .await;

        assert_eq!(response, Ok((StatusCode::CREATED, some_channel_response())));
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![channel("@somechannel")]
        );
        assert_eq!(
            *event_publisher.published.lock().unwrap(),
            vec![channel_created("@somechannel")]
        );
    }

    #[tokio::test]
    async fn it_should_store_the_avatar_and_record_its_filename_when_the_channel_has_an_avatar() {
        let channel_repository = Arc::new(FakeChannelRepository::default());
        let avatar_repository = Arc::new(FakeChannelAvatarRepository::default());
        let video_repository = Arc::new(FakeVideoRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let service = ChannelService::new(
            channel_repository.clone(),
            resolving(Some(resolved_channel_with_avatar())),
            avatar_repository.clone(),
            video_repository.clone(),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
            event_publisher.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
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
            *event_publisher.published.lock().unwrap(),
            vec![channel_created("@somechannel")]
        );
    }

    #[tokio::test]
    async fn it_should_not_store_an_avatar_when_the_channel_has_none() {
        let channel_repository = Arc::new(FakeChannelRepository::default());
        let avatar_repository = Arc::new(FakeChannelAvatarRepository::default());
        let video_repository = Arc::new(FakeVideoRepository::default());
        let service = ChannelService::new(
            channel_repository.clone(),
            resolving(Some(resolved_channel())),
            avatar_repository.clone(),
            video_repository.clone(),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
            Arc::new(FakeEventPublisher::default()),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = create(service, create_request("@somechannel")).await;

        assert_eq!(response, Ok((StatusCode::CREATED, some_channel_response())));
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![channel("@somechannel")]
        );
        assert_eq!(avatar_repository.avatars(), vec![]);
    }

    #[tokio::test]
    async fn it_should_not_record_an_avatar_filename_when_the_avatar_is_unavailable() {
        let channel_repository = Arc::new(FakeChannelRepository::default());
        let avatar_repository = Arc::new(FakeChannelAvatarRepository::unavailable());
        let video_repository = Arc::new(FakeVideoRepository::default());
        let service = ChannelService::new(
            channel_repository.clone(),
            resolving(Some(resolved_channel_with_avatar())),
            avatar_repository.clone(),
            video_repository.clone(),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
            Arc::new(FakeEventPublisher::default()),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = create(service, create_request("@somechannel")).await;

        assert_eq!(response, Ok((StatusCode::CREATED, some_channel_response())));
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![channel("@somechannel")]
        );
        assert_eq!(avatar_repository.avatars(), vec![]);
    }

    #[tokio::test]
    async fn it_should_still_create_the_channel_without_an_avatar_when_storing_the_avatar_fails() {
        let channel_repository = Arc::new(FakeChannelRepository::default());
        let avatar_repository = Arc::new(FakeChannelAvatarRepository::failing());
        let video_repository = Arc::new(FakeVideoRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let service = ChannelService::new(
            channel_repository.clone(),
            resolving(Some(resolved_channel_with_avatar())),
            avatar_repository.clone(),
            video_repository.clone(),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
            event_publisher.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = create(service, create_request("@somechannel")).await;

        assert_eq!(response, Ok((StatusCode::CREATED, some_channel_response())));
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![channel("@somechannel")]
        );
        assert_eq!(avatar_repository.avatars(), vec![]);
        assert_eq!(
            *event_publisher.published.lock().unwrap(),
            vec![channel_created("@somechannel")]
        );
    }

    #[tokio::test]
    async fn it_should_return_200_with_the_existing_channel_and_change_nothing_when_creating_a_channel_that_already_exists()
     {
        let channel_repository = repository_with(&[channel("@somechannel")]);
        let video_repository = Arc::new(FakeVideoRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let service = ChannelService::new(
            channel_repository.clone(),
            resolving(Some(resolved_channel())),
            Arc::new(FakeChannelAvatarRepository::default()),
            video_repository.clone(),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
            event_publisher.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let request = CreateChannelRequest {
            quality: Some("low".to_string()),
            video_limit: Some(5),
            path: Some("different/path".to_string()),
            ..create_request("@somechannel")
        };

        let response = create(service, request).await;

        assert_eq!(response, Ok((StatusCode::OK, some_channel_response())));
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![channel("@somechannel")]
        );
        assert_eq!(*event_publisher.published.lock().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_record_a_channel_created_event_only_once_for_repeated_creation() {
        let channel_repository = Arc::new(FakeChannelRepository::default());
        let video_repository = Arc::new(FakeVideoRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let service = ChannelService::new(
            channel_repository.clone(),
            resolving(Some(resolved_channel())),
            Arc::new(FakeChannelAvatarRepository::default()),
            video_repository.clone(),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
            event_publisher.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let repeated_request = CreateChannelRequest {
            quality: Some("low".to_string()),
            video_limit: Some(5),
            ..create_request("@somechannel")
        };

        create(service.clone(), create_request("@somechannel"))
            .await
            .unwrap();
        create(service, repeated_request).await.unwrap();

        assert_eq!(
            channel_repository.list().unwrap(),
            vec![channel("@somechannel")]
        );
        assert_eq!(
            *event_publisher.published.lock().unwrap(),
            vec![channel_created("@somechannel")]
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_channel_value_is_missing() {
        let request = CreateChannelRequest {
            channel: None,
            ..create_request("@somechannel")
        };

        let response = create(any_channel_service(), request).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Channel handle or URL must not be empty"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_channel_value_is_invalid() {
        let response = create(any_channel_service(), create_request("somechannel")).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Channel handle must start with \"@\" (got \"somechannel\")"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_quality_is_missing() {
        let request = CreateChannelRequest {
            quality: None,
            ..create_request("@somechannel")
        };

        let response = create(any_channel_service(), request).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Quality must be one of \"high\", \"mid\", or \"low\" (missing)"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_video_limit_is_missing() {
        let request = CreateChannelRequest {
            video_limit: None,
            ..create_request("@somechannel")
        };

        let response = create(any_channel_service(), request).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Video limit must be a positive integer (missing)"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_path_is_missing() {
        let request = CreateChannelRequest {
            path: None,
            ..create_request("@somechannel")
        };

        let response = create(any_channel_service(), request).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request("Channel path must not be empty"))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_and_store_nothing_when_the_youtube_channel_does_not_exist() {
        let channel_repository = Arc::new(FakeChannelRepository::default());
        let video_repository = Arc::new(FakeVideoRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let service = ChannelService::new(
            channel_repository.clone(),
            resolving(None),
            Arc::new(FakeChannelAvatarRepository::default()),
            video_repository.clone(),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
            event_publisher.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = create(service, create_request("@missing")).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "YouTube channel @missing does not exist or is not accessible"
            ))
        );
        assert_eq!(channel_repository.list().unwrap(), vec![]);
        assert_eq!(*event_publisher.published.lock().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_return_502_and_store_nothing_when_the_youtube_lookup_fails() {
        let channel_repository = Arc::new(FakeChannelRepository::default());
        let video_repository = Arc::new(FakeVideoRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let service = ChannelService::new(
            channel_repository.clone(),
            Arc::new(FailingYoutubeChannelRepository),
            Arc::new(FakeChannelAvatarRepository::default()),
            video_repository.clone(),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
            event_publisher.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = create(service, create_request("@somechannel")).await;

        assert_eq!(
            response,
            Err(ApiError::new(
                StatusCode::BAD_GATEWAY,
                "YouTube API request failed"
            ))
        );
        assert_eq!(channel_repository.list().unwrap(), vec![]);
        assert_eq!(*event_publisher.published.lock().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_return_204_remove_the_channel_and_record_a_channel_deleted_event_when_deleting_an_existing_channel()
     {
        let channel_repository = repository_with(&[channel("@somechannel")]);
        let avatar_repository = Arc::new(FakeChannelAvatarRepository::default());
        let video_repository = Arc::new(FakeVideoRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let service = ChannelService::new(
            channel_repository.clone(),
            resolving(Some(resolved_channel())),
            avatar_repository.clone(),
            video_repository.clone(),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
            event_publisher.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = delete(service, "@somechannel").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(channel_repository.list().unwrap(), vec![]);
        assert_eq!(avatar_repository.avatars(), vec![]);
        assert_eq!(
            *event_publisher.published.lock().unwrap(),
            vec![channel_deleted("@somechannel")]
        );
    }

    #[tokio::test]
    async fn it_should_delete_every_video_of_the_deleted_channel_and_keep_other_channels_videos() {
        let channel_repository =
            repository_with(&[channel("@somechannel"), channel("@otherchannel")]);
        let video_repository = Arc::new(FakeVideoRepository::default());
        let channel_video_repository =
            Arc::new(FakeChannelVideoRepository::new(video_repository.clone()));
        save_channel_video(
            &video_repository,
            &channel_video_repository,
            "@somechannel",
            "yt1",
            0,
        );
        save_channel_video(
            &video_repository,
            &channel_video_repository,
            "@somechannel",
            "yt2",
            1,
        );
        let (other_video, other_channel_video) = save_channel_video(
            &video_repository,
            &channel_video_repository,
            "@otherchannel",
            "yt3",
            0,
        );
        let service = ChannelService::new(
            channel_repository.clone(),
            resolving(Some(resolved_channel())),
            Arc::new(FakeChannelAvatarRepository::default()),
            video_repository.clone(),
            channel_video_repository.clone(),
            Arc::new(FakeEventPublisher::default()),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = delete(service, "@somechannel").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![channel("@otherchannel")]
        );
        assert_eq!(*video_repository.videos.lock().unwrap(), vec![other_video]);
        assert_eq!(
            *channel_video_repository.channel_videos.lock().unwrap(),
            vec![other_channel_video]
        );
    }

    #[tokio::test]
    async fn it_should_delete_the_avatar_file_when_deleting_a_channel_with_a_recorded_avatar() {
        let channel_repository = repository_with(&[Channel {
            avatar_filename: Some("@somechannel.jpg".to_string()),
            ..channel("@somechannel")
        }]);
        let avatar_repository = Arc::new(FakeChannelAvatarRepository::with_avatars(&[
            ("@somechannel.jpg", AVATAR_URL),
            ("@otherchannel.jpg", OTHER_AVATAR_URL),
        ]));
        let video_repository = Arc::new(FakeVideoRepository::default());
        let service = ChannelService::new(
            channel_repository.clone(),
            resolving(Some(resolved_channel())),
            avatar_repository.clone(),
            video_repository.clone(),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
            Arc::new(FakeEventPublisher::default()),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = delete(service, "@somechannel").await;

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
    async fn it_should_return_400_and_change_nothing_when_deleting_a_missing_channel() {
        let channel_repository = repository_with(&[channel("@somechannel")]);
        let video_repository = Arc::new(FakeVideoRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let service = ChannelService::new(
            channel_repository.clone(),
            resolving(Some(resolved_channel())),
            Arc::new(FakeChannelAvatarRepository::default()),
            video_repository.clone(),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
            event_publisher.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = delete(service, "@missing").await;

        assert_eq!(
            response,
            Err(ApiError::bad_request("channel @missing not found"))
        );
        assert_eq!(
            channel_repository.list().unwrap(),
            vec![channel("@somechannel")]
        );
        assert_eq!(*event_publisher.published.lock().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_return_400_when_deleting_with_an_invalid_handle() {
        let response = delete(any_channel_service(), "noatsign").await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Channel handle must start with \"@\" (got \"noatsign\")"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_an_empty_array_when_no_channels_exist() {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let service = ChannelService::new(
            Arc::new(FakeChannelRepository::default()),
            resolving(Some(resolved_channel())),
            Arc::new(FakeChannelAvatarRepository::default()),
            video_repository.clone(),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
            Arc::new(FakeEventPublisher::default()),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = list(service).await;

        assert_eq!(response, Ok(vec![]));
    }

    #[tokio::test]
    async fn it_should_return_all_existing_channels() {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let service = ChannelService::new(
            repository_with(&[channel("@somechannel")]),
            resolving(Some(resolved_channel())),
            Arc::new(FakeChannelAvatarRepository::default()),
            video_repository.clone(),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
            Arc::new(FakeEventPublisher::default()),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = list(service).await;

        assert_eq!(response, Ok(vec![some_channel_response()]));
    }

    #[tokio::test]
    async fn it_should_return_204_and_add_the_listed_videos_when_reconciling_an_existing_channel() {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let channel_video_repository =
            Arc::new(FakeChannelVideoRepository::new(video_repository.clone()));
        let task_repository = Arc::new(FakeTaskRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let reconciler = channel_video_reconciler(
            repository_with(&[channel("@somechannel")]),
            video_repository.clone(),
            channel_video_repository.clone(),
            vec![listed_video("yt1", "One", 0)],
            task_repository.clone(),
            event_publisher.clone(),
        );

        let response = reconcile(reconciler, "@somechannel").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        let videos = video_repository.videos.lock().unwrap().clone();
        let video_id = videos[0].id.clone();
        assert_eq!(
            videos,
            vec![Video {
                id: video_id.clone(),
                ..Video::create(VideoId::new("yt1").unwrap(), "One", fixed_timestamp())
            }]
        );
        assert_eq!(
            *channel_video_repository.channel_videos.lock().unwrap(),
            vec![ChannelVideo::create(
                handle("@somechannel"),
                video_id.clone(),
                0,
                fixed_timestamp()
            )]
        );
        assert_eq!(task_repository.scheduled(), vec![]);
        assert_eq!(
            *event_publisher.published.lock().unwrap(),
            vec![DomainEvent::VideoAddedToChannel {
                channel_id: "@somechannel".to_string(),
                video_id: video_id.as_str().to_string(),
            }]
        );
    }

    #[tokio::test]
    async fn it_should_evict_stored_videos_no_longer_listed_when_reconciling_a_channel() {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let channel_video_repository =
            Arc::new(FakeChannelVideoRepository::new(video_repository.clone()));
        let (evicted, _) = save_channel_video(
            &video_repository,
            &channel_video_repository,
            "@somechannel",
            "yt_old",
            0,
        );
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let reconciler = channel_video_reconciler(
            repository_with(&[channel("@somechannel")]),
            video_repository.clone(),
            channel_video_repository.clone(),
            Vec::new(),
            Arc::new(FakeTaskRepository::default()),
            event_publisher.clone(),
        );

        let response = reconcile(reconciler, "@somechannel").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(*video_repository.videos.lock().unwrap(), vec![]);
        assert_eq!(
            *channel_video_repository.channel_videos.lock().unwrap(),
            vec![]
        );
        assert_eq!(
            *event_publisher.published.lock().unwrap(),
            vec![DomainEvent::VideoRemovedFromChannel {
                channel_id: "@somechannel".to_string(),
                video_id: evicted.id.as_str().to_string(),
                title: "Video yt_old".to_string(),
                filename: None,
                thumbnail_filename: None,
                was_downloaded: false,
            }]
        );
    }

    #[tokio::test]
    async fn it_should_not_add_a_listed_video_twice_on_repeated_reconciles_of_the_same_channel() {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let channel_video_repository =
            Arc::new(FakeChannelVideoRepository::new(video_repository.clone()));
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let reconciler = channel_video_reconciler(
            repository_with(&[channel("@somechannel")]),
            video_repository.clone(),
            channel_video_repository.clone(),
            vec![listed_video("yt1", "One", 0)],
            Arc::new(FakeTaskRepository::default()),
            event_publisher.clone(),
        );

        reconcile(reconciler.clone(), "@somechannel").await.unwrap();
        let videos_after_first_reconcile = video_repository.videos.lock().unwrap().clone();
        let channel_videos_after_first_reconcile = channel_video_repository
            .channel_videos
            .lock()
            .unwrap()
            .clone();
        let events_after_first_reconcile = event_publisher.published.lock().unwrap().clone();
        let response = reconcile(reconciler, "@somechannel").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            *video_repository.videos.lock().unwrap(),
            videos_after_first_reconcile
        );
        assert_eq!(
            *channel_video_repository.channel_videos.lock().unwrap(),
            channel_videos_after_first_reconcile
        );
        assert_eq!(
            *event_publisher.published.lock().unwrap(),
            events_after_first_reconcile
        );
    }

    #[tokio::test]
    async fn it_should_return_204_and_do_nothing_when_reconciling_a_nonexistent_channel() {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let channel_video_repository =
            Arc::new(FakeChannelVideoRepository::new(video_repository.clone()));
        let task_repository = Arc::new(FakeTaskRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let reconciler = channel_video_reconciler(
            Arc::new(FakeChannelRepository::default()),
            video_repository.clone(),
            channel_video_repository.clone(),
            vec![listed_video("yt1", "One", 0)],
            task_repository.clone(),
            event_publisher.clone(),
        );

        let response = reconcile(reconciler, "@missing").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(*video_repository.videos.lock().unwrap(), vec![]);
        assert_eq!(
            *channel_video_repository.channel_videos.lock().unwrap(),
            vec![]
        );
        assert_eq!(task_repository.scheduled(), vec![]);
        assert_eq!(*event_publisher.published.lock().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_return_400_when_reconciling_with_an_invalid_handle() {
        let response = reconcile(any_channel_video_reconciler(), "noatsign").await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Channel handle must start with \"@\" (got \"noatsign\")"
            ))
        );
    }

    /// A service for tests whose request is rejected before reaching it.
    fn any_channel_service() -> ChannelService {
        let video_repository = Arc::new(FakeVideoRepository::default());
        ChannelService::new(
            Arc::new(FakeChannelRepository::default()),
            resolving(Some(resolved_channel())),
            Arc::new(FakeChannelAvatarRepository::default()),
            video_repository.clone(),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
            Arc::new(FakeEventPublisher::default()),
            Arc::new(FixedClock(fixed_timestamp())),
        )
    }

    /// Builds a reconciler around the fakes a test seeds and asserts; the
    /// remaining ports (metadata, files, thumbnails) are ones no channel
    /// reconcile test observes.
    fn channel_video_reconciler(
        channel_repository: Arc<FakeChannelRepository>,
        video_repository: Arc<FakeVideoRepository>,
        channel_video_repository: Arc<FakeChannelVideoRepository>,
        listed_videos: Vec<ChannelVideoListing>,
        task_repository: Arc<FakeTaskRepository>,
        event_publisher: Arc<FakeEventPublisher>,
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
            Arc::new(FakeVideoMetadataRepository::default()),
            event_publisher,
            task_repository,
            Arc::new(FakeVideoFileRepository::default()),
            thumbnail_fetcher,
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        )
    }

    fn any_channel_video_reconciler() -> ChannelVideoReconciler {
        let video_repository = Arc::new(FakeVideoRepository::default());
        channel_video_reconciler(
            Arc::new(FakeChannelRepository::default()),
            video_repository.clone(),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
            Vec::new(),
            Arc::new(FakeTaskRepository::default()),
            Arc::new(FakeEventPublisher::default()),
        )
    }

    struct FailingYoutubeChannelRepository;

    impl YoutubeChannelRepository for FailingYoutubeChannelRepository {
        fn resolve(&self, _handle: &ChannelHandle) -> anyhow::Result<Option<ResolvedChannel>> {
            anyhow::bail!("YouTube API request failed")
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
            PlaylistPath::new("creators/somechannel").unwrap(),
            None,
            fixed_timestamp(),
        )
    }

    fn repository_with(channels: &[Channel]) -> Arc<FakeChannelRepository> {
        let repository = FakeChannelRepository::default();
        for channel in channels {
            repository.insert(channel).unwrap();
        }
        Arc::new(repository)
    }

    fn save_channel_video(
        video_repository: &FakeVideoRepository,
        channel_video_repository: &FakeChannelVideoRepository,
        channel_handle: &str,
        youtube_id: &str,
        position: i64,
    ) -> (Video, ChannelVideo) {
        let video = Video::create(
            VideoId::new(youtube_id).unwrap(),
            format!("Video {youtube_id}"),
            fixed_timestamp(),
        );
        let channel_video = ChannelVideo::create(
            handle(channel_handle),
            video.id.clone(),
            position,
            fixed_timestamp(),
        );
        video_repository.save(&video).unwrap();
        channel_video_repository.save(&channel_video).unwrap();
        (video, channel_video)
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

    async fn create(
        service: ChannelService,
        request: CreateChannelRequest,
    ) -> Result<(StatusCode, ChannelResponse), ApiError> {
        create_channel(State(service), Json(request))
            .await
            .map(|(status, Json(channel))| (status, channel))
    }

    async fn delete(service: ChannelService, channel_handle: &str) -> Result<StatusCode, ApiError> {
        delete_channel(State(service), Path(channel_handle.to_string())).await
    }

    async fn list(service: ChannelService) -> Result<Vec<ChannelResponse>, ApiError> {
        list_channels(State(service))
            .await
            .map(|Json(channels)| channels)
    }

    async fn reconcile(
        reconciler: ChannelVideoReconciler,
        channel_handle: &str,
    ) -> Result<StatusCode, ApiError> {
        reconcile_channel(State(reconciler), Path(channel_handle.to_string())).await
    }
}
