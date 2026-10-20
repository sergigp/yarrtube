pub mod dto;

use super::error::ApiError;
use crate::domain::services::{
    CategoryCreator, CategoryDeleter, CategoryUpdater, CategoryViewSearcher, VideoWatchStateUpdater,
};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use dto::{
    CategoryListResponse, CategoryResponse, CreateCategoryRequest, SetCategoryMembersRequest,
    UpdateCategoryRequest,
};

pub async fn create_category(
    State(_category_creator): State<CategoryCreator>,
    Json(_request): Json<CreateCategoryRequest>,
) -> Result<(StatusCode, Json<CategoryResponse>), ApiError> {
    Ok((
        StatusCode::CREATED,
        Json(CategoryResponse {
            id: String::new(),
            name: String::new(),
            channels: vec![],
            playlists: vec![],
        }),
    ))
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
