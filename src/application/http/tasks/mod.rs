pub mod dto;

use super::blocking::run_blocking;
use super::error::ApiError;
use crate::domain::services::{
    TaskRunNowRequester, TaskRunNowRequesterApi, TaskViewSearcher, TaskViewSearcherApi,
};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use dto::{RunTasksNowRequest, TaskResponse};

pub async fn list_tasks(
    State(task_view_searcher): State<TaskViewSearcher>,
) -> Result<Json<Vec<TaskResponse>>, ApiError> {
    let views = run_blocking(move || task_view_searcher.search_all_pending())
        .await?
        .map_err(ApiError::internal)?;

    Ok(Json(views.into_iter().map(TaskResponse::from).collect()))
}

pub async fn run_tasks_now(
    State(task_run_now_requester): State<TaskRunNowRequester>,
    Json(request): Json<RunTasksNowRequest>,
) -> Result<StatusCode, ApiError> {
    let ids = request.ids.unwrap_or_default();

    run_blocking(move || task_run_now_requester.run_now(&ids))
        .await?
        .map_err(ApiError::internal)?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests;
