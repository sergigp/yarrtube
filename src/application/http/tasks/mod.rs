pub mod dto;

use super::blocking::run_blocking;
use super::error::ApiError;
use crate::domain::services::{TaskViewSearcher, TaskViewSearcherApi};
use axum::Json;
use axum::extract::State;
use dto::TaskResponse;

pub async fn list_tasks(
    State(task_view_searcher): State<TaskViewSearcher>,
) -> Result<Json<Vec<TaskResponse>>, ApiError> {
    let views = run_blocking(move || task_view_searcher.search_all_pending())
        .await?
        .map_err(ApiError::internal)?;

    Ok(Json(views.into_iter().map(TaskResponse::from).collect()))
}

#[cfg(test)]
mod tests;
