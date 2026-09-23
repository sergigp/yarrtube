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
    let id = PlaylistId::from_url_or_id(request.playlist).map_err(ApiError::bad_request)?;
    let name = PlaylistName::new(request.name).map_err(ApiError::bad_request)?;
    let path =
        PlaylistPath::new(required(request.path, MISSING_PATH)?).map_err(ApiError::bad_request)?;
    let quality =
        Quality::new(required(request.quality, MISSING_QUALITY)?).map_err(ApiError::bad_request)?;

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
    let id = PlaylistId::new(id).map_err(ApiError::bad_request)?;

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
    let id = PlaylistId::new(id).map_err(ApiError::bad_request)?;

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
    use crate::application::subscribers::delete_playlist_files_on_playlist_deleted::DeletePlaylistFilesOnPlaylistDeleted;
    use crate::application::tasks::delete_playlist_files_task::DeletePlaylistFilesTask;
    use crate::domain::event::DomainEvent;
    use crate::domain::playlist::{Playlist, PlaylistKind};
    use crate::domain::playlist_video::PlaylistVideo;
    use crate::domain::services::{ThumbnailFetcher, VideoFileDeleter};
    use crate::domain::shared::VideoId;
    use crate::domain::task::Task;
    use crate::domain::video::Video;
    use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
    use crate::infrastructure::repositories::filesystem_video_file_repository::{
        FakeVideoFileRepository, FilesystemVideoFileRepository,
    };
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
    use crate::infrastructure::repositories::task_handler::TaskHandler;
    use crate::infrastructure::repositories::youtube_metadata_repository::FakeYoutubeMetadataRepository;
    use crate::infrastructure::repositories::youtube_playlist_items_repository::{
        FakeYoutubePlaylistItemsRepository, YoutubePlaylistItem,
    };
    use crate::infrastructure::repositories::youtube_playlist_repository::FakeYoutubePlaylistRepository;
    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
    use crate::infrastructure::shared::domain_events::event_publisher::FakeEventPublisher;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use crate::infrastructure::shared::ytdlp::test_support::unique_temp_dir;
    use chrono::{DateTime, Utc};
    use std::sync::{Arc, Mutex};

    const DEFAULT_PATH: &str = "music/chill";

    #[tokio::test]
    async fn it_should_return_201_when_creating_a_new_playlist() {
        let (creator, _event_publisher) = playlist_creator(FakePlaylistRepository::default());

        let response = create(creator, create_request("PL1")).await;

        assert_eq!(
            response,
            Ok((StatusCode::CREATED, playlist_response("PL1", DEFAULT_PATH)))
        );
    }

    #[tokio::test]
    async fn it_should_return_200_when_creating_a_playlist_that_already_exists() {
        let (creator, _event_publisher) =
            playlist_creator(repository_with(&[playlist("PL1", DEFAULT_PATH)]));

        let response = create(
            creator,
            CreatePlaylistRequest {
                name: "Different Name".to_string(),
                ..create_request("PL1")
            },
        )
        .await;

        assert_eq!(
            response,
            Ok((StatusCode::OK, playlist_response("PL1", DEFAULT_PATH)))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_name_is_invalid() {
        let response = create_rejection(CreatePlaylistRequest {
            name: String::new(),
            ..create_request("PL1")
        })
        .await;

        assert_eq!(
            response,
            Err(ApiError::bad_request("Playlist name must not be empty"))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_path_is_missing() {
        let response = create_rejection(CreatePlaylistRequest {
            path: None,
            ..create_request("PL1")
        })
        .await;

        assert_eq!(
            response,
            Err(ApiError::bad_request("Playlist path must not be empty"))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_path_is_absolute() {
        let response = create_rejection(CreatePlaylistRequest {
            path: Some("/absolute/path".to_string()),
            ..create_request("PL1")
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
        let response = create_rejection(CreatePlaylistRequest {
            path: Some("a/../b".to_string()),
            ..create_request("PL1")
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
        let response = create_rejection(CreatePlaylistRequest {
            path: Some("a//b".to_string()),
            ..create_request("PL1")
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
    async fn it_should_return_400_when_the_path_is_already_used_by_another_playlist() {
        let (creator, _event_publisher) =
            playlist_creator(repository_with(&[playlist("PL1", "shared/path")]));

        let response = create(
            creator,
            CreatePlaylistRequest {
                path: Some("shared/path".to_string()),
                ..create_request("PL2")
            },
        )
        .await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "playlist path \"shared/path\" is already used by another playlist"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_keep_the_existing_path_when_creating_a_duplicate_with_a_different_path() {
        let (creator, _event_publisher) =
            playlist_creator(repository_with(&[playlist("PL1", "original/path")]));

        let response = create(
            creator,
            CreatePlaylistRequest {
                path: Some("different/path".to_string()),
                ..create_request("PL1")
            },
        )
        .await;

        assert_eq!(
            response,
            Ok((StatusCode::OK, playlist_response("PL1", "original/path")))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_quality_is_missing() {
        let response = create_rejection(CreatePlaylistRequest {
            quality: None,
            ..create_request("PL1")
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
        let response = create_rejection(CreatePlaylistRequest {
            quality: Some("ultra".to_string()),
            ..create_request("PL1")
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
    async fn it_should_keep_the_existing_quality_when_creating_a_duplicate_with_a_different_quality()
     {
        let (creator, _event_publisher) =
            playlist_creator(repository_with(&[playlist("PL1", DEFAULT_PATH)]));

        let response = create(
            creator,
            CreatePlaylistRequest {
                quality: Some("low".to_string()),
                ..create_request("PL1")
            },
        )
        .await;

        assert_eq!(
            response,
            Ok((StatusCode::OK, playlist_response("PL1", DEFAULT_PATH)))
        );
    }

    #[tokio::test]
    async fn it_should_return_201_when_creating_a_playlist_from_a_youtube_url() {
        let (creator, _event_publisher) = playlist_creator(FakePlaylistRepository::default());

        let response = create(
            creator,
            create_request("https://www.youtube.com/playlist?list=PL1"),
        )
        .await;

        assert_eq!(
            response,
            Ok((StatusCode::CREATED, playlist_response("PL1", DEFAULT_PATH)))
        );
    }

    #[tokio::test]
    async fn it_should_return_201_when_creating_a_playlist_from_a_watch_url_with_a_list_param() {
        let (creator, _event_publisher) = playlist_creator(FakePlaylistRepository::default());

        let response = create(
            creator,
            create_request("https://youtube.com/watch?v=vid1&list=PL1"),
        )
        .await;

        assert_eq!(
            response,
            Ok((StatusCode::CREATED, playlist_response("PL1", DEFAULT_PATH)))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_url_is_not_a_recognized_youtube_url() {
        let response =
            create_rejection(create_request("https://example.com/playlist?list=PL1")).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "\"https://example.com/playlist?list=PL1\" is not a recognized YouTube playlist URL"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_youtube_url_is_missing_the_list_parameter() {
        let response =
            create_rejection(create_request("https://www.youtube.com/watch?v=vid1")).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "YouTube URL is missing a \"list\" query parameter (got \"https://www.youtube.com/watch?v=vid1\")"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_playlist_value_is_empty() {
        let response = create_rejection(create_request("")).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Playlist ID or URL must not be empty"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_youtube_playlist_does_not_exist() {
        let (creator, _event_publisher) =
            playlist_creator_with(false, FakePlaylistRepository::default());

        let response = create(creator, create_request("PL404")).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "YouTube playlist PL404 does not exist or is not accessible"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_record_a_playlist_created_event_only_once_for_repeated_creation() {
        let (creator, event_publisher) = playlist_creator(FakePlaylistRepository::default());

        create(creator.clone(), create_request("PL1"))
            .await
            .unwrap();
        create(
            creator,
            CreatePlaylistRequest {
                name: "First Again".to_string(),
                ..create_request("PL1")
            },
        )
        .await
        .unwrap();

        assert_eq!(
            *event_publisher.published.lock().unwrap(),
            vec![DomainEvent::PlaylistCreated {
                playlist_id: "PL1".to_string()
            }]
        );
    }

    #[tokio::test]
    async fn it_should_return_204_when_deleting_an_existing_playlist() {
        let (deleter, _event_publisher) =
            playlist_deleter(repository_with(&[playlist("PL1", DEFAULT_PATH)]));

        let response = delete(deleter, "PL1").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
    }

    #[tokio::test]
    async fn it_should_record_a_playlist_deleted_event_on_successful_deletion() {
        let (deleter, event_publisher) =
            playlist_deleter(repository_with(&[playlist("PL1", DEFAULT_PATH)]));

        delete(deleter, "PL1").await.unwrap();

        assert_eq!(
            *event_publisher.published.lock().unwrap(),
            vec![DomainEvent::PlaylistDeleted {
                playlist_id: "PL1".to_string(),
                path: DEFAULT_PATH.to_string(),
            }]
        );
    }

    #[tokio::test]
    async fn it_should_delete_every_video_record_for_a_deleted_youtube_linked_playlist() {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let playlist_video_repository = Arc::new(FakePlaylistVideoRepository::default());
        let video = Video::create(VideoId::new("vid1").unwrap(), "My Video", fixed_timestamp());
        video_repository.save(&video).unwrap();
        playlist_video_repository
            .save(&PlaylistVideo::create(
                PlaylistId::new("PL1").unwrap(),
                video.id.clone(),
                fixed_timestamp(),
            ))
            .unwrap();
        let (deleter, _event_publisher) = playlist_deleter_with_videos(
            repository_with(&[playlist("PL1", DEFAULT_PATH)]),
            video_repository.clone(),
            playlist_video_repository.clone(),
        );

        let response = delete(deleter, "PL1").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(*video_repository.videos.lock().unwrap(), vec![]);
        assert_eq!(
            playlist_video_repository
                .list_for_playlist(&PlaylistId::new("PL1").unwrap())
                .unwrap(),
            vec![]
        );
    }

    #[tokio::test]
    async fn it_should_return_400_when_deleting_a_missing_playlist() {
        let (deleter, _event_publisher) = playlist_deleter(FakePlaylistRepository::default());

        let response = delete(deleter, "PL404").await;

        assert_eq!(
            response,
            Err(ApiError::bad_request("playlist PL404 not found"))
        );
    }

    #[tokio::test]
    async fn it_should_return_an_empty_array_when_no_playlists_exist() {
        let response = list(FakePlaylistRepository::default()).await;

        assert_eq!(response, Ok(vec![]));
    }

    #[tokio::test]
    async fn it_should_return_all_existing_playlists() {
        let response = list(repository_with(&[
            playlist("PL1", "music/first"),
            playlist("PL2", "music/second"),
        ]))
        .await;

        assert_eq!(
            response,
            Ok(vec![
                playlist_response("PL1", "music/first"),
                playlist_response("PL2", "music/second"),
            ])
        );
    }

    #[tokio::test]
    async fn it_should_delete_the_playlist_output_directory_from_disk_when_the_playlist_is_deleted()
    {
        let videos_root = unique_temp_dir("cascade-delete-playlist-e2e");
        let output_dir = videos_root.join("music/chill");
        std::fs::create_dir_all(&output_dir).unwrap();
        std::fs::write(output_dir.join("My Video.mp4"), b"fake video bytes").unwrap();
        let video_repository = Arc::new(FakeVideoRepository::default());
        video_repository
            .save(
                &Video::create(VideoId::new("vid1").unwrap(), "My Video", fixed_timestamp())
                    .start_download(fixed_timestamp())
                    .mark_downloaded(Quality::High, "My Video.mp4", None, None, fixed_timestamp()),
            )
            .unwrap();
        let (deleter, event_publisher) = playlist_deleter_with_videos(
            repository_with(&[playlist("PL1", "music/chill")]),
            video_repository,
            Arc::new(FakePlaylistVideoRepository::default()),
        );

        delete(deleter, "PL1").await.unwrap();
        let playlist_deleted_payload = event_publisher
            .published
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .payload()
            .to_string();
        let subscriber_task_repository = Arc::new(FakeTaskRepository::default());
        DeletePlaylistFilesOnPlaylistDeleted::new(
            subscriber_task_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        )
        .handle(&playlist_deleted_payload)
        .unwrap();
        let (task, _run_at) = subscriber_task_repository.scheduled.lock().unwrap()[0].clone();
        DeletePlaylistFilesTask::new(VideoFileDeleter::new(
            Arc::new(FilesystemVideoFileRepository),
            videos_root.to_str().unwrap(),
        ))
        .handle(&task.payload().to_string(), false)
        .unwrap();

        assert!(!output_dir.exists());
        std::fs::remove_dir_all(&videos_root).ok();
    }

    #[tokio::test]
    async fn it_should_return_204_and_apply_membership_changes_when_reconciling_a_youtube_linked_playlist()
     {
        let (reconciler, video_repository, task_repository) = video_reconciler(
            repository_with(&[playlist("PL1", DEFAULT_PATH)]),
            vec![YoutubePlaylistItem {
                video_id: "vid1".to_string(),
                title: "One".to_string(),
                position: 0,
            }],
        );

        let response = reconcile(reconciler, "PL1").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        let stored_youtube_ids: Vec<String> = video_repository
            .videos
            .lock()
            .unwrap()
            .iter()
            .map(|video| video.youtube_id.as_str().to_string())
            .collect();
        assert_eq!(stored_youtube_ids, vec!["vid1"]);
        assert_eq!(*task_repository.scheduled.lock().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_not_schedule_any_task_when_reconciling_repeatedly_on_demand() {
        let (reconciler, _video_repository, task_repository) =
            video_reconciler(repository_with(&[playlist("PL1", DEFAULT_PATH)]), vec![]);

        for _ in 0..3 {
            reconcile(reconciler.clone(), "PL1").await.unwrap();
        }

        assert_eq!(*task_repository.scheduled.lock().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_leave_an_existing_pending_reconcile_task_untouched() {
        let (reconciler, _video_repository, task_repository) =
            video_reconciler(repository_with(&[playlist("PL1", DEFAULT_PATH)]), vec![]);
        let existing_task = Task::ReconcilePlaylist {
            playlist_id: "PL1".to_string(),
        };
        let existing_run_at = fixed_timestamp() + chrono::Duration::seconds(1800);
        task_repository
            .schedule(&existing_task, existing_run_at)
            .unwrap();

        reconcile(reconciler, "PL1").await.unwrap();

        assert_eq!(
            *task_repository.scheduled.lock().unwrap(),
            vec![(existing_task, existing_run_at)]
        );
    }

    #[tokio::test]
    async fn it_should_return_204_and_do_nothing_when_reconciling_a_nonexistent_playlist() {
        let (reconciler, video_repository, task_repository) =
            video_reconciler(FakePlaylistRepository::default(), vec![]);

        let response = reconcile(reconciler, "PL404").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(*video_repository.videos.lock().unwrap(), vec![]);
        assert_eq!(*task_repository.scheduled.lock().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_return_400_when_reconciling_with_an_invalid_playlist_id() {
        let (reconciler, _video_repository, _task_repository) =
            video_reconciler(FakePlaylistRepository::default(), vec![]);

        let response = reconcile(reconciler, " ").await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "YouTube playlist ID must not be empty"
            ))
        );
    }

    fn playlist_creator(
        repository: FakePlaylistRepository,
    ) -> (PlaylistCreator, Arc<FakeEventPublisher>) {
        playlist_creator_with(true, repository)
    }

    fn playlist_creator_with(
        youtube_exists: bool,
        repository: FakePlaylistRepository,
    ) -> (PlaylistCreator, Arc<FakeEventPublisher>) {
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let creator = PlaylistCreator::new(
            Arc::new(repository),
            Arc::new(FakeYoutubePlaylistRepository {
                exists: youtube_exists,
            }),
            event_publisher.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        );
        (creator, event_publisher)
    }

    fn playlist_deleter(
        repository: FakePlaylistRepository,
    ) -> (PlaylistDeleter, Arc<FakeEventPublisher>) {
        playlist_deleter_with_videos(
            repository,
            Arc::new(FakeVideoRepository::default()),
            Arc::new(FakePlaylistVideoRepository::default()),
        )
    }

    fn playlist_deleter_with_videos(
        repository: FakePlaylistRepository,
        video_repository: Arc<FakeVideoRepository>,
        playlist_video_repository: Arc<FakePlaylistVideoRepository>,
    ) -> (PlaylistDeleter, Arc<FakeEventPublisher>) {
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let deleter = PlaylistDeleter::new(
            Arc::new(repository),
            video_repository,
            playlist_video_repository,
            event_publisher.clone(),
        );
        (deleter, event_publisher)
    }

    fn video_reconciler(
        repository: FakePlaylistRepository,
        playlist_items: Vec<YoutubePlaylistItem>,
    ) -> (
        VideoReconciler,
        Arc<FakeVideoRepository>,
        Arc<FakeTaskRepository>,
    ) {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let task_repository = Arc::new(FakeTaskRepository::default());
        let thumbnail_fetcher = Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let reconciler = VideoReconciler::new(
            Arc::new(repository),
            video_repository.clone(),
            Arc::new(FakePlaylistVideoRepository::default()),
            Arc::new(FakeYoutubePlaylistItemsRepository {
                videos: Mutex::new(playlist_items),
            }),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(FakeVideoMetadataRepository::default()),
            Arc::new(FakeEventPublisher::default()),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            thumbnail_fetcher,
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );
        (reconciler, video_repository, task_repository)
    }

    async fn create(
        creator: PlaylistCreator,
        request: CreatePlaylistRequest,
    ) -> Result<(StatusCode, PlaylistResponse), ApiError> {
        create_playlist(State(creator), Json(request))
            .await
            .map(|(status, Json(playlist))| (status, playlist))
    }

    async fn create_rejection(
        request: CreatePlaylistRequest,
    ) -> Result<(StatusCode, PlaylistResponse), ApiError> {
        let (creator, _event_publisher) = playlist_creator(FakePlaylistRepository::default());
        create(creator, request).await
    }

    async fn delete(deleter: PlaylistDeleter, id: &str) -> Result<StatusCode, ApiError> {
        delete_playlist(State(deleter), Path(id.to_string())).await
    }

    async fn list(repository: FakePlaylistRepository) -> Result<Vec<PlaylistResponse>, ApiError> {
        list_playlists(State(PlaylistSearcher::new(Arc::new(repository))))
            .await
            .map(|Json(playlists)| playlists)
    }

    async fn reconcile(reconciler: VideoReconciler, id: &str) -> Result<StatusCode, ApiError> {
        reconcile_playlist(State(reconciler), Path(id.to_string())).await
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

    fn repository_with(playlists: &[Playlist]) -> FakePlaylistRepository {
        let repository = FakePlaylistRepository::default();
        for playlist in playlists {
            repository.insert(playlist).unwrap();
        }
        repository
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }
}
