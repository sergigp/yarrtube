pub mod dto;

use super::blocking::run_blocking;
use super::error::ApiError;
use super::validation::required;
use crate::domain::llm_server::{OllamaUrl, ReadLlmServerError};
use crate::domain::services::{LlmServerSearcher, LlmServerSearcherApi};
use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use dto::{OllamaQuery, OllamaResponse};

const MISSING_OLLAMA_URL: &str = "Ollama URL must be stated (missing)";

pub async fn get_ollama(
    State(llm_server_searcher): State<LlmServerSearcher>,
    Query(query): Query<OllamaQuery>,
) -> Result<Json<OllamaResponse>, ApiError> {
    let url = OllamaUrl::new(required(query.url, MISSING_OLLAMA_URL)?)?;
    let server = run_blocking(move || llm_server_searcher.get(url))
        .await?
        .map_err(|e @ ReadLlmServerError::Unreachable { .. }| {
            ApiError::new(StatusCode::BAD_GATEWAY, e)
        })?;
    Ok(Json(OllamaResponse::from(server)))
}

#[cfg(test)]
mod tests;
