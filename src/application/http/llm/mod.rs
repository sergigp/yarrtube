pub mod dto;

use super::blocking::run_blocking;
use super::error::ApiError;
use super::validation::required;
use crate::domain::llm_server::{LlmConnection, OllamaUrl, ReadLlmServerError};
use crate::domain::services::{LlmServerSearcher, LlmServerSearcherApi};
use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use dto::{AnthropicResponse, OllamaQuery, OllamaResponse};

const MISSING_OLLAMA_URL: &str = "Ollama URL must be stated (missing)";

pub async fn get_ollama(
    State(llm_server_searcher): State<LlmServerSearcher>,
    Query(query): Query<OllamaQuery>,
) -> Result<Json<OllamaResponse>, ApiError> {
    let url = OllamaUrl::new(required(query.url, MISSING_OLLAMA_URL)?)?;
    let server = run_blocking(move || llm_server_searcher.get(LlmConnection::Ollama(url)))
        .await?
        .map_err(read_llm_server_error)?;
    Ok(Json(OllamaResponse::from(server)))
}

pub async fn get_anthropic(
    State(llm_server_searcher): State<LlmServerSearcher>,
) -> Result<Json<AnthropicResponse>, ApiError> {
    let server = run_blocking(move || llm_server_searcher.get(LlmConnection::Anthropic))
        .await?
        .map_err(read_llm_server_error)?;
    Ok(Json(AnthropicResponse::from(server)))
}

fn read_llm_server_error(error: ReadLlmServerError) -> ApiError {
    match error {
        e @ ReadLlmServerError::NotConfigured => ApiError::new(StatusCode::NOT_FOUND, e),
        e @ ReadLlmServerError::Unreachable { .. } => ApiError::new(StatusCode::BAD_GATEWAY, e),
    }
}

#[cfg(test)]
mod tests;
