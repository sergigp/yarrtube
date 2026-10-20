pub mod dto;

use super::blocking::run_blocking;
use super::error::ApiError;
use super::validation::required;
use crate::domain::category::{CategoryName, CreateCategoryError};
use crate::domain::services::{
    CategoryCreator, CategoryCreatorApi, CategoryDeleter, CategoryUpdater, CategoryViewSearcher,
    CategoryViewSearcherApi, VideoWatchStateUpdater,
};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use dto::{
    CategoryListResponse, CategoryResponse, CreateCategoryRequest, SetCategoryMembersRequest,
    UpdateCategoryRequest,
};

const MISSING_CATEGORY_NAME: &str = "Category name must not be empty (missing)";

pub async fn create_category(
    State(category_creator): State<CategoryCreator>,
    Json(request): Json<CreateCategoryRequest>,
) -> Result<(StatusCode, Json<CategoryResponse>), ApiError> {
    let name = CategoryName::new(required(request.name, MISSING_CATEGORY_NAME)?)?;

    match run_blocking(move || category_creator.create(name)).await? {
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
    State(_category_updater): State<CategoryUpdater>,
    Path(_id): Path<String>,
    Json(_request): Json<UpdateCategoryRequest>,
) -> Result<StatusCode, ApiError> {
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_category(
    State(_category_deleter): State<CategoryDeleter>,
    Path(_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    Ok(StatusCode::NO_CONTENT)
}

pub async fn set_category_members(
    State(_category_updater): State<CategoryUpdater>,
    Path(_id): Path<String>,
    Json(_request): Json<SetCategoryMembersRequest>,
) -> Result<StatusCode, ApiError> {
    Ok(StatusCode::NO_CONTENT)
}

pub async fn mark_category_watched(
    State(_video_watch_state_updater): State<VideoWatchStateUpdater>,
    Path(_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::category::{Category, CategoryId, CategoryMember};
    use crate::domain::channel::{Channel, ChannelHandle, VideoLimit};
    use crate::domain::playlist::{Playlist, PlaylistId, PlaylistKind, PlaylistName, PlaylistPath};
    use crate::domain::services::CategoryVideoCollector;
    use crate::domain::shared::{Quality, ReconcileInterval};
    use crate::infrastructure::repositories::sqlite_category_repository::{
        CategoryRepository, SqliteCategoryRepository,
    };
    use crate::infrastructure::repositories::sqlite_channel_repository::{
        ChannelRepository, SqliteChannelRepository,
    };
    use crate::infrastructure::repositories::sqlite_channel_video_repository::SqliteChannelVideoRepository;
    use crate::infrastructure::repositories::sqlite_playlist_repository::{
        PlaylistRepository, SqlitePlaylistRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::SqlitePlaylistVideoRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::SqliteVideoRepository;
    use crate::infrastructure::shared::sqlite_connection::{Database, TestDatabase};
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use dto::CategoryListItemResponse;
    use rusqlite::Connection;
    use std::sync::Arc;

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
    async fn it_should_fail_to_create_if_name_missing() {
        let response = create(any_category_creator(), CreateCategoryRequest { name: None }).await;

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

    fn any_category_creator() -> CategoryCreator {
        CategoryCreator::new(
            Arc::new(SqliteCategoryRepository::new(unused_connection())),
            Arc::new(FixedClock(fixed_timestamp())),
        )
    }

    fn unused_connection() -> Database {
        Database::single(Connection::open_in_memory().unwrap())
    }

    fn create_request(name: &str) -> CreateCategoryRequest {
        CreateCategoryRequest {
            name: Some(name.to_string()),
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
            channels: channels.iter().map(|c| c.to_string()).collect(),
            playlists: playlists.iter().map(|p| p.to_string()).collect(),
        }
    }

    /// A stored category without members, created at `fixed_timestamp()`.
    fn category(id: Option<CategoryId>, name: &str) -> Category {
        Category {
            id: id.unwrap_or_else(CategoryId::new_generated),
            name: CategoryName::new(name).unwrap(),
            members: vec![],
            created_at: fixed_timestamp(),
        }
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
            unwatched_count,
            channels: channels.iter().map(|c| c.to_string()).collect(),
            playlists: playlists.iter().map(|p| p.to_string()).collect(),
        }
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
        )
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
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
}
