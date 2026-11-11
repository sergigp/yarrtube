pub mod dto;

use super::blocking::run_blocking;
use super::error::ApiError;
use super::validation::required;
use super::videos::{edit_video_categories_error, update_watch_state_error};
use crate::domain::category::{
    CategoryColor, CategoryId, CategoryMember, CategoryName, CategoryPrompt, CreateCategoryError,
    DeleteCategoryError, UpdateCategoryError,
};
use crate::domain::channel::ChannelHandle;
use crate::domain::playlist::PlaylistId;
use crate::domain::services::{
    CategoryCreator, CategoryCreatorApi, CategoryDeleter, CategoryDeleterApi, CategoryUpdate,
    CategoryUpdater, CategoryUpdaterApi, CategoryViewSearcher, CategoryViewSearcherApi,
    NewCategory, VideoCategoryEditor, VideoCategoryEditorApi, VideoWatchStateUpdater,
    VideoWatchStateUpdaterApi,
};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use dto::{
    CategoryListResponse, CategoryResponse, CreateCategoryRequest, SetCategoryMembersRequest,
    UpdateCategoryRequest,
};

const MISSING_CATEGORY_NAME: &str = "Category name must not be empty (missing)";
const NOTHING_TO_UPDATE: &str = "Category name, color or AI prompt is required (all missing)";

pub async fn create_category(
    State(category_creator): State<CategoryCreator>,
    Json(request): Json<CreateCategoryRequest>,
) -> Result<(StatusCode, Json<CategoryResponse>), ApiError> {
    let name = CategoryName::new(required(request.name, MISSING_CATEGORY_NAME)?)?;
    let color = request.color.map(CategoryColor::new).transpose()?;
    let category = NewCategory {
        name,
        color,
        ai_prompt: request
            .ai_prompt
            .map(CategoryPrompt::new)
            .transpose()?
            .flatten(),
        members: members_from(request.channels, request.playlists)?,
    };

    match run_blocking(move || category_creator.create(category)).await? {
        Ok(category) => Ok((StatusCode::CREATED, Json(CategoryResponse::from(category)))),
        Err(e @ CreateCategoryError::NameTaken(_)) => Err(ApiError::new(StatusCode::CONFLICT, e)),
        Err(e @ CreateCategoryError::MemberNotTracked(_)) => Err(ApiError::bad_request(e)),
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
        request
            .name
            .as_ref()
            .or(request.color.as_ref())
            .or(request.ai_prompt.as_ref()),
        NOTHING_TO_UPDATE,
    )?;
    let update = CategoryUpdate {
        name: request.name.map(CategoryName::new).transpose()?,
        color: request.color.map(CategoryColor::new).transpose()?,
        ai_prompt: request.ai_prompt.map(CategoryPrompt::new).transpose()?,
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
    let members = members_from(request.channels, request.playlists)?;

    run_blocking(move || category_updater.set_members(&id, members))
        .await?
        .map_err(update_category_error)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_category_ai_verdicts(
    State(video_category_editor): State<VideoCategoryEditor>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = CategoryId::new(id)?;

    run_blocking(move || video_category_editor.rerun_category(&id))
        .await?
        .map_err(edit_video_categories_error)?;

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
fn members_from(
    channels: Vec<String>,
    playlists: Vec<String>,
) -> Result<Vec<CategoryMember>, ApiError> {
    let channels = channels
        .into_iter()
        .map(|handle| ChannelHandle::new(handle).map(CategoryMember::Channel));
    let playlists = playlists
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
mod tests;
