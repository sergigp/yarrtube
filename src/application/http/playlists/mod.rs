pub mod dto;

use super::blocking::run_blocking;
use super::error::ApiError;
use super::validation::{MISSING_QUALITY, required};
use crate::domain::playlist::{
    CreatePlaylistError, DeletePlaylistError, PlaylistName, PlaylistPath,
};
use crate::domain::services::{
    CreatePlaylistOutcome, PlaylistCreator, PlaylistDeleter, PlaylistSearcher,
    PlaylistVideoReconciler,
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
    State(playlist_video_reconciler): State<PlaylistVideoReconciler>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = PlaylistId::new(id)?;

    run_blocking(move || playlist_video_reconciler.force_reconcile(id))
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
    use crate::domain::event::{DomainEvent, ScheduledEvent};
    use crate::domain::playlist::{Playlist, PlaylistKind};
    use crate::domain::playlist_video::PlaylistVideo;
    use crate::domain::services::ThumbnailFetcher;
    use crate::domain::shared::VideoId;
    use crate::domain::task::{ScheduledTask, Task, TaskStatus};
    use crate::domain::video::Video;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_playlist_repository::{
        PlaylistRepository, SqlitePlaylistRepository,
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
    use crate::infrastructure::repositories::youtube_metadata_repository::FakeYoutubeMetadataRepository;
    use crate::infrastructure::repositories::youtube_playlist_items_repository::{
        FakeYoutubePlaylistItemsRepository, YoutubePlaylistItem,
    };
    use crate::infrastructure::repositories::youtube_playlist_repository::{
        FakeYoutubePlaylistRepository, YoutubePlaylistRepository,
    };
    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
    use crate::infrastructure::shared::domain_events::event_publisher::SqliteEventPublisher;
    use crate::infrastructure::shared::domain_events::event_repository::{
        EventRepository, SqliteEventRepository,
    };
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};

    const DEFAULT_PATH: &str = "music/chill";

    #[tokio::test]
    async fn it_should_create_a_playlist() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        let playlist_creator = PlaylistCreator::new(
            playlist_repository.clone(),
            Arc::new(FakeYoutubePlaylistRepository { exists: true }),
            event_publisher(&db),
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
            event_repository.list_eligible().unwrap(),
            vec![pending_event(1, playlist_created("PL1"))]
        );
    }

    #[tokio::test]
    async fn it_should_create_a_playlist_from_a_youtube_url() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        let playlist_creator = PlaylistCreator::new(
            playlist_repository.clone(),
            Arc::new(FakeYoutubePlaylistRepository { exists: true }),
            event_publisher(&db),
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
            event_repository.list_eligible().unwrap(),
            vec![pending_event(1, playlist_created("PL1"))]
        );
    }

    #[tokio::test]
    async fn it_should_return_the_existing_playlist_if_already_created() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        playlist_repository
            .insert(&playlist("PL1", DEFAULT_PATH))
            .unwrap();
        let playlist_creator = PlaylistCreator::new(
            playlist_repository.clone(),
            Arc::new(FakeYoutubePlaylistRepository { exists: true }),
            event_publisher(&db),
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
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_publish_playlist_created_only_once() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        let playlist_creator = PlaylistCreator::new(
            playlist_repository.clone(),
            Arc::new(FakeYoutubePlaylistRepository { exists: true }),
            event_publisher(&db),
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
            event_repository.list_eligible().unwrap(),
            vec![pending_event(1, playlist_created("PL1"))]
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_create_if_invalid_playlist_provided() {
        let response = create(any_playlist_creator(), create_request("")).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Playlist ID or URL must not be empty"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_create_if_path_missing() {
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
    async fn it_should_fail_to_create_if_quality_missing() {
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
    async fn it_should_fail_to_create_if_path_already_in_use() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        playlist_repository
            .insert(&playlist("PL1", "shared/path"))
            .unwrap();
        let playlist_creator = PlaylistCreator::new(
            playlist_repository.clone(),
            Arc::new(FakeYoutubePlaylistRepository { exists: true }),
            event_publisher(&db),
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
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_fail_to_create_if_playlist_not_found_on_youtube() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        let playlist_creator = PlaylistCreator::new(
            playlist_repository.clone(),
            Arc::new(FakeYoutubePlaylistRepository { exists: false }),
            event_publisher(&db),
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
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_fail_to_create_if_youtube_lookup_fails() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        let playlist_creator = PlaylistCreator::new(
            playlist_repository.clone(),
            Arc::new(FailingYoutubePlaylistRepository),
            event_publisher(&db),
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
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_delete_a_playlist() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        playlist_repository
            .insert(&playlist("PL1", DEFAULT_PATH))
            .unwrap();
        let playlist_deleter = PlaylistDeleter::new(
            playlist_repository.clone(),
            Arc::new(SqliteVideoRepository::new(db.connection())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection())),
            event_publisher(&db),
        );

        let response = delete(playlist_deleter, "PL1").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(playlist_repository.list().unwrap(), vec![]);
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(
                1,
                DomainEvent::PlaylistDeleted {
                    playlist_id: "PL1".to_string(),
                    path: DEFAULT_PATH.to_string(),
                }
            )]
        );
    }

    #[tokio::test]
    async fn it_should_delete_only_the_deleted_playlist_videos() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        playlist_repository
            .insert(&playlist("PL1", DEFAULT_PATH))
            .unwrap();
        playlist_repository
            .insert(&playlist("PL2", "music/other"))
            .unwrap();
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            "PL1",
            "vid1",
            0,
        );
        save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            "PL1",
            "vid2",
            1,
        );
        let (other_video, other_playlist_video) = save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            "PL2",
            "vid3",
            0,
        );
        let playlist_deleter = PlaylistDeleter::new(
            playlist_repository.clone(),
            video_repository.clone(),
            playlist_video_repository.clone(),
            event_publisher(&db),
        );

        let response = delete(playlist_deleter, "PL1").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            playlist_repository.list().unwrap(),
            vec![playlist("PL2", "music/other")]
        );
        assert_eq!(video_repository.list().unwrap(), vec![other_video]);
        assert_eq!(
            playlist_video_repository
                .list_for_playlist(&playlist_id("PL1"))
                .unwrap(),
            vec![]
        );
        assert_eq!(
            playlist_video_repository
                .list_for_playlist(&playlist_id("PL2"))
                .unwrap(),
            vec![other_playlist_video]
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_delete_a_missing_playlist() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        playlist_repository
            .insert(&playlist("PL1", DEFAULT_PATH))
            .unwrap();
        let playlist_deleter = PlaylistDeleter::new(
            playlist_repository.clone(),
            Arc::new(SqliteVideoRepository::new(db.connection())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection())),
            event_publisher(&db),
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
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_fail_to_delete_if_invalid_playlist_id_provided() {
        let response = delete(any_playlist_deleter(), " ").await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "YouTube playlist ID must not be empty"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_list_no_playlists() {
        let db = TestDatabase::new();
        let playlist_searcher =
            PlaylistSearcher::new(Arc::new(SqlitePlaylistRepository::new(db.connection())));

        let response = list(playlist_searcher).await;

        assert_eq!(response, Ok(vec![]));
    }

    #[tokio::test]
    async fn it_should_list_all_playlists() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        playlist_repository
            .insert(&playlist("PL1", "music/first"))
            .unwrap();
        playlist_repository
            .insert(&playlist("PL2", "music/second"))
            .unwrap();
        let playlist_searcher = PlaylistSearcher::new(playlist_repository);

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
    async fn it_should_add_new_videos_on_reconcile() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let task_repository = task_repository(&db);
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        playlist_repository
            .insert(&playlist("PL1", DEFAULT_PATH))
            .unwrap();
        let playlist_video_reconciler = playlist_video_reconciler(
            &db,
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository.clone(),
            vec![playlist_item("vid1", "One", 0)],
            task_repository.clone(),
        );

        let response = reconcile(playlist_video_reconciler, "PL1").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        let videos = video_repository.list().unwrap();
        let video_id = videos[0].id.clone();
        assert_eq!(
            videos,
            vec![Video {
                id: video_id.clone(),
                ..Video::create(VideoId::new("vid1").unwrap(), "One", fixed_timestamp())
            }]
        );
        assert_eq!(
            playlist_video_repository
                .list_for_playlist(&playlist_id("PL1"))
                .unwrap(),
            vec![PlaylistVideo {
                id: 1,
                ..PlaylistVideo::create_with_position(
                    playlist_id("PL1"),
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
                DomainEvent::VideoAddedToPlaylist {
                    playlist_id: "PL1".to_string(),
                    video_id: video_id.as_str().to_string(),
                }
            )]
        );
    }

    #[tokio::test]
    async fn it_should_remove_videos_gone_from_youtube_on_reconcile() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        playlist_repository
            .insert(&playlist("PL1", DEFAULT_PATH))
            .unwrap();
        let (removed, _) = save_playlist_video(
            video_repository.as_ref(),
            playlist_video_repository.as_ref(),
            "PL1",
            "vid_old",
            0,
        );
        let playlist_video_reconciler = playlist_video_reconciler(
            &db,
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository.clone(),
            Vec::new(),
            task_repository(&db),
        );

        let response = reconcile(playlist_video_reconciler, "PL1").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(
            playlist_video_repository
                .list_for_playlist(&playlist_id("PL1"))
                .unwrap(),
            vec![]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![pending_event(
                1,
                DomainEvent::VideoRemovedFromPlaylist {
                    playlist_id: "PL1".to_string(),
                    video_id: removed.id.as_str().to_string(),
                    title: "Video vid_old".to_string(),
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
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let task_repository = task_repository(&db);
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        playlist_repository
            .insert(&playlist("PL1", DEFAULT_PATH))
            .unwrap();
        let playlist_video_reconciler = playlist_video_reconciler(
            &db,
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository.clone(),
            vec![playlist_item("vid1", "One", 0)],
            task_repository.clone(),
        );

        reconcile(playlist_video_reconciler.clone(), "PL1")
            .await
            .unwrap();
        let videos_after_first_reconcile = video_repository.list().unwrap();
        let playlist_videos_after_first_reconcile = playlist_video_repository
            .list_for_playlist(&playlist_id("PL1"))
            .unwrap();
        let events_after_first_reconcile = event_repository.list_eligible().unwrap();
        let response = reconcile(playlist_video_reconciler, "PL1").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            video_repository.list().unwrap(),
            videos_after_first_reconcile
        );
        assert_eq!(
            playlist_video_repository
                .list_for_playlist(&playlist_id("PL1"))
                .unwrap(),
            playlist_videos_after_first_reconcile
        );
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            events_after_first_reconcile
        );
    }

    #[tokio::test]
    async fn it_should_not_reschedule_an_already_pending_reconcile() {
        let db = TestDatabase::new();
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.connection()));
        let task_repository = task_repository(&db);
        let existing_task = Task::ReconcilePlaylist {
            playlist_id: "PL1".to_string(),
        };
        let existing_run_at = fixed_timestamp() + chrono::Duration::seconds(1800);
        playlist_repository
            .insert(&playlist("PL1", DEFAULT_PATH))
            .unwrap();
        task_repository
            .schedule(&existing_task, existing_run_at)
            .unwrap();
        let playlist_video_reconciler = playlist_video_reconciler(
            &db,
            playlist_repository,
            Arc::new(SqliteVideoRepository::new(db.connection())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection())),
            Vec::new(),
            task_repository.clone(),
        );

        let response = reconcile(playlist_video_reconciler, "PL1").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_task(1, &existing_task, existing_run_at)]
        );
    }

    #[tokio::test]
    async fn it_should_ignore_reconcile_of_a_missing_playlist() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.connection()));
        let playlist_video_repository =
            Arc::new(SqlitePlaylistVideoRepository::new(db.connection()));
        let task_repository = task_repository(&db);
        let event_repository = SqliteEventRepository::new(db.shared_connection());
        let playlist_video_reconciler = playlist_video_reconciler(
            &db,
            Arc::new(SqlitePlaylistRepository::new(db.connection())),
            video_repository.clone(),
            playlist_video_repository.clone(),
            vec![playlist_item("vid1", "One", 0)],
            task_repository.clone(),
        );

        let response = reconcile(playlist_video_reconciler, "PL404").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(video_repository.list().unwrap(), vec![]);
        assert_eq!(
            playlist_video_repository
                .list_for_playlist(&playlist_id("PL404"))
                .unwrap(),
            vec![]
        );
        assert_eq!(task_repository.list_non_completed().unwrap(), vec![]);
        assert_eq!(event_repository.list_eligible().unwrap(), vec![]);
    }

    #[tokio::test]
    async fn it_should_fail_to_reconcile_if_invalid_playlist_id_provided() {
        let response = reconcile(any_playlist_video_reconciler(), " ").await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "YouTube playlist ID must not be empty"
            ))
        );
    }

    /// A creator for tests whose request is rejected before reaching it. Its
    /// repositories sit on an unmigrated in-memory database, so a request that
    /// wrongly got through would fail loudly instead of passing.
    fn any_playlist_creator() -> PlaylistCreator {
        PlaylistCreator::new(
            Arc::new(SqlitePlaylistRepository::new(unused_connection())),
            Arc::new(FakeYoutubePlaylistRepository { exists: true }),
            unused_event_publisher(),
            Arc::new(FixedClock(fixed_timestamp())),
        )
    }

    /// A deleter for tests whose request is rejected before reaching it (see
    /// `any_playlist_creator`).
    fn any_playlist_deleter() -> PlaylistDeleter {
        PlaylistDeleter::new(
            Arc::new(SqlitePlaylistRepository::new(unused_connection())),
            Arc::new(SqliteVideoRepository::new(unused_connection())),
            Arc::new(SqlitePlaylistVideoRepository::new(unused_connection())),
            unused_event_publisher(),
        )
    }

    /// Builds a reconciler around the repositories a test seeds and asserts;
    /// the remaining ports (metadata, files, thumbnails) are ones no playlist
    /// reconcile test observes. Events go to `db`'s outbox table.
    fn playlist_video_reconciler(
        db: &TestDatabase,
        playlist_repository: Arc<SqlitePlaylistRepository>,
        video_repository: Arc<SqliteVideoRepository>,
        playlist_video_repository: Arc<SqlitePlaylistVideoRepository>,
        playlist_items: Vec<YoutubePlaylistItem>,
        task_repository: Arc<SqliteTaskRepository>,
    ) -> PlaylistVideoReconciler {
        let thumbnail_fetcher = Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            Arc::new(FakeVideoDownloaderRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        PlaylistVideoReconciler::new(
            playlist_repository,
            video_repository,
            playlist_video_repository,
            Arc::new(FakeYoutubePlaylistItemsRepository {
                videos: Mutex::new(playlist_items),
            }),
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

    /// A reconciler for tests whose request is rejected before reaching it
    /// (see `any_playlist_creator`).
    fn any_playlist_video_reconciler() -> PlaylistVideoReconciler {
        let video_repository = Arc::new(SqliteVideoRepository::new(unused_connection()));
        PlaylistVideoReconciler::new(
            Arc::new(SqlitePlaylistRepository::new(unused_connection())),
            video_repository.clone(),
            Arc::new(SqlitePlaylistVideoRepository::new(unused_connection())),
            Arc::new(FakeYoutubePlaylistItemsRepository {
                videos: Mutex::new(Vec::new()),
            }),
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(SqliteVideoMetadataRepository::new(unused_connection())),
            unused_event_publisher(),
            Arc::new(SqliteTaskRepository::new(
                Arc::new(Mutex::new(unused_connection())),
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

    struct FailingYoutubePlaylistRepository;

    impl YoutubePlaylistRepository for FailingYoutubePlaylistRepository {
        fn exists(&self, _id: &PlaylistId) -> anyhow::Result<bool> {
            anyhow::bail!("YouTube API request failed")
        }
    }

    fn unused_connection() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    fn unused_event_publisher() -> Arc<SqliteEventPublisher> {
        Arc::new(SqliteEventPublisher::new(
            Arc::new(Mutex::new(unused_connection())),
            Arc::new(FixedClock(fixed_timestamp())),
        ))
    }

    fn event_publisher(db: &TestDatabase) -> Arc<SqliteEventPublisher> {
        Arc::new(SqliteEventPublisher::new(
            db.shared_connection(),
            Arc::new(FixedClock(fixed_timestamp())),
        ))
    }

    fn task_repository(db: &TestDatabase) -> Arc<SqliteTaskRepository> {
        Arc::new(SqliteTaskRepository::new(
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

    /// The row `SqliteTaskRepository::schedule` writes for `task`, as read
    /// back before the executor has picked it up.
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

    fn playlist_id(id: &str) -> PlaylistId {
        PlaylistId::new(id).unwrap()
    }

    fn playlist(id: &str, path: &str) -> Playlist {
        Playlist::create(
            playlist_id(id),
            PlaylistName::new("My Playlist").unwrap(),
            PlaylistPath::new(path).unwrap(),
            Quality::High,
            PlaylistKind::YoutubeLinked,
            fixed_timestamp(),
        )
    }

    /// Saves a video and its playlist membership, returning both as stored
    /// (the playlist video with its storage-assigned `id`).
    fn save_playlist_video(
        video_repository: &dyn VideoRepository,
        playlist_video_repository: &dyn PlaylistVideoRepository,
        playlist: &str,
        youtube_id: &str,
        position: i64,
    ) -> (Video, PlaylistVideo) {
        let video = Video::create(
            VideoId::new(youtube_id).unwrap(),
            format!("Video {youtube_id}"),
            fixed_timestamp(),
        );
        video_repository.save(&video).unwrap();
        playlist_video_repository
            .save(&PlaylistVideo::create_with_position(
                playlist_id(playlist),
                video.id.clone(),
                position,
                fixed_timestamp(),
            ))
            .unwrap();
        let playlist_video = playlist_video_repository
            .find_by_video(&video.id)
            .unwrap()
            .unwrap();
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
        playlist_video_reconciler: PlaylistVideoReconciler,
        id: &str,
    ) -> Result<StatusCode, ApiError> {
        reconcile_playlist(State(playlist_video_reconciler), Path(id.to_string())).await
    }
}
