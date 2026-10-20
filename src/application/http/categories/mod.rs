pub mod dto;

use super::blocking::run_blocking;
use super::error::ApiError;
use super::validation::required;
use crate::domain::category::{CategoryName, CreateCategoryError};
use crate::domain::services::{
    CategoryCreator, CategoryCreatorApi, CategoryDeleter, CategoryUpdater, CategoryViewSearcher,
    VideoWatchStateUpdater,
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
    State(_category_view_searcher): State<CategoryViewSearcher>,
) -> Result<Json<CategoryListResponse>, ApiError> {
    Ok(Json(CategoryListResponse {
        categories: vec![],
        unwatched_total: 0,
    }))
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
    use crate::domain::category::{Category, CategoryId};
    use crate::infrastructure::repositories::sqlite_category_repository::{
        CategoryRepository, SqliteCategoryRepository,
    };
    use crate::infrastructure::shared::sqlite_connection::{Database, TestDatabase};
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
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

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
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
