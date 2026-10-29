pub mod dto;

use super::blocking::run_blocking;
use super::error::ApiError;
use super::validation::required;
use super::videos::update_watch_state_error;
use crate::domain::category::{
    CategoryColor, CategoryId, CategoryMember, CategoryName, CreateCategoryError,
    DeleteCategoryError, UpdateCategoryError,
};
use crate::domain::channel::ChannelHandle;
use crate::domain::playlist::PlaylistId;
use crate::domain::services::{
    CategoryCreator, CategoryCreatorApi, CategoryDeleter, CategoryDeleterApi, CategoryUpdate,
    CategoryUpdater, CategoryUpdaterApi, CategoryViewSearcher, CategoryViewSearcherApi,
    VideoWatchStateUpdater, VideoWatchStateUpdaterApi,
};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use dto::{
    CategoryListResponse, CategoryResponse, CreateCategoryRequest, SetCategoryMembersRequest,
    UpdateCategoryRequest,
};

const MISSING_CATEGORY_NAME: &str = "Category name must not be empty (missing)";
const MISSING_CATEGORY_NAME_AND_COLOR: &str = "Category name or color is required (both missing)";

pub async fn create_category(
    State(category_creator): State<CategoryCreator>,
    Json(request): Json<CreateCategoryRequest>,
) -> Result<(StatusCode, Json<CategoryResponse>), ApiError> {
    let name = CategoryName::new(required(request.name, MISSING_CATEGORY_NAME)?)?;
    let color = request.color.map(CategoryColor::new).transpose()?;

    match run_blocking(move || category_creator.create(name, color)).await? {
        Ok(category) => Ok((StatusCode::CREATED, Json(CategoryResponse::from(category)))),
        Err(e @ CreateCategoryError::NameTaken(_)) => Err(ApiError::new(StatusCode::CONFLICT, e)),
        Err(e @ CreateCategoryError::Repository(_)) => Err(ApiError::internal(e)),
    }
}

pub async fn list_categories(
    State(category_view_searcher): State<CategoryViewSearcher>,
) -> Result<Json<CategoryListResponse>, ApiError> {
    let categories = run_blocking(move || category_view_searcher.search_all())
        .await?
        .map_err(ApiError::internal)?;
    Ok(Json(CategoryListResponse::from(categories)))
}

pub async fn update_category(
    State(category_updater): State<CategoryUpdater>,
    Path(id): Path<String>,
    Json(request): Json<UpdateCategoryRequest>,
) -> Result<StatusCode, ApiError> {
    let id = CategoryId::new(id)?;
    required(
        request.name.as_ref().or(request.color.as_ref()),
        MISSING_CATEGORY_NAME_AND_COLOR,
    )?;
    let update = CategoryUpdate {
        name: request.name.map(CategoryName::new).transpose()?,
        color: request.color.map(CategoryColor::new).transpose()?,
    };

    run_blocking(move || category_updater.update(&id, update))
        .await?
        .map_err(update_category_error)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_category(
    State(category_deleter): State<CategoryDeleter>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = CategoryId::new(id)?;

    match run_blocking(move || category_deleter.delete(&id)).await? {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(e @ DeleteCategoryError::NotFound(_)) => Err(ApiError::new(StatusCode::NOT_FOUND, e)),
        Err(e @ DeleteCategoryError::Repository(_)) => Err(ApiError::internal(e)),
    }
}

pub async fn set_category_members(
    State(category_updater): State<CategoryUpdater>,
    Path(id): Path<String>,
    Json(request): Json<SetCategoryMembersRequest>,
) -> Result<StatusCode, ApiError> {
    let id = CategoryId::new(id)?;
    let members = members_from(request)?;

    run_blocking(move || category_updater.set_members(&id, members))
        .await?
        .map_err(update_category_error)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn mark_category_watched(
    State(video_watch_state_updater): State<VideoWatchStateUpdater>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = CategoryId::new(id)?;

    run_blocking(move || video_watch_state_updater.mark_category_watched(&id))
        .await?
        .map_err(update_watch_state_error)?;

    Ok(StatusCode::NO_CONTENT)
}

/// The requested channels, then the requested playlists, as members.
fn members_from(request: SetCategoryMembersRequest) -> Result<Vec<CategoryMember>, ApiError> {
    let channels = request
        .channels
        .into_iter()
        .map(|handle| ChannelHandle::new(handle).map(CategoryMember::Channel));
    let playlists = request
        .playlists
        .into_iter()
        .map(|id| PlaylistId::new(id).map(CategoryMember::Playlist));
    Ok(channels.chain(playlists).collect::<Result<_, _>>()?)
}

fn update_category_error(error: UpdateCategoryError) -> ApiError {
    match error {
        e @ UpdateCategoryError::NotFound(_) => ApiError::new(StatusCode::NOT_FOUND, e),
        e @ UpdateCategoryError::NameTaken(_) => ApiError::new(StatusCode::CONFLICT, e),
        e @ UpdateCategoryError::MemberNotTracked(_) => ApiError::bad_request(e),
        e @ UpdateCategoryError::Repository(_) => ApiError::internal(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::category::Category;
    use crate::domain::channel::{Channel, VideoLimit};
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::event::{DomainEvent, ScheduledEvent};
    use crate::domain::playlist::{Playlist, PlaylistKind, PlaylistName, PlaylistPath};
    use crate::domain::playlist_video::PlaylistVideo;
    use crate::domain::services::CategoryVideoCollector;
    use crate::domain::services::VideoCategoryAssigner;
    use crate::domain::shared::{Quality, ReconcileInterval};
    use crate::domain::video::{Video, VideoId};
    use crate::domain::video_category::{VideoCategories, VideoCategoriesChanged};
    use crate::infrastructure::repositories::sqlite_category_repository::{
        CategoryRepository, SqliteCategoryRepository,
    };
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
    use crate::infrastructure::repositories::sqlite_video_categories_repository::SqliteVideoCategoriesRepository;
    use crate::infrastructure::repositories::sqlite_video_categories_repository::VideoCategoriesRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::{
        SqliteVideoRepository, VideoRepository,
    };
    use crate::infrastructure::shared::domain_events::event_publisher::SqliteEventPublisher;
    use crate::infrastructure::shared::domain_events::event_repository::{
        EventRepository, SqliteEventRepository,
    };
    use crate::infrastructure::shared::sqlite_connection::{Database, TestDatabase};
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use dto::CategoryListItemResponse;
    use rusqlite::Connection;
    use std::sync::Arc;

    const INVALID_COLOR: &str = "Category color must be # followed by 6 hexadecimal digits";

    #[tokio::test]
    async fn it_should_create_a_category() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let category_creator =
            CategoryCreator::new(repository.clone(), Arc::new(FixedClock(fixed_timestamp())));

        let response = create(category_creator, create_request("  Chess  ")).await;

        let stored = repository.list().unwrap();
        let id = stored.first().map(|category| category.id.clone());
        assert_eq!(
            response,
            Ok((
                StatusCode::CREATED,
                category_response(id.as_ref(), "Chess", &[], &[])
            ))
        );
        assert_eq!(stored, vec![category(id, "Chess")]);
    }

    #[tokio::test]
    async fn it_should_create_a_category_with_a_color() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let category_creator =
            CategoryCreator::new(repository.clone(), Arc::new(FixedClock(fixed_timestamp())));

        let response = create(
            category_creator,
            CreateCategoryRequest {
                color: Some("#D3ECF8".to_string()),
                ..create_request("Chess")
            },
        )
        .await;

        let stored = repository.list().unwrap();
        let id = stored.first().map(|category| category.id.clone());
        assert_eq!(
            response,
            Ok((
                StatusCode::CREATED,
                CategoryResponse {
                    color: "#d3ecf8".to_string(),
                    ..category_response(id.as_ref(), "Chess", &[], &[])
                }
            ))
        );
        assert_eq!(
            stored,
            vec![Category {
                color: color("#d3ecf8"),
                ..category(id, "Chess")
            }]
        );
    }

    #[tokio::test]
    async fn it_should_create_the_first_category_with_the_first_palette_color() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let category_creator =
            CategoryCreator::new(repository.clone(), Arc::new(FixedClock(fixed_timestamp())));

        let response = create(category_creator, create_request("Chess")).await;

        let stored = repository.list().unwrap();
        let id = stored.first().map(|category| category.id.clone());
        assert_eq!(
            response,
            Ok((
                StatusCode::CREATED,
                CategoryResponse {
                    color: "#fff1bf".to_string(),
                    ..category_response(id.as_ref(), "Chess", &[], &[])
                }
            ))
        );
        assert_eq!(
            stored,
            vec![Category {
                color: color("#fff1bf"),
                ..category(id, "Chess")
            }]
        );
    }

    #[tokio::test]
    async fn it_should_create_a_category_with_the_least_used_palette_color() {
        let butter_and_coral = vec!["#fff1bf", "#ffd9d4"];
        let every_color_and_butter_again = CategoryColor::PALETTE
            .into_iter()
            .chain(["#fff1bf"])
            .collect::<Vec<_>>();
        let mut colors = Vec::new();
        for used in [butter_and_coral, every_color_and_butter_again] {
            let db = TestDatabase::new();
            let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
            used.iter().enumerate().for_each(|(index, hex)| {
                let existing = Category {
                    color: color(hex),
                    ..category(None, &format!("Existing {index}"))
                };
                repository.insert(&existing).unwrap();
            });
            let category_creator =
                CategoryCreator::new(repository, Arc::new(FixedClock(fixed_timestamp())));

            let response = create(category_creator, create_request("Chess")).await;

            colors.push(response.map(|(_, category)| category.color));
        }

        assert_eq!(
            colors,
            vec![Ok("#ffe0c7".to_string()), Ok("#ffe0c7".to_string())]
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_create_if_color_invalid() {
        let mut responses = Vec::new();
        for invalid in ["#fff", "yellow", "#gggggg"] {
            let request = CreateCategoryRequest {
                color: Some(invalid.to_string()),
                ..create_request("Chess")
            };
            responses.push(create(any_category_creator(), request).await);
        }

        assert_eq!(
            responses,
            vec![
                Err(ApiError::bad_request(INVALID_COLOR)),
                Err(ApiError::bad_request(INVALID_COLOR)),
                Err(ApiError::bad_request(INVALID_COLOR)),
            ]
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_create_if_name_missing() {
        let response = create(
            any_category_creator(),
            CreateCategoryRequest {
                name: None,
                color: None,
            },
        )
        .await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Category name must not be empty (missing)"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_create_if_name_invalid() {
        let response = create(any_category_creator(), create_request("   ")).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request("Category name must not be empty"))
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_create_if_name_taken_ignoring_case() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let chess = category(None, "Chess");
        repository.insert(&chess).unwrap();
        let category_creator =
            CategoryCreator::new(repository.clone(), Arc::new(FixedClock(fixed_timestamp())));

        let response = create(category_creator, create_request("chess")).await;

        assert_eq!(
            response,
            Err(ApiError::new(
                StatusCode::CONFLICT,
                "a category named \"chess\" already exists"
            ))
        );
        assert_eq!(repository.list().unwrap(), vec![chess]);
    }

    #[tokio::test]
    async fn it_should_list_categories_by_name_ignoring_case() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        channel_repository.insert(&channel("@GothamChess")).unwrap();
        playlist_repository.insert(&playlist("PLendgames")).unwrap();
        let science = category(None, "science");
        let chess = Category {
            members: vec![
                channel_member("@GothamChess"),
                playlist_member("PLendgames"),
            ],
            ..category(None, "Chess")
        };
        let sports = category(None, "Sports");
        [&science, &chess, &sports]
            .into_iter()
            .for_each(|category| repository.insert(category).unwrap());
        let category_view_searcher = CategoryViewSearcher::new(
            repository,
            channel_repository,
            playlist_repository,
            category_video_collector(&db),
        );

        let response = list(category_view_searcher).await;

        assert_eq!(
            response,
            Ok(CategoryListResponse {
                categories: vec![
                    list_item(&chess, 0, &["@GothamChess"], &["PLendgames"]),
                    list_item(&science, 0, &[], &[]),
                    list_item(&sports, 0, &[], &[]),
                ],
                unwatched_total: 0,
            })
        );
    }

    #[tokio::test]
    async fn it_should_list_categories_with_their_color() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let chess = Category {
            color: color("#d3ecf8"),
            ..category(None, "Chess")
        };
        repository.insert(&chess).unwrap();
        let category_view_searcher = CategoryViewSearcher::new(
            repository,
            Arc::new(SqliteChannelRepository::new(db.database())),
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            category_video_collector(&db),
        );

        let response = list(category_view_searcher).await;

        assert_eq!(
            response,
            Ok(CategoryListResponse {
                categories: vec![CategoryListItemResponse {
                    color: "#d3ecf8".to_string(),
                    ..list_item(&chess, 0, &[], &[])
                }],
                unwatched_total: 0,
            })
        );
    }

    #[tokio::test]
    async fn it_should_count_distinct_unwatched_videos_per_category() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = SqliteVideoRepository::new(db.database());
        let channel_video_repository = SqliteChannelVideoRepository::new(db.database());
        let playlist_video_repository = SqlitePlaylistVideoRepository::new(db.database());
        channel_repository.insert(&channel("@chess")).unwrap();
        playlist_repository.insert(&playlist("PLchess")).unwrap();
        [
            downloaded_video("shared"),
            downloaded_video("channel-only"),
            pending_video("pending"),
        ]
        .iter()
        .for_each(|video| {
            save_channel_video(
                &video_repository,
                &channel_video_repository,
                "@chess",
                video,
            )
        });
        [
            downloaded_video("shared"),
            downloaded_video("playlist-only"),
            downloaded_video("watched").mark_watched(fixed_timestamp()),
        ]
        .iter()
        .for_each(|video| {
            save_playlist_video(
                &video_repository,
                &playlist_video_repository,
                "PLchess",
                video,
            )
        });
        let chess = Category {
            members: vec![channel_member("@chess"), playlist_member("PLchess")],
            ..category(None, "Chess")
        };
        repository.insert(&chess).unwrap();
        let category_view_searcher = CategoryViewSearcher::new(
            repository,
            channel_repository,
            playlist_repository,
            category_video_collector(&db),
        );

        let response = list(category_view_searcher).await;

        assert_eq!(
            response,
            Ok(CategoryListResponse {
                categories: vec![list_item(&chess, 3, &["@chess"], &["PLchess"])],
                unwatched_total: 3,
            })
        );
    }

    #[tokio::test]
    async fn it_should_count_the_total_once_across_categories() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = SqliteVideoRepository::new(db.database());
        channel_repository.insert(&channel("@science")).unwrap();
        playlist_repository.insert(&playlist("PLchess")).unwrap();
        save_channel_video(
            &video_repository,
            &SqliteChannelVideoRepository::new(db.database()),
            "@science",
            &downloaded_video("shared"),
        );
        save_playlist_video(
            &video_repository,
            &SqlitePlaylistVideoRepository::new(db.database()),
            "PLchess",
            &downloaded_video("shared"),
        );
        let chess = Category {
            members: vec![playlist_member("PLchess")],
            ..category(None, "Chess")
        };
        let science = Category {
            members: vec![channel_member("@science")],
            ..category(None, "Science")
        };
        repository.insert(&chess).unwrap();
        repository.insert(&science).unwrap();
        let category_view_searcher = CategoryViewSearcher::new(
            repository,
            channel_repository,
            playlist_repository,
            category_video_collector(&db),
        );

        let response = list(category_view_searcher).await;

        assert_eq!(
            response,
            Ok(CategoryListResponse {
                categories: vec![
                    list_item(&chess, 1, &[], &["PLchess"]),
                    list_item(&science, 1, &["@science"], &[]),
                ],
                unwatched_total: 1,
            })
        );
    }

    #[tokio::test]
    async fn it_should_not_count_videos_of_a_playlist_hiding_its_count_in_a_category() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = SqliteVideoRepository::new(db.database());
        channel_repository.insert(&channel("@chess")).unwrap();
        playlist_repository
            .insert(&playlist("PLmusic").with_hide_unwatched_count(true))
            .unwrap();
        save_channel_video(
            &video_repository,
            &SqliteChannelVideoRepository::new(db.database()),
            "@chess",
            &downloaded_video("channel-only"),
        );
        [downloaded_video("song-1"), downloaded_video("song-2")]
            .iter()
            .for_each(|video| {
                save_playlist_video(
                    &video_repository,
                    &SqlitePlaylistVideoRepository::new(db.database()),
                    "PLmusic",
                    video,
                )
            });
        let chess = Category {
            members: vec![channel_member("@chess"), playlist_member("PLmusic")],
            ..category(None, "Chess")
        };
        repository.insert(&chess).unwrap();
        let category_view_searcher = CategoryViewSearcher::new(
            repository,
            channel_repository,
            playlist_repository,
            category_video_collector(&db),
        );

        let response = list(category_view_searcher).await;

        assert_eq!(
            response,
            Ok(CategoryListResponse {
                categories: vec![list_item(&chess, 1, &["@chess"], &["PLmusic"])],
                unwatched_total: 1,
            })
        );
    }

    #[tokio::test]
    async fn it_should_count_once_a_video_of_a_hidden_playlist_also_in_a_counted_member() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = SqliteVideoRepository::new(db.database());
        channel_repository.insert(&channel("@chess")).unwrap();
        playlist_repository
            .insert(&playlist("PLmusic").with_hide_unwatched_count(true))
            .unwrap();
        save_channel_video(
            &video_repository,
            &SqliteChannelVideoRepository::new(db.database()),
            "@chess",
            &downloaded_video("shared"),
        );
        save_playlist_video(
            &video_repository,
            &SqlitePlaylistVideoRepository::new(db.database()),
            "PLmusic",
            &downloaded_video("shared"),
        );
        let chess = Category {
            members: vec![channel_member("@chess"), playlist_member("PLmusic")],
            ..category(None, "Chess")
        };
        repository.insert(&chess).unwrap();
        let category_view_searcher = CategoryViewSearcher::new(
            repository,
            channel_repository,
            playlist_repository,
            category_video_collector(&db),
        );

        let response = list(category_view_searcher).await;

        assert_eq!(
            response,
            Ok(CategoryListResponse {
                categories: vec![list_item(&chess, 1, &["@chess"], &["PLmusic"])],
                unwatched_total: 1,
            })
        );
    }

    #[tokio::test]
    async fn it_should_not_total_videos_reached_only_through_hidden_playlists() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        playlist_repository
            .insert(&playlist("PLmusic").with_hide_unwatched_count(true))
            .unwrap();
        save_playlist_video(
            &SqliteVideoRepository::new(db.database()),
            &SqlitePlaylistVideoRepository::new(db.database()),
            "PLmusic",
            &downloaded_video("song"),
        );
        let music = Category {
            members: vec![playlist_member("PLmusic")],
            ..category(None, "Music")
        };
        repository.insert(&music).unwrap();
        let category_view_searcher = CategoryViewSearcher::new(
            repository,
            channel_repository,
            playlist_repository,
            category_video_collector(&db),
        );

        let response = list(category_view_searcher).await;

        assert_eq!(
            response,
            Ok(CategoryListResponse {
                categories: vec![list_item(&music, 0, &[], &["PLmusic"])],
                unwatched_total: 0,
            })
        );
    }

    #[tokio::test]
    async fn it_should_skip_members_already_deleted() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        channel_repository.insert(&channel("@kept")).unwrap();
        save_channel_video(
            &SqliteVideoRepository::new(db.database()),
            &SqliteChannelVideoRepository::new(db.database()),
            "@gone",
            &downloaded_video("orphan"),
        );
        let chess = Category {
            members: vec![
                channel_member("@gone"),
                channel_member("@kept"),
                playlist_member("PLgone"),
            ],
            ..category(None, "Chess")
        };
        repository.insert(&chess).unwrap();
        let category_view_searcher = CategoryViewSearcher::new(
            repository,
            channel_repository,
            playlist_repository,
            category_video_collector(&db),
        );

        let response = list(category_view_searcher).await;

        assert_eq!(
            response,
            Ok(CategoryListResponse {
                categories: vec![list_item(&chess, 0, &["@kept"], &[])],
                unwatched_total: 0,
            })
        );
    }

    #[tokio::test]
    async fn it_should_rename_a_category() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        channel_repository.insert(&channel("@chess")).unwrap();
        let chess = Category {
            members: vec![channel_member("@chess")],
            ..category(None, "Chess")
        };
        repository.insert(&chess).unwrap();
        let category_updater = CategoryUpdater::new(
            repository.clone(),
            channel_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            video_category_assigner(db.database()),
        );

        let response = rename(category_updater, chess.id.as_str(), "Board games").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            repository.list().unwrap(),
            vec![Category {
                name: CategoryName::new("Board games").unwrap(),
                ..chess
            }]
        );
    }

    #[tokio::test]
    async fn it_should_recolor_a_category() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        channel_repository.insert(&channel("@chess")).unwrap();
        let chess = Category {
            members: vec![channel_member("@chess")],
            ..category(None, "Chess")
        };
        repository.insert(&chess).unwrap();
        let category_updater = CategoryUpdater::new(
            repository.clone(),
            channel_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            video_category_assigner(db.database()),
        );

        let response = update(
            category_updater,
            chess.id.as_str(),
            UpdateCategoryRequest {
                name: None,
                color: Some("#e9def8".to_string()),
            },
        )
        .await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            repository.list().unwrap(),
            vec![Category {
                color: color("#e9def8"),
                ..chess
            }]
        );
    }

    #[tokio::test]
    async fn it_should_rename_and_recolor_a_category() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let chess = category(None, "Chess");
        repository.insert(&chess).unwrap();
        let category_updater = CategoryUpdater::new(
            repository.clone(),
            Arc::new(SqliteChannelRepository::new(db.database())),
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            video_category_assigner(db.database()),
        );

        let response = update(
            category_updater,
            chess.id.as_str(),
            UpdateCategoryRequest {
                name: Some("Board games".to_string()),
                color: Some("#e9def8".to_string()),
            },
        )
        .await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            repository.list().unwrap(),
            vec![Category {
                name: CategoryName::new("Board games").unwrap(),
                color: color("#e9def8"),
                ..chess
            }]
        );
    }

    #[tokio::test]
    async fn it_should_rename_a_category_to_a_case_variant_of_its_name() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let chess = category(None, "chess");
        repository.insert(&chess).unwrap();
        let category_updater = CategoryUpdater::new(
            repository.clone(),
            Arc::new(SqliteChannelRepository::new(db.database())),
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            video_category_assigner(db.database()),
        );

        let response = rename(category_updater, chess.id.as_str(), "Chess").await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            repository.list().unwrap(),
            vec![Category {
                name: CategoryName::new("Chess").unwrap(),
                ..chess
            }]
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_rename_if_name_taken() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let chess = category(None, "Chess");
        let sports = category(None, "Sports");
        repository.insert(&chess).unwrap();
        repository.insert(&sports).unwrap();
        let category_updater = CategoryUpdater::new(
            repository.clone(),
            Arc::new(SqliteChannelRepository::new(db.database())),
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            video_category_assigner(db.database()),
        );

        let response = rename(category_updater, chess.id.as_str(), "sports").await;

        assert_eq!(
            response,
            Err(ApiError::new(
                StatusCode::CONFLICT,
                "a category named \"sports\" already exists"
            ))
        );
        assert_eq!(repository.list().unwrap(), vec![chess, sports]);
    }

    #[tokio::test]
    async fn it_should_not_recolor_if_name_taken() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let chess = category(None, "Chess");
        let sports = category(None, "Sports");
        repository.insert(&chess).unwrap();
        repository.insert(&sports).unwrap();
        let category_updater = CategoryUpdater::new(
            repository.clone(),
            Arc::new(SqliteChannelRepository::new(db.database())),
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            video_category_assigner(db.database()),
        );

        let response = update(
            category_updater,
            chess.id.as_str(),
            UpdateCategoryRequest {
                name: Some("sports".to_string()),
                color: Some("#e9def8".to_string()),
            },
        )
        .await;

        assert_eq!(
            response,
            Err(ApiError::new(
                StatusCode::CONFLICT,
                "a category named \"sports\" already exists"
            ))
        );
        assert_eq!(repository.list().unwrap(), vec![chess, sports]);
    }

    #[tokio::test]
    async fn it_should_fail_to_update_without_name_or_color() {
        let response = update(
            any_category_updater(),
            "chess",
            UpdateCategoryRequest {
                name: None,
                color: None,
            },
        )
        .await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Category name or color is required (both missing)"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_update_if_color_invalid() {
        let response = update(
            any_category_updater(),
            "chess",
            UpdateCategoryRequest {
                name: None,
                color: Some("yellow".to_string()),
            },
        )
        .await;

        assert_eq!(response, Err(ApiError::bad_request(INVALID_COLOR)));
    }

    #[tokio::test]
    async fn it_should_fail_to_rename_if_name_invalid() {
        let response = rename(any_category_updater(), "chess", "   ").await;

        assert_eq!(
            response,
            Err(ApiError::bad_request("Category name must not be empty"))
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_rename_a_missing_category() {
        let db = TestDatabase::new();
        let category_updater = CategoryUpdater::new(
            Arc::new(SqliteCategoryRepository::new(db.database())),
            Arc::new(SqliteChannelRepository::new(db.database())),
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            video_category_assigner(db.database()),
        );

        let response = rename(category_updater, "missing", "Chess").await;

        assert_eq!(
            response,
            Err(ApiError::new(
                StatusCode::NOT_FOUND,
                "category missing not found"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_delete_a_category_keeping_its_sources() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let channel_repository = SqliteChannelRepository::new(db.database());
        let playlist_repository = SqlitePlaylistRepository::new(db.database());
        let video_repository = SqliteVideoRepository::new(db.database());
        let gotham = channel("@GothamChess");
        let endgames = playlist("PLendgames");
        let video = downloaded_video("vid1");
        channel_repository.insert(&gotham).unwrap();
        playlist_repository.insert(&endgames).unwrap();
        save_channel_video(
            &video_repository,
            &SqliteChannelVideoRepository::new(db.database()),
            "@GothamChess",
            &video,
        );
        let chess = Category {
            members: vec![
                channel_member("@GothamChess"),
                playlist_member("PLendgames"),
            ],
            ..category(None, "Chess")
        };
        repository.insert(&chess).unwrap();
        let category_deleter =
            CategoryDeleter::new(repository.clone(), video_category_assigner(db.database()));

        let response = delete(category_deleter, chess.id.as_str()).await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(repository.list().unwrap(), vec![]);
        assert_eq!(channel_repository.list().unwrap(), vec![gotham]);
        assert_eq!(playlist_repository.list().unwrap(), vec![endgames]);
        assert_eq!(video_repository.find(&video.id).unwrap(), Some(video));
    }

    #[tokio::test]
    async fn it_should_fail_to_delete_a_missing_category() {
        let db = TestDatabase::new();
        let category_deleter = CategoryDeleter::new(
            Arc::new(SqliteCategoryRepository::new(db.database())),
            video_category_assigner(db.database()),
        );

        let response = delete(category_deleter, "missing").await;

        assert_eq!(
            response,
            Err(ApiError::new(
                StatusCode::NOT_FOUND,
                "category missing not found"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_set_the_members_of_a_category() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        channel_repository.insert(&channel("@GothamChess")).unwrap();
        channel_repository.insert(&channel("@Hikaru")).unwrap();
        playlist_repository.insert(&playlist("PLendgames")).unwrap();
        let chess = category(None, "Chess");
        repository.insert(&chess).unwrap();
        let category_updater = CategoryUpdater::new(
            repository.clone(),
            channel_repository,
            playlist_repository,
            video_category_assigner(db.database()),
        );

        let response = set_members(
            category_updater,
            chess.id.as_str(),
            members_request(&["@GothamChess", "@Hikaru"], &["PLendgames"]),
        )
        .await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            repository.list().unwrap(),
            vec![Category {
                members: vec![
                    channel_member("@GothamChess"),
                    channel_member("@Hikaru"),
                    playlist_member("PLendgames"),
                ],
                ..chess
            }]
        );
    }

    #[tokio::test]
    async fn it_should_replace_the_members_of_a_category() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        channel_repository.insert(&channel("@GothamChess")).unwrap();
        channel_repository.insert(&channel("@Hikaru")).unwrap();
        let chess = Category {
            members: vec![channel_member("@GothamChess"), channel_member("@Hikaru")],
            ..category(None, "Chess")
        };
        repository.insert(&chess).unwrap();
        let category_updater = CategoryUpdater::new(
            repository.clone(),
            channel_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            video_category_assigner(db.database()),
        );

        let response = set_members(
            category_updater,
            chess.id.as_str(),
            members_request(&["@Hikaru"], &[]),
        )
        .await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            repository.list().unwrap(),
            vec![Category {
                members: vec![channel_member("@Hikaru")],
                ..chess
            }]
        );
    }

    #[tokio::test]
    async fn it_should_keep_a_repeated_member_once() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        channel_repository.insert(&channel("@Hikaru")).unwrap();
        playlist_repository.insert(&playlist("PLendgames")).unwrap();
        let chess = category(None, "Chess");
        repository.insert(&chess).unwrap();
        let category_updater = CategoryUpdater::new(
            repository.clone(),
            channel_repository,
            playlist_repository,
            video_category_assigner(db.database()),
        );

        let response = set_members(
            category_updater,
            chess.id.as_str(),
            members_request(&["@Hikaru", "@Hikaru"], &["PLendgames", "PLendgames"]),
        )
        .await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            repository.list().unwrap(),
            vec![Category {
                members: vec![channel_member("@Hikaru"), playlist_member("PLendgames")],
                ..chess
            }]
        );
    }

    #[tokio::test]
    async fn it_should_let_a_source_be_in_several_categories() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        channel_repository.insert(&channel("@Veritasium")).unwrap();
        let chess = Category {
            members: vec![channel_member("@Veritasium")],
            ..category(None, "Chess")
        };
        let science = category(None, "Science");
        repository.insert(&chess).unwrap();
        repository.insert(&science).unwrap();
        let category_updater = CategoryUpdater::new(
            repository.clone(),
            channel_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            video_category_assigner(db.database()),
        );

        let response = set_members(
            category_updater,
            science.id.as_str(),
            members_request(&["@Veritasium"], &[]),
        )
        .await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            repository.list().unwrap(),
            vec![
                chess,
                Category {
                    members: vec![channel_member("@Veritasium")],
                    ..science
                }
            ]
        );
    }

    #[tokio::test]
    async fn it_should_categorize_the_videos_of_an_added_member() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = SqliteVideoRepository::new(db.database());
        let channel_video_repository = SqliteChannelVideoRepository::new(db.database());
        let video_categories_repository = SqliteVideoCategoriesRepository::new(db.database());
        let event_repository = SqliteEventRepository::new(db.database());
        channel_repository.insert(&channel("@GothamChess")).unwrap();
        let first = pending_video("vid1");
        let second = downloaded_video("vid2");
        [&first, &second].into_iter().for_each(|video| {
            save_channel_video(
                &video_repository,
                &channel_video_repository,
                "@GothamChess",
                video,
            )
        });
        let chess = category(None, "Chess");
        repository.insert(&chess).unwrap();
        let category_updater = CategoryUpdater::new(
            repository,
            channel_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            video_category_assigner(db.database()),
        );

        let response = set_members(
            category_updater,
            chess.id.as_str(),
            members_request(&["@GothamChess"], &[]),
        )
        .await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            video_categories_repository
                .find_many(&[first.id.clone(), second.id.clone()])
                .unwrap(),
            vec![
                VideoCategories::new(first.id.clone(), vec![chess.id.clone()]),
                VideoCategories::new(second.id.clone(), vec![chess.id.clone()]),
            ]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![categories_changed_event(1, &[&first, &second])]
        );
    }

    #[tokio::test]
    async fn it_should_uncategorize_the_videos_of_a_removed_member() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = SqliteVideoRepository::new(db.database());
        let video_categories_repository = SqliteVideoCategoriesRepository::new(db.database());
        let event_repository = SqliteEventRepository::new(db.database());
        channel_repository.insert(&channel("@GothamChess")).unwrap();
        let video = downloaded_video("vid1");
        save_channel_video(
            &video_repository,
            &SqliteChannelVideoRepository::new(db.database()),
            "@GothamChess",
            &video,
        );
        let chess = Category {
            members: vec![channel_member("@GothamChess")],
            ..category(None, "Chess")
        };
        repository.insert(&chess).unwrap();
        video_categories_repository
            .save_many(&[VideoCategories::new(
                video.id.clone(),
                vec![chess.id.clone()],
            )])
            .unwrap();
        let category_updater = CategoryUpdater::new(
            repository,
            channel_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            video_category_assigner(db.database()),
        );

        let response = set_members(
            category_updater,
            chess.id.as_str(),
            members_request(&[], &[]),
        )
        .await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            video_categories_repository
                .find_many(&[video.id.clone()])
                .unwrap(),
            vec![VideoCategories::none(video.id.clone())]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![categories_changed_event(1, &[&video])]
        );
    }

    #[tokio::test]
    async fn it_should_keep_the_other_categories_of_a_member_removed_from_one() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let video_repository = SqliteVideoRepository::new(db.database());
        let video_categories_repository = SqliteVideoCategoriesRepository::new(db.database());
        let event_repository = SqliteEventRepository::new(db.database());
        channel_repository.insert(&channel("@GothamChess")).unwrap();
        let video = downloaded_video("vid1");
        save_channel_video(
            &video_repository,
            &SqliteChannelVideoRepository::new(db.database()),
            "@GothamChess",
            &video,
        );
        let chess = Category {
            members: vec![channel_member("@GothamChess")],
            ..category(None, "Chess")
        };
        let funny = Category {
            members: vec![channel_member("@GothamChess")],
            ..category(None, "Funny")
        };
        repository.insert(&chess).unwrap();
        repository.insert(&funny).unwrap();
        video_categories_repository
            .save_many(&[VideoCategories::new(
                video.id.clone(),
                vec![chess.id.clone(), funny.id.clone()],
            )])
            .unwrap();
        let category_updater = CategoryUpdater::new(
            repository,
            channel_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            video_category_assigner(db.database()),
        );

        let response = set_members(
            category_updater,
            funny.id.as_str(),
            members_request(&[], &[]),
        )
        .await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            video_categories_repository
                .find_many(&[video.id.clone()])
                .unwrap(),
            vec![VideoCategories::new(
                video.id.clone(),
                vec![chess.id.clone()]
            )]
        );
        assert_eq!(
            event_repository.list_eligible().unwrap(),
            vec![categories_changed_event(1, &[&video])]
        );
    }

    #[tokio::test]
    async fn it_should_not_categorize_other_copies_of_a_member_video() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let playlist_repository = Arc::new(SqlitePlaylistRepository::new(db.database()));
        let video_repository = SqliteVideoRepository::new(db.database());
        let video_categories_repository = SqliteVideoCategoriesRepository::new(db.database());
        channel_repository.insert(&channel("@GothamChess")).unwrap();
        playlist_repository.insert(&playlist("PLendgames")).unwrap();
        let channel_copy = downloaded_video("vid1");
        let playlist_copy = downloaded_video("vid1");
        save_channel_video(
            &video_repository,
            &SqliteChannelVideoRepository::new(db.database()),
            "@GothamChess",
            &channel_copy,
        );
        save_playlist_video(
            &video_repository,
            &SqlitePlaylistVideoRepository::new(db.database()),
            "PLendgames",
            &playlist_copy,
        );
        let chess = category(None, "Chess");
        repository.insert(&chess).unwrap();
        let category_updater = CategoryUpdater::new(
            repository,
            channel_repository,
            playlist_repository,
            video_category_assigner(db.database()),
        );

        let response = set_members(
            category_updater,
            chess.id.as_str(),
            members_request(&["@GothamChess"], &[]),
        )
        .await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            video_categories_repository
                .find_many(&[channel_copy.id.clone(), playlist_copy.id.clone()])
                .unwrap(),
            vec![
                VideoCategories::new(channel_copy.id.clone(), vec![chess.id.clone()]),
                VideoCategories::none(playlist_copy.id.clone()),
            ]
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_set_members_if_a_source_is_not_tracked() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        channel_repository.insert(&channel("@Hikaru")).unwrap();
        let chess = Category {
            members: vec![channel_member("@Hikaru")],
            ..category(None, "Chess")
        };
        repository.insert(&chess).unwrap();
        let category_updater = CategoryUpdater::new(
            repository.clone(),
            channel_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            video_category_assigner(db.database()),
        );

        let response = set_members(
            category_updater,
            chess.id.as_str(),
            members_request(&["@Hikaru", "@Ghost"], &["PLghost"]),
        )
        .await;

        assert_eq!(
            response,
            Err(ApiError::bad_request("channel @Ghost is not tracked"))
        );
        assert_eq!(repository.list().unwrap(), vec![chess]);
    }

    #[tokio::test]
    async fn it_should_fail_to_set_members_if_a_handle_is_invalid() {
        let response = set_members(
            any_category_updater(),
            "category-1",
            members_request(&["Hikaru"], &[]),
        )
        .await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Channel handle must start with \"@\" (got \"Hikaru\")"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_set_members_of_a_missing_category() {
        let db = TestDatabase::new();
        let category_updater = CategoryUpdater::new(
            Arc::new(SqliteCategoryRepository::new(db.database())),
            Arc::new(SqliteChannelRepository::new(db.database())),
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            video_category_assigner(db.database()),
        );

        let response = set_members(category_updater, "missing", members_request(&[], &[])).await;

        assert_eq!(
            response,
            Err(ApiError::new(
                StatusCode::NOT_FOUND,
                "category missing not found"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_mark_a_category_watched() {
        let db = TestDatabase::new();
        let repository = Arc::new(SqliteCategoryRepository::new(db.database()));
        let channel_repository = Arc::new(SqliteChannelRepository::new(db.database()));
        let playlist_repository = SqlitePlaylistRepository::new(db.database());
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(db.database()));
        let playlist_video_repository = SqlitePlaylistVideoRepository::new(db.database());
        channel_repository.insert(&channel("@chess")).unwrap();
        playlist_repository.insert(&playlist("PLchess")).unwrap();
        playlist_repository.insert(&playlist("PLother")).unwrap();
        let channel_video = downloaded_video("channel-video");
        let pending = pending_video("pending");
        let playlist_video = downloaded_video("playlist-video");
        let copy_outside = downloaded_video("channel-video");
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            "@chess",
            &channel_video,
        );
        save_channel_video(
            video_repository.as_ref(),
            channel_video_repository.as_ref(),
            "@chess",
            &pending,
        );
        save_playlist_video(
            video_repository.as_ref(),
            &playlist_video_repository,
            "PLchess",
            &playlist_video,
        );
        save_playlist_video(
            video_repository.as_ref(),
            &playlist_video_repository,
            "PLother",
            &copy_outside,
        );
        let chess = Category {
            members: vec![channel_member("@chess"), playlist_member("PLchess")],
            ..category(None, "Chess")
        };
        repository.insert(&chess).unwrap();
        let video_watch_state_updater = VideoWatchStateUpdater::new(
            video_repository.clone(),
            channel_repository,
            channel_video_repository,
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            Arc::new(FixedClock(watched_timestamp())),
            repository,
            category_video_collector(&db),
        );

        let response = mark_watched(video_watch_state_updater, chess.id.as_str()).await;

        assert_eq!(response, Ok(StatusCode::NO_CONTENT));
        assert_eq!(
            [&channel_video, &pending, &playlist_video, &copy_outside]
                .map(|video| video_repository.find(&video.id).unwrap()),
            [
                Some(channel_video.clone().mark_watched(watched_timestamp())),
                Some(pending.clone()),
                Some(playlist_video.clone().mark_watched(watched_timestamp())),
                Some(copy_outside.clone().mark_watched(watched_timestamp())),
            ]
        );
    }

    #[tokio::test]
    async fn it_should_fail_to_mark_a_missing_category_watched() {
        let db = TestDatabase::new();
        let video_watch_state_updater = VideoWatchStateUpdater::new(
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(SqliteChannelRepository::new(db.database())),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            Arc::new(FixedClock(watched_timestamp())),
            Arc::new(SqliteCategoryRepository::new(db.database())),
            category_video_collector(&db),
        );

        let response = mark_watched(video_watch_state_updater, "missing").await;

        assert_eq!(
            response,
            Err(ApiError::new(
                StatusCode::NOT_FOUND,
                "category missing not found"
            ))
        );
    }

    fn any_category_creator() -> CategoryCreator {
        CategoryCreator::new(
            Arc::new(SqliteCategoryRepository::new(unused_connection())),
            Arc::new(FixedClock(fixed_timestamp())),
        )
    }

    fn any_category_updater() -> CategoryUpdater {
        CategoryUpdater::new(
            Arc::new(SqliteCategoryRepository::new(unused_connection())),
            Arc::new(SqliteChannelRepository::new(unused_connection())),
            Arc::new(SqlitePlaylistRepository::new(unused_connection())),
            video_category_assigner(unused_connection()),
        )
    }

    fn unused_connection() -> Database {
        Database::single(Connection::open_in_memory().unwrap())
    }

    fn members_request(channels: &[&str], playlists: &[&str]) -> SetCategoryMembersRequest {
        SetCategoryMembersRequest {
            channels: channels.iter().map(|c| c.to_string()).collect(),
            playlists: playlists.iter().map(|p| p.to_string()).collect(),
        }
    }

    fn create_request(name: &str) -> CreateCategoryRequest {
        CreateCategoryRequest {
            name: Some(name.to_string()),
            color: None,
        }
    }

    fn category_response(
        id: Option<&CategoryId>,
        name: &str,
        channels: &[&str],
        playlists: &[&str],
    ) -> CategoryResponse {
        CategoryResponse {
            id: id.map(|id| id.as_str().to_string()).unwrap_or_default(),
            name: name.to_string(),
            color: CategoryColor::PALETTE[0].to_string(),
            channels: channels.iter().map(|c| c.to_string()).collect(),
            playlists: playlists.iter().map(|p| p.to_string()).collect(),
        }
    }

    /// A stored category without members, created at `fixed_timestamp()`.
    fn category(id: Option<CategoryId>, name: &str) -> Category {
        Category {
            id: id.unwrap_or_else(CategoryId::new_generated),
            name: CategoryName::new(name).unwrap(),
            color: CategoryColor::new(CategoryColor::PALETTE[0]).unwrap(),
            members: vec![],
            created_at: fixed_timestamp(),
        }
    }

    fn color(hex: &str) -> CategoryColor {
        CategoryColor::new(hex).unwrap()
    }

    fn list_item(
        category: &Category,
        unwatched_count: usize,
        channels: &[&str],
        playlists: &[&str],
    ) -> CategoryListItemResponse {
        CategoryListItemResponse {
            id: category.id.as_str().to_string(),
            name: category.name.as_str().to_string(),
            color: category.color.as_str().to_string(),
            unwatched_count,
            channels: channels.iter().map(|c| c.to_string()).collect(),
            playlists: playlists.iter().map(|p| p.to_string()).collect(),
        }
    }

    fn downloaded_video(youtube_id: &str) -> Video {
        pending_video(youtube_id).mark_downloaded(
            Quality::High,
            format!("{youtube_id}.mp4"),
            None,
            None,
            fixed_timestamp(),
        )
    }

    fn pending_video(youtube_id: &str) -> Video {
        Video::create(
            VideoId::new(youtube_id).unwrap(),
            format!("Video {youtube_id}"),
            fixed_timestamp(),
        )
    }

    fn save_channel_video(
        video_repository: &dyn VideoRepository,
        channel_video_repository: &dyn ChannelVideoRepository,
        handle: &str,
        video: &Video,
    ) {
        video_repository.save(video).unwrap();
        channel_video_repository
            .save(&ChannelVideo::create(
                ChannelHandle::new(handle).unwrap(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
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
                0,
                fixed_timestamp(),
            ))
            .unwrap();
    }

    fn channel_member(handle: &str) -> CategoryMember {
        CategoryMember::Channel(ChannelHandle::new(handle).unwrap())
    }

    fn playlist_member(id: &str) -> CategoryMember {
        CategoryMember::Playlist(PlaylistId::new(id).unwrap())
    }

    fn channel(handle: &str) -> Channel {
        Channel::create(
            ChannelHandle::new(handle).unwrap(),
            format!("Channel {handle}"),
            format!("UC{handle}"),
            Quality::High,
            VideoLimit::new(10).unwrap(),
            ReconcileInterval::default_interval(),
            PlaylistPath::new(format!("channels/{handle}")).unwrap(),
            None,
            fixed_timestamp(),
        )
    }

    fn playlist(id: &str) -> Playlist {
        Playlist::create(
            PlaylistId::new(id).unwrap(),
            PlaylistName::new(format!("Playlist {id}")).unwrap(),
            PlaylistPath::new(format!("playlists/{id}")).unwrap(),
            Quality::High,
            PlaylistKind::YoutubeLinked,
            false,
            ReconcileInterval::default_interval(),
            fixed_timestamp(),
        )
    }

    /// The collector the services read category videos through, on `db`
    /// like the repositories each test seeds.
    fn category_video_collector(db: &TestDatabase) -> CategoryVideoCollector {
        CategoryVideoCollector::new(
            Arc::new(SqliteChannelRepository::new(db.database())),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(SqliteVideoCategoriesRepository::new(db.database())),
        )
    }

    /// The pending `VideoCategoriesChanged` event `id`, naming `videos` in
    /// record id order.
    fn categories_changed_event(id: i64, videos: &[&Video]) -> ScheduledEvent {
        let mut video_ids: Vec<String> = videos
            .iter()
            .map(|video| video.id.as_str().to_string())
            .collect();
        video_ids.sort();
        let event = DomainEvent::VideoCategoriesChanged(VideoCategoriesChanged { video_ids });
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

    async fn mark_watched(
        video_watch_state_updater: VideoWatchStateUpdater,
        id: &str,
    ) -> Result<StatusCode, ApiError> {
        mark_category_watched(State(video_watch_state_updater), Path(id.to_string())).await
    }

    async fn set_members(
        category_updater: CategoryUpdater,
        id: &str,
        request: SetCategoryMembersRequest,
    ) -> Result<StatusCode, ApiError> {
        set_category_members(State(category_updater), Path(id.to_string()), Json(request)).await
    }

    async fn delete(category_deleter: CategoryDeleter, id: &str) -> Result<StatusCode, ApiError> {
        delete_category(State(category_deleter), Path(id.to_string())).await
    }

    async fn rename(
        category_updater: CategoryUpdater,
        id: &str,
        name: &str,
    ) -> Result<StatusCode, ApiError> {
        update(
            category_updater,
            id,
            UpdateCategoryRequest {
                name: Some(name.to_string()),
                color: None,
            },
        )
        .await
    }

    async fn update(
        category_updater: CategoryUpdater,
        id: &str,
        request: UpdateCategoryRequest,
    ) -> Result<StatusCode, ApiError> {
        update_category(State(category_updater), Path(id.to_string()), Json(request)).await
    }

    async fn list(
        category_view_searcher: CategoryViewSearcher,
    ) -> Result<CategoryListResponse, ApiError> {
        list_categories(State(category_view_searcher))
            .await
            .map(|Json(categories)| categories)
    }

    async fn create(
        category_creator: CategoryCreator,
        request: CreateCategoryRequest,
    ) -> Result<(StatusCode, CategoryResponse), ApiError> {
        create_category(State(category_creator), Json(request))
            .await
            .map(|(status, Json(category))| (status, category))
    }

    fn video_category_assigner(database: Database) -> VideoCategoryAssigner {
        VideoCategoryAssigner::new(
            Arc::new(SqliteVideoCategoriesRepository::new(database.clone())),
            Arc::new(SqliteCategoryRepository::new(database.clone())),
            Arc::new(SqliteChannelVideoRepository::new(database.clone())),
            Arc::new(SqlitePlaylistVideoRepository::new(database.clone())),
            Arc::new(SqliteEventPublisher::new(
                database,
                Arc::new(FixedClock(fixed_timestamp())),
            )),
        )
    }
}
