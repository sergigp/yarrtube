pub mod dto;

use super::blocking::run_blocking;
use super::error::ApiError;
use super::validation::{MISSING_QUALITY, required};
use crate::domain::playlist::{
    CreatePlaylistError, DeletePlaylistError, PlaylistName, PlaylistPath,
};
use crate::domain::services::{
    CreatePlaylistOutcome, PlaylistCreator, PlaylistDeleter, PlaylistSearcher, VideoReconciler,
};
use crate::domain::shared::{PlaylistId, Quality};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use dto::{CreatePlaylistRequest, PlaylistResponse};

const MISSING_PATH: &str = "Playlist path must not be empty";

pub async fn create_playlist(
    State(playlist_creator): State<PlaylistCreator>,
    Json(request): Json<CreatePlaylistRequest>,
) -> Result<(StatusCode, Json<PlaylistResponse>), ApiError> {
    let id = PlaylistId::from_url_or_id(request.playlist)?;
    let name = PlaylistName::new(request.name)?;
    let path = PlaylistPath::new(required(request.path, MISSING_PATH)?)?;
    let quality = Quality::new(required(request.quality, MISSING_QUALITY)?)?;

    let outcome = run_blocking(move || playlist_creator.create(id, name, path, quality)).await?;

    match outcome {
        Ok(CreatePlaylistOutcome::Created(playlist)) => {
            Ok((StatusCode::CREATED, Json(PlaylistResponse::from(playlist))))
        }
        Ok(CreatePlaylistOutcome::AlreadyExisted(playlist)) => {
            Ok((StatusCode::OK, Json(PlaylistResponse::from(playlist))))
        }
        Err(e @ CreatePlaylistError::YoutubePlaylistNotFound(_)) => Err(ApiError::bad_request(e)),
        Err(e @ CreatePlaylistError::PathAlreadyInUse(_)) => Err(ApiError::bad_request(e)),
        Err(e @ CreatePlaylistError::Lookup(_)) => Err(ApiError::new(StatusCode::BAD_GATEWAY, e)),
        Err(e @ CreatePlaylistError::Repository(_)) => Err(ApiError::internal(e)),
    }
}

pub async fn delete_playlist(
    State(playlist_deleter): State<PlaylistDeleter>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = PlaylistId::new(id)?;

    match run_blocking(move || playlist_deleter.delete(id)).await? {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(e @ DeletePlaylistError::NotFound(_)) => Err(ApiError::bad_request(e)),
        Err(e @ DeletePlaylistError::Repository(_)) => Err(ApiError::internal(e)),
    }
}

pub async fn reconcile_playlist(
    State(video_reconciler): State<VideoReconciler>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = PlaylistId::new(id)?;

    run_blocking(move || video_reconciler.force_reconcile(id))
        .await?
        .map_err(ApiError::internal)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_playlists(
    State(playlist_searcher): State<PlaylistSearcher>,
) -> Result<Json<Vec<PlaylistResponse>>, ApiError> {
    let playlists = run_blocking(move || playlist_searcher.search_all())
        .await?
        .map_err(ApiError::internal)?;
    Ok(Json(
        playlists.into_iter().map(PlaylistResponse::from).collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event::DomainEvent;
    use crate::domain::playlist::{Playlist, PlaylistKind};
    use crate::domain::playlist_video::PlaylistVideo;
    use crate::domain::services::ThumbnailFetcher;
    use crate::domain::shared::VideoId;
    use crate::domain::task::Task;
    use crate::domain::video::Video;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_playlist_repository::{
        FakePlaylistRepository, PlaylistRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::{
        FakePlaylistVideoRepository, PlaylistVideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_task_repository::{
        FakeTaskRepository, TaskRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_metadata_repository::FakeVideoMetadataRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::{
        FakeVideoRepository, VideoRepository,
    };
    use crate::infrastructure::repositories::youtube_metadata_repository::FakeYoutubeMetadataRepository;
    use crate::infrastructure::repositories::youtube_playlist_items_repository::{
        FakeYoutubePlaylistItemsRepository, YoutubePlaylistItem,
    };
    use crate::infrastructure::repositories::youtube_playlist_repository::{
        FakeYoutubePlaylistRepository, YoutubePlaylistRepository,
    };
    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
    use crate::infrastructure::shared::domain_events::event_publisher::FakeEventPublisher;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use std::sync::{Arc, Mutex};

    const DEFAULT_PATH: &str = "music/chill";

    #[tokio::test]
    async fn it_should_return_201_and_store_the_playlist_when_creating_a_new_playlist() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let playlist_creator = PlaylistCreator::new(
            playlist_repository.clone(),
            Arc::new(FakeYoutubePlaylistRepository { exists: true }),
            event_publisher.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = create(playlist_creator, create_request("PL1")).await;

        assert_eq!(
            response,
            Ok((StatusCode::CREATED, playlist_response("PL1", DEFAULT_PATH)))
        );
        assert_eq!(
            playlist_repository.list().unwrap(),
            vec![playlist("PL1", DEFAULT_PATH)]
        );
        assert_eq!(
            *event_publisher.published.lock().unwrap(),
            vec![playlist_created("PL1")]
        );
    }

    #[tokio::test]
    async fn it_should_return_201_and_store_the_playlist_when_creating_a_playlist_from_a_youtube_url()
     {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let playlist_creator = PlaylistCreator::new(
            playlist_repository.clone(),
            Arc::new(FakeYoutubePlaylistRepository { exists: true }),
            event_publisher.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = create(
            playlist_creator,
            create_request("https://www.youtube.com/playlist?list=PL1"),
        )
        .await;

        assert_eq!(
            response,
            Ok((StatusCode::CREATED, playlist_response("PL1", DEFAULT_PATH)))
        );
        assert_eq!(
            playlist_repository.list().unwrap(),
            vec![playlist("PL1", DEFAULT_PATH)]
        );
        assert_eq!(
            *event_publisher.published.lock().unwrap(),
            vec![playlist_created("PL1")]
        );
    }

    #[tokio::test]
    async fn it_should_return_200_with_the_existing_playlist_and_change_nothing_when_creating_a_playlist_that_already_exists()
     {
        let playlist_repository = playlist_repository_with(&[playlist("PL1", DEFAULT_PATH)]);
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let playlist_creator = PlaylistCreator::new(
            playlist_repository.clone(),
            Arc::new(FakeYoutubePlaylistRepository { exists: true }),
            event_publisher.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let request = CreatePlaylistRequest {
            name: "Different Name".to_string(),
            path: Some("different/path".to_string()),
            quality: Some("low".to_string()),
            ..create_request("PL1")
        };

        let response = create(playlist_creator, request).await;

        assert_eq!(
            response,
            Ok((StatusCode::OK, playlist_response("PL1", DEFAULT_PATH)))
        );
        assert_eq!(
            playlist_repository.list().unwrap(),
            vec![playlist("PL1", DEFAULT_PATH)]
        );
        assert_eq!(*event_publisher.published.lock().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_record_a_playlist_created_event_only_once_for_repeated_creation() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let playlist_creator = PlaylistCreator::new(
            playlist_repository.clone(),
            Arc::new(FakeYoutubePlaylistRepository { exists: true }),
            event_publisher.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        create(playlist_creator.clone(), create_request("PL1"))
            .await
            .unwrap();
        create(playlist_creator, create_request("PL1"))
            .await
            .unwrap();

        assert_eq!(
            playlist_repository.list().unwrap(),
            vec![playlist("PL1", DEFAULT_PATH)]
        );
        assert_eq!(
            *event_publisher.published.lock().unwrap(),
            vec![playlist_created("PL1")]
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_playlist_value_is_invalid() {
        let response = create(any_playlist_creator(), create_request("")).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Playlist ID or URL must not be empty"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_path_is_missing() {
        let request = CreatePlaylistRequest {
            path: None,
            ..create_request("PL1")
        };

        let response = create(any_playlist_creator(), request).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request("Playlist path must not be empty"))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_quality_is_missing() {
        let request = CreatePlaylistRequest {
            quality: None,
            ..create_request("PL1")
        };

        let response = create(any_playlist_creator(), request).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Quality must be one of \"high\", \"mid\", or \"low\" (missing)"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_and_store_nothing_when_the_path_is_already_used_by_another_playlist()
     {
        let playlist_repository = playlist_repository_with(&[playlist("PL1", "shared/path")]);
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let playlist_creator = PlaylistCreator::new(
            playlist_repository.clone(),
            Arc::new(FakeYoutubePlaylistRepository { exists: true }),
            event_publisher.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );
        let request = CreatePlaylistRequest {
            path: Some("shared/path".to_string()),
            ..create_request("PL2")
        };

        let response = create(playlist_creator, request).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "playlist path \"shared/path\" is already used by another playlist"
            ))
        );
        assert_eq!(
            playlist_repository.list().unwrap(),
            vec![playlist("PL1", "shared/path")]
        );
        assert_eq!(*event_publisher.published.lock().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_return_400_and_store_nothing_when_the_youtube_playlist_does_not_exist() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let playlist_creator = PlaylistCreator::new(
            playlist_repository.clone(),
            Arc::new(FakeYoutubePlaylistRepository { exists: false }),
            event_publisher.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = create(playlist_creator, create_request("PL404")).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "YouTube playlist PL404 does not exist or is not accessible"
            ))
        );
        assert_eq!(playlist_repository.list().unwrap(), vec![]);
        assert_eq!(*event_publisher.published.lock().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_return_502_and_store_nothing_when_the_youtube_lookup_fails() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let playlist_creator = PlaylistCreator::new(
            playlist_repository.clone(),
            Arc::new(FailingYoutubePlaylistRepository),
            event_publisher.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );

        let response = create(playlist_creator, create_request("PL1")).await;

        assert_eq!(
            response,
            Err(ApiError::new(
                StatusCode::BAD_GATEWAY,
                "YouTube API request failed"
            ))
        );
        assert_eq!(playlist_repository.list().unwrap(), vec![]);
        assert_eq!(*event_publisher.published.lock().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_return_204_remove_the_playlist_and_record_a_playlist_deleted_event_when_deleting_an_existing_playlist()
     {
        let playlist_repository = playlist_repository_with(&[playlist("PL1", DEFAULT_PATH)]);
        let video_repository = Arc::new(FakeVideoRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let playlist_deleter = PlaylistDeleter::new(
            playlist_repository.clone(),
            video_repository.clone(),
            Arc::new(FakePlaylistVideoRepository::new(video_repository)),
            event_publisher.clone(),
        );

        let response = delete(playlist_deleter, "PL1").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(playlist_repository.list().unwrap(), vec![]);
        assert_eq!(
            *event_publisher.published.lock().unwrap(),
            vec![DomainEvent::PlaylistDeleted {
                playlist_id: "PL1".to_string(),
                path: DEFAULT_PATH.to_string(),
            }]
        );
    }

    #[tokio::test]
    async fn it_should_delete_every_video_of_the_deleted_playlist_and_keep_other_playlists_videos()
    {
        let playlist_repository = playlist_repository_with(&[
            playlist("PL1", DEFAULT_PATH),
            playlist("PL2", "music/other"),
        ]);
        let video_repository = Arc::new(FakeVideoRepository::default());
        let playlist_video_repository =
            Arc::new(FakePlaylistVideoRepository::new(video_repository.clone()));
        save_playlist_video(
            &video_repository,
            &playlist_video_repository,
            "PL1",
            "vid1",
            0,
        );
        save_playlist_video(
            &video_repository,
            &playlist_video_repository,
            "PL1",
            "vid2",
            1,
        );
        let (other_video, other_playlist_video) = save_playlist_video(
            &video_repository,
            &playlist_video_repository,
            "PL2",
            "vid3",
            0,
        );
        let playlist_deleter = PlaylistDeleter::new(
            playlist_repository.clone(),
            video_repository.clone(),
            playlist_video_repository.clone(),
            Arc::new(FakeEventPublisher::default()),
        );

        let response = delete(playlist_deleter, "PL1").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            playlist_repository.list().unwrap(),
            vec![playlist("PL2", "music/other")]
        );
        assert_eq!(*video_repository.videos.lock().unwrap(), vec![other_video]);
        assert_eq!(
            *playlist_video_repository.playlist_videos.lock().unwrap(),
            vec![other_playlist_video]
        );
    }

    #[tokio::test]
    async fn it_should_return_400_and_change_nothing_when_deleting_a_missing_playlist() {
        let playlist_repository = playlist_repository_with(&[playlist("PL1", DEFAULT_PATH)]);
        let video_repository = Arc::new(FakeVideoRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let playlist_deleter = PlaylistDeleter::new(
            playlist_repository.clone(),
            video_repository.clone(),
            Arc::new(FakePlaylistVideoRepository::new(video_repository)),
            event_publisher.clone(),
        );

        let response = delete(playlist_deleter, "PL404").await;

        assert_eq!(
            response,
            Err(ApiError::bad_request("playlist PL404 not found"))
        );
        assert_eq!(
            playlist_repository.list().unwrap(),
            vec![playlist("PL1", DEFAULT_PATH)]
        );
        assert_eq!(*event_publisher.published.lock().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_return_400_when_deleting_with_an_invalid_playlist_id() {
        let response = delete(any_playlist_deleter(), " ").await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "YouTube playlist ID must not be empty"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_an_empty_array_when_no_playlists_exist() {
        let playlist_searcher = PlaylistSearcher::new(Arc::new(FakePlaylistRepository::default()));

        let response = list(playlist_searcher).await;

        assert_eq!(response, Ok(vec![]));
    }

    #[tokio::test]
    async fn it_should_return_all_existing_playlists() {
        let playlist_searcher = PlaylistSearcher::new(playlist_repository_with(&[
            playlist("PL1", "music/first"),
            playlist("PL2", "music/second"),
        ]));

        let response = list(playlist_searcher).await;

        assert_eq!(
            response,
            Ok(vec![
                playlist_response("PL1", "music/first"),
                playlist_response("PL2", "music/second"),
            ])
        );
    }

    #[tokio::test]
    async fn it_should_return_204_and_add_the_listed_videos_when_reconciling_a_playlist() {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let playlist_video_repository =
            Arc::new(FakePlaylistVideoRepository::new(video_repository.clone()));
        let task_repository = Arc::new(FakeTaskRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let video_reconciler = video_reconciler(
            playlist_repository_with(&[playlist("PL1", DEFAULT_PATH)]),
            video_repository.clone(),
            playlist_video_repository.clone(),
            vec![playlist_item("vid1", "One", 0)],
            task_repository.clone(),
            event_publisher.clone(),
        );

        let response = reconcile(video_reconciler, "PL1").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        let videos = video_repository.videos.lock().unwrap().clone();
        let video_id = videos[0].id.clone();
        assert_eq!(
            videos,
            vec![Video {
                id: video_id.clone(),
                ..Video::create(VideoId::new("vid1").unwrap(), "One", fixed_timestamp())
            }]
        );
        assert_eq!(
            *playlist_video_repository.playlist_videos.lock().unwrap(),
            vec![PlaylistVideo::create_with_position(
                PlaylistId::new("PL1").unwrap(),
                video_id.clone(),
                0,
                fixed_timestamp()
            )]
        );
        assert_eq!(task_repository.scheduled(), vec![]);
        assert_eq!(
            *event_publisher.published.lock().unwrap(),
            vec![DomainEvent::VideoAddedToPlaylist {
                playlist_id: "PL1".to_string(),
                video_id: video_id.as_str().to_string(),
            }]
        );
    }

    #[tokio::test]
    async fn it_should_remove_stored_videos_no_longer_on_youtube_when_reconciling_a_playlist() {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let playlist_video_repository =
            Arc::new(FakePlaylistVideoRepository::new(video_repository.clone()));
        let (removed, _) = save_playlist_video(
            &video_repository,
            &playlist_video_repository,
            "PL1",
            "vid_old",
            0,
        );
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let video_reconciler = video_reconciler(
            playlist_repository_with(&[playlist("PL1", DEFAULT_PATH)]),
            video_repository.clone(),
            playlist_video_repository.clone(),
            Vec::new(),
            Arc::new(FakeTaskRepository::default()),
            event_publisher.clone(),
        );

        let response = reconcile(video_reconciler, "PL1").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(*video_repository.videos.lock().unwrap(), vec![]);
        assert_eq!(
            *playlist_video_repository.playlist_videos.lock().unwrap(),
            vec![]
        );
        assert_eq!(
            *event_publisher.published.lock().unwrap(),
            vec![DomainEvent::VideoRemovedFromPlaylist {
                playlist_id: "PL1".to_string(),
                video_id: removed.id.as_str().to_string(),
                title: "Video vid_old".to_string(),
                filename: None,
                thumbnail_filename: None,
                was_downloaded: false,
            }]
        );
    }

    #[tokio::test]
    async fn it_should_not_add_a_listed_video_twice_or_schedule_tasks_on_repeated_reconciles() {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let playlist_video_repository =
            Arc::new(FakePlaylistVideoRepository::new(video_repository.clone()));
        let task_repository = Arc::new(FakeTaskRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let video_reconciler = video_reconciler(
            playlist_repository_with(&[playlist("PL1", DEFAULT_PATH)]),
            video_repository.clone(),
            playlist_video_repository.clone(),
            vec![playlist_item("vid1", "One", 0)],
            task_repository.clone(),
            event_publisher.clone(),
        );

        reconcile(video_reconciler.clone(), "PL1").await.unwrap();
        let videos_after_first_reconcile = video_repository.videos.lock().unwrap().clone();
        let playlist_videos_after_first_reconcile = playlist_video_repository
            .playlist_videos
            .lock()
            .unwrap()
            .clone();
        let events_after_first_reconcile = event_publisher.published.lock().unwrap().clone();
        let response = reconcile(video_reconciler, "PL1").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            *video_repository.videos.lock().unwrap(),
            videos_after_first_reconcile
        );
        assert_eq!(
            *playlist_video_repository.playlist_videos.lock().unwrap(),
            playlist_videos_after_first_reconcile
        );
        assert_eq!(task_repository.scheduled(), vec![]);
        assert_eq!(
            *event_publisher.published.lock().unwrap(),
            events_after_first_reconcile
        );
    }

    #[tokio::test]
    async fn it_should_leave_an_existing_pending_reconcile_task_untouched() {
        let task_repository = Arc::new(FakeTaskRepository::default());
        let existing_task = Task::ReconcilePlaylist {
            playlist_id: "PL1".to_string(),
        };
        let existing_run_at = fixed_timestamp() + chrono::Duration::seconds(1800);
        task_repository
            .schedule(&existing_task, existing_run_at)
            .unwrap();
        let video_repository = Arc::new(FakeVideoRepository::default());
        let video_reconciler = video_reconciler(
            playlist_repository_with(&[playlist("PL1", DEFAULT_PATH)]),
            video_repository.clone(),
            Arc::new(FakePlaylistVideoRepository::new(video_repository)),
            Vec::new(),
            task_repository.clone(),
            Arc::new(FakeEventPublisher::default()),
        );

        let response = reconcile(video_reconciler, "PL1").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            task_repository.scheduled(),
            vec![(existing_task, existing_run_at)]
        );
    }

    #[tokio::test]
    async fn it_should_return_204_and_do_nothing_when_reconciling_a_nonexistent_playlist() {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let playlist_video_repository =
            Arc::new(FakePlaylistVideoRepository::new(video_repository.clone()));
        let task_repository = Arc::new(FakeTaskRepository::default());
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let video_reconciler = video_reconciler(
            Arc::new(FakePlaylistRepository::default()),
            video_repository.clone(),
            playlist_video_repository.clone(),
            vec![playlist_item("vid1", "One", 0)],
            task_repository.clone(),
            event_publisher.clone(),
        );

        let response = reconcile(video_reconciler, "PL404").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(*video_repository.videos.lock().unwrap(), vec![]);
        assert_eq!(
            *playlist_video_repository.playlist_videos.lock().unwrap(),
            vec![]
        );
        assert_eq!(task_repository.scheduled(), vec![]);
        assert_eq!(*event_publisher.published.lock().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_return_400_when_reconciling_with_an_invalid_playlist_id() {
        let response = reconcile(any_video_reconciler(), " ").await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "YouTube playlist ID must not be empty"
            ))
        );
    }

    /// A creator for tests whose request is rejected before reaching it.
    fn any_playlist_creator() -> PlaylistCreator {
        PlaylistCreator::new(
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakeYoutubePlaylistRepository { exists: true }),
            Arc::new(FakeEventPublisher::default()),
            Arc::new(FixedClock(fixed_timestamp())),
        )
    }

    /// A deleter for tests whose request is rejected before reaching it.
    fn any_playlist_deleter() -> PlaylistDeleter {
        let video_repository = Arc::new(FakeVideoRepository::default());
        PlaylistDeleter::new(
            Arc::new(FakePlaylistRepository::default()),
            video_repository.clone(),
            Arc::new(FakePlaylistVideoRepository::new(video_repository)),
            Arc::new(FakeEventPublisher::default()),
        )
    }

    /// Builds a reconciler around the fakes a test seeds and asserts; the
    /// remaining ports (metadata, files, thumbnails) are ones no playlist
    /// reconcile test observes.
    fn video_reconciler(
        playlist_repository: Arc<FakePlaylistRepository>,
        video_repository: Arc<FakeVideoRepository>,
        playlist_video_repository: Arc<FakePlaylistVideoRepository>,
        playlist_items: Vec<YoutubePlaylistItem>,
        task_repository: Arc<FakeTaskRepository>,
        event_publisher: Arc<FakeEventPublisher>,
    ) -> VideoReconciler {
        let thumbnail_fetcher = Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        VideoReconciler::new(
            playlist_repository,
            video_repository,
            playlist_video_repository,
            Arc::new(FakeYoutubePlaylistItemsRepository {
                videos: Mutex::new(playlist_items),
            }),
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

    /// A reconciler for tests whose request is rejected before reaching it.
    fn any_video_reconciler() -> VideoReconciler {
        let video_repository = Arc::new(FakeVideoRepository::default());
        video_reconciler(
            Arc::new(FakePlaylistRepository::default()),
            video_repository.clone(),
            Arc::new(FakePlaylistVideoRepository::new(video_repository)),
            Vec::new(),
            Arc::new(FakeTaskRepository::default()),
            Arc::new(FakeEventPublisher::default()),
        )
    }

    struct FailingYoutubePlaylistRepository;

    impl YoutubePlaylistRepository for FailingYoutubePlaylistRepository {
        fn exists(&self, _id: &PlaylistId) -> anyhow::Result<bool> {
            anyhow::bail!("YouTube API request failed")
        }
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn playlist(id: &str, path: &str) -> Playlist {
        Playlist::create(
            PlaylistId::new(id).unwrap(),
            PlaylistName::new("My Playlist").unwrap(),
            PlaylistPath::new(path).unwrap(),
            Quality::High,
            PlaylistKind::YoutubeLinked,
            fixed_timestamp(),
        )
    }

    fn playlist_repository_with(playlists: &[Playlist]) -> Arc<FakePlaylistRepository> {
        let repository = FakePlaylistRepository::default();
        for playlist in playlists {
            repository.insert(playlist).unwrap();
        }
        Arc::new(repository)
    }

    fn save_playlist_video(
        video_repository: &FakeVideoRepository,
        playlist_video_repository: &FakePlaylistVideoRepository,
        playlist_id: &str,
        youtube_id: &str,
        position: i64,
    ) -> (Video, PlaylistVideo) {
        let video = Video::create(
            VideoId::new(youtube_id).unwrap(),
            format!("Video {youtube_id}"),
            fixed_timestamp(),
        );
        let playlist_video = PlaylistVideo::create_with_position(
            PlaylistId::new(playlist_id).unwrap(),
            video.id.clone(),
            position,
            fixed_timestamp(),
        );
        video_repository.save(&video).unwrap();
        playlist_video_repository.save(&playlist_video).unwrap();
        (video, playlist_video)
    }

    fn playlist_item(video_id: &str, title: &str, position: i64) -> YoutubePlaylistItem {
        YoutubePlaylistItem {
            video_id: video_id.to_string(),
            title: title.to_string(),
            position,
        }
    }

    fn playlist_created(playlist_id: &str) -> DomainEvent {
        DomainEvent::PlaylistCreated {
            playlist_id: playlist_id.to_string(),
        }
    }

    fn create_request(playlist: &str) -> CreatePlaylistRequest {
        CreatePlaylistRequest {
            playlist: playlist.to_string(),
            name: "My Playlist".to_string(),
            path: Some(DEFAULT_PATH.to_string()),
            quality: Some("high".to_string()),
        }
    }

    fn playlist_response(id: &str, path: &str) -> PlaylistResponse {
        PlaylistResponse {
            id: id.to_string(),
            name: "My Playlist".to_string(),
            path: path.to_string(),
            quality: "high".to_string(),
            kind: "youtube_linked".to_string(),
            created_at: fixed_timestamp(),
        }
    }

    async fn create(
        playlist_creator: PlaylistCreator,
        request: CreatePlaylistRequest,
    ) -> Result<(StatusCode, PlaylistResponse), ApiError> {
        create_playlist(State(playlist_creator), Json(request))
            .await
            .map(|(status, Json(playlist))| (status, playlist))
    }

    async fn delete(playlist_deleter: PlaylistDeleter, id: &str) -> Result<StatusCode, ApiError> {
        delete_playlist(State(playlist_deleter), Path(id.to_string())).await
    }

    async fn list(playlist_searcher: PlaylistSearcher) -> Result<Vec<PlaylistResponse>, ApiError> {
        list_playlists(State(playlist_searcher))
            .await
            .map(|Json(playlists)| playlists)
    }

    async fn reconcile(
        video_reconciler: VideoReconciler,
        id: &str,
    ) -> Result<StatusCode, ApiError> {
        reconcile_playlist(State(video_reconciler), Path(id.to_string())).await
    }
}
