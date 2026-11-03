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
mod tests {
    use super::*;
    use crate::domain::llm_server::{LlmModel, LlmServer};
    use crate::infrastructure::repositories::ollama_llm_server_repository::FakeLlmServerRepository;
    use dto::OllamaModelResponse;
    use std::sync::Arc;

    #[tokio::test]
    async fn it_should_return_the_server_version_and_models_sorted_by_name() {
        let repository = Arc::new(FakeLlmServerRepository::new(LlmServer {
            version: "0.40.13".to_string(),
            models: vec![
                LlmModel {
                    name: "qwen3:4b".to_string(),
                    size_bytes: 2_497_293_918,
                    thinking: true,
                },
                LlmModel {
                    name: "gemma3:4b".to_string(),
                    size_bytes: 3_338_801_804,
                    thinking: false,
                },
            ],
        }));
        let llm_server_searcher = LlmServerSearcher::new(repository);
        let query = ollama_query();

        let response = get(llm_server_searcher, query).await;

        assert_eq!(
            response,
            Ok(OllamaResponse {
                version: "0.40.13".to_string(),
                models: vec![
                    OllamaModelResponse {
                        name: "gemma3:4b".to_string(),
                        size_bytes: 3_338_801_804,
                        thinking: false,
                    },
                    OllamaModelResponse {
                        name: "qwen3:4b".to_string(),
                        size_bytes: 2_497_293_918,
                        thinking: true,
                    },
                ],
            })
        );
    }

    #[tokio::test]
    async fn it_should_return_no_models_if_none_installed() {
        let repository = Arc::new(FakeLlmServerRepository::new(LlmServer {
            version: "0.40.13".to_string(),
            models: vec![],
        }));
        let llm_server_searcher = LlmServerSearcher::new(repository);
        let query = ollama_query();

        let response = get(llm_server_searcher, query).await;

        assert_eq!(
            response,
            Ok(OllamaResponse {
                version: "0.40.13".to_string(),
                models: vec![],
            })
        );
    }

    #[tokio::test]
    async fn it_should_fail_if_the_url_is_missing() {
        let response = get(any_llm_server_searcher(), OllamaQuery::default()).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request("Ollama URL must be stated (missing)"))
        );
    }

    #[tokio::test]
    async fn it_should_fail_if_the_url_is_not_http() {
        let query = OllamaQuery {
            url: Some("mac.local:11434".to_string()),
        };

        let response = get(any_llm_server_searcher(), query).await;

        assert_eq!(
            response,
            Err(ApiError::bad_request(
                "Ollama URL must be an http(s) URL (got \"mac.local:11434\")"
            ))
        );
    }

    #[tokio::test]
    async fn it_should_fail_if_ollama_is_unreachable() {
        let llm_server_searcher =
            LlmServerSearcher::new(Arc::new(FakeLlmServerRepository::failing()));
        let query = ollama_query();

        let response = get(llm_server_searcher, query).await;

        assert_eq!(
            response,
            Err(ApiError::new(
                StatusCode::BAD_GATEWAY,
                "Could not reach Ollama at http://mac.local:11434: connection refused"
            ))
        );
    }

    async fn get(
        llm_server_searcher: LlmServerSearcher,
        query: OllamaQuery,
    ) -> Result<OllamaResponse, ApiError> {
        get_ollama(State(llm_server_searcher), Query(query))
            .await
            .map(|Json(response)| response)
    }

    /// A searcher for tests whose request is rejected before reaching it: its
    /// server is down, so a request that wrongly got through would fail with
    /// another error instead of passing.
    fn any_llm_server_searcher() -> LlmServerSearcher {
        LlmServerSearcher::new(Arc::new(FakeLlmServerRepository::failing()))
    }

    fn ollama_query() -> OllamaQuery {
        OllamaQuery {
            url: Some("http://mac.local:11434".to_string()),
        }
    }
}
